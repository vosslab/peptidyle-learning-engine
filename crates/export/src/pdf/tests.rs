use super::*;
use crate::FlowBlock;
use std::fs;
use std::process::Command;

fn append_png_chunk(bytes: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    bytes.extend_from_slice(
        &u32::try_from(data.len())
            .expect("compact PNG fixture chunk")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(&png_crc32(&kind, data).to_be_bytes());
}

fn png_header(width: u32, height: u32, color: u8) -> [u8; 13] {
    let mut header = [0_u8; 13];
    header[..4].copy_from_slice(&width.to_be_bytes());
    header[4..8].copy_from_slice(&height.to_be_bytes());
    header[8] = 8;
    header[9] = color;
    header
}

fn png_fixture(width: u32, height: u32, color: u8, scanlines: &[u8]) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    append_png_chunk(&mut bytes, *b"IHDR", &png_header(width, height, color));
    append_png_chunk(&mut bytes, *b"IDAT", &deflate(scanlines));
    append_png_chunk(&mut bytes, *b"IEND", &[]);
    bytes
}

fn rendered_pdf(pages: &[Vec<RenderBlock>], font: &EmbeddedFont) -> Vec<u8> {
    pdf(&objects_for_pages(pages, &[], font))
}

#[test]
fn bounded_png_decoder_preserves_valid_rgb_and_rgba_pixels() {
    let rgb = decode_png(&png_fixture(1, 1, 2, &[0, 0x12, 0x34, 0x56])).expect("valid RGB PNG");
    assert_eq!((rgb.width, rgb.height, rgb.color), (1, 1, 2));
    assert_eq!(rgb.raw, [0x12, 0x34, 0x56]);

    let rgba = decode_png(&png_fixture(1, 1, 6, &[0, 1, 2, 3, 4])).expect("valid RGBA PNG");
    assert_eq!((rgba.width, rgba.height, rgba.color), (1, 1, 6));
    assert_eq!(rgba.raw, [1, 2, 3, 4]);
}

#[test]
fn bounded_png_decoder_streams_consecutive_idat_chunks() {
    let compressed = deflate(&[0, 4, 5, 6]);
    let split = compressed.len() / 2;
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    append_png_chunk(&mut bytes, *b"IHDR", &png_header(1, 1, 2));
    append_png_chunk(&mut bytes, *b"IDAT", &compressed[..split]);
    append_png_chunk(&mut bytes, *b"IDAT", &compressed[split..]);
    append_png_chunk(&mut bytes, *b"IEND", &[]);

    assert_eq!(
        decode_png(&bytes).expect("split IDAT stream").raw,
        [4, 5, 6]
    );
}

#[test]
fn tiny_png_cannot_expand_past_its_declared_raster() {
    let hostile = png_fixture(1, 1, 2, &vec![0; 4_096]);
    assert!(hostile.len() < 128, "fixture should remain a compact bomb");
    assert!(decode_png(&hostile).is_err());
}

#[test]
fn png_dimensions_are_checked_before_inflation_or_allocation() {
    for (width, height, color) in [
        (u32::MAX, 1, 2),
        (u32::MAX, u32::MAX, 6),
        (
            u32::try_from(MAX_PNG_DECODED_PIXELS + 1).expect("pixel limit fits u32"),
            1,
            2,
        ),
        (
            1,
            u32::try_from(MAX_PNG_DECODED_PIXELS).expect("pixel limit fits u32"),
            6,
        ),
    ] {
        assert!(decode_png(&png_fixture(width, height, color, &[])).is_err());
    }
}

#[test]
fn png_chunk_structure_must_be_complete_and_ordered() {
    let compressed = deflate(&[0, 7, 8, 9]);

    let mut idat_first = b"\x89PNG\r\n\x1a\n".to_vec();
    append_png_chunk(&mut idat_first, *b"IDAT", &compressed);
    append_png_chunk(&mut idat_first, *b"IHDR", &png_header(1, 1, 2));
    append_png_chunk(&mut idat_first, *b"IEND", &[]);
    assert!(decode_png(&idat_first).is_err());

    let split = compressed.len() / 2;
    let mut interrupted_idat = b"\x89PNG\r\n\x1a\n".to_vec();
    append_png_chunk(&mut interrupted_idat, *b"IHDR", &png_header(1, 1, 2));
    append_png_chunk(&mut interrupted_idat, *b"IDAT", &compressed[..split]);
    append_png_chunk(&mut interrupted_idat, *b"tEXt", b"note");
    append_png_chunk(&mut interrupted_idat, *b"IDAT", &compressed[split..]);
    append_png_chunk(&mut interrupted_idat, *b"IEND", &[]);
    assert!(decode_png(&interrupted_idat).is_err());

    let mut trailing = png_fixture(1, 1, 2, &[0, 7, 8, 9]);
    trailing.push(0);
    assert!(decode_png(&trailing).is_err());

    let mut oversized_chunk = b"\x89PNG\r\n\x1a\n".to_vec();
    oversized_chunk.extend_from_slice(&u32::MAX.to_be_bytes());
    oversized_chunk.extend_from_slice(b"IHDR");
    assert!(decode_png(&oversized_chunk).is_err());
}

#[test]
fn long_tokens_wrap_to_embedded_font_width() {
    let font = EmbeddedFont::parse(DEJAVU_SANS_FONT).expect("font");
    assert!(
        wrap(&"W".repeat(200), &font, TEXT_WIDTH)
            .iter()
            .all(|line| font.text_width(line, TEXT_SIZE) <= TEXT_WIDTH)
    );
}

#[test]
#[ignore = "opt-in external PDF reader and rasterizer acceptance"]
fn wide_and_narrow_glyphs_stay_inside_pdf_text_area() {
    let font = EmbeddedFont::parse(DEJAVU_SANS_FONT).expect("font");
    let text = format!("{} {}", "W".repeat(200), "i".repeat(600));
    let lines = wrap(&text, &font, TEXT_WIDTH);
    assert!(
        lines
            .iter()
            .all(|line| font.text_width(line, TEXT_SIZE) <= TEXT_WIDTH)
    );
    let artifact = rendered_pdf(
        &[vec![RenderBlock::Text {
            lines,
            keep_with_next: false,
        }]],
        &font,
    );
    let directory = std::env::temp_dir().join(format!("ple-export-width-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("width.pdf");
    fs::write(&path, artifact).expect("PDF write");
    let output = Command::new("pdftotext")
        .args(["-bbox", path.to_str().expect("path"), "-"])
        .output()
        .expect("pdftotext");
    assert!(output.status.success());
    let bbox = String::from_utf8(output.stdout).expect("bbox UTF-8");
    for word in bbox.lines().filter(|line| line.contains("<word ")) {
        let value = word
            .split("xMax=\"")
            .nth(1)
            .and_then(|field| field.split('"').next())
            .expect("xMax attribute")
            .parse::<f32>()
            .expect("numeric xMax");
        assert!(value <= PAGE_WIDTH - RIGHT_MARGIN + 0.1, "overflow: {word}");
    }
    let raster = directory.join("width-raster");
    let rendered = Command::new("pdftoppm")
        .args(["-png", "-singlefile"])
        .arg(&path)
        .arg(&raster)
        .output()
        .expect("pdftoppm");
    assert!(rendered.status.success());
    assert!(std::path::PathBuf::from(format!("{}.png", raster.display())).is_file());
    fs::remove_dir_all(directory).expect("temporary cleanup");
}

#[test]
#[ignore = "opt-in external PDF text-reader acceptance"]
fn long_question_keeps_heading_with_first_following_line() {
    let font = EmbeddedFont::parse(DEJAVU_SANS_FONT).expect("font");
    let filler = (0..44)
        .map(|index| FlowBlock::Text {
            text: format!("filler-{index:02}"),
            keep_with_next: false,
        })
        .collect::<Vec<_>>();
    let mut long_question = vec![FlowBlock::Text {
        text: "Question heading".to_string(),
        keep_with_next: true,
    }];
    long_question.extend((0..45).map(|index| FlowBlock::Text {
        text: format!("follow-{index:02}"),
        keep_with_next: false,
    }));
    let pages = paginate(vec![filler, long_question], &font);
    assert_eq!(pages.len(), 3);
    assert!(matches!(
        pages[1].first(),
        Some(RenderBlock::Text { lines, .. }) if lines == &["Question heading"]
    ));
    assert!(matches!(
        pages[1].get(1),
        Some(RenderBlock::Text { lines, .. }) if lines == &["follow-00"]
    ));
    let directory = std::env::temp_dir().join(format!("ple-export-keep-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("keep.pdf");
    fs::write(&path, rendered_pdf(&pages, &font)).expect("PDF write");
    let output = Command::new("pdftotext")
        .arg(&path)
        .arg("-")
        .output()
        .expect("pdftotext");
    assert!(output.status.success());
    let pages = String::from_utf8(output.stdout).expect("text UTF-8");
    let extracted_pages = pages.split('\u{000c}').collect::<Vec<_>>();
    assert!(extracted_pages[1].contains("Question heading\nfollow-00"));
    fs::remove_dir_all(directory).expect("temporary cleanup");
}

#[test]
fn bundled_font_name_hash_and_license_are_dejavu_consistent() {
    let pdf = rendered_pdf(
        &[vec![]],
        &EmbeddedFont::parse(DEJAVU_SANS_FONT).expect("font"),
    );
    assert!(String::from_utf8_lossy(&pdf).contains("/PLE+DejaVuSans"));
    assert_eq!(
        objects::Sha256Digest::compute(DEJAVU_SANS_FONT).to_string(),
        "56d0092e2a6260d764e8a8a1a1b21a76a4e600d72af4ab2fafdb0f64f49c8742"
    );
    let license = include_str!("../../assets/LICENSE-DejaVuSansPLE.txt");
    assert!(license.contains("DejaVu Sans"));
    assert!(license.contains("DejaVuSansPLE.ttf"));
    assert!(license.contains("56d0092e2a6260d764e8a8a1a1b21a76a4e600d72af4ab2fafdb0f64f49c8742"));
}
