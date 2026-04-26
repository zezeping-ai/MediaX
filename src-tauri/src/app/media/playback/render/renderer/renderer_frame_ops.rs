use super::{ColorParams, Renderer, VideoFrame, VideoScaleMode};
use crate::app::media::playback::render::renderer::helpers::{
    create_plane_texture, push_f32, sanitize_surface_size,
};

impl Renderer {
    pub(super) fn set_video_scale_mode(&mut self, mode: VideoScaleMode) {
        self.video_scale_mode = mode;
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

    pub(super) fn upload_frame(&mut self, frame: &VideoFrame) {
        self.ensure_texture(frame.width.max(1), frame.height.max(1));
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
            row0: [frame.color_matrix[0][0], frame.color_matrix[0][1], frame.color_matrix[0][2], 0.0],
            row1: [frame.color_matrix[1][0], frame.color_matrix[1][1], frame.color_matrix[1][2], 0.0],
            row2: [frame.color_matrix[2][0], frame.color_matrix[2][1], frame.color_matrix[2][2], 0.0],
        };
        let mut bytes = Vec::with_capacity(64);
        push_f32(&mut bytes, params.y_offset);
        push_f32(&mut bytes, params.y_scale);
        push_f32(&mut bytes, params.uv_offset);
        push_f32(&mut bytes, params.uv_scale);
        for value in params.row0.into_iter().chain(params.row1).chain(params.row2) {
            push_f32(&mut bytes, value);
        }
        self.queue.write_buffer(&self.color_params_buffer, 0, &bytes);
    }
}
