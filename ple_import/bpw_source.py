"""Discover BPW files using its existing filename associations and metadata."""

# Standard Library
import re
import pathlib
import dataclasses

# PIP3 modules
import yaml

COLLECTIONS = (
	"genetics", "biochemistry", "biotechnology", "biostatistics",
	"molecular_biology", "laboratory", "other",
)
DECLARED_TYPES = {
	"MC": "multipleChoice", "MA": "multipleAnswer", "FIB": "fillInBlank",
	"FIB_PLUS": "multipleFillInBlank", "NUM": "numeric", "MAT": "matching",
	"ORD": "ordering", "HOTSPOT": "hotspot",
}
# These existing standalone families were inspected for initial editable metadata.
STANDALONE_TYPES = {
	"match_amino_acid_structures": "matching", "match_purine_structures": "matching",
	"match_pyrimidine_structures": "matching", "tetrapeptide_net_charge": "multipleChoice",
	"which_amino_acid": "multipleChoice", "alpha_helix_h-bonds": "multipleChoice",
	"metabolic_pathway_inhibitor": "multipleChoice", "classify_fischer": "multipleChoice",
	"D_to_L_Fischer_configuration": "multipleChoice",
	"D_to_L_Haworth_configuration": "multipleChoice",
	"convert_Fischer_to_Haworth": "multipleChoice",
	"convert_Haworth_to_Fischer": "multipleChoice",
	"michaelis_menten_table_km": "numeric", "michaelis_menten_table_inhibition": "numeric",
}
# BPW's demonstrated family names, including the supported ring variants.
FAMILY_SOURCES = {
	"match_amino_acid_structures-4_choices": "match_amino_acid_structures",
	"match_purine_structures-3_choices": "match_purine_structures",
	"match_pyrimidine_structures-4_choices": "match_pyrimidine_structures",
	"tetrapeptide_net_charge-tetrapeptide_net_charge-5_choices": "tetrapeptide_net_charge",
	"which_amino_acid-MC-7_choices": "which_amino_acid",
	"alpha_helix_h-bonds-MC": "alpha_helix_h-bonds",
	"metabolic_pathway_inhibitor-BCHM355": "metabolic_pathway_inhibitor",
	"michaelis_menten_table-Km": "michaelis_menten_table_km",
	"michaelis_menten_table-inhibition": "michaelis_menten_table_inhibition",
	"D_to_L_Fischer_configuration-MC-with_hint-5_choices": "D_to_L_Fischer_configuration",
	"D_to_L_Haworth_configuration-MC-with_hint-5_choices": "D_to_L_Haworth_configuration",
	"convert_Fischer_to_Haworth-MC-with_hint-PYRAN-5_choices": "convert_Fischer_to_Haworth",
	"convert_Fischer_to_Haworth-MC-with_hint-FURAN-5_choices": "convert_Fischer_to_Haworth",
	"convert_Haworth_to_Fischer-MC-with_hint-PYRAN-5_choices": "convert_Haworth_to_Fischer",
	"convert_Haworth_to_Fischer-MC-with_hint-FURAN-5_choices": "convert_Haworth_to_Fischer",
}
INSPECTED_CARDINALITIES = {"match_purine_structures": 3, "match_pyrimidine_structures": 4}
# These eight actual static sources require QPM's existing RDKit/table renderer.
RENDERED_BANKS = {
	"which_macromolecule-MC", "polypeptide_fib_sequence-FIB-2aa",
	"polypeptide_mc_sequence-2_amino_acids", "polypeptide_mc_sequence-3_amino_acids",
	"polypeptide_mc_sequence-4_amino_acids", "which_amino_acid-FIB",
	"wordle_peptides-FIB", "order_glycolysis_molecules-ORD-4_choices",
}


@dataclasses.dataclass
class SourceSet:
	"""One published algorithmic source or complete static bank."""
	collection: str
	topic: str
	path: pathlib.Path
	title: str
	description: str
	question_type: str | None
	companion_bank: pathlib.Path | None = None


#============================================
def core_name(bank: pathlib.Path) -> str:
	"""Remove BPW's fixed bank wrapper from its filename."""
	name = bank.name.removeprefix("bbq-").removesuffix("-questions.txt")
	return name


#============================================
def algorithmic_path(bank: pathlib.Path) -> pathlib.Path | None:
	"""Mirror BPW topic_page.find_pgml_file's demonstrated associations."""
	name = core_name(bank)
	if name in FAMILY_SOURCES:
		name = FAMILY_SOURCES[name]
	elif name.startswith("MATCH-"):
		name = name.removeprefix("MATCH-") + "-matching"
	elif name.startswith(("WOMC-", "MC-")):
		name = name.split("-", 1)[1] + "-which_one"
	elif name.startswith("TFMS-"):
		name = name.removeprefix("TFMS-")
	for folder in (bank.parent / "downloads", bank.parent):
		for suffix in (".pgml", ".pg"):
			candidate = folder / (name + suffix)
			if candidate.is_file():
				return candidate
	return None


#============================================
def declared_bank_type(bank: pathlib.Path) -> str:
	"""Read the companion's explicit type label only; content belongs to QPM."""
	with bank.open(encoding="utf-8") as handle:
		label = next(line for line in handle if line.strip()).split("\t", 1)[0]
	question_type = DECLARED_TYPES[label]
	return question_type


#============================================
def source_title(path: pathlib.Path) -> str:
	"""Read an existing PG title annotation without interpreting PG code."""
	match = re.search(r"^## TITLE\(['\"](.*?)['\"]\)", path.read_text(), re.MULTILINE)
	if match is not None:
		return match.group(1)
	title = path.stem.replace("_", " ").replace("-", " ")
	return title


#============================================
def algorithmic_points(item: SourceSet) -> tuple[str, bool]:
	"""Read literal author cardinality metadata for editable matching defaults."""
	if item.question_type not in ("matching", "ordering", "multipleFillInBlank"):
		return "1", False
	if item.path.stem in INSPECTED_CARDINALITIES:
		points = 0.5 * INSPECTED_CARDINALITIES[item.path.stem]
		return str(points), False
	match = re.search(r"(?:my\s+)?\$(?:n|num_choices)\s*=\s*(\d+)\s*;",
		item.path.read_text())
	if match is None:
		return "1", True
	count = int(match.group(1))
	points = count if item.question_type == "multipleFillInBlank" else 0.5 * count
	return str(points), False


#============================================
def discover(bpw_root: pathlib.Path) -> tuple[list, dict]:
	"""Read all seven collections directly from the BPW build outputs."""
	missing = [collection for collection in COLLECTIONS
		if not (bpw_root / "site_docs" / collection).is_dir()]
	if missing:
		raise ValueError("Missing required BPW collection directories: " + ", ".join(missing))
	# ASVS 1.5.2: YAML is data; no Python object constructors are enabled.
	metadata = yaml.safe_load((bpw_root / "topics_metadata.yml").read_text())
	titles = yaml.safe_load((bpw_root / "problem_set_titles.yml").read_text())
	sources = []
	for collection in COLLECTIONS:
		for topic_folder in sorted((bpw_root / "site_docs" / collection).glob("topic*")):
			topic = topic_folder.name
			description = metadata[collection]["topics"][topic]["description"]
			used = set()
			for bank in sorted(topic_folder.glob("bbq-*-questions.txt")):
				pg = algorithmic_path(bank)
				path = bank if pg is None else pg
				if pg is not None:
					if pg.stem in used:
						continue
					used.add(pg.stem)
				question_type = None if pg is None else declared_bank_type(bank)
				if pg is not None and pg.stem in STANDALONE_TYPES:
					question_type = STANDALONE_TYPES[pg.stem]
				sources.append(SourceSet(collection, topic, path, titles[bank.name],
					description, question_type, bank))
			for pg in sorted((topic_folder / "downloads").glob("*.pg*")):
				if pg.suffix not in (".pg", ".pgml") or pg.stem in used:
					continue
				# Prefer the editable PGML when both formats of one source exist.
				if pg.suffix == ".pg" and pg.with_suffix(".pgml").is_file():
					continue
				question_type = STANDALONE_TYPES.get(pg.stem)
				if pg.stem.endswith("-matching"):
					question_type = "matching"
				elif pg.stem.endswith("-which_one"):
					question_type = "multipleChoice"
				sources.append(SourceSet(collection, topic, pg, source_title(pg),
					description, question_type))
	return sources, metadata
