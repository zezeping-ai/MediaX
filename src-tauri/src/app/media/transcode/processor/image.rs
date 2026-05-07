use super::progress::{update_progress, TRANSCODE_ESTIMATE_EVENT};
use super::JobResult;
use crate::app::media::transcode::state::TranscodeJob;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ColorType, ImageEncoder, ImageFormat};
use mozjpeg::{ColorSpace as MozColorSpace, Compress as MozCompress};
use oxipng::Options as OxipngOptions;
use serde::Serialize;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use webp::Encoder as WebpEncoder;

pub(super) fn compress_image_lossless(
    app: &AppHandle,
    job: &TranscodeJob,
) -> Result<JobResult, String> {
    let source_path = Path::new(&job.source_path);
    let output_path = Path::new(&job.output_path);
    let input_size = fs::metadata(source_path)
        .map_err(|err| format!("读取输入文件元数据失败: {err}"))?
        .len();
    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    update_progress(app, job.id, 12.0, None, None, None);
    update_progress(app, job.id, 28.0, None, None, None);
    let image = image::open(source_path).map_err(|err| format!("读取图片失败: {err}"))?;
    update_progress(app, job.id, 46.0, None, None, None);
    if extension == "png" {
        update_progress(app, job.id, 64.0, None, None, None);
        let optimized = optimize_png_lossless(source_path)?;
        update_progress(app, job.id, 86.0, Some(optimized.len() as u64), None, None);
        if optimized.len() as u64 >= input_size {
            return Ok(JobResult::Skipped(
                input_size,
                format!(
                    "无损压缩未减小体积（原始: {} B, 输出: {} B）",
                    input_size,
                    optimized.len()
                ),
            ));
        }
        fs::write(output_path, &optimized).map_err(|err| format!("写入输出文件失败: {err}"))?;
        let size = optimized.len() as u64;
        update_progress(app, job.id, 95.0, Some(size), None, None);
        return Ok(JobResult::Success(size));
    } else if extension == "jpg" || extension == "jpeg" {
        update_progress(app, job.id, 64.0, None, None, None);
        let optimized = optimize_jpeg_near_lossless(&image, 95)?;
        update_progress(app, job.id, 86.0, Some(optimized.len() as u64), None, None);
        if optimized.len() as u64 >= input_size {
            return Ok(JobResult::Skipped(
                input_size,
                format!(
                    "无损压缩未减小体积（原始: {} B, 输出: {} B）",
                    input_size,
                    optimized.len()
                ),
            ));
        }
        fs::write(output_path, &optimized).map_err(|err| format!("写入输出文件失败: {err}"))?;
        let size = optimized.len() as u64;
        update_progress(app, job.id, 95.0, Some(size), None, None);
        return Ok(JobResult::Success(size));
    }
    Err("当前仅支持 PNG/JPG/JPEG 的优化压缩".to_string())
}

pub(super) fn compress_image_lossy(
    app: &AppHandle,
    job: &TranscodeJob,
) -> Result<JobResult, String> {
    let source_path = Path::new(&job.source_path);
    let output_path = Path::new(&job.output_path);
    let input_size = fs::metadata(source_path)
        .map_err(|err| format!("读取输入文件元数据失败: {err}"))?
        .len();
    let image = image::open(source_path).map_err(|err| format!("读取图片失败: {err}"))?;
    update_progress(app, job.id, 20.0, None, None, None);
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let quality = job.quality.unwrap_or(75).clamp(1, 100);
    let target_format = job.format.as_deref().unwrap_or("jpeg");
    let tuned_quality = tuned_lossy_quality(quality, target_format);
    let encoded = if target_format == "webp" {
        let encoder = WebpEncoder::from_rgb(rgb.as_raw(), width, height);
        encoder.encode(tuned_quality as f32).to_vec()
    } else if target_format == "png" {
        let rgba = image.to_rgba8();
        let (rgba_width, rgba_height) = rgba.dimensions();
        let mut png = Vec::new();
        let mut cursor = Cursor::new(&mut png);
        let encoder =
            PngEncoder::new_with_quality(&mut cursor, CompressionType::Best, FilterType::Adaptive);
        encoder
            .write_image(&rgba, rgba_width, rgba_height, ColorType::Rgba8.into())
            .map_err(|err| format!("PNG 编码失败: {err}"))?;
        png
    } else if target_format == "jpeg" {
        optimize_jpeg_near_lossless(&image, tuned_quality)?
    } else if target_format == "gif" {
        encode_with_image_format(&image, ImageFormat::Gif)
            .map_err(|err| format!("GIF 编码失败: {err}"))?
    } else if target_format == "bmp" {
        encode_with_image_format(&image, ImageFormat::Bmp)
            .map_err(|err| format!("BMP 编码失败: {err}"))?
    } else {
        return Err(format!("不支持的有损输出格式: {target_format}"));
    };
    let size = encoded.len() as u64;
    update_progress(app, job.id, 86.0, Some(size), None, None);
    if size >= input_size {
        return Ok(JobResult::Skipped(
            input_size,
            format!(
                "有损压缩未减小体积（原始: {} B, 输出: {} B）",
                input_size, size
            ),
        ));
    }
    fs::write(output_path, &encoded).map_err(|err| format!("写入输出文件失败: {err}"))?;
    update_progress(app, job.id, 95.0, Some(size), None, None);
    Ok(JobResult::Success(size))
}

pub fn emit_estimate(app: &AppHandle, job_id: u64, estimated_output_size_bytes: u64) {
    #[derive(Clone, Serialize)]
    struct EstimatePayload {
        job_id: u64,
        estimated_output_size_bytes: u64,
    }
    let _ = app.emit(
        TRANSCODE_ESTIMATE_EVENT,
        EstimatePayload {
            job_id,
            estimated_output_size_bytes,
        },
    );
}

fn optimize_png_lossless(source_path: &Path) -> Result<Vec<u8>, String> {
    let data = fs::read(source_path).map_err(|err| format!("读取 PNG 失败: {err}"))?;
    let mut opts = OxipngOptions::max_compression();
    opts.strip = oxipng::StripChunks::All;
    oxipng::optimize_from_memory(&data, &opts).map_err(|err| format!("PNG 优化失败: {err}"))
}

fn optimize_jpeg_near_lossless(
    image: &image::DynamicImage,
    quality: u8,
) -> Result<Vec<u8>, String> {
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let mut comp = MozCompress::new(MozColorSpace::JCS_RGB);
    comp.set_size(width as usize, height as usize);
    comp.set_quality(quality as f32);
    comp.set_progressive_mode();
    comp.set_optimize_coding(true);
    let mut out = Vec::new();
    let mut writer = comp
        .start_compress(&mut out)
        .map_err(|err| format!("mozjpeg 启动失败: {err}"))?;
    writer
        .write_scanlines(rgb.as_raw())
        .map_err(|err| format!("mozjpeg 编码失败: {err}"))?;
    writer
        .finish()
        .map_err(|err| format!("mozjpeg 收尾失败: {err}"))?;
    Ok(out)
}

fn tuned_lossy_quality(input_quality: u8, target_format: &str) -> u8 {
    let quality = input_quality.clamp(1, 100);
    let tuned = match target_format {
        "jpeg" => quality.saturating_sub(10),
        "webp" => quality.saturating_sub(8),
        _ => quality,
    };
    tuned.clamp(1, 100)
}

fn encode_with_image_format(
    image: &image::DynamicImage,
    format: ImageFormat,
) -> Result<Vec<u8>, image::ImageError> {
    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, format)?;
    Ok(cursor.into_inner())
}
