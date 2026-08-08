//! Server-side allowlist sanitizer for supplied WeBWorK markup.
//!
//! This deliberately keeps a small rendering language.  It is applied to
//! every renderer response before caching, rather than relying on a renderer
//! deployment to claim its HTML was already safe.

use std::fmt::Write as _;

/// Removes executable markup and non-internal resource URLs from renderer HTML.
///
/// Allowed elements cover ordinary explanatory text, tables, MathML, and
/// images served from the platform's asset route.  All event attributes,
/// styles, comments, unknown elements, and malformed tags are discarded.
pub fn sanitize_webwork_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    let mut suppressed_depth = 0_u16;

    while let Some(start) = rest.find('<') {
        if suppressed_depth == 0 {
            escape_text(&rest[..start], &mut output);
        }
        rest = &rest[start..];
        let Some(end) = tag_end(rest) else {
            if suppressed_depth == 0 {
                escape_text(rest, &mut output);
            }
            break;
        };
        let token = &rest[1..end];
        rest = &rest[end + 1..];

        if token.starts_with("!--") {
            continue;
        }
        let parsed = parse_tag(token);
        let Some(ParsedTag {
            closing,
            name,
            attributes,
        }) = parsed
        else {
            continue;
        };
        if is_suppressed(&name) {
            if closing {
                suppressed_depth = suppressed_depth.saturating_sub(1);
            } else {
                suppressed_depth = suppressed_depth.saturating_add(1);
            }
            continue;
        }
        if suppressed_depth != 0 || !allowed_element(&name) {
            continue;
        }
        if closing {
            let _ = write!(output, "</{name}>");
        } else {
            let _ = write!(output, "<{name}");
            for (attribute, value) in attributes {
                if allowed_attribute(&name, &attribute, &value) {
                    let _ = write!(output, " {attribute}=\"");
                    escape_attribute(&value, &mut output);
                    output.push('\"');
                }
            }
            output.push('>');
        }
    }
    output
}

fn tag_end(input: &str) -> Option<usize> {
    let mut quote = None;
    for (offset, character) in input.char_indices().skip(1) {
        match (quote, character) {
            (None, '\"' | '\'') => quote = Some(character),
            (Some(active), next) if active == next => quote = None,
            (None, '>') => return Some(offset),
            _ => {}
        }
    }
    None
}

struct ParsedTag {
    closing: bool,
    name: String,
    attributes: Vec<(String, String)>,
}

fn parse_tag(token: &str) -> Option<ParsedTag> {
    let token = token.trim();
    if token.starts_with('!') || token.starts_with('?') {
        return None;
    }
    let closing = token.starts_with('/');
    let body = token.trim_start_matches('/').trim_end_matches('/').trim();
    let mut split = body.splitn(2, char::is_whitespace);
    let name = split.next()?.to_ascii_lowercase();
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return None;
    }
    let attributes = if closing {
        Vec::new()
    } else {
        parse_attributes(split.next().unwrap_or_default())
    };
    Some(ParsedTag {
        closing,
        name,
        attributes,
    })
}

fn parse_attributes(input: &str) -> Vec<(String, String)> {
    let mut attributes = Vec::new();
    let mut rest = input.trim();
    while !rest.is_empty() {
        let key_end = rest
            .find(|character: char| character.is_whitespace() || character == '=')
            .unwrap_or(rest.len());
        let key = rest[..key_end].to_ascii_lowercase();
        rest = rest[key_end..].trim_start();
        if key.is_empty() || !rest.starts_with('=') {
            break;
        }
        rest = rest[1..].trim_start();
        let (value, remaining) = match rest.chars().next() {
            Some('\"' | '\'') => {
                let quote = rest.chars().next().expect("checked above");
                let content = &rest[quote.len_utf8()..];
                let Some(end) = content.find(quote) else {
                    break;
                };
                (&content[..end], &content[end + quote.len_utf8()..])
            }
            Some(_) => {
                let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
                (&rest[..end], &rest[end..])
            }
            None => break,
        };
        attributes.push((key, value.to_string()));
        rest = remaining.trim_start();
    }
    attributes
}

fn allowed_element(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "span"
            | "br"
            | "strong"
            | "b"
            | "em"
            | "i"
            | "sub"
            | "sup"
            | "ul"
            | "ol"
            | "li"
            | "table"
            | "thead"
            | "tbody"
            | "tr"
            | "th"
            | "td"
            | "caption"
            | "math"
            | "semantics"
            | "mrow"
            | "mi"
            | "mn"
            | "mo"
            | "msup"
            | "msub"
            | "mfrac"
            | "annotation"
            | "img"
    )
}

fn is_suppressed(name: &str) -> bool {
    matches!(
        name,
        "script" | "style" | "iframe" | "object" | "embed" | "svg"
    )
}

fn allowed_attribute(element: &str, attribute: &str, value: &str) -> bool {
    match attribute {
        "title" | "aria-label" | "alt" => true,
        "colspan" | "rowspan" if matches!(element, "th" | "td") => {
            value.parse::<u16>().is_ok_and(|count| count > 0)
        }
        "src" if element == "img" => internal_asset_url(value),
        _ => false,
    }
}

fn internal_asset_url(value: &str) -> bool {
    let Some(asset) = value.strip_prefix("/api/assets/") else {
        return false;
    };
    !asset.is_empty()
        && asset
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn escape_text(value: &str, output: &mut String) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }
}

fn escape_attribute(value: &str, output: &mut String) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '\"' => output.push_str("&quot;"),
            _ => output.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_webwork_html;

    #[test]
    fn hostile_markup_is_reduced_to_browser_safe_allowlisted_html() {
        let sanitized = sanitize_webwork_html(
            r#"<p onclick="steal()">Safe <strong>text</strong></p><script>alert(1)</script><img src="javascript:alert(1)" onerror="steal()"><img src="/api/assets/asset-1" alt="diagram"><iframe src="https://evil.test">nope</iframe><a href="https://evil.test">link</a>"#,
        );
        assert_eq!(
            sanitized,
            r#"<p>Safe <strong>text</strong></p><img><img src="/api/assets/asset-1" alt="diagram">link"#
        );
        assert!(!sanitized.contains("script"));
        assert!(!sanitized.contains("onclick"));
        assert!(!sanitized.contains("javascript:"));
        assert!(!sanitized.contains("evil.test"));
    }
}
