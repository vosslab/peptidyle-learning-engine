//! Deterministic DOCX writer (MOD-EXPORT).

use crate::{ExportArtifact, FlowBlock, PrintExam, PrintLayout, exam_flow};
use question_model::QuestionAssetId;

/// Writes a minimal OOXML package with explicit US Letter geometry, ordinary
/// page margins, embedded PNG media, and paragraph keep controls.
pub fn write(exam: &PrintExam, layout: PrintLayout) -> ExportArtifact {
    let flow = exam_flow(exam, layout);
    let mut media = Vec::<(QuestionAssetId, Vec<u8>)>::new();
    for question in &flow {
        for block in question {
            if let FlowBlock::Image { asset, .. } = block
                && !media.iter().any(|(id, _)| id == asset)
            {
                media.push((
                    *asset,
                    exam.asset(*asset).expect("validated asset").bytes.clone(),
                ));
            }
        }
    }
    let document = document_xml(&flow, &media);
    let mut entries = vec![
        (
            "[Content_Types].xml".to_string(),
            content_types(!media.is_empty()).into_bytes(),
        ),
        ("_rels/.rels".to_string(), relationships().into_bytes()),
        ("word/document.xml".to_string(), document.into_bytes()),
    ];
    if !media.is_empty() {
        entries.push((
            "word/_rels/document.xml.rels".to_string(),
            document_relationships(&media).into_bytes(),
        ));
        for (index, (_, bytes)) in media.iter().enumerate() {
            entries.push((format!("word/media/image{}.png", index + 1), bytes.clone()));
        }
    }
    ExportArtifact {
        filename: format!("{}.docx", layout.filename_segment()),
        media_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        bytes: zip(&entries),
    }
}

fn content_types(images: bool) -> String {
    let png = if images {
        "<Default Extension=\"png\" ContentType=\"image/png\"/>"
    } else {
        ""
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>{png}<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>"
    )
}
fn relationships() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>".to_string()
}
fn document_relationships(media: &[(QuestionAssetId, Vec<u8>)]) -> String {
    let image_rels = media.iter().enumerate().map(|(index, _)| format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image{}.png\"/>", index + 1, index + 1)).collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{image_rels}</Relationships>"
    )
}

fn document_xml(flow: &[Vec<FlowBlock>], media: &[(QuestionAssetId, Vec<u8>)]) -> String {
    let mut body = String::new();
    for question in flow {
        for block in question {
            match block {
                FlowBlock::Text {
                    text,
                    keep_with_next,
                } => body.push_str(&paragraph(text, *keep_with_next)),
                FlowBlock::Image { asset, alternative } => {
                    let index = media
                        .iter()
                        .position(|(id, _)| id == asset)
                        .expect("validated media")
                        + 1;
                    body.push_str(&image(index, alternative));
                }
            }
        }
    }
    // Letter 8.5x11 inches in twentieths of a point; one-inch margins.
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><w:body>{body}<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"
    )
}

fn paragraph(text: &str, keep_with_next: bool) -> String {
    let keep = if keep_with_next { "<w:keepNext/>" } else { "" };
    format!(
        "<w:p><w:pPr>{keep}<w:widowControl/></w:pPr><w:r><w:rPr><w:rFonts w:ascii=\"Arial\" w:hAnsi=\"Arial\" w:cs=\"Arial\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml(text)
    )
}

fn image(index: usize, alternative: &str) -> String {
    // 4.5-inch bounding box. Word keeps it as one drawing, so it cannot split
    // across pages; the required alternative travels in docPr metadata.
    format!(
        "<w:p><w:pPr><w:keepNext/></w:pPr><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"4114800\" cy=\"2743200\"/><wp:docPr id=\"{index}\" name=\"Figure {index}\" descr=\"{}\"/><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"0\" name=\"Figure {index}\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"rId{index}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"4114800\" cy=\"2743200\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>",
        xml(alternative)
    )
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn zip(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut directory = Vec::new();
    for (name, body) in entries {
        let offset = u32::try_from(bytes.len()).expect("DOCX package under 4 GiB");
        let crc = crc32(body);
        let size = u32::try_from(body.len()).expect("DOCX entry under 4 GiB");
        let name_bytes = name.as_bytes();
        bytes.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        bytes.extend_from_slice(&20_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&crc.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(
            &u16::try_from(name_bytes.len())
                .expect("short DOCX name")
                .to_le_bytes(),
        );
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(name_bytes);
        bytes.extend_from_slice(body);
        directory.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        directory.extend_from_slice(&20_u16.to_le_bytes());
        directory.extend_from_slice(&20_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(
            &u16::try_from(name_bytes.len())
                .expect("short DOCX name")
                .to_le_bytes(),
        );
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u32.to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name_bytes);
    }
    let directory_offset = u32::try_from(bytes.len()).expect("DOCX package under 4 GiB");
    let directory_size = u32::try_from(directory.len()).expect("DOCX directory under 4 GiB");
    bytes.extend_from_slice(&directory);
    let count = u16::try_from(entries.len()).expect("few DOCX entries");
    bytes.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&directory_size.to_le_bytes());
    bytes.extend_from_slice(&directory_offset.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320_u32 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
