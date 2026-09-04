"""Shrink a Graphify graph.svg into a decorative, committable cluster figure.

Graphify renders its graph with matplotlib, which emits every text label as a
run of per-glyph `<use>` references. On a 452-node map that is roughly 18,000
elements and 1.9 MB, and the 452 filename labels overlap into unreadable
spaghetti. The community names a reader actually needs live in the legend.

The matplotlib group structure makes the separation exact rather than heuristic:
labels are `text_N` groups, and the community labels are the `text_N` groups
nested inside `legend_1`. Matplotlib defines a glyph at its first use, which may
be inside a node label even when the legend also uses it. The cleaner therefore
moves every legend glyph definition to the root `<defs>` before removing labels.
Three passes then run in order, each reporting what it removed so a size change
is attributable:

	1. drop every `text_*` group that is not inside the legend
	2. collect font glyph definitions left unreferenced by pass 1
	3. round coordinates, which matplotlib emits at about six decimals

The result is decorative. It conveys cluster shape and scale, not readable
detail, so pass 3 rounds hard.
"""

# Standard Library
import re
import pathlib

# PIP3 modules
import lxml.etree


LEGEND_GROUP_ID = "legend_1"
LABEL_GROUP_PREFIX = "text_"
DEFS_TAG_NAME = "defs"
COORDINATE_PRECISION = 2
ROUNDED_ATTRIBUTE_NAMES = ("d", "transform", "x", "y", "width", "height")
XLINK_HREF_ATTRIBUTE = "{http://www.w3.org/1999/xlink}href"

DECIMAL_NUMBER_PATTERN = re.compile(r"-?\d+\.\d+")
URL_REFERENCE_PATTERN = re.compile(r"url\(#([^)]+)\)")


#============================================


def is_element(node: object) -> bool:
	"""Return whether a parsed node is a real element.

	lxml yields comments and processing instructions from iter(). Their get()
	returns None whatever default is supplied, so every walk filters here first.
	"""
	tag = getattr(node, "tag", None)
	node_is_element = isinstance(tag, str)
	return node_is_element


#============================================


def element_id(element: lxml.etree._Element) -> str:
	"""Return one element's id attribute, or an empty string when it has none."""
	raw_id = element.get("id")
	if raw_id is None:
		return ""
	return raw_id


#============================================


def legend_label_ids(root: lxml.etree._Element) -> set[str]:
	"""Return the ids of label groups nested inside the legend.

	These name the communities and are the whole reason the figure is readable,
	so they are collected before anything is removed.
	"""
	label_ids = set()
	for element in root.iter():
		if not is_element(element):
			continue
		if element_id(element) != LEGEND_GROUP_ID:
			continue
		for descendant in element.iter():
			if not is_element(descendant):
				continue
			descendant_id = element_id(descendant)
			if descendant_id.startswith(LABEL_GROUP_PREFIX):
				label_ids.add(descendant_id)
	return label_ids


#============================================


def remove_node_labels(root: lxml.etree._Element) -> int:
	"""Remove per-node text labels while keeping the legend's community labels."""
	# A legend may reference glyphs first defined inside a node label. Move those
	# definitions out before their original label groups are removed.
	preserve_legend_definitions(root)
	keep_ids = legend_label_ids(root)
	removed_count = 0
	for element in list(root.iter()):
		if not is_element(element):
			continue
		current_id = element_id(element)
		if not current_id.startswith(LABEL_GROUP_PREFIX):
			continue
		if current_id in keep_ids:
			continue
		parent = element.getparent()
		if parent is None:
			continue
		parent.remove(element)
		removed_count += 1
	return removed_count


#============================================


def collect_referenced_ids(root: lxml.etree._Element) -> set[str]:
	"""Return every id still referenced anywhere in the document.

	Both reference forms are counted. Glyphs are reached through href on `<use>`,
	while clip paths are reached through `url(#id)` inside another attribute
	value. Counting only one form would collect definitions that are still live.
	"""
	referenced_ids = set()
	for element in root.iter():
		if not is_element(element):
			continue
		for attribute_name in (XLINK_HREF_ATTRIBUTE, "href"):
			reference = element.get(attribute_name)
			if reference is not None and reference.startswith("#"):
				referenced_ids.add(reference[1:])
		for attribute_value in element.attrib.values():
			for match in URL_REFERENCE_PATTERN.finditer(attribute_value):
				referenced_ids.add(match.group(1))
	return referenced_ids


#============================================


def find_element_by_id(
	root: lxml.etree._Element,
	target_id: str,
) -> lxml.etree._Element | None:
	"""Return the first element with one id, or None when it is absent."""
	for element in root.iter():
		if is_element(element) and element_id(element) == target_id:
			return element
	return None


#============================================


def root_definitions(root: lxml.etree._Element) -> lxml.etree._Element:
	"""Return or create the root-level definitions container."""
	for child in root:
		if is_element(child) and child.tag.endswith(DEFS_TAG_NAME):
			return child

	root_tag = root.tag
	namespace_prefix = ""
	if root_tag.startswith("{"):
		namespace_prefix = f"{root_tag.split('}', 1)[0]}}}"
	definitions = lxml.etree.Element(f"{namespace_prefix}{DEFS_TAG_NAME}")
	root.insert(0, definitions)
	return definitions


#============================================


def preserve_legend_definitions(root: lxml.etree._Element) -> int:
	"""Move definitions referenced by the legend into root-level `<defs>`.

	Matplotlib stores each glyph definition beside its first use. Deleting a node
	label can otherwise delete a glyph that a later legend label still references,
	leaving only whichever characters happened to be defined inside the legend.
	"""
	legend = find_element_by_id(root, LEGEND_GROUP_ID)
	if legend is None:
		return 0
	referenced_ids = collect_referenced_ids(legend)
	if not referenced_ids:
		return 0

	target_definitions = root_definitions(root)
	preserved_ids = {
		element_id(definition)
		for definition in target_definitions
		if is_element(definition)
	}
	preserved_count = 0
	for definitions in list(root.iter()):
		if not is_element(definitions) or not definitions.tag.endswith(DEFS_TAG_NAME):
			continue
		if definitions is target_definitions:
			continue
		for definition in list(definitions):
			definition_id = element_id(definition)
			if definition_id not in referenced_ids or definition_id in preserved_ids:
				continue
			target_definitions.append(definition)
			preserved_ids.add(definition_id)
			preserved_count += 1
	return preserved_count


#============================================


def remove_dead_definitions(root: lxml.etree._Element) -> int:
	"""Remove definitions nothing references, after the labels are gone."""
	referenced_ids = collect_referenced_ids(root)
	removed_count = 0
	for element in list(root.iter()):
		if not is_element(element):
			continue
		if not element.tag.endswith(DEFS_TAG_NAME):
			continue
		for definition in list(element):
			if not is_element(definition):
				continue
			definition_id = element_id(definition)
			if not definition_id or definition_id in referenced_ids:
				continue
			element.remove(definition)
			removed_count += 1
	return removed_count


#============================================


def unresolved_reference_ids(root: lxml.etree._Element) -> set[str]:
	"""Return local SVG references whose target id is missing."""
	defined_ids = {
		element_id(element)
		for element in root.iter()
		if is_element(element) and element_id(element)
	}
	unresolved_ids = collect_referenced_ids(root) - defined_ids
	return unresolved_ids


#============================================


def round_number_text(value: str) -> str:
	"""Round decimal numbers in one attribute value, leaving commands intact."""

	def replace_number(match: re.Match) -> str:
		rounded_value = round(float(match.group(0)), COORDINATE_PRECISION)
		# Render integral results without a trailing ".0" to save further bytes.
		if rounded_value == int(rounded_value):
			return str(int(rounded_value))
		return str(rounded_value)

	rounded_text = DECIMAL_NUMBER_PATTERN.sub(replace_number, value)
	return rounded_text


#============================================


def round_coordinates(root: lxml.etree._Element) -> int:
	"""Reduce coordinate precision across the geometry-bearing attributes."""
	changed_count = 0
	for element in root.iter():
		if not is_element(element):
			continue
		for attribute_name in ROUNDED_ATTRIBUTE_NAMES:
			attribute_value = element.get(attribute_name)
			if attribute_value is None:
				continue
			rounded_value = round_number_text(attribute_value)
			if rounded_value != attribute_value:
				element.set(attribute_name, rounded_value)
				changed_count += 1
	return changed_count


#============================================


def clean_svg_tree(tree: lxml.etree._ElementTree) -> dict:
	"""Run the three cleaning passes in order and report what each removed."""
	root = tree.getroot()
	kept_labels = len(legend_label_ids(root))
	removed_labels = remove_node_labels(root)
	removed_definitions = remove_dead_definitions(root)
	rounded_attributes = round_coordinates(root)
	unresolved_ids = unresolved_reference_ids(root)
	if unresolved_ids:
		unresolved_text = ", ".join(sorted(unresolved_ids))
		raise ValueError(f"Cleaned SVG has unresolved local references: {unresolved_text}")
	summary = {
		"kept_labels": kept_labels,
		"removed_labels": removed_labels,
		"removed_definitions": removed_definitions,
		"rounded_attributes": rounded_attributes,
	}
	return summary


#============================================


def clean_svg_file(source_path: pathlib.Path, target_path: pathlib.Path) -> dict:
	"""Clean one exported graph SVG and write the decorative figure.

	Args:
		source_path: Graphify's exported graph.svg.
		target_path: Destination for the cleaned figure.

	Returns:
		Pass counts plus the before and after byte sizes.
	"""
	# ASVS 5.3.2: both paths are supplied by the caller from fixed artifact locations.
	tree = lxml.etree.parse(str(source_path))
	summary = clean_svg_tree(tree)
	tree.write(str(target_path))
	summary["source_bytes"] = source_path.stat().st_size
	summary["target_bytes"] = target_path.stat().st_size
	return summary
