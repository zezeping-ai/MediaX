use super::renderer_types::RenderStageTimings;
use super::{QueuedFrame, Renderer, VideoScaleMode};
use std::time::Instant;

impl Renderer {
    pub(super) fn render(
        &mut self,
        frame: Option<&QueuedFrame>,
        force_if_idle: bool,
    ) -> Result<RenderStageTimings, String> {
        let resized = self.resize_if_needed();
        if frame.is_none() && !resized && !force_if_idle {
            return Ok(RenderStageTimings::default());
        }
        let upload_started_at = Instant::now();
        if let Some(frame) = frame {
            self.upload_frame(frame);
        }
        let upload_frame = upload_started_at.elapsed();
        let acquire_started_at = Instant::now();
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(RenderStageTimings {
                    upload_frame,
                    acquire_surface: acquire_started_at.elapsed(),
                    ..RenderStageTimings::default()
                });
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(RenderStageTimings {
                    upload_frame,
                    acquire_surface: acquire_started_at.elapsed(),
                    ..RenderStageTimings::default()
                });
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("wgpu surface validation error".to_string());
            }
        };
        let acquire_surface = acquire_started_at.elapsed();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encode_and_submit_started_at = Instant::now();
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
        let encode_and_submit = encode_and_submit_started_at.elapsed();
        let present_started_at = Instant::now();
        output.present();
        Ok(RenderStageTimings {
            upload_frame,
            acquire_surface,
            encode_and_submit,
            present: present_started_at.elapsed(),
        })
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
