//! Rewrite one untrusted HOTSPOT SVG into a bounded WebP still image.
//!
//! The stored Question Image Asset remains a PNG, JPEG, or WebP file. SVG is
//! accepted only as upload input and is not retained or served.

use std::sync::Arc;

use image::ExtendedColorType;
use image::codecs::webp::WebPEncoder;
use resvg::tiny_skia::{self, Pixmap};
use resvg::usvg::{self, ImageHrefResolver, Tree};

use super::{MAX_STILL_IMAGE_BYTES, MAX_STILL_IMAGE_DECODED_PIXELS, StillImageError};

/// Rasterizes one SVG upload into lossless WebP.
// ASVS 1.5.1, 5.2.1, 5.2.2, and 5.2.6: reject XML entities and compressed SVG,
// refuse file and network image loads, and keep the pixmap inside the pixel cap.
pub(super) fn rasterize_svg_question_image(bytes: &[u8]) -> Result<Vec<u8>, StillImageError> {
    if bytes.is_empty() {
        return Err(StillImageError::Malformed);
    }
    if bytes.len() > MAX_STILL_IMAGE_BYTES {
        return Err(StillImageError::ByteLimit);
    }
    if bytes.starts_with(&[0x1f, 0x8b]) {
        return Err(StillImageError::UnsupportedMediaType);
    }
    let source = std::str::from_utf8(bytes).map_err(|_| StillImageError::Malformed)?;
    if declares_doctype_or_entity(source) || !source.to_ascii_lowercase().contains("<svg") {
        return Err(StillImageError::Malformed);
    }
    // ASVS 5.2.2: validate the SVG before storing a rewritten raster. Text is
    // rejected here because the empty font database would drop those labels.
    if declares_unsupported_svg_text(source) {
        return Err(StillImageError::UnsupportedSvgText);
    }
    let options = usvg::Options {
        resources_dir: None,
        fontdb: Arc::new(usvg::fontdb::Database::new()),
        image_href_resolver: ImageHrefResolver {
            resolve_data: Box::new(|_mime, _data, _options| None),
            resolve_string: Box::new(|_href, _options| None),
        },
        ..usvg::Options::default()
    };
    let tree = Tree::from_data(bytes, &options).map_err(|_| StillImageError::Malformed)?;
    let size = tree.size();
    let width = measured_side(size.width())?;
    let height = measured_side(size.height())?;
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(StillImageError::DecodedPixelLimit)?;
    if pixels > MAX_STILL_IMAGE_DECODED_PIXELS {
        return Err(StillImageError::DecodedPixelLimit);
    }
    let mut pixmap = Pixmap::new(width, height).ok_or(StillImageError::DecodedPixelLimit)?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    let mut straight = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        straight.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    let mut output = Vec::new();
    WebPEncoder::new_lossless(&mut output)
        .encode(&straight, width, height, ExtendedColorType::Rgba8)
        .map_err(|_| StillImageError::Malformed)?;
    if output.is_empty() || output.len() > MAX_STILL_IMAGE_BYTES {
        return Err(StillImageError::ByteLimit);
    }
    Ok(output)
}

fn declares_unsupported_svg_text(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        if starts_with_ignore_ascii(source, index, "<!--") {
            index = skip_markup(source, index + 4, "-->").unwrap_or(source.len());
            continue;
        }
        if starts_with_ignore_ascii(source, index, "<![CDATA[") {
            index = skip_markup(source, index + 9, "]]>").unwrap_or(source.len());
            continue;
        }
        if starts_with_ignore_ascii(source, index, "<?") {
            index = skip_markup(source, index + 2, "?>").unwrap_or(source.len());
            continue;
        }
        let mut name_at = index + 1;
        if bytes.get(name_at) == Some(&b'/') {
            name_at += 1;
        }
        let mut name_end = name_at;
        while name_end < bytes.len() && is_xml_name_byte(bytes[name_end]) {
            name_end += 1;
        }
        let name = &source[name_at..name_end];
        let local = name.rsplit(':').next().unwrap_or(name);
        if local.eq_ignore_ascii_case("text")
            || local.eq_ignore_ascii_case("tspan")
            || local.eq_ignore_ascii_case("textPath")
            || local.eq_ignore_ascii_case("altGlyph")
        {
            return true;
        }
        index = name_end.max(index + 1);
    }
    false
}

fn starts_with_ignore_ascii(source: &str, index: usize, marker: &str) -> bool {
    source.as_bytes().get(index..).is_some_and(|rest| {
        rest.get(..marker.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(marker.as_bytes()))
    })
}

fn skip_markup(source: &str, from: usize, marker: &str) -> Option<usize> {
    let rest = source.get(from..)?;
    let matched = rest
        .as_bytes()
        .windows(marker.len())
        .position(|window| window.eq_ignore_ascii_case(marker.as_bytes()))?;
    Some(from + matched + marker.len())
}

fn is_xml_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b':' || byte == b'-' || byte == b'.'
}

fn declares_doctype_or_entity(source: &str) -> bool {
    let lowered = source.to_ascii_lowercase();
    let mut rest = lowered.as_str();
    while let Some(index) = rest.find("<!") {
        let after = rest[index + 2..].trim_start();
        if after.starts_with("doctype") || after.starts_with("entity") {
            return true;
        }
        let next = index + 2;
        if next >= rest.len() {
            break;
        }
        rest = &rest[next..];
    }
    false
}

fn measured_side(value: f32) -> Result<u32, StillImageError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(StillImageError::ZeroDimensions);
    }
    let side = value.ceil();
    if side > u32::MAX as f32 {
        return Err(StillImageError::DecodedPixelLimit);
    }
    let side = side as u32;
    if side == 0 {
        return Err(StillImageError::ZeroDimensions);
    }
    Ok(side)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder, ImageReader, Rgb, RgbImage};

    use super::super::{
        PreparedQuestionImage, StillImageError, StillImageMediaType, prepare_question_image,
    };

    const RECT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"><rect width="12" height="8" fill="red"/></svg>"#;

    #[test]
    fn hotspot_content_uses_supported_still_images_and_svg() {
        let png = png_bytes();
        let prepared = prepare_question_image("image/png", &png).expect("png upload");
        assert!(matches!(prepared, PreparedQuestionImage::Original(_, _)));
        assert_eq!(prepared.bytes(), png.as_slice());
        assert_eq!(prepared.verified().media_type, StillImageMediaType::Png);

        let webp = prepare_question_image("image/svg+xml", RECT.as_bytes()).expect("svg upload");
        assert!(matches!(webp, PreparedQuestionImage::Rewritten(_, _)));
        assert_eq!(webp.verified().media_type, StillImageMediaType::WebP);
        assert_eq!((webp.verified().width, webp.verified().height), (12, 8));
        assert!(webp.bytes().starts_with(b"RIFF"));
        assert!(!webp.bytes().windows(4).any(|window| window == b"<svg"));
        assert_eq!(pixel(webp.bytes(), 6, 4), [255, 0, 0, 255]);

        let probe = blue_png_file();
        let path_text = probe.path.display().to_string();
        let hostile = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="12" height="8"><rect width="12" height="8" fill="red"/><image href="{path}" xlink:href="{path}" width="12" height="8"/><image href="https://evil.example/x.png" width="12" height="8"/><script>alert(1)</script></svg>"#,
            path = path_text,
        );
        let rewritten =
            prepare_question_image("image/svg+xml", hostile.as_bytes()).expect("hostile svg");
        let stored = rewritten.bytes();
        assert_eq!(pixel(stored, 6, 4), [255, 0, 0, 255]);
        for absent in [
            b"<svg".as_slice(),
            b"<script".as_slice(),
            b"alert(1)".as_slice(),
            b"evil.example".as_slice(),
            path_text.as_bytes(),
        ] {
            assert!(
                !stored.windows(absent.len()).any(|window| window == absent),
                "stored WebP retained {}",
                String::from_utf8_lossy(absent),
            );
        }

        assert_eq!(
            prepare_question_image("image/png", RECT.as_bytes()),
            Err(StillImageError::UnsupportedMediaType)
        );
        assert!(prepare_question_image("image/svg+xml", &png).is_err());
        assert_eq!(
            prepare_question_image("image/svg+xml", b""),
            Err(StillImageError::Malformed)
        );
        assert_eq!(
            prepare_question_image("image/svg+xml", &[0x1f, 0x8b, 0x08]),
            Err(StillImageError::UnsupportedMediaType)
        );
        assert_eq!(
            prepare_question_image(
                "image/svg+xml",
                br#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"></svg>"#,
            ),
            Err(StillImageError::Malformed)
        );
        assert_eq!(
            prepare_question_image(
                "image/svg+xml",
                b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"12\" height=\"8\"><!\nENTITY></svg>",
            ),
            Err(StillImageError::Malformed)
        );
        assert_eq!(
            prepare_question_image(
                "image/svg+xml",
                br#"<svg xmlns="http://www.w3.org/2000/svg" width="20000000" height="2"></svg>"#,
            ),
            Err(StillImageError::DecodedPixelLimit)
        );
        assert_eq!(
            prepare_question_image(
                "image/svg+xml",
                br#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"><text x="1" y="6">ATP</text></svg>"#,
            ),
            Err(StillImageError::UnsupportedSvgText)
        );
        let multibyte_text = r#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"><text>&alpha;&beta;</text></svg>"#;
        assert_eq!(
            prepare_question_image("image/svg+xml", multibyte_text.as_bytes()),
            Err(StillImageError::UnsupportedSvgText)
        );
        assert_eq!(
            StillImageError::UnsupportedSvgText.user_message(),
            "remove text from the SVG, or upload a PNG, JPEG, or WebP image"
        );
        assert_eq!(
            prepare_question_image(
                "image/svg+xml",
                br#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"><tspan>ATP</tspan></svg>"#,
            ),
            Err(StillImageError::UnsupportedSvgText)
        );
        let commented = r#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"><!-- <text>ATP</text> --><rect width="12" height="8" fill="red"/></svg>"#;
        let kept = prepare_question_image("image/svg+xml", commented.as_bytes()).expect("comment");
        assert_eq!(pixel(kept.bytes(), 6, 4), [255, 0, 0, 255]);
    }

    fn pixel(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
        let image = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .expect("format")
            .decode()
            .expect("webp decodes")
            .into_rgba8();
        image.get_pixel(x, y).0
    }

    fn png_bytes() -> Vec<u8> {
        let image = RgbImage::from_pixel(3, 2, Rgb([12, 34, 56]));
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("png fixture");
        bytes
    }

    struct BluePngFile {
        path: std::path::PathBuf,
    }

    impl Drop for BluePngFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn blue_png_file() -> BluePngFile {
        let image = RgbImage::from_pixel(12, 8, Rgb([0, 0, 255]));
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("blue png");
        let path = std::env::temp_dir().join(format!(
            "ple-hotspot-svg-probe-{}-{}.png",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(&path, bytes).expect("probe file");
        BluePngFile { path }
    }
}
