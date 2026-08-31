//! Generates TypeScript definitions from the Rust question model.
//!
//! This facade owns the explicit-contract-roots generator operation. Source discovery, Serde
//! metadata, declaration modelling, and output rendering each remain in their
//! focused internal modules.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use syn::{Fields, Item, Type};

use model::{Generated, doc_lines, generate_enum, generate_struct};
use output::{prepare_out_dir, render};
use source::{OriginGenerated, collect_contract_sources, declaration_names, is_exported};

mod model;
mod output;
mod serde;
mod source;

/// Reads every production `.rs` file under the explicit `contract_roots` and
/// writes TypeScript into `out_dir`.
///
/// Returns the number of types generated.
pub fn run(contract_roots: &[&Path], out_dir: &Path) -> Result<usize> {
    if contract_roots.is_empty() {
        bail!("at least one contract root is required");
    }
    let generated = generate_declarations(contract_roots)?;
    let generated_declaration_names = declaration_names(&generated)?;
    prepare_out_dir(out_dir)?;
    for declaration in &generated {
        let path = out_dir.join(format!("{}.ts", declaration.generated.name));
        fs::write(
            &path,
            render(&declaration.generated, &generated_declaration_names),
        )
        .with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(generated.len())
}

/// Parses generated declarations before any output directory is changed.
///
/// Every root is supplied by the caller, keeping discovery independent from
/// the application-owned default contract-root list.
fn generate_declarations(contract_roots: &[&Path]) -> Result<Vec<OriginGenerated>> {
    let mut generated = Vec::new();
    let mut source_paths = Vec::new();
    for contract_root in contract_roots {
        collect_contract_sources(contract_root, &mut source_paths)?;
    }
    source_paths.sort();
    for path in &source_paths {
        let source =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let parsed =
            syn::parse_file(&source).with_context(|| format!("parsing {}", path.display()))?;
        let manually_serialized = manually_serialized_public_types(&parsed.items);
        for item in parsed.items {
            let declaration = match item {
                Item::Struct(item)
                    if is_exported(&item.vis, &item.attrs)
                        || manually_serialized.contains(&item.ident.to_string()) =>
                {
                    Some(
                        generate_struct(&item)
                            .with_context(|| format!("generating {}", item.ident))?,
                    )
                }
                Item::Enum(item)
                    if is_exported(&item.vis, &item.attrs)
                        || manually_serialized.contains(&item.ident.to_string()) =>
                {
                    Some(
                        generate_enum(&item)
                            .with_context(|| format!("generating {}", item.ident))?,
                    )
                }
                Item::Const(item)
                    if matches!(item.vis, syn::Visibility::Public(_))
                        && matches!(*item.ty, syn::Type::Path(ref path) if path.path.is_ident("usize") || path.path.is_ident("u32"))
                        && matches!(
                            &*item.expr,
                            syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Int(_),
                                ..
                            })
                        ) =>
                {
                    let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(value),
                        ..
                    }) = &*item.expr
                    else {
                        unreachable!()
                    };
                    Some(Generated {
                        name: item.ident.to_string(),
                        dependencies: BTreeSet::new(),
                        docs: doc_lines(&item.attrs),
                        body: format!("__CONST__ {}", value.base10_digits()),
                    })
                }
                _ => None,
            };
            if let Some(generated_declaration) = declaration {
                generated.push(OriginGenerated {
                    origin: path.clone(),
                    generated: generated_declaration,
                });
            }
        }
    }
    Ok(generated)
}

/// Finds transparent collection wrappers whose hand-written Serde
/// implementations preserve their single inner wire value.
///
/// A named struct with manual serialization may construct a private wire DTO
/// and therefore requires an explicit browser projection instead. Tuple
/// wrappers are the safe structural case: their one owned field is the exact
/// serialized value and can be emitted beside a public record that names it.
fn manually_serialized_public_types(items: &[Item]) -> BTreeSet<String> {
    let mut manually_serialized = BTreeSet::new();
    for item in items {
        let Item::Impl(implementation) = item else {
            continue;
        };
        let Some((trait_path, _)) = &implementation.trait_ else {
            continue;
        };
        let Some(trait_name) = trait_path.segments.last() else {
            continue;
        };
        if trait_name.ident != "Serialize" && trait_name.ident != "Deserialize" {
            continue;
        }
        let Type::Path(self_type) = implementation.self_ty.as_ref() else {
            continue;
        };
        let Some(type_name) = self_type.path.segments.last() else {
            continue;
        };
        manually_serialized.insert(type_name.ident.to_string());
    }
    items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item)
                if matches!(&item.fields, Fields::Unnamed(fields) if fields.unnamed.len() == 1)
                    && manually_serialized.contains(&item.ident.to_string()) =>
            {
                Some(item.ident.to_string())
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod blueprint_course_tests;

#[cfg(test)]
mod tests;
