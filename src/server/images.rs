//! Uploaded images, avatars and post pictures: whatever the browser sends is
//! decoded and redrawn as a JPEG (or PNG, if it has transparency). Nothing
//! of the original survives, so no EXIF (GPS included), no oversized files,
//! and no polyglot files that are also HTML or scripts; and since only
//! raster formats decode, no SVG. GIFs keep their first frame.

use std::{io::Cursor, sync::LazyLock};

use image::{
    DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, codecs::jpeg::JpegEncoder,
    imageops::FilterType,
};
use tokio::sync::Semaphore;

/// Bigger than any phone photo; keeps a tiny file that claims to be huge
/// from asking for gigabytes.
const MAX_DIMENSION: u32 = 12_000;
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;

/// Decoding a phone photo takes a core and ~50 MB for a moment; a couple at
/// a time is plenty.
static PROCESSING: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));

/// The shape an upload is redrawn to.
#[derive(Clone, Copy, Debug)]
pub enum Fit {
    /// Cropped to a square this many pixels wide.
    Square(u32),
    /// Shrunk, never enlarged, until neither side is longer than this.
    Within(u32),
}

pub struct Encoded {
    /// `image/jpeg` or `image/png`.
    pub content_type: &'static str,
    pub data: Vec<u8>,
}

pub enum ImageError {
    NotAnImage,
    Internal,
}

/// Turns an upload into the image to store.
pub async fn process(upload: Vec<u8>, fit: Fit) -> Result<Encoded, ImageError> {
    let _permit = PROCESSING.acquire().await.expect("PROCESSING is never closed");
    tokio::task::spawn_blocking(move || process_now(&upload, fit))
        .await
        .map_err(|err| {
            tracing::error!("image processing task failed: {err}");
            ImageError::Internal
        })?
}

fn process_now(upload: &[u8], fit: Fit) -> Result<Encoded, ImageError> {
    let mut image = decode(upload).ok_or(ImageError::NotAnImage)?;
    image = match fit {
        Fit::Square(side) => image.resize_to_fill(side, side, FilterType::Lanczos3),
        Fit::Within(side) if image.width() > side || image.height() > side => {
            image.resize(side, side, FilterType::Lanczos3)
        }
        Fit::Within(_) => image,
    };

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
        tracing::error!("image encoding failed: {err}");
        ImageError::Internal
    })?;

    Ok(Encoded { content_type, data })
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
    use crate::models::account::AVATAR_SIZE;
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    fn encode(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        image.write_to(Cursor::new(&mut out), format).unwrap();
        out
    }

    fn dimensions(encoded: &Encoded) -> (u32, u32) {
        let back = image::load_from_memory(&encoded.data).unwrap();
        (back.width(), back.height())
    }

    fn photo(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, Rgb([200, 80, 30])));
        encode(image, ImageFormat::Png)
    }

    #[test]
    fn avatars_become_small_square_jpegs() {
        let avatar = process_now(&photo(900, 400), Fit::Square(AVATAR_SIZE)).ok().unwrap();
        assert_eq!(avatar.content_type, "image/jpeg");
        assert_eq!(dimensions(&avatar), (AVATAR_SIZE, AVATAR_SIZE));
    }

    #[test]
    fn big_pictures_shrink_keeping_their_shape() {
        let picture = process_now(&photo(3200, 1600), Fit::Within(1600)).ok().unwrap();
        assert_eq!(dimensions(&picture), (1600, 800));
    }

    #[test]
    fn small_pictures_keep_their_size() {
        let picture = process_now(&photo(640, 480), Fit::Within(1600)).ok().unwrap();
        assert_eq!(dimensions(&picture), (640, 480));
    }

    #[test]
    fn transparency_stays_png() {
        let logo = DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0])));
        let avatar = process_now(&encode(logo, ImageFormat::Png), Fit::Square(AVATAR_SIZE)).ok().unwrap();
        assert_eq!(avatar.content_type, "image/png");
    }

    #[test]
    fn rejects_non_images() {
        for junk in [&b""[..], b"hello", b"<svg xmlns='http://www.w3.org/2000/svg'/>", b"GIF89a"] {
            assert!(matches!(process_now(junk, Fit::Within(1600)), Err(ImageError::NotAnImage)));
        }
    }
}
