//! Deterministic PDF writer (MOD-EXPORT).

use std::collections::BTreeMap;
use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use question_model::AssetId;

use crate::{ExportArtifact, FlowBlock, PrintExam, PrintLayout, PrintableAsset, exam_flow};

const LINES_PER_PAGE: usize = 45;
const IMAGE_LINES: usize = 14;
const LEFT_MARGIN: f32 = 72.0;
const RIGHT_MARGIN: f32 = 72.0;
const PAGE_WIDTH: f32 = 612.0;
const TEXT_SIZE: f32 = 11.0;
const TEXT_WIDTH: f32 = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;
const DEJAVU_SANS_FONT: &[u8] = include_bytes!("../assets/DejaVuSansPLE.ttf");

/// Writes a self-contained PDF with actual PNG image XObjects. Input is
/// validated by `PrintExam::build_with_assets`, so no writer fallback changes
/// what a student sees.
pub fn write(exam: &PrintExam, layout: PrintLayout) -> ExportArtifact {
    let flow = exam_flow(exam, layout);
    let assets = collect_assets(exam, &flow);
    let font = EmbeddedFont::parse(DEJAVU_SANS_FONT).expect("bundled Unicode subset is valid");
    let pages = paginate(flow, &font);
    let objects = objects_for_pages(&pages, &assets, &font);
    ExportArtifact {
        filename: format!("{}.pdf", layout.filename_segment()),
        media_type: "application/pdf",
        bytes: pdf(&objects),
    }
}

#[derive(Debug, Clone)]
enum RenderBlock {
    Text {
        lines: Vec<String>,
        keep_with_next: bool,
    },
    Image(AssetId),
}

fn collect_assets(exam: &PrintExam, flow: &[Vec<FlowBlock>]) -> Vec<(AssetId, PngImage)> {
    let mut assets = Vec::new();
    for question in flow {
        for block in question {
            if let FlowBlock::Image { asset, .. } = block
                && !assets.iter().any(|(id, _)| id == asset)
            {
                assets.push((
                    *asset,
                    decode_png(exam.asset(*asset).expect("validated asset"))
                        .expect("validated PNG"),
                ));
            }
        }
    }
    assets
}

fn paginate(questions: Vec<Vec<FlowBlock>>, font: &EmbeddedFont) -> Vec<Vec<RenderBlock>> {
    let mut pages = vec![Vec::new()];
    let mut used = 0;
    for question in questions {
        let blocks = flatten(question, font);
        let cost = blocks.iter().map(block_lines).sum::<usize>();
        // Keep an ordinary question intact. An unusually long question starts
        // on a fresh page then splits only at complete visual/text blocks.
        if cost <= LINES_PER_PAGE && used > 0 && used + cost > LINES_PER_PAGE {
            pages.push(Vec::new());
            used = 0;
        }
        for (index, block) in blocks.iter().enumerate() {
            let block_cost = block_lines(block);
            // A heading, choice label, or ordering label belongs with the first
            // following visual block. When the whole pair fits a page, keep it
            // intact; when the following block is longer than a page, keep at
            // least its first rendered line with the label.
            let following_cost = blocks.get(index + 1).map(block_lines);
            let required = match (&block, following_cost) {
                (
                    RenderBlock::Text {
                        keep_with_next: true,
                        ..
                    },
                    Some(next),
                ) => block_cost + next.min(1),
                _ => block_cost,
            };
            if used > 0 && required <= LINES_PER_PAGE && used + required > LINES_PER_PAGE {
                pages.push(Vec::new());
                used = 0;
            }
            if used > 0 && used + block_cost > LINES_PER_PAGE {
                pages.push(Vec::new());
                used = 0;
            }
            if block_cost > LINES_PER_PAGE {
                // long text is already wrapped; emit across pages safely.
                if let RenderBlock::Text { lines, .. } = block {
                    for line in lines {
                        if used == LINES_PER_PAGE {
                            pages.push(Vec::new());
                            used = 0;
                        }
                        pages.last_mut().expect("page").push(RenderBlock::Text {
                            lines: vec![line.clone()],
                            keep_with_next: false,
                        });
                        used += 1;
                    }
                } else {
                    pages.last_mut().expect("page").push(block.clone());
                    used += IMAGE_LINES;
                }
            } else {
                pages.last_mut().expect("page").push(block.clone());
                used += block_cost;
            }
        }
    }
    pages
}

fn flatten(question: Vec<FlowBlock>, font: &EmbeddedFont) -> Vec<RenderBlock> {
    let mut output = Vec::new();
    for block in question {
        match block {
            FlowBlock::Text {
                text,
                keep_with_next,
            } => output.push(RenderBlock::Text {
                lines: wrap(&text, font, TEXT_WIDTH),
                keep_with_next,
            }),
            FlowBlock::Image { asset, .. } => output.push(RenderBlock::Image(asset)),
        }
    }
    output
}
fn block_lines(block: &RenderBlock) -> usize {
    match block {
        RenderBlock::Text { lines, .. } => lines.len(),
        RenderBlock::Image(_) => IMAGE_LINES,
    }
}

/// Wraps whitespace-delimited prose and also splits an over-wide token. This
/// prevents an accession number, URL, or code identifier from running off the
/// printed page.
fn wrap(line: &str, font: &EmbeddedFont, width: f32) -> Vec<String> {
    if font.text_width(line, TEXT_SIZE) <= width {
        return vec![line.to_string()];
    }
    let mut output = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        for piece in split_token(word, font, width) {
            let candidate = if current.is_empty() {
                piece.clone()
            } else {
                format!("{current} {piece}")
            };
            if font.text_width(&candidate, TEXT_SIZE) > width && !current.is_empty() {
                output.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&piece);
        }
    }
    if !current.is_empty() {
        output.push(current);
    }
    output
}
fn split_token(token: &str, font: &EmbeddedFont, width: f32) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut piece = String::new();
    for character in token.chars() {
        let mut candidate = piece.clone();
        candidate.push(character);
        if !piece.is_empty() && font.text_width(&candidate, TEXT_SIZE) > width {
            pieces.push(std::mem::take(&mut piece));
        }
        piece.push(character);
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    pieces
}

fn objects_for_pages(
    pages: &[Vec<RenderBlock>],
    assets: &[(AssetId, PngImage)],
    font: &EmbeddedFont,
) -> Vec<Vec<u8>> {
    let page_count = pages.len().max(1);
    let font_id = 3 + page_count * 2;
    let image_start = font_id + 5;
    let kids = (0..page_count)
        .map(|index| format!("{} 0 R", 3 + index * 2))
        .collect::<Vec<_>>()
        .join(" ");
    let xobjects = assets
        .iter()
        .enumerate()
        .map(|(index, _)| format!("/Im{} {} 0 R", index + 1, image_start + index))
        .collect::<Vec<_>>()
        .join(" ");
    let asset_indexes = assets
        .iter()
        .enumerate()
        .map(|(index, (id, _))| (*id, index + 1))
        .collect::<BTreeMap<_, _>>();
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {page_count} >>").into_bytes(),
    ];
    for (index, page) in
        (0..page_count).map(|i| (i, pages.get(i).map(Vec::as_slice).unwrap_or(&[])))
    {
        let page_id = 3 + index * 2;
        let content_id = page_id + 1;
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 {font_id} 0 R >> /XObject << {xobjects} >> >> /Contents {content_id} 0 R >>").into_bytes());
        let stream = content_stream(page, &asset_indexes, font);
        let mut content = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
        content.extend_from_slice(&stream);
        content.extend_from_slice(b"endstream");
        objects.push(content);
    }
    objects.push(format!("<< /Type /Font /Subtype /Type0 /BaseFont /PLE+DejaVuSans /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>", font_id + 1, font_id + 4).into_bytes());
    objects.push(format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /PLE+DejaVuSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /DW {} /W {} /CIDToGIDMap /Identity >>", font_id + 2, font.pdf_width(font.default_width), font.pdf_widths()).into_bytes());
    objects.push(format!("<< /Type /FontDescriptor /FontName /PLE+DejaVuSans /Flags 32 /FontBBox [0 -300 2000 1100] /ItalicAngle 0 /Ascent 1069 /Descent -293 /CapHeight 714 /StemV 80 /FontFile2 {} 0 R >>", font_id + 3).into_bytes());
    let mut font_file = format!("<< /Length {} >>\nstream\n", DEJAVU_SANS_FONT.len()).into_bytes();
    font_file.extend_from_slice(DEJAVU_SANS_FONT);
    font_file.extend_from_slice(b"\nendstream");
    objects.push(font_file);
    let cmap = font.to_unicode_cmap();
    let mut cmap_object = format!("<< /Length {} >>\nstream\n", cmap.len()).into_bytes();
    cmap_object.extend_from_slice(cmap.as_bytes());
    cmap_object.extend_from_slice(b"endstream");
    objects.push(cmap_object);
    for (_, image) in assets {
        objects.push(image_object(image));
    }
    objects
}

fn content_stream(
    page: &[RenderBlock],
    indexes: &BTreeMap<AssetId, usize>,
    font: &EmbeddedFont,
) -> Vec<u8> {
    // PDF text matrices are relative.  Keeping the cursor in ordinary page
    // coordinates makes the pagination model and the drawing model the same
    // model: every block consumes exactly the space it was assigned above.
    let mut y = 744_i32;
    let mut stream = Vec::new();
    for block in page {
        match block {
            RenderBlock::Text { lines, .. } => {
                for line in lines {
                    debug_assert!(font.text_width(line, TEXT_SIZE) <= TEXT_WIDTH);
                    stream.extend_from_slice(
                        format!("BT\n/F1 11 Tf\n72 {y} Td\n<{}> Tj\nET\n", font.encode(line))
                            .as_bytes(),
                    );
                    y -= 14;
                }
            }
            RenderBlock::Image(asset) => {
                // Leave the same vertical footprint (`IMAGE_LINES`) used by
                // pagination.  Images therefore never reuse a fixed page
                // location or cover preceding/following prose.
                let image_y = y - 168;
                stream.extend_from_slice(format!("q\n324 0 0 168 72 {image_y} cm\n").as_bytes());
                stream.extend_from_slice(format!("/Im{} Do\nQ\n", indexes[asset]).as_bytes());
                y -= (IMAGE_LINES as i32) * 14;
            }
        }
    }
    stream
}

#[derive(Debug)]
struct EmbeddedFont {
    glyphs: BTreeMap<u32, u16>,
    advances: Vec<u16>,
    default_width: u16,
    missing_width: u16,
    units_per_em: u16,
}

impl EmbeddedFont {
    // The committed open-licensed DejaVu Sans subset uses a TrueType cmap format 4. This
    // small reader deliberately supports that stable, reviewed asset rather
    // than consulting host font directories at build or runtime.
    fn parse(bytes: &[u8]) -> Result<Self, ()> {
        let table = |name: &[u8; 4]| -> Result<&[u8], ()> {
            let count =
                u16::from_be_bytes(bytes.get(4..6).ok_or(())?.try_into().map_err(|_| ())?) as usize;
            for index in 0..count {
                let start = 12 + index * 16;
                if bytes.get(start..start + 4) == Some(name) {
                    let offset = u32::from_be_bytes(
                        bytes
                            .get(start + 8..start + 12)
                            .ok_or(())?
                            .try_into()
                            .map_err(|_| ())?,
                    ) as usize;
                    let length = u32::from_be_bytes(
                        bytes
                            .get(start + 12..start + 16)
                            .ok_or(())?
                            .try_into()
                            .map_err(|_| ())?,
                    ) as usize;
                    return bytes.get(offset..offset + length).ok_or(());
                }
            }
            Err(())
        };
        let cmap = table(b"cmap")?;
        let head = table(b"head")?;
        let hhea = table(b"hhea")?;
        let hmtx = table(b"hmtx")?;
        let units_per_em = read_u16(head, 18)?;
        let metric_count = usize::from(read_u16(hhea, 34)?);
        if metric_count == 0 || hmtx.len() < metric_count * 4 {
            return Err(());
        }
        let advances = (0..metric_count)
            .map(|index| read_u16(hmtx, index * 4))
            .collect::<Result<Vec<_>, _>>()?;
        let record_count =
            u16::from_be_bytes(cmap.get(2..4).ok_or(())?.try_into().map_err(|_| ())?) as usize;
        let mut subtable = None;
        for index in 0..record_count {
            let base = 4 + index * 8;
            let offset = u32::from_be_bytes(
                cmap.get(base + 4..base + 8)
                    .ok_or(())?
                    .try_into()
                    .map_err(|_| ())?,
            ) as usize;
            if cmap.get(offset..offset + 2) == Some(&[0, 4]) {
                subtable = cmap.get(offset..);
                break;
            }
        }
        let cmap = subtable.ok_or(())?;
        let seg_count =
            u16::from_be_bytes(cmap.get(6..8).ok_or(())?.try_into().map_err(|_| ())?) as usize / 2;
        let end = 14;
        let start = end + seg_count * 2 + 2;
        let delta = start + seg_count * 2;
        let range = delta + seg_count * 2;
        let mut glyphs = BTreeMap::new();
        for segment in 0..seg_count {
            let end_code = read_u16(cmap, end + segment * 2)?;
            let start_code = read_u16(cmap, start + segment * 2)?;
            let delta_value = read_u16(cmap, delta + segment * 2)?;
            let range_offset = read_u16(cmap, range + segment * 2)? as usize;
            for code in start_code..=end_code {
                if code == 0xffff {
                    continue;
                }
                let glyph = if range_offset == 0 {
                    code.wrapping_add(delta_value)
                } else {
                    let at =
                        range + segment * 2 + range_offset + usize::from(code - start_code) * 2;
                    let raw = read_u16(cmap, at)?;
                    if raw == 0 {
                        0
                    } else {
                        raw.wrapping_add(delta_value)
                    }
                };
                if glyph != 0 {
                    glyphs.insert(u32::from(code), glyph);
                }
            }
        }
        let missing_width = advances[0];
        let default_width = *advances.last().ok_or(())?;
        Ok(Self {
            glyphs,
            default_width,
            advances,
            missing_width,
            units_per_em,
        })
    }
    fn glyph_advance(&self, character: char) -> u16 {
        let glyph = self.glyphs.get(&(character as u32)).copied().unwrap_or(0);
        self.advances.get(usize::from(glyph)).copied().unwrap_or({
            if glyph == 0 {
                self.missing_width
            } else {
                self.default_width
            }
        })
    }
    fn text_width(&self, text: &str, size: f32) -> f32 {
        text.chars()
            .map(|character| f32::from(self.glyph_advance(character)))
            .sum::<f32>()
            * size
            / f32::from(self.units_per_em)
    }
    fn pdf_width(&self, advance: u16) -> u16 {
        ((u32::from(advance) * 1000 + u32::from(self.units_per_em) / 2)
            / u32::from(self.units_per_em)) as u16
    }
    fn pdf_widths(&self) -> String {
        format!(
            "[0 [{}]]",
            self.advances
                .iter()
                .map(|advance| self.pdf_width(*advance).to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
    fn encode(&self, text: &str) -> String {
        text.chars()
            .map(|character| {
                format!(
                    "{:04X}",
                    self.glyphs.get(&(character as u32)).copied().unwrap_or(0)
                )
            })
            .collect()
    }
    fn to_unicode_cmap(&self) -> String {
        let mut mappings = self
            .glyphs
            .iter()
            .map(|(scalar, gid)| format!("<{gid:04X}> <{scalar:04X}>"))
            .collect::<Vec<_>>();
        mappings.sort();
        format!(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /PLEToUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n{} beginbfchar\n{}\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n",
            mappings.len(),
            mappings.join("\n")
        )
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ()> {
    Ok(u16::from_be_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or(())?
            .try_into()
            .map_err(|_| ())?,
    ))
}

#[derive(Debug, Clone)]
struct PngImage {
    width: u32,
    height: u32,
    color: u8,
    raw: Vec<u8>,
}
/// True when a PNG can be carried faithfully by both deterministic writers.
pub fn png_is_supported(bytes: &[u8]) -> bool {
    decode_png(&PrintableAsset {
        media_type: "image/png".to_string(),
        bytes: bytes.to_vec(),
    })
    .is_ok()
}
fn decode_png(asset: &PrintableAsset) -> Result<PngImage, ()> {
    let bytes = &asset.bytes;
    if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(());
    }
    let mut position = 8;
    let mut width = 0;
    let mut height = 0;
    let mut color = 255;
    let mut idat = Vec::new();
    while position + 12 <= bytes.len() {
        let length =
            u32::from_be_bytes(bytes[position..position + 4].try_into().map_err(|_| ())?) as usize;
        let kind = &bytes[position + 4..position + 8];
        let data_start = position + 8;
        let data_end = data_start.checked_add(length).ok_or(())?;
        if data_end + 4 > bytes.len() {
            return Err(());
        }
        if kind == b"IHDR" {
            if length != 13 {
                return Err(());
            }
            width = u32::from_be_bytes(
                bytes[data_start..data_start + 4]
                    .try_into()
                    .map_err(|_| ())?,
            );
            height = u32::from_be_bytes(
                bytes[data_start + 4..data_start + 8]
                    .try_into()
                    .map_err(|_| ())?,
            );
            if bytes[data_start + 8] != 8 || bytes[data_start + 12] != 0 {
                return Err(());
            }
            color = bytes[data_start + 9];
            if color != 2 && color != 6 {
                return Err(());
            }
        } else if kind == b"IDAT" {
            idat.extend_from_slice(&bytes[data_start..data_end]);
        } else if kind == b"IEND" {
            break;
        }
        position = data_end + 4;
    }
    if width == 0 || height == 0 || idat.is_empty() {
        return Err(());
    }
    let mut inflated = Vec::new();
    ZlibDecoder::new(idat.as_slice())
        .read_to_end(&mut inflated)
        .map_err(|_| ())?;
    let channels = if color == 2 { 3 } else { 4 };
    let stride = width as usize * channels;
    if inflated.len() != (stride + 1) * height as usize {
        return Err(());
    }
    let mut raw = vec![0; stride * height as usize];
    for row in 0..height as usize {
        let filter = inflated[row * (stride + 1)];
        let source = &inflated[row * (stride + 1) + 1..(row + 1) * (stride + 1)];
        let start = row * stride;
        for x in 0..stride {
            let left = if x >= channels {
                raw[start + x - channels]
            } else {
                0
            };
            let up = if row > 0 { raw[start + x - stride] } else { 0 };
            let up_left = if row > 0 && x >= channels {
                raw[start + x - stride - channels]
            } else {
                0
            };
            raw[start + x] = match filter {
                0 => source[x],
                1 => source[x].wrapping_add(left),
                2 => source[x].wrapping_add(up),
                3 => source[x].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => source[x].wrapping_add(paeth(left, up, up_left)),
                _ => return Err(()),
            };
        }
    }
    Ok(PngImage {
        width,
        height,
        color,
        raw,
    })
}
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i32::from(a) + i32::from(b) - i32::from(c);
    let pa = (p - i32::from(a)).abs();
    let pb = (p - i32::from(b)).abs();
    let pc = (p - i32::from(c)).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}
fn deflate(bytes: &[u8]) -> Vec<u8> {
    let mut out = ZlibEncoder::new(Vec::new(), Compression::default());
    out.write_all(bytes).expect("memory write");
    out.finish().expect("memory finish")
}
fn image_object(image: &PngImage) -> Vec<u8> {
    let rgb = if image.color == 2 {
        image.raw.clone()
    } else {
        image
            .raw
            .chunks_exact(4)
            .flat_map(|pixel| pixel[..3].iter().copied())
            .collect()
    };
    let rgb = deflate(&rgb);
    let mut object = format!("<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n", image.width, image.height, rgb.len()).into_bytes();
    object.extend_from_slice(&rgb);
    object.extend_from_slice(b"\nendstream");
    object
}

fn pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn rendered_pdf(pages: &[Vec<RenderBlock>], font: &EmbeddedFont) -> Vec<u8> {
        pdf(&objects_for_pages(pages, &[], font))
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
        let directory =
            std::env::temp_dir().join(format!("ple-export-width-{}", std::process::id()));
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
        let directory =
            std::env::temp_dir().join(format!("ple-export-keep-{}", std::process::id()));
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
        let license = include_str!("../assets/LICENSE-DejaVuSansPLE.txt");
        assert!(license.contains("DejaVu Sans"));
        assert!(license.contains("DejaVuSansPLE.ttf"));
        assert!(
            license.contains("56d0092e2a6260d764e8a8a1a1b21a76a4e600d72af4ab2fafdb0f64f49c8742")
        );
    }
}
