use std::collections::BTreeSet;
use std::fmt::Write as _;

use anyhow::{Result, bail};
use syn::{Attribute, Expr, Fields, Lit, Meta, Type};

use super::serde::{
    effective_name, is_skipped, rename_all, rename_all_fields, serde_string_value, serde_tag,
    skips_when_none,
};

const PRINT_WIDTH: usize = 100;

pub(super) struct Generated {
    pub(super) name: String,
    pub(super) dependencies: BTreeSet<String>,
    pub(super) docs: Vec<String>,
    pub(super) body: String,
}

pub(super) fn doc_lines(attrs: &[Attribute]) -> Vec<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        let Meta::NameValue(name_value) = &attr.meta else {
            continue;
        };
        let Expr::Lit(literal) = &name_value.value else {
            continue;
        };
        let Lit::Str(text) = &literal.lit else {
            continue;
        };
        let line = text.value();
        let trimmed = line.trim();
        if trimmed.starts_with("# Examples") || trimmed.starts_with("# Example") {
            break;
        }
        lines.push(line.trim_end().to_string());
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    lines
}

fn map_type(rust_type: &Type, dependencies: &mut BTreeSet<String>) -> Result<String> {
    let Type::Path(path) = rust_type else {
        bail!("unsupported type shape: only path types are mapped");
    };
    let Some(segment) = path.path.segments.last() else {
        bail!("type path with no segments");
    };
    let name = segment.ident.to_string();
    let arguments: Vec<&Type> = match &segment.arguments {
        syn::PathArguments::AngleBracketed(bracketed) => bracketed
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(inner) => Some(inner),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let mapped = match name.as_str() {
        "String" | "str" | "Uuid" | "PathBuf" => "string".to_string(),
        "bool" => "boolean".to_string(),
        "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" | "f32"
        | "f64" | "NonZeroU32" | "NonZeroU64" => "number".to_string(),
        "Option" => format!(
            "{} | null",
            first_argument(&arguments, "Option", dependencies)?
        ),
        "Box" => first_argument(&arguments, "Box", dependencies)?,
        "Vec" | "BTreeSet" | "HashSet" | "VecDeque" => format!(
            "Array<{}>",
            first_argument(&arguments, &name, dependencies)?
        ),
        "BTreeMap" | "HashMap" => {
            if arguments.len() != 2 {
                bail!("{name} needs two type arguments");
            }
            let key = map_type(arguments[0], dependencies)?;
            let value = map_type(arguments[1], dependencies)?;
            format!("Record<{key}, {value}>")
        }
        _ => {
            dependencies.insert(name.clone());
            name
        }
    };
    Ok(mapped)
}
fn first_argument(
    arguments: &[&Type],
    container: &str,
    dependencies: &mut BTreeSet<String>,
) -> Result<String> {
    let Some(inner) = arguments.first() else {
        bail!("{container} needs one type argument");
    };
    map_type(inner, dependencies)
}

pub(super) fn generate_struct(item: &syn::ItemStruct) -> Result<Generated> {
    let mut dependencies = BTreeSet::new();
    let rule = rename_all(&item.attrs)?;
    let body = match &item.fields {
        Fields::Unnamed(fields)
            if fields.unnamed.len() == 1
                && serde_string_value(&item.attrs, "into").as_deref() == Some("String") =>
        {
            "string".to_string()
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            map_type(&fields.unnamed[0].ty, &mut dependencies)?
        }
        Fields::Named(fields) => {
            let mut lines = Vec::new();
            for field in &fields.named {
                if is_skipped(&field.attrs) {
                    continue;
                }
                let Some(ident) = &field.ident else {
                    bail!("named field without an identifier");
                };
                let name = effective_name(&field.attrs, &ident.to_string(), rule.as_deref())?;
                let mapped = map_type(&field.ty, &mut dependencies)?;
                let optional = skips_when_none(&field.attrs);
                let mapped = if optional {
                    option_inner_type(&field.ty, &mut dependencies)?
                } else {
                    mapped
                };
                let marker = if optional { "?" } else { "" };
                lines.push(format!("  {}{marker}: {mapped};", property_key(&name)));
            }
            if lines.is_empty() {
                "Record<string, never>".to_string()
            } else {
                format!("{{\n{}\n}}", lines.join("\n"))
            }
        }
        Fields::Unit => "Record<string, never>".to_string(),
        Fields::Unnamed(_) => bail!("tuple structs with more than one field are not mapped"),
    };
    Ok(Generated {
        name: item.ident.to_string(),
        dependencies,
        docs: doc_lines(&item.attrs),
        body,
    })
}
fn option_inner_type(rust_type: &Type, dependencies: &mut BTreeSet<String>) -> Result<String> {
    let Type::Path(path) = rust_type else {
        bail!("skip_serializing_if Option::is_none needs Option<T>");
    };
    let Some(segment) = path.path.segments.last() else {
        bail!("skip_serializing_if Option::is_none needs Option<T>");
    };
    if segment.ident != "Option" {
        bail!("skip_serializing_if Option::is_none needs Option<T>");
    }
    let arguments: Vec<&Type> = match &segment.arguments {
        syn::PathArguments::AngleBracketed(bracketed) => bracketed
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(inner) => Some(inner),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    first_argument(&arguments, "Option", dependencies)
}

pub(super) fn generate_enum(item: &syn::ItemEnum) -> Result<Generated> {
    let mut dependencies = BTreeSet::new();
    let rule = rename_all(&item.attrs)?;
    let field_rule = rename_all_fields(&item.attrs)?;
    let tag = serde_tag(&item.attrs)?;
    let mut members = Vec::new();
    for variant in &item.variants {
        if is_skipped(&variant.attrs) {
            continue;
        }
        let name = effective_name(&variant.attrs, &variant.ident.to_string(), rule.as_deref())?;
        match (&variant.fields, &tag) {
            (Fields::Unit, None) => members.push(wire_string_literal(&name)),
            (Fields::Unnamed(fields), None) if fields.unnamed.len() == 1 => {
                let mapped = map_type(&fields.unnamed[0].ty, &mut dependencies)?;
                members.push(format!("{{ {}: {mapped} }}", property_key(&name)));
            }
            (Fields::Named(fields), None) => {
                let mut lines = Vec::new();
                for field in &fields.named {
                    if is_skipped(&field.attrs) {
                        continue;
                    }
                    let Some(ident) = &field.ident else {
                        bail!("named field without an identifier");
                    };
                    let field_name =
                        effective_name(&field.attrs, &ident.to_string(), field_rule.as_deref())?;
                    let optional = skips_when_none(&field.attrs);
                    let mapped = if optional {
                        option_inner_type(&field.ty, &mut dependencies)?
                    } else {
                        map_type(&field.ty, &mut dependencies)?
                    };
                    let marker = if optional { "?" } else { "" };
                    lines.push(format!(
                        "    {}{marker}: {mapped};",
                        property_key(&field_name)
                    ));
                }
                members.push(format!(
                    "{{ {}: {{\n{}\n  }} }}",
                    property_key(&name),
                    lines.join("\n")
                ));
            }
            (Fields::Unit, Some(tag_name)) => members.push(format!(
                "{{ {}: {} }}",
                property_key(tag_name),
                wire_string_literal(&name)
            )),
            (Fields::Unnamed(fields), Some(tag_name)) if fields.unnamed.len() == 1 => {
                let mapped = map_type(&fields.unnamed[0].ty, &mut dependencies)?;
                members.push(format!(
                    "{{ {}: {} }} & {mapped}",
                    property_key(tag_name),
                    wire_string_literal(&name)
                ));
            }
            (Fields::Named(fields), Some(tag_name)) => {
                let mut lines = vec![format!(
                    "      {}: {};",
                    property_key(tag_name),
                    wire_string_literal(&name)
                )];
                for field in &fields.named {
                    if is_skipped(&field.attrs) {
                        continue;
                    }
                    let Some(ident) = &field.ident else {
                        bail!("named field without an identifier");
                    };
                    let field_name =
                        effective_name(&field.attrs, &ident.to_string(), field_rule.as_deref())?;
                    let optional = skips_when_none(&field.attrs);
                    let mapped = if optional {
                        option_inner_type(&field.ty, &mut dependencies)?
                    } else {
                        map_type(&field.ty, &mut dependencies)?
                    };
                    let marker = if optional { "?" } else { "" };
                    lines.push(format!(
                        "      {}{marker}: {mapped};",
                        property_key(&field_name)
                    ));
                }
                members.push(format!("{{\n{}\n    }}", lines.join("\n")));
            }
            _ => bail!(
                "variant {}::{} has an unsupported shape",
                item.ident,
                variant.ident
            ),
        }
    }
    Ok(Generated {
        name: item.ident.to_string(),
        dependencies,
        docs: doc_lines(&item.attrs),
        body: join_union(&item.ident.to_string(), &members),
    })
}

fn property_key(name: &str) -> String {
    if is_ascii_typescript_identifier(name) {
        name.to_string()
    } else {
        wire_string_literal(name)
    }
}

fn is_ascii_typescript_identifier(name: &str) -> bool {
    let mut characters = name.bytes();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || matches!(first, b'_' | b'$'))
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, b'_' | b'$'))
}

fn wire_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string to JSON cannot fail")
}

fn join_union(type_name: &str, members: &[String]) -> String {
    let single_line = members.join(" | ");
    let declaration_width = "export type ".len() + type_name.len() + " = ".len() + 1;
    if single_line.len() + declaration_width <= PRINT_WIDTH && !single_line.contains('\n') {
        return single_line;
    }
    if single_line.len() + 3 <= PRINT_WIDTH && !single_line.contains('\n') {
        return format!("\n  {single_line}");
    }
    let mut out = String::new();
    for member in members {
        let _ = write!(out, "\n  | {member}");
    }
    out
}
