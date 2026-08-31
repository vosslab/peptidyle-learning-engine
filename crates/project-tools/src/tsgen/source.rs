use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use syn::{Attribute, Meta, Visibility};

use super::model::Generated;

/// A generated declaration paired with the Rust source file that declared it.
///
/// The generator retains this provenance until it has verified the complete
/// public namespace, so errors identify Rust contracts rather than generated
/// output paths.
pub(super) struct OriginGenerated {
    pub(super) origin: PathBuf,
    pub(super) generated: Generated,
}

/// Recursively discovers production Rust modules below one contract root.
pub(super) fn collect_contract_sources(
    directory: &Path,
    source_paths: &mut Vec<PathBuf>,
) -> Result<()> {
    let entries =
        fs::read_dir(directory).with_context(|| format!("reading {}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("reading an entry in {}", directory.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("reading file type for {}", entry.path().display()))?;
        let path = entry.path();
        if file_type.is_symlink() {
            bail!(
                "refusing symlink in contract source tree: {}",
                path.display()
            );
        }
        if file_type.is_dir() {
            if entry.file_name() != "tests" {
                collect_contract_sources(&path, source_paths)?;
            }
            continue;
        }
        if file_type.is_file()
            && path.extension().is_some_and(|extension| extension == "rs")
            && path.file_stem().is_none_or(|stem| stem != "tests")
        {
            source_paths.push(path);
        }
    }
    Ok(())
}

pub(super) fn is_exported(visibility: &Visibility, attrs: &[Attribute]) -> bool {
    matches!(visibility, Visibility::Public(_)) && derives_serde(attrs) && !is_server_held(attrs)
}

/// `#[doc(hidden)]` marks a public Rust type that is intentionally available
/// to trusted server crates but has no browser contract. Keeping that marker
/// at the declaration prevents a generated TypeScript declaration from making
/// a server-held record appear importable by browser code.
fn is_server_held(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("doc")
            && matches!(&attr.meta, Meta::List(list) if list.tokens.to_string() == "hidden")
    })
}

/// Returns the complete generated declaration namespace after rejecting a
/// duplicate public declaration with both Rust source origins.
pub(super) fn declaration_names(
    declarations: &[OriginGenerated],
) -> Result<std::collections::BTreeSet<String>> {
    let mut origins = std::collections::BTreeMap::new();
    for declaration in declarations {
        let name = &declaration.generated.name;
        if let Some(first_origin) = origins.insert(name.clone(), declaration.origin.clone()) {
            bail!(
                "duplicate public declaration {name} in {} and {}",
                first_origin.display(),
                declaration.origin.display()
            );
        }
    }
    Ok(origins.into_keys().collect())
}

fn derives_serde(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("Serialize") || meta.path.is_ident("Deserialize") {
                found = true;
            }
            Ok(())
        });
        if found {
            return true;
        }
    }
    false
}
