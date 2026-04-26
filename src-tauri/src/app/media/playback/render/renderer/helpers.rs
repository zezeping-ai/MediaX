use super::RendererInner;
use std::time::Duration;

pub(super) fn wait_for_render_signal(inner: &RendererInner, timeout: Duration) -> bool {
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

pub(super) fn select_present_mode(candidates: &[wgpu::PresentMode]) -> wgpu::PresentMode {
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

pub(super) fn create_plane_texture(
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

pub(super) fn sanitize_surface_size(size: &mut tauri::PhysicalSize<u32>, max_extent: u32) {
    size.width = size.width.max(1).min(max_extent.max(1));
    size.height = size.height.max(1).min(max_extent.max(1));
}

pub(super) fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_ne_bytes());
}
