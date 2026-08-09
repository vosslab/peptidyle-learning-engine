//! Bounded XML tree construction for hostile QTI package inputs.

use std::collections::BTreeMap;

use xmlparser::{ElementEnd, Token, Tokenizer};

use crate::model::{QtiImportError, QtiImportLimits};

const XML_NAMESPACE_URI: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS_NAMESPACE_URI: &str = "http://www.w3.org/2000/xmlns/";

#[allow(dead_code)] // The profile parser consumes this crate-private foundation next.
#[derive(Debug, Clone)]
pub(crate) struct XmlNode {
    name: String,
    prefix: Option<String>,
    namespace_uri: Option<String>,
    namespace_bindings: BTreeMap<String, String>,
    attrs: BTreeMap<String, String>,
    attribute_details: Vec<XmlAttribute>,
    children: Vec<XmlNode>,
    content: Vec<XmlContent>,
    text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct XmlAttribute {
    local_name: String,
    prefix: Option<String>,
    namespace_uri: Option<String>,
    value: String,
}

#[allow(dead_code)] // Future profile parsers need namespace-aware attribute inspection.
impl XmlAttribute {
    pub(crate) fn local_name(&self) -> &str {
        &self.local_name
    }

    pub(crate) fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    pub(crate) fn namespace_uri(&self) -> Option<&str> {
        self.namespace_uri.as_deref()
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}

#[allow(dead_code)] // The ordered stream becomes live with strict markup projection.
#[derive(Debug, Clone)]
enum XmlContent {
    Text(String),
    Cdata(String),
    Child(usize),
    Comment(String),
    ProcessingInstruction {
        target: String,
        content: Option<String>,
    },
}

#[allow(dead_code)] // The ordered stream is intentionally crate-private.
#[derive(Debug, Clone, Copy)]
pub(crate) enum XmlContentRef<'a> {
    Text(&'a str),
    Cdata(&'a str),
    Child(usize),
    Comment(&'a str),
    ProcessingInstruction {
        target: &'a str,
        content: Option<&'a str>,
    },
}

#[allow(dead_code)] // Future profile parsers consume namespace and ordered-content accessors.
impl XmlNode {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    pub(crate) fn namespace_uri(&self) -> Option<&str> {
        self.namespace_uri.as_deref()
    }

    pub(crate) fn attribute(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }

    pub(crate) fn attributes(&self) -> impl Iterator<Item = &XmlAttribute> {
        self.attribute_details.iter()
    }

    pub(crate) fn children(&self) -> &[XmlNode] {
        &self.children
    }

    pub(crate) fn content(&self) -> impl Iterator<Item = XmlContentRef<'_>> {
        self.content.iter().map(|content| match content {
            XmlContent::Text(value) => XmlContentRef::Text(value),
            XmlContent::Cdata(value) => XmlContentRef::Cdata(value),
            XmlContent::Child(index) => XmlContentRef::Child(*index),
            XmlContent::Comment(value) => XmlContentRef::Comment(value),
            XmlContent::ProcessingInstruction { target, content } => {
                XmlContentRef::ProcessingInstruction {
                    target,
                    content: content.as_deref(),
                }
            }
        })
    }

    pub(crate) fn descendants_named(&self, name: &str) -> Vec<&XmlNode> {
        let mut found = Vec::new();
        let mut pending = vec![self];
        while let Some(current) = pending.pop() {
            if current.name == name {
                found.push(current);
            }
            pending.extend(current.children.iter().rev());
        }
        found
    }

    pub(crate) fn contains_named(&self, name: &str) -> bool {
        let mut pending = vec![self];
        while let Some(current) = pending.pop() {
            if current.name == name {
                return true;
            }
            pending.extend(current.children.iter().rev());
        }
        false
    }

    pub(crate) fn normalized_text_except(&self, excluded_name: &str) -> String {
        let mut pieces = Vec::new();
        let mut pending = vec![self];
        while let Some(current) = pending.pop() {
            if current.name != excluded_name {
                pieces.push(current.text.as_str());
                pending.extend(current.children.iter().rev());
            }
        }
        pieces
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug)]
struct NodeStart {
    name: String,
    prefix: Option<String>,
    attrs: BTreeMap<String, String>,
    attribute_details: Vec<PendingAttribute>,
}

#[derive(Debug)]
struct PendingAttribute {
    local_name: String,
    prefix: Option<String>,
    value: String,
}

/// Parses XML through `xmlparser`, refusing every DTD/entity token and
/// validating balanced nesting ourselves (a documented tokenizer limitation).
pub(crate) fn parse_xml(
    path: &str,
    bytes: &[u8],
    limits: QtiImportLimits,
) -> Result<XmlNode, QtiImportError> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid_xml(path, "XML is not UTF-8"))?;
    let mut roots = Vec::new();
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut pending: Option<NodeStart> = None;
    let mut token_count = 0_usize;
    let mut node_count = 0_usize;
    for token in Tokenizer::from(text) {
        token_count = token_count.saturating_add(1);
        if token_count > limits.max_xml_tokens {
            return Err(xml_resource_limit(
                path,
                "token count",
                token_count,
                limits.max_xml_tokens,
            ));
        }
        let token = token.map_err(|error| {
            invalid_xml(path, &format!("XML parser rejected document: {error}"))
        })?;
        match token {
            Token::DtdStart { .. }
            | Token::EmptyDtd { .. }
            | Token::EntityDeclaration { .. }
            | Token::DtdEnd { .. } => {
                return Err(invalid_xml(
                    path,
                    "DOCTYPE and entity declarations are forbidden",
                ));
            }
            Token::ElementStart { prefix, local, .. } => {
                if pending.is_some() {
                    return Err(invalid_xml(path, "element started before prior tag closed"));
                }
                pending = Some(NodeStart {
                    name: local.as_str().to_string(),
                    prefix: span_prefix(prefix.as_str()),
                    attrs: BTreeMap::new(),
                    attribute_details: Vec::new(),
                });
            }
            Token::Attribute {
                prefix,
                local,
                value,
                ..
            } => {
                let Some(start) = pending.as_mut() else {
                    return Err(invalid_xml(path, "attribute outside element start"));
                };
                let value = xml_unescape(value.as_str());
                if start
                    .attrs
                    .insert(local.as_str().to_string(), value.clone())
                    .is_some()
                {
                    return Err(invalid_xml(path, "duplicate attribute"));
                }
                start.attribute_details.push(PendingAttribute {
                    local_name: local.as_str().to_string(),
                    prefix: span_prefix(prefix.as_str()),
                    value,
                });
            }
            Token::ElementEnd {
                end: ElementEnd::Open,
                ..
            } => {
                node_count = node_count.saturating_add(1);
                ensure_xml_node_limits(path, &stack, node_count, limits)?;
                let namespace_bindings = inherited_namespaces(&stack);
                stack.push(node_from_pending(path, &mut pending, namespace_bindings)?);
            }
            Token::ElementEnd {
                end: ElementEnd::Empty,
                ..
            } => {
                node_count = node_count.saturating_add(1);
                ensure_xml_node_limits(path, &stack, node_count, limits)?;
                let namespace_bindings = inherited_namespaces(&stack);
                append_node(
                    path,
                    &mut stack,
                    &mut roots,
                    node_from_pending(path, &mut pending, namespace_bindings)?,
                )?;
            }
            Token::ElementEnd {
                end: ElementEnd::Close(_, local),
                ..
            } => {
                if pending.is_some() {
                    return Err(invalid_xml(path, "closing element before prior tag closed"));
                }
                let node = stack
                    .pop()
                    .ok_or_else(|| invalid_xml(path, "closing element without open element"))?;
                if node.name != local.as_str() {
                    return Err(invalid_xml(path, "mismatched closing element"));
                }
                append_node(path, &mut stack, &mut roots, node)?;
            }
            Token::Text { text } => append_text(path, &mut stack, text.as_str(), false)?,
            Token::Cdata { text, .. } => append_text(path, &mut stack, text.as_str(), true)?,
            Token::Comment { text, .. } => {
                if let Some(node) = stack.last_mut() {
                    node.content
                        .push(XmlContent::Comment(text.as_str().to_string()));
                }
            }
            Token::ProcessingInstruction {
                target, content, ..
            } => {
                if let Some(node) = stack.last_mut() {
                    node.content.push(XmlContent::ProcessingInstruction {
                        target: target.as_str().to_string(),
                        content: content.map(|value| value.as_str().to_string()),
                    });
                }
            }
            Token::Declaration { .. } => {}
        }
    }
    if pending.is_some() || !stack.is_empty() || roots.len() != 1 {
        return Err(invalid_xml(
            path,
            "document must have one balanced root element",
        ));
    }
    Ok(roots.remove(0))
}

fn invalid_xml(path: &str, reason: &str) -> QtiImportError {
    QtiImportError::InvalidXml {
        path: path.into(),
        reason: reason.into(),
    }
}

fn xml_resource_limit(path: &str, resource: &str, actual: usize, limit: usize) -> QtiImportError {
    invalid_xml(
        path,
        &format!("XML resource limit exceeded: {resource} {actual} exceeds limit {limit}"),
    )
}

fn ensure_xml_node_limits(
    path: &str,
    stack: &[XmlNode],
    node_count: usize,
    limits: QtiImportLimits,
) -> Result<(), QtiImportError> {
    let depth = stack.len().saturating_add(1);
    if depth > limits.max_xml_depth {
        return Err(xml_resource_limit(
            path,
            "element depth",
            depth,
            limits.max_xml_depth,
        ));
    }
    if node_count > limits.max_xml_nodes {
        return Err(xml_resource_limit(
            path,
            "element node count",
            node_count,
            limits.max_xml_nodes,
        ));
    }
    Ok(())
}

fn node_from_pending(
    path: &str,
    pending: &mut Option<NodeStart>,
    mut namespace_bindings: BTreeMap<String, String>,
) -> Result<XmlNode, QtiImportError> {
    let start = pending
        .take()
        .ok_or_else(|| invalid_xml(path, "element end without start"))?;
    for attribute in &start.attribute_details {
        if let Some(namespace_prefix) = namespace_declaration_prefix(attribute) {
            if attribute.value.is_empty() {
                namespace_bindings.remove(namespace_prefix);
            } else {
                namespace_bindings.insert(namespace_prefix.to_string(), attribute.value.clone());
            }
        }
    }
    let namespace_uri = namespace_for_element(&namespace_bindings, start.prefix.as_deref());
    let attribute_details = start
        .attribute_details
        .into_iter()
        .map(|attribute| XmlAttribute {
            namespace_uri: namespace_for_attribute(&namespace_bindings, &attribute),
            local_name: attribute.local_name,
            prefix: attribute.prefix,
            value: attribute.value,
        })
        .collect();
    Ok(XmlNode {
        name: start.name,
        prefix: start.prefix,
        namespace_uri,
        namespace_bindings,
        attrs: start.attrs,
        attribute_details,
        children: Vec::new(),
        content: Vec::new(),
        text: String::new(),
    })
}

fn inherited_namespaces(stack: &[XmlNode]) -> BTreeMap<String, String> {
    let mut namespaces = BTreeMap::from([(String::from("xml"), String::from(XML_NAMESPACE_URI))]);
    if let Some(parent) = stack.last() {
        namespaces.extend(parent.namespace_bindings.clone());
    }
    namespaces
}

fn append_node(
    path: &str,
    stack: &mut [XmlNode],
    roots: &mut Vec<XmlNode>,
    node: XmlNode,
) -> Result<(), QtiImportError> {
    if let Some(parent) = stack.last_mut() {
        let child_index = parent.children.len();
        parent.children.push(node);
        parent.content.push(XmlContent::Child(child_index));
    } else {
        roots.push(node);
        if roots.len() > 1 {
            return Err(invalid_xml(path, "document has multiple root elements"));
        }
    }
    Ok(())
}

fn append_text(
    path: &str,
    stack: &mut [XmlNode],
    raw: &str,
    is_cdata: bool,
) -> Result<(), QtiImportError> {
    if let Some(node) = stack.last_mut() {
        let legacy_value = xml_unescape(raw);
        node.text.push_str(&legacy_value);
        node.content.push(if is_cdata {
            XmlContent::Cdata(raw.to_string())
        } else {
            XmlContent::Text(legacy_value)
        });
        return Ok(());
    }
    if !raw.trim().is_empty() {
        return Err(invalid_xml(path, "text outside root element"));
    }
    Ok(())
}

fn span_prefix(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn namespace_declaration_prefix(attribute: &PendingAttribute) -> Option<&str> {
    namespace_declaration_prefix_from_parts(attribute.prefix.as_deref(), &attribute.local_name)
}

fn namespace_declaration_prefix_from_parts<'a>(
    prefix: Option<&str>,
    local_name: &'a str,
) -> Option<&'a str> {
    match (prefix, local_name) {
        (None, "xmlns") => Some(""),
        (Some("xmlns"), prefix) => Some(prefix),
        _ => None,
    }
}

fn namespace_for_element(
    bindings: &BTreeMap<String, String>,
    prefix: Option<&str>,
) -> Option<String> {
    bindings.get(prefix.unwrap_or("")).cloned()
}

fn namespace_for_attribute(
    bindings: &BTreeMap<String, String>,
    attribute: &PendingAttribute,
) -> Option<String> {
    if namespace_declaration_prefix(attribute).is_some() {
        return Some(XMLNS_NAMESPACE_URI.to_string());
    }
    attribute
        .prefix
        .as_deref()
        .and_then(|prefix| bindings.get(prefix))
        .cloned()
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> XmlNode {
        parse_xml("fixture.xml", source.as_bytes(), QtiImportLimits::default()).expect("XML parses")
    }

    #[test]
    fn records_interleaved_text_and_children_in_source_order() {
        let node = parse("<p>a<strong>b</strong>c</p>");
        let content: Vec<_> = node.content().collect();
        assert!(matches!(
            content.as_slice(),
            [
                XmlContentRef::Text("a"),
                XmlContentRef::Child(0),
                XmlContentRef::Text("c")
            ]
        ));
        assert_eq!(node.children()[0].normalized_text_except(""), "b");
    }

    #[test]
    fn resolves_default_and_prefixed_namespaces_through_inheritance() {
        let node = parse(
            "<root xmlns='urn:default' xmlns:q='urn:q'><q:child q:attr='v'><grandchild/></q:child></root>",
        );
        let child = &node.children()[0];
        assert_eq!(node.namespace_uri(), Some("urn:default"));
        assert_eq!(child.prefix(), Some("q"));
        assert_eq!(child.namespace_uri(), Some("urn:q"));
        let attribute = child.attributes().next().expect("one attribute");
        assert_eq!(attribute.namespace_uri(), Some("urn:q"));
        assert_eq!(child.children()[0].namespace_uri(), Some("urn:default"));

        let unresolved = parse("<root><unknown:child/></root>");
        let unresolved_child = &unresolved.children()[0];
        assert_eq!(unresolved_child.prefix(), Some("unknown"));
        assert_eq!(unresolved_child.namespace_uri(), None);
    }

    #[test]
    fn distinguishes_text_cdata_comments_and_processing_instructions() {
        let node = parse("<p>a&amp;<![CDATA[b &amp; c]]><!--c--><?pi data?>d</p>");
        let content: Vec<_> = node.content().collect();
        assert!(matches!(
            content.as_slice(),
            [
                XmlContentRef::Text("a&"),
                XmlContentRef::Cdata("b &amp; c"),
                XmlContentRef::Comment("c"),
                XmlContentRef::ProcessingInstruction {
                    target: "pi",
                    content: Some("data")
                },
                XmlContentRef::Text("d")
            ]
        ));
    }
}
