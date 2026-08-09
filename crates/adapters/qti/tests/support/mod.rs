//! Reusable fixture loading and deterministic logical ZIP construction.
//!
//! Future vendor-parser integration tests use these readable package members
//! instead of copying ZIP blobs. Tests compare extracted member paths and
//! contents, never ZIP container bytes or timestamps.

use std::io::{Cursor, Read, Write};

use xmlparser::{ElementEnd, Token, Tokenizer};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const CANVAS_MANIFEST: &str = include_str!("../fixtures/profiles/canvas_positive_manifest.xml");
pub const CANVAS_ITEM: &str = include_str!("../fixtures/profiles/canvas_positive_item.xml");
pub const CANVAS_META: &str = include_str!("../fixtures/profiles/canvas_assessment_meta.xml");
pub const CANVAS_NEAR_MISS_MANIFEST: &str =
    include_str!("../fixtures/profiles/canvas_near_miss_manifest.xml");
pub const CANVAS_NEAR_MISS_ITEM: &str =
    include_str!("../fixtures/profiles/canvas_near_miss_item.xml");
pub const BLACKBOARD_MANIFEST: &str =
    include_str!("../fixtures/profiles/blackboard_positive_manifest.xml");
pub const BLACKBOARD_ITEM: &str = include_str!("../fixtures/profiles/blackboard_positive_item.xml");
pub const BLACKBOARD_META: &str =
    include_str!("../fixtures/profiles/blackboard_assessment_meta.xml");
pub const BLACKBOARD_NEAR_MISS_MANIFEST: &str =
    include_str!("../fixtures/profiles/blackboard_near_miss_manifest.xml");
pub const BLACKBOARD_NEAR_MISS_ITEM: &str =
    include_str!("../fixtures/profiles/blackboard_near_miss_item.xml");
pub const BLACKBOARD_UNEXPECTED_ASSET: &str =
    include_str!("../fixtures/profiles/blackboard_unexpected_asset.txt");

#[derive(Clone, Copy)]
pub struct FixtureEntry {
    pub path: &'static str,
    pub contents: &'static str,
}

/// Builds a deterministic logical fixture archive with sorted, safe members.
pub fn build_fixture_archive(entries: &[FixtureEntry]) -> Vec<u8> {
    let mut sorted = entries.to_vec();
    sorted.sort_by_key(|entry| entry.path);
    for entry in &sorted {
        assert_safe_unique_path(entry.path, &sorted);
    }

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for entry in sorted {
        writer
            .start_file(entry.path, options)
            .expect("fixture path starts");
        writer
            .write_all(entry.contents.as_bytes())
            .expect("fixture bytes write");
    }
    writer
        .finish()
        .expect("fixture archive finishes")
        .into_inner()
}

fn assert_safe_unique_path(path: &str, entries: &[FixtureEntry]) {
    assert!(
        !path.is_empty()
            && !path.starts_with('/')
            && !path.contains('\\')
            && path
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        "fixture archive path must be package-relative and normalized: {path}"
    );
    assert_eq!(
        entries.iter().filter(|entry| entry.path == path).count(),
        1,
        "fixture archive paths must be unique: {path}"
    );
}

pub fn read_fixture_archive(bytes: &[u8]) -> Vec<(String, String)> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("fixture archive opens");
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .expect("fixture archive entry opens");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("fixture member remains UTF-8");
        entries.push((file.name().to_string(), contents));
    }
    entries
}

/// Checks XML tokenization plus the nesting that `xmlparser::Tokenizer` leaves
/// to callers. Fixture tests need a real single-root XML document before a
/// profile parser owns its semantic validation.
pub fn is_well_formed(xml: &str) -> Result<(), String> {
    let mut roots = 0_usize;
    let mut stack = Vec::<String>::new();
    let mut pending_start = None::<String>;
    for token in Tokenizer::from(xml) {
        let token = token.map_err(|error| error.to_string())?;
        match token {
            Token::ElementStart { local, .. } => {
                if pending_start.is_some() {
                    return Err("element started before prior tag closed".into());
                }
                pending_start = Some(local.as_str().to_string());
            }
            Token::Attribute { .. } if pending_start.is_none() => {
                return Err("attribute outside element start".into());
            }
            Token::Attribute { .. } => {}
            Token::ElementEnd {
                end: ElementEnd::Open,
                ..
            } => {
                let name = pending_start
                    .take()
                    .ok_or_else(|| "open element without element start".to_string())?;
                if stack.is_empty() {
                    roots = roots.saturating_add(1);
                }
                stack.push(name);
            }
            Token::ElementEnd {
                end: ElementEnd::Empty,
                ..
            } => {
                pending_start
                    .take()
                    .ok_or_else(|| "empty element without element start".to_string())?;
                if stack.is_empty() {
                    roots = roots.saturating_add(1);
                }
            }
            Token::ElementEnd {
                end: ElementEnd::Close(_, local),
                ..
            } => {
                if pending_start.is_some() {
                    return Err("closing element before prior tag closed".into());
                }
                let opened = stack
                    .pop()
                    .ok_or_else(|| "closing element without open element".to_string())?;
                if opened != local.as_str() {
                    return Err("mismatched closing element".into());
                }
            }
            Token::Text { text } | Token::Cdata { text, .. }
                if stack.is_empty() && !text.as_str().trim().is_empty() =>
            {
                return Err("text outside root element".into());
            }
            _ => {}
        }
        if roots > 1 {
            return Err("document has more than one root element".into());
        }
    }
    if pending_start.is_some() || !stack.is_empty() || roots != 1 {
        return Err("document must have one balanced root element".into());
    }
    Ok(())
}

pub fn assert_well_formed(name: &str, xml: &str) {
    assert!(
        is_well_formed(xml).is_ok(),
        "{name} must remain well-formed XML"
    );
}
