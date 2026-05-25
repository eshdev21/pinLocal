use std::path::Path;
use std::fs;
use crate::error::{AppResult, AppError};
use fast_image_resize as fr;

const THUMB_QUALITY: f32 = 75.0;

pub fn generate_thumbnail(source: &Path, dest: &Path, width: u32) -> AppResult<(u32, u32)> {
    // 1. Open image and guess format from content (magic bytes)
    // This allows decoding PNGs that are mislabeled as .jpg, etc.
    let img = image::ImageReader::open(source)?
        .with_guessed_format()?
        .decode()?;
    let (orig_w, orig_h) = (img.width(), img.height());
    
    // 2. Calculate dimensions
    let ratio = orig_h as f32 / orig_w as f32;
    let height = (width as f32 * ratio) as u32;
    
    // 3. Resize using fast_image_resize (SIMD accelerated)
    let src_image = fr::images::Image::from_vec_u8(
        orig_w,
        orig_h,
        img.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    ).map_err(|e| AppError::Internal(format!("Failed to create source image for resize: {}", e)))?;

    let mut dst_image = fr::images::Image::new(
        width,
        height,
        fr::PixelType::U8x4,
    );

    let mut resizer = fr::Resizer::new();
    resizer.resize(&src_image, &mut dst_image, None)
        .map_err(|e| AppError::Internal(format!("Image resize failed: {}", e)))?;

    // 4. Ensure destination directory exists
    let parent = dest.parent().ok_or_else(|| AppError::Internal("Invalid destination path".into()))?;
    fs::create_dir_all(parent)?;

    // 5. Atomic Write: Use a named temporary file in the same directory to ensure a safe rename/move
    let mut temp_file = tempfile::NamedTempFile::new_in(parent)?;
    {
        let encoder = webp::Encoder::from_rgba(dst_image.buffer(), width, height);
        let webp_data = encoder.encode(THUMB_QUALITY);
        use std::io::Write;
        temp_file.write_all(&webp_data)?;
    }

    // Persist the temporary file to the final destination
    temp_file.persist(dest).map_err(|e| AppError::Internal(format!("Failed to persist thumbnail: {}", e)))?;

    Ok((orig_w, orig_h))
}


