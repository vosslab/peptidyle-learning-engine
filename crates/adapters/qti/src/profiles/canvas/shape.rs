use super::{CANVAS_ITEM_NAMESPACE, IMS_CONTENT_PACKAGING_NAMESPACE, QtiProfileResourceEvidence};
use crate::xml::{XmlContentRef, XmlNode};

const MANIFEST: &str = "imsmanifest.xml";
const PREFIX: &str = "canvas_qti12_questions/";
const META: &str = "canvas_qti12_questions/assessment_meta.xml";

pub(super) fn allowed(path: &str) -> bool {
    path == MANIFEST
        || path == META
        || (path.starts_with(PREFIX) && path.ends_with(".xml") && normalized(path))
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
pub(super) fn manifest_resources(
    root: &XmlNode,
) -> Option<(Option<String>, Vec<QtiProfileResourceEvidence>)> {
    if root.name() != "manifest" || root.namespace_uri()? != IMS_CONTENT_PACKAGING_NAMESPACE {
        return None;
    }
    if names(root) != ["metadata", "organizations", "resources"] || !structural_text(root) {
        return None;
    }
    let metadata = exactly(root, "metadata")?;
    if metadata.namespace_uri() != Some(IMS_CONTENT_PACKAGING_NAMESPACE)
        || names(metadata) != ["schema", "schemaversion", "lom"]
        || !structural_text(metadata)
    {
        return None;
    }
    let schema_node = exactly(metadata, "schema")?;
    let version_node = exactly(metadata, "schemaversion")?;
    let lom_node = exactly(metadata, "lom")?;
    if schema_node.namespace_uri() != Some(IMS_CONTENT_PACKAGING_NAMESPACE)
        || !schema_node.children().is_empty()
        || text(schema_node).trim() != "IMS Content"
        || version_node.namespace_uri() != Some(IMS_CONTENT_PACKAGING_NAMESPACE)
        || !version_node.children().is_empty()
        || text(version_node).trim() != "1.1.3"
        || lom_node.namespace_uri() != Some("http://www.imsglobal.org/xsd/imsmd_v1p2")
    {
        return None;
    }
    let schema = Some(text(schema_node));
    let resources_node = exactly(root, "resources")?;
    if resources_node.namespace_uri() != Some(IMS_CONTENT_PACKAGING_NAMESPACE) {
        return None;
    }
    let resources = resources_node
        .children()
        .iter()
        .filter(|n| {
            n.name() == "resource" && n.namespace_uri() == Some(IMS_CONTENT_PACKAGING_NAMESPACE)
        })
        .filter(|n| valid_resource(n))
        .map(|n| QtiProfileResourceEvidence {
            identifier: n.attribute("identifier").unwrap_or_default().to_string(),
            resource_type: n.attribute("type").map(str::to_string),
            href: n.attribute("href").map(str::to_string),
            dependencies: n
                .children()
                .iter()
                .filter(|c| {
                    c.name() == "dependency"
                        && c.namespace_uri() == Some(IMS_CONTENT_PACKAGING_NAMESPACE)
                })
                .filter_map(|c| c.attribute("identifierref").map(str::to_string))
                .collect(),
        })
        .collect::<Vec<_>>();
    if resources.len() != resources_node.children().len() {
        return None;
    }
    Some((schema, resources))
}
pub(super) fn valid_meta(root: &XmlNode) -> bool {
    root.name() == "quiz"
        && root.namespace_uri() == Some("http://canvas.instructure.com/xsd/cccv1p0")
        && root.attribute("identifier") == Some("assessment_meta")
        && attributes_are(root, &["identifier"])
        && names(root) == ["title", "assignment"]
        && structural_text(root)
        && root
            .children()
            .iter()
            .all(|child| child.namespace_uri() == root.namespace_uri())
        && exactly(root, "title")
            .is_some_and(|title| title.children().is_empty() && !text(title).trim().is_empty())
        && exactly(root, "assignment").is_some_and(|assignment| {
            attributes_are(assignment, &["identifier"])
                && names(assignment) == ["title"]
                && structural_text(assignment)
                && assignment.children()[0].namespace_uri() == root.namespace_uri()
                && assignment.children()[0].children().is_empty()
                && !text(&assignment.children()[0]).trim().is_empty()
        })
}
fn valid_resource(node: &XmlNode) -> bool {
    let Some(href) = node.attribute("href") else {
        return false;
    };
    attributes_are(node, &["identifier", "type", "href"])
        && node.children().iter().all(|child| {
            child.namespace_uri() == Some(IMS_CONTENT_PACKAGING_NAMESPACE)
                && matches!(child.name(), "file" | "dependency")
        })
        && node
            .children()
            .iter()
            .filter(|child| child.name() == "file")
            .count()
            == 1
        && node
            .children()
            .iter()
            .filter(|child| child.name() == "file")
            .all(|child| attributes_are(child, &["href"]) && child.attribute("href") == Some(href))
        && node
            .children()
            .iter()
            .filter(|child| child.name() == "dependency")
            .all(|child| {
                attributes_are(child, &["identifierref"])
                    && child.attribute("identifierref").is_some()
            })
}
pub(super) fn canvas_items(root: &XmlNode) -> Option<Vec<&XmlNode>> {
    if root.name() != "questestinterop"
        || root.namespace_uri() != Some(CANVAS_ITEM_NAMESPACE)
        || names(root) != ["assessment"]
        || !structural_text(root)
    {
        return None;
    }
    let assessment = exactly(root, "assessment")?;
    if assessment.namespace_uri() != Some(CANVAS_ITEM_NAMESPACE)
        || !attributes_are(assessment, &["ident", "title"])
        || names(assessment) != ["section"]
        || !structural_text(assessment)
    {
        return None;
    }
    let section = exactly(assessment, "section")?;
    if section.namespace_uri() != Some(CANVAS_ITEM_NAMESPACE)
        || !attributes_are(section, &["ident"])
        || !structural_text(section)
    {
        return None;
    }
    let items: Vec<_> = section
        .children()
        .iter()
        .filter(|n| n.name() == "item" && n.namespace_uri() == Some(CANVAS_ITEM_NAMESPACE))
        .collect();
    (items.len() == section.children().len() && !items.is_empty()).then_some(items)
}
pub(super) fn exact_item_tree(item: &XmlNode) -> bool {
    fn names(node: &XmlNode) -> Vec<&str> {
        node.children().iter().map(|child| child.name()).collect()
    }
    fn valid(node: &XmlNode) -> bool {
        let children = names(node);
        let expected = match node.name() {
            "item" => children == ["itemmetadata", "presentation", "resprocessing"],
            "itemmetadata" => children == ["qtimetadata"],
            "qtimetadata" => {
                !children.is_empty() && children.iter().all(|name| *name == "qtimetadatafield")
            }
            "qtimetadatafield" => children == ["fieldlabel", "fieldentry"],
            "fieldlabel" | "fieldentry" | "mattext" | "varequal" | "setvar" => children.is_empty(),
            "presentation" => children == ["material", "response_lid"],
            "response_lid" => children == ["render_choice"],
            "render_choice" => {
                (2..=100).contains(&children.len())
                    && children.iter().all(|name| *name == "response_label")
            }
            "response_label" => children == ["material"],
            "material" => children == ["mattext"],
            "resprocessing" => children == ["respcondition"],
            "respcondition" => children == ["conditionvar", "setvar"],
            "conditionvar" => children == ["varequal"],
            _ => false,
        };
        let leaf = matches!(
            node.name(),
            "fieldlabel" | "fieldentry" | "mattext" | "varequal" | "setvar"
        );
        expected
            && node.content().all(|content| match content {
                XmlContentRef::Text(value) | XmlContentRef::Cdata(value) => {
                    leaf || value.trim().is_empty()
                }
                XmlContentRef::Child(_) => true,
                XmlContentRef::Comment(_) | XmlContentRef::ProcessingInstruction { .. } => false,
            })
            && node.children().iter().all(valid)
    }
    valid(item)
}
fn exactly<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    let all: Vec<_> = node
        .children()
        .iter()
        .filter(|n| n.name() == name)
        .collect();
    (all.len() == 1).then_some(all[0])
}
fn names(node: &XmlNode) -> Vec<&str> {
    node.children().iter().map(|child| child.name()).collect()
}
fn structural_text(node: &XmlNode) -> bool {
    node.content().all(|content| match content {
        XmlContentRef::Text(value) | XmlContentRef::Cdata(value) => value.trim().is_empty(),
        XmlContentRef::Child(_) => true,
        XmlContentRef::Comment(_) | XmlContentRef::ProcessingInstruction { .. } => false,
    })
}
fn text(node: &XmlNode) -> String {
    node.normalized_text_except("")
}
fn attributes_are(node: &XmlNode, names: &[&str]) -> bool {
    node.attributes().all(|attribute| {
        attribute.namespace_uri() == Some("http://www.w3.org/2000/xmlns/")
            || (attribute.namespace_uri().is_none() && names.contains(&attribute.local_name()))
    })
}
