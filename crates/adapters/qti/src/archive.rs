//! Bounded ZIP extraction for hostile QTI package inputs.
//!
//! This module owns archive-wide limits and archive-entry safety. Callers keep
//! their profile-specific grammar in the `entry_allowed` predicate; this helper
//! never broadens that grammar.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use zip::ZipArchive;

use crate::model::{QtiImportError, QtiImportLimits};

/// Entries admitted by one bounded archive read.
///
/// The private map prevents later profile parsers from bypassing the archive
/// reader's validation by constructing an equivalent-looking entry set.
#[derive(Debug)]
pub(crate) struct BoundedArchiveEntries(BTreeMap<String, Vec<u8>>);

impl BoundedArchiveEntries {
    pub(crate) fn get(&self, path: &str) -> Option<&[u8]> {
        self.0.get(path).map(Vec::as_slice)
    }

    pub(crate) fn paths(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

/// Refuses absolute, ambiguous, and traversal-bearing package references.
///
/// Callers use this after extracting an XML attribute, before treating it as
/// an archive member name.
pub(crate) fn validate_relative_reference(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        Err("reference must be a nonempty relative slash-separated path without traversal".into())
    } else {
        Ok(())
    }
}

/// Reads one ZIP archive after applying its size, path, link, and expansion
/// limits. `entry_allowed` is called only for non-directory entries after
/// path and symlink validation.
pub(crate) fn read_bounded_archive(
    bytes: &[u8],
    limits: QtiImportLimits,
    entry_allowed: impl Fn(&str) -> bool,
) -> Result<BoundedArchiveEntries, QtiImportError> {
    if bytes.len() > limits.max_archive_bytes {
        return Err(QtiImportError::ArchiveTooLarge {
            actual: bytes.len(),
            limit: limits.max_archive_bytes,
        });
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| QtiImportError::InvalidArchive(error.to_string()))?;
    if archive.len() > limits.max_entries {
        return Err(QtiImportError::UnsafeEntry {
            path: "<archive>".into(),
            reason: format!(
                "contains {} entries; limit is {}",
                archive.len(),
                limits.max_entries
            ),
        });
    }

    let mut entries = BTreeMap::new();
    let mut expanded = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| QtiImportError::InvalidArchive(error.to_string()))?;
        let path = entry.name().to_string();
        validate_entry_path(&path, &entry, &entry_allowed)?;
        if entry.size() > limits.max_file_bytes {
            return Err(QtiImportError::UnsafeEntry {
                path,
                reason: format!(
                    "expanded size {} exceeds per-file limit {}",
                    entry.size(),
                    limits.max_file_bytes
                ),
            });
        }
        if entry.is_dir() {
            continue;
        }

        let mut contents = Vec::new();
        entry
            .by_ref()
            .take(limits.max_file_bytes.saturating_add(1))
            .read_to_end(&mut contents)
            .map_err(|error| QtiImportError::InvalidArchive(error.to_string()))?;
        let actual = u64::try_from(contents.len()).map_err(|_| QtiImportError::UnsafeEntry {
            path: path.clone(),
            reason: "expanded entry length overflow".into(),
        })?;
        if actual > limits.max_file_bytes {
            return Err(QtiImportError::UnsafeEntry {
                path,
                reason: format!(
                    "expanded size {actual} exceeds per-file limit {}",
                    limits.max_file_bytes
                ),
            });
        }
        expanded = expanded
            .checked_add(actual)
            .ok_or_else(|| QtiImportError::UnsafeEntry {
                path: "<archive>".into(),
                reason: "expanded size overflow".into(),
            })?;
        if expanded > limits.max_expanded_bytes {
            return Err(QtiImportError::UnsafeEntry {
                path,
                reason: format!(
                    "expanded archive size {expanded} exceeds limit {}",
                    limits.max_expanded_bytes
                ),
            });
        }
        if entries.insert(path.clone(), contents).is_some() {
            return Err(QtiImportError::UnsafeEntry {
                path,
                reason: "duplicate entry path".into(),
            });
        }
    }
    Ok(BoundedArchiveEntries(entries))
}

fn validate_entry_path(
    path: &str,
    entry: &zip::read::ZipFile<'_, Cursor<&[u8]>>,
    entry_allowed: &impl Fn(&str) -> bool,
) -> Result<(), QtiImportError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(QtiImportError::UnsafeEntry {
            path: path.into(),
            reason: "path must be a nonempty relative slash-separated path".into(),
        });
    }
    if entry.enclosed_name().is_none() {
        return Err(QtiImportError::UnsafeEntry {
            path: path.into(),
            reason: "path escapes extraction root".into(),
        });
    }
    if entry
        .unix_mode()
        .is_some_and(|mode| mode & 0o170000 == 0o120000)
    {
        return Err(QtiImportError::UnsafeEntry {
            path: path.into(),
            reason: "symbolic links are not accepted".into(),
        });
    }
    if !entry.is_dir() && !entry_allowed(path) {
        return Err(QtiImportError::UnsafeEntry {
            path: path.into(),
            reason: "entry is outside supported manifest/items/assets layout".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn accepts_only_the_callers_narrow_entry_grammar() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("vendor/item.xml", zip::write::SimpleFileOptions::default())
            .expect("start test entry");
        std::io::Write::write_all(&mut writer, b"fixture").expect("write test entry");
        let bytes = writer.finish().expect("finish archive").into_inner();

        let accepted = read_bounded_archive(&bytes, QtiImportLimits::default(), |path| {
            path.starts_with("vendor/")
        })
        .expect("vendor parser grammar admits its entry");
        assert_eq!(accepted.get("vendor/item.xml"), Some(&b"fixture"[..]));

        let error = read_bounded_archive(&bytes, QtiImportLimits::default(), |_| false)
            .expect_err("a parser cannot read entries outside its grammar");
        assert!(matches!(error, QtiImportError::UnsafeEntry { .. }));
    }
}
