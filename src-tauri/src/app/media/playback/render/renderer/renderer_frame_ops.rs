use super::{ColorParams, QueuedFrame, Renderer, VideoFrame, VideoFramePlanes, VideoScaleMode};
use crate::app::media::playback::render::picture_tune::{picture_tune_shader_values, VideoPictureTune};
use crate::app::media::playback::render::renderer::helpers::{
    create_plane_texture, sanitize_surface_size,
};
use ffmpeg_next::format;

struct FrameColorUniformInput {
    y_offset: f32,
    y_scale: f32,
    uv_offset: f32,
    uv_scale: f32,
    row0: [f32; 4],
    row1_xyz: [f32; 3],
    row2_xyz: [f32; 3],
}

impl Renderer {
    pub(super) fn set_video_scale_mode(&mut self, mode: VideoScaleMode) {
        self.video_scale_mode = mode;
    }

    pub(super) fn set_picture_tune(&mut self, tune: VideoPictureTune) {
        let tune = tune.clamped();
        self.picture_brightness = tune.brightness;
        self.picture_contrast = tune.contrast;
        self.picture_saturation = tune.saturation;
        self.picture_gamma = tune.gamma;
        self.picture_hue = tune.hue;
    }

    fn current_picture_tune(&self) -> VideoPictureTune {
        VideoPictureTune {
            brightness: self.picture_brightness,
            contrast: self.picture_contrast,
            saturation: self.picture_saturation,
            gamma: self.picture_gamma,
            hue: self.picture_hue,
        }
    }

    pub(super) fn set_backdrop_color(&mut self, color: wgpu::Color) {
        self.backdrop_color = color;
    }

    pub(super) fn clear_uploaded_frame(&mut self) {
        self.has_uploaded_frame = false;
    }

    pub(super) fn resize_if_needed(&mut self) -> bool {
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

    pub(super) fn ensure_texture(&mut self, width: u32, height: u32) {
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
        let (texture_y_16, texture_y_16_view) = create_plane_texture(
            &self.device,
            "mediax-video-y16",
            width,
            height,
            wgpu::TextureFormat::R16Unorm,
        );
        let chroma_width = (width / 2).max(1);
        let chroma_height = (height / 2).max(1);
        let (texture_u, texture_u_view) = create_plane_texture(
            &self.device,
            "mediax-video-u",
            chroma_width,
            chroma_height,
            wgpu::TextureFormat::R8Unorm,
        );
        let (texture_v, texture_v_view) = create_plane_texture(
            &self.device,
            "mediax-video-v",
            chroma_width,
            chroma_height,
            wgpu::TextureFormat::R8Unorm,
        );
        let (texture_uv, texture_uv_view) = create_plane_texture(
            &self.device,
            "mediax-video-uv",
            chroma_width,
            chroma_height,
            wgpu::TextureFormat::Rg8Unorm,
        );
        let (texture_uv_16, texture_uv_16_view) = create_plane_texture(
            &self.device,
            "mediax-video-uv16",
            chroma_width,
            chroma_height,
            wgpu::TextureFormat::Rg16Unorm,
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
                    resource: wgpu::BindingResource::TextureView(&texture_u_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&texture_v_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&texture_uv_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&texture_y_16_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&texture_uv_16_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: self.color_params_buffer.as_entire_binding(),
                },
            ],
        });
        self.texture_y = texture_y;
        self.texture_y_view = texture_y_view;
        self.texture_y_16 = texture_y_16;
        self.texture_y_16_view = texture_y_16_view;
        self.texture_u = texture_u;
        self.texture_u_view = texture_u_view;
        self.texture_v = texture_v;
        self.texture_v_view = texture_v_view;
        self.texture_uv = texture_uv;
        self.texture_uv_view = texture_uv_view;
        self.texture_uv_16 = texture_uv_16;
        self.texture_uv_16_view = texture_uv_16_view;
        self.bind_group = bind_group;
        self.texture_size = (width, height);
    }

    pub(super) fn upload_frame(&mut self, frame: &QueuedFrame) {
        match frame {
            QueuedFrame::Prepared(frame) => self.upload_prepared_frame(frame),
            QueuedFrame::Decoded(frame) => self.upload_decoded_frame(frame),
        }
    }

    fn upload_prepared_frame(&mut self, frame: &VideoFrame) {
        self.ensure_texture(frame.width.max(1), frame.height.max(1));
        let uv_width = (frame.width / 2).max(1);
        let uv_height = (frame.height / 2).max(1);
        match &frame.planes {
            VideoFramePlanes::Nv12 { y_plane, uv_plane } => {
                upload_r8_plane(
                    &self.queue,
                    &self.texture_y,
                    y_plane,
                    frame.plane_strides[0].max(frame.width),
                    frame.width,
                    frame.height,
                );
                upload_rg8_plane(
                    &self.queue,
                    &self.texture_uv,
                    uv_plane,
                    frame.plane_strides[1].max(uv_width * 2),
                    uv_width,
                    uv_height,
                );
            }
        }
        self.upload_color_params(frame);
        self.has_uploaded_frame = true;
    }

    fn upload_decoded_frame(&mut self, frame: &super::DecodedVideoFrame) {
        let width = frame.frame.width().max(1);
        let height = frame.frame.height().max(1);
        self.ensure_texture(width, height);
        let uv_width = (width / 2).max(1);
        let uv_height = (height / 2).max(1);
        match frame.frame.format() {
            format::pixel::Pixel::NV12 => {
                upload_r8_plane(
                    &self.queue,
                    &self.texture_y,
                    frame.frame.data(0),
                    frame.frame.stride(0) as u32,
                    width,
                    height,
                );
                upload_rg8_plane(
                    &self.queue,
                    &self.texture_uv,
                    frame.frame.data(1),
                    frame.frame.stride(1) as u32,
                    uv_width,
                    uv_height,
                );
            }
            format::pixel::Pixel::P010LE | format::pixel::Pixel::P010BE => {
                upload_r16_plane(
                    &self.queue,
                    &self.texture_y_16,
                    frame.frame.data(0),
                    frame.frame.stride(0) as u32,
                    width,
                    height,
                );
                upload_rg16_plane(
                    &self.queue,
                    &self.texture_uv_16,
                    frame.frame.data(1),
                    frame.frame.stride(1) as u32,
                    uv_width,
                    uv_height,
                );
            }
            format::pixel::Pixel::YUV420P => {
                upload_r8_plane(
                    &self.queue,
                    &self.texture_y,
                    frame.frame.data(0),
                    frame.frame.stride(0) as u32,
                    width,
                    height,
                );
                upload_r8_plane(
                    &self.queue,
                    &self.texture_u,
                    frame.frame.data(1),
                    frame.frame.stride(1) as u32,
                    uv_width,
                    uv_height,
                );
                upload_r8_plane(
                    &self.queue,
                    &self.texture_v,
                    frame.frame.data(2),
                    frame.frame.stride(2) as u32,
                    uv_width,
                    uv_height,
                );
            }
            _ => return,
        }
        self.upload_decoded_color_params(frame);
        self.has_uploaded_frame = true;
    }

    fn upload_color_params(&self, frame: &VideoFrame) {
        self.write_color_params_uniform(self.build_color_params(FrameColorUniformInput {
            y_offset: frame.y_offset,
            y_scale: frame.y_scale,
            uv_offset: frame.uv_offset,
            uv_scale: frame.uv_scale,
            row0: [
                frame.color_matrix[0][0],
                frame.color_matrix[0][1],
                frame.color_matrix[0][2],
                1.0,
            ],
            row1_xyz: [
                frame.color_matrix[1][0],
                frame.color_matrix[1][1],
                frame.color_matrix[1][2],
            ],
            row2_xyz: [
                frame.color_matrix[2][0],
                frame.color_matrix[2][1],
                frame.color_matrix[2][2],
            ],
        }));
    }

    fn upload_decoded_color_params(&self, frame: &super::DecodedVideoFrame) {
        let layout_mode = match frame.frame.format() {
            format::pixel::Pixel::NV12 => 1.0,
            format::pixel::Pixel::P010LE | format::pixel::Pixel::P010BE => 2.0,
            _ => 0.0,
        };
        self.write_color_params_uniform(self.build_color_params(FrameColorUniformInput {
            y_offset: frame.y_offset,
            y_scale: frame.y_scale,
            uv_offset: frame.uv_offset,
            uv_scale: frame.uv_scale,
            row0: [
                frame.color_matrix[0][0],
                frame.color_matrix[0][1],
                frame.color_matrix[0][2],
                layout_mode,
            ],
            row1_xyz: [
                frame.color_matrix[1][0],
                frame.color_matrix[1][1],
                frame.color_matrix[1][2],
            ],
            row2_xyz: [
                frame.color_matrix[2][0],
                frame.color_matrix[2][1],
                frame.color_matrix[2][2],
            ],
        }));
    }

    fn build_color_params(&self, input: FrameColorUniformInput) -> ColorParams {
        let shader = picture_tune_shader_values(self.current_picture_tune());
        ColorParams {
            y_offset: input.y_offset,
            y_scale: input.y_scale,
            uv_offset: input.uv_offset,
            uv_scale: input.uv_scale,
            row0: input.row0,
            row1: [
                input.row1_xyz[0],
                input.row1_xyz[1],
                input.row1_xyz[2],
                shader.brightness,
            ],
            row2: [
                input.row2_xyz[0],
                input.row2_xyz[1],
                input.row2_xyz[2],
                shader.contrast,
            ],
            tune_extra: [shader.saturation, shader.gamma, shader.hue, 0.0],
        }
    }

    pub(super) fn sync_picture_tune_uniform(&self) {
        let shader = picture_tune_shader_values(self.current_picture_tune());
        self.write_picture_tune_uniform(shader);
    }

    fn write_picture_tune_uniform(
        &self,
        shader: crate::app::media::playback::render::picture_tune::PictureTuneShaderValues,
    ) {
        self.write_uniform_f32(PICTURE_BRIGHTNESS_UNIFORM_OFFSET, shader.brightness);
        self.write_uniform_f32(PICTURE_CONTRAST_UNIFORM_OFFSET, shader.contrast);
        self.write_uniform_f32(PICTURE_SATURATION_UNIFORM_OFFSET, shader.saturation);
        self.write_uniform_f32(PICTURE_GAMMA_UNIFORM_OFFSET, shader.gamma);
        self.write_uniform_f32(PICTURE_HUE_UNIFORM_OFFSET, shader.hue);
    }

    fn write_uniform_f32(&self, offset: u64, value: f32) {
        self.queue.write_buffer(
            &self.color_params_buffer,
            offset,
            unsafe {
                std::slice::from_raw_parts(
                    (&value as *const f32) as *const u8,
                    std::mem::size_of::<f32>(),
                )
            },
        );
    }

    fn write_color_params_uniform(&self, params: ColorParams) {
        let words = flatten_color_params(params);
        self.queue.write_buffer(
            &self.color_params_buffer,
            0,
            unsafe {
                std::slice::from_raw_parts(
                    words.as_ptr() as *const u8,
                    std::mem::size_of_val(&words),
                )
            },
        );
    }
}

const PICTURE_BRIGHTNESS_UNIFORM_OFFSET: u64 = 44;
const PICTURE_CONTRAST_UNIFORM_OFFSET: u64 = 60;
const PICTURE_SATURATION_UNIFORM_OFFSET: u64 = 64;
const PICTURE_GAMMA_UNIFORM_OFFSET: u64 = 68;
const PICTURE_HUE_UNIFORM_OFFSET: u64 = 72;

fn flatten_color_params(params: ColorParams) -> [f32; 20] {
    [
        params.y_offset,
        params.y_scale,
        params.uv_offset,
        params.uv_scale,
        params.row0[0],
        params.row0[1],
        params.row0[2],
        params.row0[3],
        params.row1[0],
        params.row1[1],
        params.row1[2],
        params.row1[3],
        params.row2[0],
        params.row2[1],
        params.row2[2],
        params.row2[3],
        params.tune_extra[0],
        params.tune_extra[1],
        params.tune_extra[2],
        params.tune_extra[3],
    ]
}

fn upload_r8_plane(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    plane: &[u8],
    stride_bytes: u32,
    width: u32,
    height: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        plane,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride_bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn upload_rg8_plane(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    plane: &[u8],
    stride_bytes: u32,
    width: u32,
    height: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        plane,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride_bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn upload_r16_plane(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    plane: &[u8],
    stride_bytes: u32,
    width: u32,
    height: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        plane,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride_bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn upload_rg16_plane(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    plane: &[u8],
    stride_bytes: u32,
    width: u32,
    height: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        plane,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride_bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}
