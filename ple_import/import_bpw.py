"""Import all BPW collections through ordinary Instructor HTTP authoring."""

# Standard Library
import sys
import json
import getpass
import argparse
import pathlib
import mimetypes
import tempfile
import subprocess
import html.parser

# local repo modules
import ple_import.ple_api
import ple_import.bpw_source
import ple_import.course_assembly


class ImageNames(html.parser.HTMLParser):
	"""Collect display-field image filenames without modifying Native JSON."""

	def __init__(self) -> None:
		super().__init__()
		self.names: set[str] = set()

	def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
		if tag == "img":
			for name, value in attrs:
				if name == "src" and value is not None and not value.startswith(("data:", "https:")):
					self.names.add(value)


#============================================
def image_names(value: object) -> set[str]:
	"""Collect supplied images from QPM output, leaving content interpretation to PLE."""
	parser = ImageNames()
	if isinstance(value, str):
		parser.feed(value)
	elif isinstance(value, dict):
		for child in value.values():
			parser.names.update(image_names(child))
	elif isinstance(value, list):
		for child in value:
			parser.names.update(image_names(child))
	return parser.names


#============================================
def upload_images(api: ple_import.ple_api.PleApi, draft_id: str, edit: str,
	document: dict, output: pathlib.Path) -> None:
	"""Bind supplied relative assets through the existing Draft image route."""
	for name in sorted(image_names(document)):
		# ASVS 1.2.2: filenames are header values; file reads stay in QPM output.
		path = (output / name).resolve()
		if not path.is_relative_to(output.resolve()):
			raise ValueError("QPM image path leaves its output directory")
		media_type = mimetypes.guess_type(path.name)[0]
		if media_type is None:
			raise ValueError(f"Unknown image media type: {name}")
		api.request("POST", f"/api/authoring/drafts/{draft_id}/images", body=path.read_bytes(),
			headers={"If-Match": f'"{edit}"', "Content-Type": media_type,
				"X-PLE-Source-Relative-Name": name})


#============================================
def publish(api: ple_import.ple_api.PleApi, item: ple_import.bpw_source.SourceSet, source: str,
	classification: dict, citation: str, document: dict | None = None,
	output: pathlib.Path | None = None) -> dict:
	"""Create and publish source unchanged through the existing authoring handlers."""
	native = document is not None
	metadata = {"questionTitle": item.title, "questionDescription": item.description,
		"tags": ["BiologyProblems.org"], "questionLicense": "CC-BY-4.0",
		"questionCitation": citation, "language": "en"}
	draft = api.json("POST", "/api/authoring/drafts", {"metadata": metadata,
		"questionBackend": "ple" if native else "webwork",
		"questionFormat": "pleQuestionJson" if native else
			("webworkPgml" if item.path.suffix == ".pgml" else "webworkPg"),
		"webworkPgPath": None if native else f"{item.collection}/{item.topic}/{item.path.name}",
		"source": source})
	draft_id = draft["draftQuestionId"]
	edit = draft["draftQuestionEditNumber"]
	if native:
		if output is None:
			raise ValueError("Native images require the QPM output directory")
		upload_images(api, draft_id, edit, document, output)
	else:
		_, headers = api.request("PUT", f"/api/authoring/drafts/{draft_id}/metadata",
			payload={"metadata": metadata, "questionType": item.question_type,
				"generalFeedback": None}, headers={"If-Match": f'"{edit}"'})
		edit = headers["etag"].strip('"')
		# Reuse the saved Draft's algorithmic preview; QPM never sees PG/PGML.
		seed = 1
		diagnostics = api.json("GET", f"/api/authoring/drafts/{draft_id}/preview"
			f"?draftQuestionEditNumber={edit}&seed={seed}&diagnostics=true")
		for warning in diagnostics["pgWarn"]:
			print(f"  {item.path.name} (seed {seed}): {warning}", file=sys.stderr, flush=True)
		if diagnostics["errorFlag"]:
			raise ValueError(f"{item.path}: WeBWorK renderer reported errors at seed {seed}; "
				f"Draft {draft_id} was not published")
	payload = {name: classification[name] for name in
		("disciplineUuid", "subjectUuid", "topicUuid", "subtopicUuid")}
	payload["authors"] = ["Neil R. Voss"]
	published = api.json("POST", f"/api/authoring/drafts/{draft_id}/publish",
		payload, headers={"If-Match": f'"{edit}"'})
	reference = {"publishedQuestionId": published["questionId"], "revisionNumber": 1}
	return reference


#============================================
def convert(item: ple_import.bpw_source.SourceSet, cli: pathlib.Path, output: pathlib.Path) -> list:
	"""Let QPM perform complete static-bank conversion without a variant limit."""
	command = [str(cli), "-i", str(item.path), "-f", "ple_native_json",
		"-o", str(output), "-q"]
	if ple_import.bpw_source.core_name(item.path) in ple_import.bpw_source.RENDERED_BANKS:
		command.append("--html-to-image")
	# ASVS 1.2.5: arguments are passed directly, with no shell interpolation.
	result = subprocess.run(command, capture_output=True, text=True, check=False)
	if result.returncode:
		raise RuntimeError(f"QPM conversion failed for {item.path}: {result.stderr.strip()}")
	items = sorted(output.glob("item_*.json"))
	if not items:
		raise ValueError(f"QPM emitted no Questions for {item.path}")
	return items


#============================================
def import_catalog(api: ple_import.ple_api.PleApi, sources: list, metadata: dict,
	bpw_root: pathlib.Path, qpm_cli: pathlib.Path, report: dict) -> None:
	"""Publish all Questions and Pools, then assemble four public base Blueprints."""
	classifications = {}
	topic_entries = {}
	for index, item in enumerate(sources):
		print(f"[{index + 1}/{len(sources)}] {item.collection}/{item.topic}/{item.path.name}",
			flush=True)
		key = (item.collection, item.topic)
		if key not in classifications:
			subject = metadata[item.collection]["title"]
			discipline = "Mathematics" if item.collection == "biostatistics" else "Biology"
			classifications[key] = api.classification(discipline, subject,
				metadata[item.collection]["topics"][item.topic]["title"])
		classification = classifications[key]
		if item.path.is_relative_to(bpw_root):
			citation_path = item.path.relative_to(bpw_root).as_posix()
			citation = "https://github.com/vosslab/biology-problems-website/blob/main/" + citation_path
		else:
			citation = "Bundled Genetics source: " + item.path.as_posix()
		citation += "; Neil R. Voss; content CC-BY-4.0; source code LGPL-3.0-or-later."
		bonus = ple_import.course_assembly.is_bonus(item.title)
		if item.path.suffix in (".pg", ".pgml"):
			source = item.path.read_bytes().decode("utf-8")
			reference = publish(api, item, source, classification, citation)
			report["questions"] += 1
			points, unknown = ple_import.bpw_source.algorithmic_points(item)
			if unknown:
				report["pointDefaults"].append({"source": str(item.path), "points": points,
					"reason": "Algorithmic choice count is not exposed in source metadata"})
			entry = ple_import.course_assembly.entry(reference, False, points, bonus)
		else:
			with tempfile.TemporaryDirectory(prefix="ple_bpw_") as folder:
				output = pathlib.Path(folder) / "native"
				paths = convert(item, qpm_cli, output)
				members = []
				points = None
				point_values = set()
				for item_index, path in enumerate(paths):
					source = path.read_text()
					document = json.loads(source)
					item_points = ple_import.course_assembly.native_points(document)
					point_values.add(item_points)
					if points is None:
						points = item_points
					members.append(publish(api, item, source, classification, citation, document, output))
					report["questions"] += 1
					print(f"  Question {item_index + 1}/{len(paths)} published", flush=True)
				if len(point_values) > 1:
					report["pointDefaults"].append({"source": str(item.path), "points": points,
						"reason": "Pool variants have different response counts; editable first-item default"})
				reference = api.json("POST", "/api/question-pools", {"title": item.title,
					"description": item.description, "members": members, "tags": ["BiologyProblems.org"]})
				report["pools"] += 1
				entry = ple_import.course_assembly.entry(reference, True, points, bonus)
		topic_entries.setdefault(key, []).append(entry)
	for collection, theme in ple_import.course_assembly.COURSE_THEMES.items():
		modules = []
		for topic_id, topic in metadata[collection]["topics"].items():
			entries = topic_entries.get((collection, topic_id), [])
			if entries:
				modules.append(ple_import.course_assembly.module(topic, entries))
		title = metadata[collection]["title"]
		discipline = "Mathematics" if collection == "biostatistics" else "Biology"
		blueprint = api.create_blueprint({"classification": api.classification(discipline, title),
			"short_name": title, "long_name": title, "theme": theme, "modules": modules})
		report["blueprints"][collection] = blueprint["id"]


#============================================
def parse_args() -> argparse.Namespace:
	"""Accept only operator-selected paths and the ordinary API origin."""
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("-b", "--bpw-root", dest="bpw_root", type=pathlib.Path, required=True)
	parser.add_argument("-a", "--api-url", dest="api_url", required=True)
	parser.add_argument("-q", "--qpm-cli", dest="qpm_cli", type=pathlib.Path, required=True)
	parser.add_argument("-r", "--result-file", dest="result_file", type=pathlib.Path, required=True)
	parser.add_argument("-c", "--ca-file", dest="ca_file")
	args = parser.parse_args()
	return args


#============================================
def main() -> None:
	"""Run one import and write a non-recovery result, including incomplete errors."""
	args = parse_args()
	report = {"blueprints": {}, "questions": 0, "pools": 0, "pointDefaults": [],
		"complete": False, "errors": []}
	try:
		api = sign_in(args.api_url, args.ca_file)
		sources, metadata = ple_import.bpw_source.discover(args.bpw_root.resolve())
		untyped = [str(item.path) for item in sources if item.path.suffix != ".txt"
			and item.question_type is None]
		if untyped:
			raise ValueError("Missing declared WeBWorK type metadata: " + ", ".join(untyped))
		import_catalog(api, sources, metadata, args.bpw_root.resolve(), args.qpm_cli.resolve(), report)
		report["complete"] = True
	except Exception as error:
		report["errors"].append(str(error))
		raise
	finally:
		args.result_file.parent.mkdir(parents=True, exist_ok=True)
		args.result_file.write_text(json.dumps(report, indent=2) + "\n")
	print(f"Imported {report['questions']} Questions and {report['pools']} Pools", flush=True)


#============================================
def sign_in(api_url: str, ca_file: str | None) -> ple_import.ple_api.PleApi:
	"""Use ordinary email-code sign-in or the installer's protected session handoff."""
	if sys.stdin.isatty():
		api = ple_import.ple_api.PleApi(api_url, None, ca_file)
		email = input("Instructor email: ").strip()
		challenge = api.json("POST", "/api/auth/email-code/start", {"email": email})
		code = getpass.getpass("Email sign-in code (hidden): ")
		api.json("POST", "/api/auth/email-code/complete/" + challenge["challengeId"], {"code": code})
	else:
		cookie = json.loads(sys.stdin.readline())["sessionCookie"]
		api = ple_import.ple_api.PleApi(api_url, cookie, ca_file)
	session = api.json("GET", "/api/auth/session")
	if session["account"]["userRole"] != "instructor":
		raise ValueError("The importer requires an Instructor session")
	return api

