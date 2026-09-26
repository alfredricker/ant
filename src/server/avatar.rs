//! Uploaded avatars: whatever the browser sends is decoded and redrawn as a
//! small square JPEG (or PNG, if it has transparency). Nothing of the
//! original survives, so no EXIF (GPS included), no oversized files, and no
//! polyglot files that are also HTML or scripts; and since only raster
//! formats decode, no SVG.

use std::{io::Cursor, sync::LazyLock};

use image::{
    DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, codecs::jpeg::JpegEncoder,
    imageops::FilterType,
};
use tokio::sync::Semaphore;

use crate::{models::account::AVATAR_SIZE, server::db::account::Avatar};

/// Bigger than any phone photo; keeps a tiny file that claims to be huge
/// from asking for gigabytes.
const MAX_DIMENSION: u32 = 12_000;
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;

/// Decoding a phone photo takes a core and ~50 MB for a moment; a couple at
/// a time is plenty.
static PROCESSING: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));

pub enum AvatarError {
    NotAnImage,
    Internal,
}

/// Turns an upload into the avatar to store.
pub async fn process(upload: Vec<u8>) -> Result<Avatar, AvatarError> {
    let _permit = PROCESSING.acquire().await.expect("PROCESSING is never closed");
    tokio::task::spawn_blocking(move || process_now(&upload))
        .await
        .map_err(|err| {
            tracing::error!("avatar processing task failed: {err}");
            AvatarError::Internal
        })?
}

fn process_now(upload: &[u8]) -> Result<Avatar, AvatarError> {
    let mut image = decode(upload).ok_or(AvatarError::NotAnImage)?;
    image = image.resize_to_fill(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3);

    let mut data = Vec::new();
    let encoded = if image.has_alpha() {
        image.write_to(Cursor::new(&mut data), ImageFormat::Png).map(|()| "image/png")
    } else {
        image
            .to_rgb8()
            .write_with_encoder(JpegEncoder::new_with_quality(&mut data, 85))
            .map(|()| "image/jpeg")
    };
    let content_type = encoded.map_err(|err| {
        tracing::error!("avatar encoding failed: {err}");
        AvatarError::Internal
    })?;

    Ok(Avatar { content_type: content_type.to_owned(), data })
}

/// The format comes from the bytes, never the file name or the browser's
/// content type.
fn decode(upload: &[u8]) -> Option<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(upload)).with_guessed_format().ok()?;
    let format = reader.format()?;
    if !matches!(format, ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP | ImageFormat::Gif) {
        return None;
    }

    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);

    let mut decoder = reader.into_decoder().ok()?;
    // Phones store photos sideways plus an EXIF "rotate me" flag.
    let orientation = decoder.orientation().ok()?;
    let mut image = DynamicImage::from_decoder(decoder).ok()?;
    image.apply_orientation(orientation);
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    fn encode(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        image.write_to(Cursor::new(&mut out), format).unwrap();
        out
    }

    #[test]
    fn photos_become_small_square_jpegs() {
        let photo = DynamicImage::ImageRgb8(RgbImage::from_pixel(900, 400, Rgb([200, 80, 30])));
        let avatar = process_now(&encode(photo, ImageFormat::Png)).ok().unwrap();
        assert_eq!(avatar.content_type, "image/jpeg");
        let back = image::load_from_memory(&avatar.data).unwrap();
        assert_eq!((back.width(), back.height()), (AVATAR_SIZE, AVATAR_SIZE));
    }

    #[test]
    fn transparency_stays_png() {
        let logo = DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0])));
        let avatar = process_now(&encode(logo, ImageFormat::Png)).ok().unwrap();
        assert_eq!(avatar.content_type, "image/png");
    }

    #[test]
    fn rejects_non_images() {
        for junk in [&b""[..], b"hello", b"<svg xmlns='http://www.w3.org/2000/svg'/>", b"GIF89a"] {
            assert!(matches!(process_now(junk), Err(AvatarError::NotAnImage)));
        }
    }
}
