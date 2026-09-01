use super::super::{
    BLACKBOARD_ITEM_NAMESPACE, IMS_CONTENT_PACKAGING_NAMESPACE, QtiProfileResourceEvidence,
};
use crate::xml::{XmlContentRef, XmlNode};

const MANIFEST: &str = "imsmanifest.xml";
const PREFIX: &str = "qti21_items/";

pub(super) fn allowed(path: &str) -> bool {
    path == MANIFEST || (path.starts_with(PREFIX) && path.ends_with(".xml") && normalized(path))
}

pub(super) fn manifest_resources(root: &XmlNode) -> Option<Vec<QtiProfileResourceEvidence>> {
    if root.name() != "manifest"
        || root.namespace_uri()? != IMS_CONTENT_PACKAGING_NAMESPACE
        || !root_attrs(root, &["identifier"])
        || names(root) != ["metadata", "organizations", "resources"]
        || !structural(root)
    {
        return None;
    }
    let metadata = one(root, "metadata")?;
    if metadata.namespace_uri() != Some(IMS_CONTENT_PACKAGING_NAMESPACE)
        || !attrs(metadata, &[])
        || names(metadata) != ["schema", "schemaversion", "lom"]
        || !structural(metadata)
    {
        return None;
    }
    let schema = one(metadata, "schema")?;
    let version = one(metadata, "schemaversion")?;
    if leaf_text(schema, IMS_CONTENT_PACKAGING_NAMESPACE).as_deref() != Some("QTIv2.1")
        || leaf_text(version, IMS_CONTENT_PACKAGING_NAMESPACE).as_deref() != Some("2.0")
    {
        return None;
    }
    let lom = one(metadata, "lom")?;
    // LOM is retained opaque archive metadata. Its nested vocabulary never
    // participates in profile detection, mapping, or grading.
    if lom.namespace_uri() != Some("http://www.imsglobal.org/xsd/imsmd_v1p2") || !structural(lom) {
        return None;
    }
    let organizations = one(root, "organizations")?;
    if organizations.namespace_uri() != Some(IMS_CONTENT_PACKAGING_NAMESPACE)
        || !attrs(organizations, &[])
        || !organizations.children().is_empty()
        || !structural(organizations)
    {
        return None;
    }
    let resources = one(root, "resources")?;
    if resources.namespace_uri() != Some(IMS_CONTENT_PACKAGING_NAMESPACE)
        || !attrs(resources, &[])
        || !structural(resources)
    {
        return None;
    }
    let result = resources
        .children()
        .iter()
        .map(resource)
        .collect::<Option<Vec<_>>>()?;
    (!result.is_empty()).then_some(result)
}

pub(super) fn valid_meta(root: &XmlNode, resources: &[QtiProfileResourceEvidence]) -> bool {
    let items = resources
        .iter()
        .filter(|r| r.resource_type.as_deref() == Some("imsqti_item_xmlv2p1"))
        .collect::<Vec<_>>();
    root.name() == "assessmentTest"
        && root.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
        && root_attrs(root, &["identifier", "title"])
        && root.attribute("identifier") == Some("assessment_meta")
        && names(root) == ["testPart"]
        && structural(root)
        && one(root, "testPart").is_some_and(|part| {
            attrs(part, &["identifier", "navigationMode", "submissionMode"])
                && part.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
                && part.attribute("navigationMode") == Some("nonlinear")
                && part.attribute("submissionMode") == Some("simultaneous")
                && names(part) == ["assessmentSection"]
                && structural(part)
                && one(part, "assessmentSection").is_some_and(|section| {
                    attrs(section, &["identifier", "visible", "title"])
                        && section.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
                        && section.attribute("visible") == Some("false")
                        && structural(section)
                        && section.children().len() == items.len()
                        && section
                            .children()
                            .iter()
                            .zip(items)
                            .all(|(node, resource)| {
                                node.name() == "assessmentItemRef"
                                    && node.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
                                    && attrs(node, &["identifier", "href"])
                                    && node.attribute("identifier")
                                        == Some(resource.identifier.as_str())
                                    && node.attribute("href")
                                        == resource
                                            .href
                                            .as_deref()
                                            .and_then(|href| href.rsplit('/').next())
                                    && node.children().is_empty()
                                    && structural(node)
                            })
                })
        })
}

pub(super) fn valid_item_envelope(item: &XmlNode) -> bool {
    item.name() == "assessmentItem"
        && item.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
        && root_attrs(item, &["identifier", "title", "adaptive", "timeDependent"])
        && item
            .attribute("identifier")
            .is_some_and(|v| !v.trim().is_empty())
        && item.attribute("adaptive") == Some("false")
        && item.attribute("timeDependent") == Some("false")
        && matches!(
            names(item).as_slice(),
            ["responseDeclaration", "itemBody"]
                | ["responseDeclaration", "itemBody", "responseProcessing"]
                | ["responseDeclaration", "outcomeDeclaration", "itemBody"]
                | [
                    "responseDeclaration",
                    "outcomeDeclaration",
                    "itemBody",
                    "responseProcessing"
                ]
        )
        && structural(item)
        && item
            .children()
            .iter()
            .all(|child| child.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE))
        && item.children().iter().all(|child| {
            matches!(
                child.name(),
                "responseDeclaration" | "outcomeDeclaration" | "itemBody" | "responseProcessing"
            )
        })
        && item
            .children()
            .iter()
            .filter(|child| child.name() == "responseDeclaration")
            .count()
            == 1
        && item
            .children()
            .iter()
            .filter(|child| child.name() == "itemBody")
            .count()
            == 1
        && item
            .children()
            .iter()
            .filter(|child| child.name() == "responseProcessing")
            .count()
            <= 1
        && item
            .children()
            .iter()
            .filter(|child| child.name() == "outcomeDeclaration")
            .all(inert_score)
}

pub(super) fn response(item: &XmlNode) -> Option<(String, String)> {
    let declaration = one(item, "responseDeclaration")?;
    if !attrs(declaration, &["identifier", "cardinality", "baseType"])
        || declaration.namespace_uri() != Some(BLACKBOARD_ITEM_NAMESPACE)
        || declaration
            .attribute("identifier")
            .filter(|v| !v.trim().is_empty())
            .is_none()
        || declaration.attribute("cardinality") != Some("single")
        || declaration.attribute("baseType") != Some("identifier")
        || names(declaration) != ["correctResponse"]
        || !structural(declaration)
    {
        return None;
    }
    let correct = one(declaration, "correctResponse")?;
    if correct.namespace_uri() != Some(BLACKBOARD_ITEM_NAMESPACE)
        || !attrs(correct, &[])
        || names(correct) != ["value"]
        || !structural(correct)
    {
        return None;
    }
    let value = one(correct, "value")?;
    let value_text = leaf_text(value, BLACKBOARD_ITEM_NAMESPACE)?;
    (!value_text.is_empty()).then_some((
        declaration.attribute("identifier")?.to_string(),
        value_text.trim().to_string(),
    ))
}

pub(super) fn body_and_interaction(item: &XmlNode) -> Option<(&XmlNode, &XmlNode)> {
    let body = one(item, "itemBody")?;
    if body.namespace_uri() != Some(BLACKBOARD_ITEM_NAMESPACE)
        || !attrs(body, &[])
        || !structural_except_markup(body)
    {
        return None;
    }
    let interactions = body
        .children()
        .iter()
        .filter(|node| node.name() == "choiceInteraction")
        .collect::<Vec<_>>();
    (interactions.len() == 1).then_some((body, interactions[0]))
}

pub(super) fn valid_interaction(interaction: &XmlNode, response: &str) -> bool {
    interaction.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
        && attrs(
            interaction,
            &["responseIdentifier", "maxChoices", "shuffle"],
        )
        && interaction.attribute("responseIdentifier") == Some(response)
        && interaction.attribute("maxChoices") == Some("1")
        && interaction
            .attribute("shuffle")
            .is_none_or(|v| v == "false" || v == "true")
        && structural(interaction)
        && (2..=100).contains(&interaction.children().len())
        && interaction.children().iter().all(|choice| {
            choice.name() == "simpleChoice"
                && choice.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
                && attrs(choice, &["identifier", "fixed"])
                && choice
                    .attribute("identifier")
                    .is_some_and(|v| !v.trim().is_empty())
                && choice
                    .attribute("fixed")
                    .is_none_or(|v| v == "true" || v == "false")
        })
        && (interaction.attribute("shuffle") != Some("true")
            || interaction
                .children()
                .iter()
                .all(|choice| choice.attribute("fixed") == Some("true")))
}

pub(super) fn valid_processing(item: &XmlNode, response: &str) -> bool {
    let processing = item
        .children()
        .iter()
        .find(|node| node.name() == "responseProcessing");
    processing.is_none_or(|node| {
        node.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
            && attrs(node, &[])
            && names(node) == ["responseCondition"]
            && structural(node)
            && one(node, "responseCondition").is_some_and(|condition| {
                condition.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
                    && attrs(condition, &[])
                    && names(condition) == ["responseIf"]
                    && structural(condition)
                    && one(condition, "responseIf").is_some_and(|if_node| {
                        if_node.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
                            && attrs(if_node, &[])
                            && names(if_node) == ["match"]
                            && structural(if_node)
                            && one(if_node, "match").is_some_and(|match_node| {
                                match_node.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
                                    && attrs(match_node, &[])
                                    && names(match_node) == ["variable", "correct"]
                                    && structural(match_node)
                                    && match_node.children().iter().all(|child| {
                                        child.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
                                            && attrs(child, &["identifier"])
                                            && child.attribute("identifier") == Some(response)
                                            && child.children().is_empty()
                                            && structural(child)
                                    })
                            })
                    })
            })
    })
}

fn resource(node: &XmlNode) -> Option<QtiProfileResourceEvidence> {
    if node.name() != "resource"
        || node.namespace_uri() != Some(IMS_CONTENT_PACKAGING_NAMESPACE)
        || !attrs(node, &["identifier", "type", "href"])
        || !structural(node)
    {
        return None;
    }
    let identifier = node.attribute("identifier")?.to_string();
    let resource_type = node.attribute("type")?.to_string();
    let href = node.attribute("href")?.to_string();
    if identifier.trim().is_empty()
        || !normalized(&href)
        || names(node).first().copied() != Some("file")
        || node
            .children()
            .iter()
            .filter(|c| c.name() == "file")
            .count()
            != 1
    {
        return None;
    }
    let mut dependencies = Vec::new();
    for child in node.children() {
        match child.name() {
            "file"
                if child.namespace_uri() == Some(IMS_CONTENT_PACKAGING_NAMESPACE)
                    && attrs(child, &["href"])
                    && child.attribute("href") == Some(href.as_str())
                    && child.children().is_empty()
                    && structural(child) => {}
            "dependency"
                if child.namespace_uri() == Some(IMS_CONTENT_PACKAGING_NAMESPACE)
                    && attrs(child, &["identifierref"])
                    && child.attribute("identifierref").is_some()
                    && child.children().is_empty()
                    && structural(child) =>
            {
                dependencies.push(child.attribute("identifierref")?.to_string())
            }
            _ => return None,
        }
    }
    Some(QtiProfileResourceEvidence {
        identifier,
        resource_type: Some(resource_type),
        href: Some(href),
        dependencies,
    })
}

fn inert_score(node: &XmlNode) -> bool {
    node.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
        && attrs(node, &["identifier", "cardinality", "baseType"])
        && node.attribute("identifier") == Some("SCORE")
        && node.attribute("cardinality") == Some("single")
        && node.attribute("baseType") == Some("float")
        && node.children().is_empty()
        && structural(node)
}
fn one<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    let all = node
        .children()
        .iter()
        .filter(|n| n.name() == name)
        .collect::<Vec<_>>();
    (all.len() == 1).then_some(all[0])
}
fn names(node: &XmlNode) -> Vec<&str> {
    node.children().iter().map(|node| node.name()).collect()
}
fn leaf_text(node: &XmlNode, namespace: &str) -> Option<String> {
    if node.namespace_uri() != Some(namespace) || !attrs(node, &[]) || !node.children().is_empty() {
        return None;
    }
    let mut value = String::new();
    for content in node.content() {
        match content {
            XmlContentRef::Text(part) | XmlContentRef::Cdata(part) => value.push_str(part),
            XmlContentRef::Child(_)
            | XmlContentRef::Comment(_)
            | XmlContentRef::ProcessingInstruction { .. } => return None,
        }
    }
    Some(value.trim().to_string())
}
fn normalized(path: &str) -> bool {
    path.split('/').all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    })
}
fn attrs(node: &XmlNode, allowed: &[&str]) -> bool {
    node.attributes().all(|attribute| {
        attribute.namespace_uri() == Some("http://www.w3.org/2000/xmlns/")
            || (attribute.namespace_uri().is_none() && allowed.contains(&attribute.local_name()))
    })
}
fn root_attrs(node: &XmlNode, allowed: &[&str]) -> bool {
    node.attributes().all(|attribute| {
        attribute.namespace_uri() == Some("http://www.w3.org/2000/xmlns/")
            || (attribute.namespace_uri().is_none() && allowed.contains(&attribute.local_name()))
            || (attribute.namespace_uri() == Some("http://www.w3.org/2001/XMLSchema-instance")
                && attribute.local_name() == "schemaLocation"
                && !attribute.value().trim().is_empty())
    })
}
fn structural(node: &XmlNode) -> bool {
    node.content().all(|content| match content {
        XmlContentRef::Text(value) | XmlContentRef::Cdata(value) => value.trim().is_empty(),
        XmlContentRef::Child(_) => true,
        XmlContentRef::Comment(_) | XmlContentRef::ProcessingInstruction { .. } => false,
    })
}
fn structural_except_markup(node: &XmlNode) -> bool {
    node.content().all(|content| {
        !matches!(
            content,
            XmlContentRef::Comment(_) | XmlContentRef::ProcessingInstruction { .. }
        )
    })
}
