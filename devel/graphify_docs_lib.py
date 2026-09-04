"""Render a GitHub-viewable repository map page from Graphify artifacts.

Graphify can already emit visuals, but none of them are a good fit for GitHub.
Its interactive graph and collapsible tree need JavaScript, which GitHub will
not run. Its call-flow export does embed Mermaid, which GitHub renders natively,
but it hardcodes a dark theme and HTML labels that break in light mode and risk
sanitization, and its per-section diagrams are large dependency dumps.

So the diagram here is generated from graph.json directly. That removes any
dependency on Graphify's HTML markup, reuses the loaders in
graphify_context_lib, and keeps theme neutrality and label sanitization under
local control. The full-graph figure stays decorative: it shows cluster shape
and scale, while the Mermaid overview explains the architecture and the tables
carry the detail.
"""

# Standard Library
import re
import html
import pathlib
import subprocess

# PIP3 modules
import lxml.etree

# local repo modules
import graphify_context_lib
import graphify_clean_svg


PAGE_FILE_NAME = "GRAPHIFY.md"
FIGURE_FILE_NAME = "GRAPHIFY_map.svg"
EXPORTED_SVG_NAME = "graph.svg"
MAX_DIAGRAM_COMMUNITIES = 12
MAX_DETAIL_SYMBOLS = 8

BENCHMARK_REDUCTION_PATTERN = re.compile(r"Reduction:\s*(\S+)")
NON_IDENTIFIER_PATTERN = re.compile(r"[^A-Za-z0-9_]")
HTML_TAG_PATTERN = re.compile(r"<[^>]*>")
UNSAFE_LABEL_PATTERN = re.compile(r"[^A-Za-z0-9 _.,'()/+:=\-]")
WHITESPACE_PATTERN = re.compile(r"\s+")


#============================================


def ascii_only(value: str) -> str:
	"""Return text safe for this repository's ASCII-only Markdown rule."""
	cleaned = graphify_context_lib.clean_graph_text(value)
	ascii_text = cleaned.encode("ascii", "ignore").decode("ascii")
	return ascii_text


#============================================


def safe_community_name(value: str) -> str:
	"""Return an ASCII display name safe in Mermaid, headings, and tables."""
	decoded_text = html.unescape(ascii_only(value))
	plain_text = HTML_TAG_PATTERN.sub(" ", decoded_text)
	plain_text = plain_text.replace("&", " and ")
	safe_text = UNSAFE_LABEL_PATTERN.sub(" ", plain_text)
	safe_name = WHITESPACE_PATTERN.sub(" ", safe_text).strip()
	return safe_name


#============================================


def community_key(node: dict) -> str | None:
	"""Return one node's community identifier as text, when it has one."""
	community_id = node.get("community")
	if isinstance(community_id, (str, int)):
		return str(community_id)
	return None


#============================================


def community_names(graph_data: dict, labels_data: dict | None) -> dict[str, str]:
	"""Map each community identifier to its display name."""
	names = {}
	for node in graph_data["nodes"]:
		key = community_key(node)
		if key is None or key in names:
			continue
		name = graphify_context_lib.graph_community_name(node, labels_data)
		if name is not None:
			safe_name = safe_community_name(name)
			if safe_name:
				names[key] = safe_name
	return names


#============================================


def community_node_counts(graph_data: dict) -> dict[str, int]:
	"""Count how many symbols each community holds."""
	counts: dict[str, int] = {}
	for node in graph_data["nodes"]:
		key = community_key(node)
		if key is None:
			continue
		counts[key] = counts.get(key, 0) + 1
	return counts


#============================================


def node_communities(graph_data: dict) -> dict[str, str]:
	"""Map each node id to its community identifier."""
	assignments = {}
	for node in graph_data["nodes"]:
		key = community_key(node)
		if key is not None:
			assignments[node["id"]] = key
	return assignments


#============================================


def community_edge_weights(graph_data: dict) -> dict[tuple[str, str], int]:
	"""Count links running between two different communities."""
	assignments = node_communities(graph_data)
	weights: dict[tuple[str, str], int] = {}
	for link in graph_data["links"]:
		source_key = assignments.get(link["source"])
		target_key = assignments.get(link["target"])
		if source_key is None or target_key is None or source_key == target_key:
			continue
		pair = (source_key, target_key) if source_key < target_key else (target_key, source_key)
		weights[pair] = weights.get(pair, 0) + 1
	return weights


#============================================


def node_degrees(graph_data: dict) -> dict[str, int]:
	"""Count how many links touch each node."""
	degrees: dict[str, int] = {}
	for link in graph_data["links"]:
		for endpoint in (link["source"], link["target"]):
			degrees[endpoint] = degrees.get(endpoint, 0) + 1
	return degrees


#============================================


def ranked_communities(graph_data: dict, labels_data: dict | None) -> list[tuple[str, str, int]]:
	"""Return communities as (key, name, size), largest first."""
	counts = community_node_counts(graph_data)
	names = community_names(graph_data, labels_data)
	ranked = []
	for key, size in counts.items():
		name = names.get(key, f"Community {key}")
		ranked.append((key, name, size))
	ranked.sort(key=lambda entry: (-entry[2], entry[0]))
	return ranked


#============================================


def mermaid_identifier(key: str, name: str) -> str:
	"""Return a Mermaid-safe node id derived from a community name."""
	slug = NON_IDENTIFIER_PATTERN.sub("_", name).strip("_")
	identifier = f"c{key}_{slug}" if slug else f"c{key}"
	return identifier


#============================================


def format_mermaid_overview(graph_data: dict, labels_data: dict | None) -> str:
	"""Return a GitHub-native Mermaid diagram of the community structure.

	No init directive is emitted, so GitHub themes the diagram for whichever
	appearance the reader uses. Labels are plain text for the same reason:
	GitHub sanitizes Mermaid HTML labels, so <br/> and <small> cannot be relied
	on the way Graphify's own export uses them.
	"""
	ranked = ranked_communities(graph_data, labels_data)[:MAX_DIAGRAM_COMMUNITIES]
	shown_keys = {key for key, _name, _size in ranked}
	identifiers = {key: mermaid_identifier(key, name) for key, name, _size in ranked}

	lines = ["flowchart LR"]
	for key, name, size in ranked:
		lines.append(f'    {identifiers[key]}["{name} ({size})"]')
	weights = community_edge_weights(graph_data)
	for (source_key, target_key), weight in sorted(weights.items()):
		if source_key not in shown_keys or target_key not in shown_keys:
			continue
		lines.append(
			f"    {identifiers[source_key]} ---|{weight}| {identifiers[target_key]}"
		)
	diagram = "\n".join(lines)
	return diagram


#============================================


def file_type_counts(graph_data: dict) -> list[tuple[str, int]]:
	"""Count symbols per source-file extension, most common first."""
	counts: dict[str, int] = {}
	for node in graph_data["nodes"]:
		source_file = node.get("source_file")
		if not isinstance(source_file, str) or not source_file:
			continue
		suffix = pathlib.PurePosixPath(source_file).suffix
		if not suffix:
			continue
		counts[suffix] = counts.get(suffix, 0) + 1
	ranked = sorted(counts.items(), key=lambda entry: (-entry[1], entry[0]))
	return ranked


#============================================


def benchmark_reduction(graphify_executable: str, repo_root: pathlib.Path) -> str | None:
	"""Return Graphify's reported token reduction, or None when unavailable.

	Advisory: a benchmark that fails or changes its output format omits one row
	from the summary rather than failing the page.
	"""
	# ASVS 1.2.5: fixed argv, no shell, and a failure is reported as absence.
	result = subprocess.run(
		[graphify_executable, "benchmark"],
		cwd=repo_root,
		capture_output=True,
		text=True,
		check=False,
	)
	if result.returncode != 0:
		return None
	match = BENCHMARK_REDUCTION_PATTERN.search(result.stdout)
	if match is None:
		return None
	return ascii_only(match.group(1))


#============================================


def format_summary_table(
	graph_data: dict,
	labels_data: dict | None,
	reduction: str | None,
) -> list[str]:
	"""Return the map-summary table rows."""
	ranked = ranked_communities(graph_data, labels_data)
	lines = [
		"| Measure | Value |",
		"| --- | --- |",
		f"| Symbols | {len(graph_data['nodes'])} |",
		f"| Relationships | {len(graph_data['links'])} |",
		f"| Communities | {len(ranked)} |",
	]
	for suffix, count in file_type_counts(graph_data)[:5]:
		lines.append(f"| Symbols in `{suffix}` files | {count} |")
	if reduction is not None:
		lines.append(f"| Token reduction per query | {reduction} |")
	return lines


#============================================


def format_community_table(graph_data: dict, labels_data: dict | None) -> list[str]:
	"""Return one row per community, largest first."""
	lines = ["| Community | Symbols |", "| --- | --- |"]
	for _key, name, size in ranked_communities(graph_data, labels_data):
		lines.append(f"| {name} | {size} |")
	return lines


#============================================


def community_symbol_rows(
	graph_data: dict,
	target_key: str,
	degrees: dict[str, int],
) -> list[str]:
	"""Return the most-connected symbols in one community as table rows."""
	members = []
	for node in graph_data["nodes"]:
		if community_key(node) != target_key:
			continue
		label = node.get("label")
		if not isinstance(label, str):
			continue
		source_file = node.get("source_file")
		source_files = (source_file,) if isinstance(source_file, str) else ()
		if graphify_context_lib.is_test_symbol(label, source_files):
			continue
		members.append((degrees.get(node["id"], 0), ascii_only(label)))
	members.sort(key=lambda entry: (-entry[0], entry[1]))
	rows = ["| Symbol | Connections |", "| --- | --- |"]
	for degree, label in members[:MAX_DETAIL_SYMBOLS]:
		rows.append(f"| `{label}` | {degree} |")
	return rows


#============================================


def format_community_sections(graph_data: dict, labels_data: dict | None) -> list[str]:
	"""Return a detail section per community, largest first."""
	degrees = node_degrees(graph_data)
	lines = []
	for key, name, size in ranked_communities(graph_data, labels_data):
		lines.append("")
		lines.append(f"### {name}")
		lines.append("")
		lines.append(f"{size} symbols.")
		lines.append("")
		lines.extend(community_symbol_rows(graph_data, key, degrees))
	return lines


#============================================


def build_figure(graphify_executable: str, repo_root: pathlib.Path) -> dict | None:
	"""Export and clean the decorative full-graph figure.

	Returns None when the figure cannot be produced. Graphify renders this
	export with matplotlib, which is not one of its required dependencies, so a
	machine without it simply gets a page with no figure.
	"""
	# ASVS 1.2.5: fixed argv, no shell, and a failure is reported as absence.
	result = subprocess.run(
		[graphify_executable, "export", "svg"],
		cwd=repo_root,
		capture_output=True,
		text=True,
		check=False,
	)
	exported_path = repo_root / graphify_context_lib.OUTPUT_DIR_NAME / EXPORTED_SVG_NAME
	if result.returncode != 0 or not exported_path.is_file():
		return None
	figure_path = repo_root / "docs" / FIGURE_FILE_NAME
	try:
		summary = graphify_clean_svg.clean_svg_file(exported_path, figure_path)
	except (OSError, ValueError, lxml.etree.LxmlError) as error:
		print(f"Figure cleanup unavailable: {error}")
		return None
	return summary


#============================================


def format_page(
	graph_data: dict,
	labels_data: dict | None,
	reduction: str | None,
	has_figure: bool,
) -> str:
	"""Assemble the complete repository-map page."""
	lines = [
		"# Repository map",
		"",
		"Generated from the Graphify code map by",
		"[devel/graphify_map_repo.py](../devel/graphify_map_repo.py). Rebuild it with `--page`",
		"after the map changes.",
		"",
		"## Overview",
		"",
		"Each box is a community of related symbols; each line is the number of relationships",
		"crossing between two of them.",
		"",
		"```mermaid",
		format_mermaid_overview(graph_data, labels_data),
		"```",
		"",
		"## Map summary",
		"",
	]
	lines.extend(format_summary_table(graph_data, labels_data, reduction))
	lines.extend(["", "## Communities", ""])
	lines.extend(format_community_table(graph_data, labels_data))
	if has_figure:
		lines.extend([
			"",
			"## Full graph",
			"",
			"Every symbol and relationship at once. Decorative: it shows cluster shape and scale",
			"rather than readable detail.",
			"",
			f"![Repository graph]({FIGURE_FILE_NAME})",
		])
	lines.extend(["", "## Community detail", ""])
	lines.append("The most-connected symbols in each community.")
	lines.extend(format_community_sections(graph_data, labels_data))
	page_text = "\n".join(lines)
	return page_text


#============================================


def write_page(
	graphify_executable: str,
	repo_root: pathlib.Path,
) -> tuple[pathlib.Path, dict | None]:
	"""Write the repository-map page and its figure, returning both results."""
	graph_data = graphify_context_lib.load_graph_data(repo_root)
	labels_data = graphify_context_lib.load_labels_data(repo_root)
	reduction = benchmark_reduction(graphify_executable, repo_root)
	figure_summary = build_figure(graphify_executable, repo_root)
	page_text = format_page(
		graph_data, labels_data, reduction, figure_summary is not None
	)
	# ASVS 5.3.2: the page path is fixed beneath the repository docs directory.
	page_path = repo_root / "docs" / PAGE_FILE_NAME
	page_path.write_text(f"{page_text}\n", encoding="utf-8")
	return page_path, figure_summary
