//! PDF object and embedded-font rendering for deterministic exports.

use std::collections::BTreeMap;
use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use question_model::AssetId;

use crate::{ExportArtifact, PrintExam, PrintLayout, exam_flow};

use super::assets::{PngImage, collect_assets};
use super::layout::{IMAGE_LINES, RenderBlock, TEXT_SIZE, TEXT_WIDTH, paginate};

pub(super) const DEJAVU_SANS_FONT: &[u8] = include_bytes!("../../assets/DejaVuSansPLE.ttf");

/// Writes a self-contained PDF with actual PNG image XObjects. Input is
/// validated by `PrintExam::build_with_assets`, so no writer fallback changes
/// what a student sees.
pub(super) fn write(exam: &PrintExam, layout: PrintLayout) -> ExportArtifact {
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

pub(super) fn objects_for_pages(
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
pub(super) struct EmbeddedFont {
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
    pub(super) fn parse(bytes: &[u8]) -> Result<Self, ()> {
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
    pub(super) fn text_width(&self, text: &str, size: f32) -> f32 {
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

pub(super) fn deflate(bytes: &[u8]) -> Vec<u8> {
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

pub(super) fn pdf(objects: &[Vec<u8>]) -> Vec<u8> {
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
