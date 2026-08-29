use anyhow::{Context, Result, bail};
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, Lit, Meta, Token};

pub(super) fn rename_all(attrs: &[Attribute]) -> Result<Option<String>> {
    let rule = serde_name_value(attrs, "rename_all")?;
    if let Some(rule) = &rule {
        validate_rename_rule(rule)?;
    }
    Ok(rule)
}

pub(super) fn rename_all_fields(attrs: &[Attribute]) -> Result<Option<String>> {
    let rule = serde_name_value(attrs, "rename_all_fields")?;
    if let Some(rule) = &rule {
        validate_rename_rule(rule)?;
    }
    Ok(rule)
}

/// Returns the exact serialized discriminator key. Serde tag values are literals,
/// rather than identifiers that participate in a container naming rule.
pub(super) fn serde_tag(attrs: &[Attribute]) -> Result<Option<String>> {
    serde_name_value(attrs, "tag")
}

/// Resolves a field or variant's public wire name.
///
/// An explicit literal `rename` has priority over the surrounding container rule.
/// Directional metadata and aliases intentionally have no one-wire-name equivalent,
/// so this generator refuses them rather than choosing a partial view of the contract.
pub(super) fn effective_name(
    attrs: &[Attribute],
    rust_name: &str,
    container_rule: Option<&str>,
) -> Result<String> {
    validate_naming_metadata(attrs)?;
    if let Some(name) = serde_name_value(attrs, "rename")? {
        return Ok(name);
    }
    apply_rename(rust_name, container_rule)
}

pub(super) fn serde_string_value(attrs: &[Attribute], key: &str) -> Option<String> {
    let mut found = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            let is_target = meta.path.is_ident(key);
            if let Ok(value) = meta.value()
                && let Ok(syn::Lit::Str(text)) = value.parse::<syn::Lit>()
                && is_target
            {
                found = Some(text.value());
            }
            Ok(())
        });
        if found.is_some() {
            break;
        }
    }
    found
}

pub(super) fn is_skipped(attrs: &[Attribute]) -> bool {
    let mut skipped = false;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") || meta.path.is_ident("skip_serializing") {
                skipped = true;
            }
            Ok(())
        });
    }
    skipped
}

pub(super) fn skips_when_none(attrs: &[Attribute]) -> bool {
    serde_string_value(attrs, "skip_serializing_if").is_some_and(|value| value == "Option::is_none")
}

pub(super) fn apply_rename(name: &str, rule: Option<&str>) -> Result<String> {
    match rule {
        None => Ok(name.to_string()),
        Some("camelCase") => Ok(to_camel_case(name)),
        Some("snake_case") => Ok(to_snake_case(name)),
        Some("kebab-case") => Ok(to_snake_case(name).replace('_', "-")),
        Some(other) => bail!("unsupported serde rename rule: {other}"),
    }
}

fn validate_naming_metadata(attrs: &[Attribute]) -> Result<()> {
    if serde_name_value(attrs, "alias")?.is_some() {
        bail!("serde alias metadata is unsupported for generated browser contracts");
    }
    let _ = serde_name_value(attrs, "rename")?;
    let _ = rename_all(attrs)?;
    let _ = rename_all_fields(attrs)?;
    Ok(())
}

fn serde_name_value(attrs: &[Attribute], key: &str) -> Result<Option<String>> {
    let mut found = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let Meta::List(list) = &attr.meta else {
            continue;
        };
        let nested = list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .with_context(|| format!("parsing serde metadata for {key}"))?;
        for meta in nested {
            if !meta.path().is_ident(key) {
                continue;
            }
            let Meta::NameValue(name_value) = meta else {
                bail!("directional or nonliteral serde {key} metadata is unsupported");
            };
            let Expr::Lit(expression) = name_value.value else {
                bail!("serde {key} metadata must use a string literal");
            };
            let Lit::Str(value) = expression.lit else {
                bail!("serde {key} metadata must use a string literal");
            };
            if found.replace(value.value()).is_some() {
                bail!("multiple serde {key} metadata values are unsupported");
            }
        }
    }
    Ok(found)
}

fn validate_rename_rule(rule: &str) -> Result<()> {
    match rule {
        "camelCase" | "snake_case" | "kebab-case" => Ok(()),
        _ => bail!("unsupported serde rename rule: {rule}"),
    }
}

fn to_camel_case(name: &str) -> String {
    let snake = to_snake_case(name);
    let mut out = String::with_capacity(snake.len());
    let mut capitalize_next = false;
    for character in snake.chars() {
        if character == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            out.extend(character.to_uppercase());
            capitalize_next = false;
        } else {
            out.push(character);
        }
    }
    out
}

fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() {
            if index != 0 {
                out.push('_');
            }
            out.extend(character.to_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}
