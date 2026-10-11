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
