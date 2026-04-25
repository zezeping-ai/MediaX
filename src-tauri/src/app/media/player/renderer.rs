use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use std::collections::VecDeque;

use tauri::Manager;

const FRAME_QUEUE_CAPACITY: usize = 6;

pub struct VideoFrame {
    pub pts_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub y_plane: Vec<u8>,
    pub uv_plane: Vec<u8>,
    pub color_matrix: [[f32; 3]; 3],
    pub y_offset: f32,
    pub y_scale: f32,
    pub uv_offset: f32,
    pub uv_scale: f32,
}

#[derive(Clone)]
pub struct RendererState {
    inner: Arc<RendererInner>,
}

#[derive(Clone, Copy, Default)]
pub struct RendererMetricsSnapshot {
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub last_render_cost_ms: f64,
    pub last_present_lag_ms: f64,
}

struct RendererInner {
    stop: AtomicBool,
    renderer: Mutex<Option<Renderer>>,
    queued_frames: Mutex<VecDeque<VideoFrame>>,
    last_queued_pts: Mutex<Option<f64>>,
    clock: Mutex<ClockState>,
    render_task_in_flight: AtomicBool,
    pending_render: Mutex<bool>,
    render_cv: Condvar,
    video_scale_mode: AtomicU8,
    last_render_cost_micros: AtomicU64,
    last_present_lag_ms_bits: AtomicU32,
}

#[derive(Clone, Copy)]
struct ClockState {
    anchor_instant: Instant,
    anchor_media_seconds: f64,
    rate: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum VideoScaleMode {
    Contain,
    Cover,
}

impl VideoScaleMode {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Cover,
            _ => Self::Contain,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            Self::Contain => 0,
            Self::Cover => 1,
        }
    }
}

impl TryFrom<&str> for VideoScaleMode {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "contain" => Ok(Self::Contain),
            "cover" => Ok(Self::Cover),
            other => Err(format!("unsupported video scale mode: {other}")),
        }
    }
}

impl RendererState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            inner: Arc::new(RendererInner {
                stop: AtomicBool::new(false),
                renderer: Mutex::new(None),
                queued_frames: Mutex::new(VecDeque::with_capacity(16)),
                last_queued_pts: Mutex::new(None),
                clock: Mutex::new(ClockState {
                    anchor_instant: now,
                    anchor_media_seconds: 0.0,
                    rate: 1.0,
                }),
                render_task_in_flight: AtomicBool::new(false),
                pending_render: Mutex::new(false),
                render_cv: Condvar::new(),
                video_scale_mode: AtomicU8::new(VideoScaleMode::Contain.as_u8()),
                last_render_cost_micros: AtomicU64::new(0),
                last_present_lag_ms_bits: AtomicU32::new(0f32.to_bits()),
            }),
        }
    }

    pub fn set_video_scale_mode(&self, mode: VideoScaleMode) {
        self.inner
            .video_scale_mode
            .store(mode.as_u8(), Ordering::Relaxed);
    }

    /// Start the renderer loop.
    pub fn start_render_loop(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "main window not found".to_string())?;

        let renderer = pollster::block_on(Renderer::new(window))?;
        {
            let mut guard = self
                .inner
                .renderer
                .lock()
                .map_err(|_| "renderer state poisoned".to_string())?;
            *guard = Some(renderer);
        }
        let _ = app.run_on_main_thread({
            let inner = self.inner.clone();
            move || {
                if let Ok(mut guard) = inner.renderer.lock() {
                    if let Some(renderer) = guard.as_mut() {
                        let _ = renderer.render(None, true);
                    }
                }
            }
        });

        let app_handle = app.clone();
        let inner = self.inner.clone();
        thread::spawn(move || {
            // Present continuously to align cadence with display vsync. When no new video
            // frame is due, we'll still present the previously uploaded texture, which
            // avoids "slideshow" stalls on 60/120Hz displays.
            let idle_tick = Duration::from_millis(16);
            while !inner.stop.load(Ordering::Relaxed) {
                let _timed_out = wait_for_render_signal(&inner, idle_tick);
                // Prevent enqueueing an unbounded number of main-thread render tasks
                // when the UI thread is busy (which can cause macOS beachballing).
                if inner
                    .render_task_in_flight
                    .swap(true, Ordering::AcqRel)
                {
                    continue;
                }
                let _ = app_handle.run_on_main_thread({
                    let inner = inner.clone();
                    move || {
                        let now_media_seconds = {
                            let clock = match inner.clock.lock() {
                                Ok(guard) => *guard,
                                Err(_) => ClockState {
                                    anchor_instant: Instant::now(),
                                    anchor_media_seconds: 0.0,
                                    rate: 1.0,
                                },
                            };
                            let elapsed = Instant::now().saturating_duration_since(clock.anchor_instant);
                            clock.anchor_media_seconds + elapsed.as_secs_f64() * clock.rate.max(0.25)
                        };
                        let frame = pick_frame_for_present(&inner, now_media_seconds);
                        if let Ok(mut guard) = inner.renderer.lock() {
                            if let Some(renderer) = guard.as_mut() {
                                let render_start = Instant::now();
                                renderer.set_video_scale_mode(VideoScaleMode::from_u8(
                                    inner.video_scale_mode.load(Ordering::Relaxed),
                                ));
                                // Always present each tick; upload only when we have a new frame.
                                let _ = renderer.render(frame.as_ref(), true);
                                let elapsed = render_start.elapsed();
                                inner.last_render_cost_micros.store(
                                    u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX),
                                    Ordering::Relaxed,
                                );
                                let lag_ms = frame
                                    .as_ref()
                                    .map(|f| ((now_media_seconds - f.pts_seconds).max(0.0) * 1000.0) as f32)
                                    .unwrap_or(0.0);
                                inner
                                    .last_present_lag_ms_bits
                                    .store(lag_ms.to_bits(), Ordering::Relaxed);
                            }
                        }
                        inner.render_task_in_flight.store(false, Ordering::Release);
                    }
                });
            }
        });

        Ok(())
    }

    pub fn update_clock(&self, media_seconds: f64, playback_rate: f64) {
        if let Ok(mut guard) = self.inner.clock.lock() {
            *guard = ClockState {
                anchor_instant: Instant::now(),
                anchor_media_seconds: media_seconds.max(0.0),
                rate: if playback_rate.is_finite() && playback_rate > 0.0 {
                    playback_rate.min(3.0)
                } else {
                    1.0
                },
            };
        }
    }

    pub fn reset_timeline(&self, media_seconds: f64, playback_rate: f64) {
        if let Ok(mut queue) = self.inner.queued_frames.lock() {
            queue.clear();
        }
        if let Ok(mut last_pts) = self.inner.last_queued_pts.lock() {
            *last_pts = None;
        }
        self.update_clock(media_seconds, playback_rate);
        if let Ok(mut pending) = self.inner.pending_render.lock() {
            *pending = true;
            self.inner.render_cv.notify_one();
        }
    }

    pub fn can_accept_frame(&self) -> bool {
        self.inner
            .queued_frames
            .lock()
            .ok()
            .map(|q| q.len() < FRAME_QUEUE_CAPACITY)
            .unwrap_or(true)
    }

    pub fn queue_depth(&self) -> usize {
        self.inner
            .queued_frames
            .lock()
            .ok()
            .map(|q| q.len())
            .unwrap_or(0)
    }

    pub fn submit_frame(&self, frame: VideoFrame) {
        if let Ok(mut pts_guard) = self.inner.last_queued_pts.lock() {
            // Drop non-monotonic frames to avoid queue churn after timestamp discontinuities.
            if let Some(last) = *pts_guard {
                if frame.pts_seconds.is_finite() && frame.pts_seconds + 0.001 < last {
                    return;
                }
            }
            if frame.pts_seconds.is_finite() {
                *pts_guard = Some(frame.pts_seconds);
            }
        }
        if let Ok(mut queue) = self.inner.queued_frames.lock() {
            // Low-latency / no-stutter policy: keep only a tiny queue and
            // drop aggressively when producer outruns presentation.
            while queue.len() >= FRAME_QUEUE_CAPACITY {
                queue.pop_front();
            }
            queue.push_back(frame);
        }
        if let Ok(mut pending) = self.inner.pending_render.lock() {
            *pending = true;
            self.inner.render_cv.notify_one();
        }
    }

    pub fn metrics_snapshot(&self) -> RendererMetricsSnapshot {
        RendererMetricsSnapshot {
            queue_depth: self.queue_depth(),
            queue_capacity: FRAME_QUEUE_CAPACITY,
            last_render_cost_ms: (self.inner.last_render_cost_micros.load(Ordering::Relaxed) as f64)
                / 1000.0,
            last_present_lag_ms: f32::from_bits(
                self.inner.last_present_lag_ms_bits.load(Ordering::Relaxed),
            ) as f64,
        }
    }
}

fn pick_frame_for_present(inner: &RendererInner, now_media_seconds: f64) -> Option<VideoFrame> {
    let present_lead = 0.010; // 10ms lead allows small jitter without presenting too early.
    let deadline = now_media_seconds + present_lead;
    let mut queue = inner.queued_frames.lock().ok()?;
    let Some(front) = queue.front() else {
        return None;
    };
    let pts = front.pts_seconds;
    if !pts.is_finite() || pts <= deadline {
        return queue.pop_front();
    }
    None
}

struct Renderer {
    window: tauri::WebviewWindow,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    max_surface_extent: u32,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    color_params_buffer: wgpu::Buffer,
    texture_y: wgpu::Texture,
    texture_y_view: wgpu::TextureView,
    texture_uv: wgpu::Texture,
    texture_uv_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    texture_size: (u32, u32),
    has_uploaded_frame: bool,
    video_scale_mode: VideoScaleMode,
}

struct ColorParams {
    y_offset: f32,
    y_scale: f32,
    uv_offset: f32,
    uv_scale: f32,
    row0: [f32; 4],
    row1: [f32; 4],
    row2: [f32; 4],
}

impl Renderer {
    async fn new(window: tauri::WebviewWindow) -> Result<Self, String> {
        // NOTE: On macOS/Metal, surface creation must happen on the main thread.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|err| format!("create surface failed: {err}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|err| format!("request adapter failed: {err}"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("mediax-renderer-device"),
                required_features: wgpu::Features::empty(),
                required_limits: adapter.limits(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
            })
            .await
            .map_err(|err| format!("request device failed: {err}"))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let mut size = window
            .inner_size()
            .map_err(|err| format!("get inner size failed: {err}"))?;
        let max_surface_extent = device.limits().max_texture_dimension_2d;
        sanitize_surface_size(&mut size, max_surface_extent);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: select_present_mode(&surface_caps.present_modes),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mediax-video-shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 3.0, -1.0),
    vec2<f32>(-1.0,  3.0)
  );
  var uvs = array<vec2<f32>, 3>(
    vec2<f32>(0.0, 1.0),
    vec2<f32>(2.0, 1.0),
    vec2<f32>(0.0, -1.0)
  );
  var out: VsOut;
  out.pos = vec4<f32>(positions[idx], 0.0, 1.0);
  out.uv = uvs[idx];
  return out;
}

@group(0) @binding(0) var tex_y: texture_2d<f32>;
@group(0) @binding(1) var tex_uv: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
struct ColorParams {
  y_offset: f32,
  y_scale: f32,
  uv_offset: f32,
  uv_scale: f32,
  row0: vec4<f32>,
  row1: vec4<f32>,
  row2: vec4<f32>,
};
@group(0) @binding(3) var<uniform> color_params: ColorParams;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let y = (textureSample(tex_y, samp, in.uv).r - color_params.y_offset) * color_params.y_scale;
  let uv = textureSample(tex_uv, samp, in.uv).rg;
  let u = (uv.x - color_params.uv_offset) * color_params.uv_scale;
  let v = (uv.y - color_params.uv_offset) * color_params.uv_scale;
  let yuv = vec3<f32>(y, u, v);
  let r = dot(color_params.row0.xyz, yuv);
  let g = dot(color_params.row1.xyz, yuv);
  let b = dot(color_params.row2.xyz, yuv);
  return vec4<f32>(r, g, b, 1.0);
}
"#
                .into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mediax-video-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mediax-video-pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mediax-video-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mediax-video-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let color_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mediax-color-params"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (texture_y, texture_y_view) = create_plane_texture(
            &device,
            "mediax-video-y",
            1,
            1,
            wgpu::TextureFormat::R8Unorm,
        );
        let (texture_uv, texture_uv_view) = create_plane_texture(
            &device,
            "mediax-video-uv",
            1,
            1,
            wgpu::TextureFormat::Rg8Unorm,
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mediax-video-bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_y_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_uv_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: color_params_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            max_surface_extent,
            pipeline,
            bind_group_layout,
            sampler,
            color_params_buffer,
            texture_y,
            texture_y_view,
            texture_uv,
            texture_uv_view,
            bind_group,
            texture_size: (1, 1),
            has_uploaded_frame: false,
            video_scale_mode: VideoScaleMode::Contain,
        })
    }

    fn set_video_scale_mode(&mut self, mode: VideoScaleMode) {
        self.video_scale_mode = mode;
    }

    fn resize_if_needed(&mut self) -> bool {
        // Resize handling (polling for Milestone 0 simplicity).
        if let Ok(mut next) = self.window.inner_size() {
            sanitize_surface_size(&mut next, self.max_surface_extent);
            if next.width > 0
                && next.height > 0
                && (next.width != self.config.width || next.height != self.config.height)
            {
                self.config.width = next.width;
                self.config.height = next.height;
                self.surface.configure(&self.device, &self.config);
                return true;
            }
        }
        false
    }

    fn ensure_texture(&mut self, width: u32, height: u32) {
        if self.texture_size == (width, height) {
            return;
        }
        let (texture_y, texture_y_view) = create_plane_texture(
            &self.device,
            "mediax-video-y",
            width,
            height,
            wgpu::TextureFormat::R8Unorm,
        );
        let (texture_uv, texture_uv_view) = create_plane_texture(
            &self.device,
            "mediax-video-uv",
            (width / 2).max(1),
            (height / 2).max(1),
            wgpu::TextureFormat::Rg8Unorm,
        );
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mediax-video-bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_y_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_uv_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.color_params_buffer.as_entire_binding(),
                },
            ],
        });
        self.texture_y = texture_y;
        self.texture_y_view = texture_y_view;
        self.texture_uv = texture_uv;
        self.texture_uv_view = texture_uv_view;
        self.bind_group = bind_group;
        self.texture_size = (width, height);
    }

    fn upload_frame(&mut self, frame: &VideoFrame) {
        self.ensure_texture(frame.width.max(1), frame.height.max(1));

        // Y plane
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture_y,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.y_plane,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(frame.width),
                rows_per_image: Some(frame.height),
            },
            wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
        );

        // UV plane (RG8), width/2 x height/2, 2 bytes per pixel.
        let uv_width = (frame.width / 2).max(1);
        let uv_height = (frame.height / 2).max(1);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture_uv,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.uv_plane,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(uv_width * 2),
                rows_per_image: Some(uv_height),
            },
            wgpu::Extent3d {
                width: uv_width,
                height: uv_height,
                depth_or_array_layers: 1,
            },
        );
        self.upload_color_params(frame);
        self.has_uploaded_frame = true;
    }

    fn upload_color_params(&self, frame: &VideoFrame) {
        let params = ColorParams {
            y_offset: frame.y_offset,
            y_scale: frame.y_scale,
            uv_offset: frame.uv_offset,
            uv_scale: frame.uv_scale,
            row0: [
                frame.color_matrix[0][0],
                frame.color_matrix[0][1],
                frame.color_matrix[0][2],
                0.0,
            ],
            row1: [
                frame.color_matrix[1][0],
                frame.color_matrix[1][1],
                frame.color_matrix[1][2],
                0.0,
            ],
            row2: [
                frame.color_matrix[2][0],
                frame.color_matrix[2][1],
                frame.color_matrix[2][2],
                0.0,
            ],
        };
        let mut bytes = Vec::with_capacity(64);
        push_f32(&mut bytes, params.y_offset);
        push_f32(&mut bytes, params.y_scale);
        push_f32(&mut bytes, params.uv_offset);
        push_f32(&mut bytes, params.uv_scale);
        for value in params
            .row0
            .into_iter()
            .chain(params.row1)
            .chain(params.row2)
        {
            push_f32(&mut bytes, value);
        }
        self.queue
            .write_buffer(&self.color_params_buffer, 0, &bytes);
    }

    fn render(&mut self, frame: Option<&VideoFrame>, force_if_idle: bool) -> Result<(), String> {
        let resized = self.resize_if_needed();
        if frame.is_none() && !resized && !force_if_idle {
            return Ok(());
        }
        if let Some(frame) = frame {
            self.upload_frame(frame);
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("wgpu surface validation error".to_string())
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mediax-renderer-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mediax-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            if self.has_uploaded_frame {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                let (x, y, width, height) = self.compute_content_viewport();
                pass.set_viewport(x, y, width, height, 0.0, 1.0);
                pass.draw(0..3, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
        output.present();
        Ok(())
    }

    fn compute_content_viewport(&self) -> (f32, f32, f32, f32) {
        let surface_w = self.config.width.max(1) as f32;
        let surface_h = self.config.height.max(1) as f32;
        let video_w = self.texture_size.0.max(1) as f32;
        let video_h = self.texture_size.1.max(1) as f32;
        let scale_x = surface_w / video_w;
        let scale_y = surface_h / video_h;
        let scale = match self.video_scale_mode {
            VideoScaleMode::Contain => scale_x.min(scale_y),
            VideoScaleMode::Cover => scale_x.max(scale_y),
        };
        let width = (video_w * scale).max(1.0);
        let height = (video_h * scale).max(1.0);
        let x = ((surface_w - width) / 2.0).round();
        let y = ((surface_h - height) / 2.0).round();
        (x, y, width, height)
    }
}

fn wait_for_render_signal(inner: &RendererInner, timeout: Duration) -> bool {
    let mut pending = match inner.pending_render.lock() {
        Ok(guard) => guard,
        Err(_) => return true,
    };
    if *pending {
        *pending = false;
        return false;
    }
    let result = inner.render_cv.wait_timeout(pending, timeout);
    let (mut pending, wait_result) = match result {
        Ok(tuple) => tuple,
        Err(_) => return true,
    };
    if *pending {
        *pending = false;
        false
    } else {
        wait_result.timed_out()
    }
}

fn select_present_mode(candidates: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    if candidates.contains(&wgpu::PresentMode::Fifo) {
        return wgpu::PresentMode::Fifo;
    }
    if candidates.contains(&wgpu::PresentMode::AutoVsync) {
        return wgpu::PresentMode::AutoVsync;
    }
    candidates
        .first()
        .copied()
        .unwrap_or(wgpu::PresentMode::Fifo)
}

fn create_plane_texture(
    device: &wgpu::Device,
    label: &'static str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn sanitize_surface_size(size: &mut tauri::PhysicalSize<u32>, max_extent: u32) {
    size.width = size.width.max(1).min(max_extent.max(1));
    size.height = size.height.max(1).min(max_extent.max(1));
}

fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_ne_bytes());
}
