#!/usr/bin/env python3
"""Build and validate the Human Guidance implementation compliance checklist."""

# Standard Library
import argparse
import collections
import difflib
import pathlib
import re


REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
HUMAN_GUIDANCE_PATH = REPO_ROOT / "docs/HUMAN_GUIDANCE.md"
CHECKLIST_PATH = REPO_ROOT / "docs/active_plans/audits/human_guidance_implementation_checklist.md"
PARTS_DIRECTORY = REPO_ROOT / "docs/active_plans/audits/hg_checklist_parts"
PART_NAME_PATTERN = re.compile(r"[0-9]{2}_[a-z0-9_]+\.md\Z")
HEADING_PATTERN = re.compile(r"^(#{1,6}) (.+)$")
SOURCE_BULLET_PATTERN = re.compile(r"^(\s*)- (.+)$")
CHECKLIST_BULLET_PATTERN = re.compile(r"^(\s*)- \[([ x])\] (.+)$")
NOT_APPLICABLE_PATTERN = re.compile(r"^(\s*)- N/A (.+)$")
STATUS_LINE_PATTERN = re.compile(r"^\s*- (Evidence \((?:source|test|runtime)\)|Mismatch|Reason|Owner|Question|Decision|Generated evidence stale):")
EVIDENCE_PATTERN = re.compile(r"^\s*- Evidence \((source|test|runtime)\): (.+)$")

# Named source roots make missing, added, substituted, and reordered audit content a hard failure.
PART_MANIFEST: dict[str, tuple[str, ...]] = {
	"01_development.md": ("How to use this guidance", "Development principles", "Product vocabulary"),
	"02_accounts.md": ("Accounts and roles",),
	"03_shell.md": ("General interface design", "Ribbon and page layout", "User top bar", "Breadcrumbs"),
	"04_instructor_ui.md": ("Instructor interface",),
	"05_student_sysadmin_ui.md": ("Student interface", "Sysadmin interface"),
	"06_data.md": ("Data and history",),
	"07_questions.md": ("Questions",),
	"08_courses.md": ("Courses",),
	"09_assessments.md": ("Assessments",),
}
PART_ROOT_LEVELS: dict[str, tuple[int, ...]] = {
	"01_development.md": (2, 2, 2), "02_accounts.md": (2,), "03_shell.md": (3, 3, 3, 3),
	"04_instructor_ui.md": (3,), "05_student_sysadmin_ui.md": (3, 3), "06_data.md": (2,),
	"07_questions.md": (2,), "08_courses.md": (2,), "09_assessments.md": (2,),
}
PART_CONTEXT_HEADINGS: dict[str, tuple[str, ...]] = {
	"03_shell.md": ("## Interface design",),
}
HOW_TO_USE_HEADING = "How to use this guidance"
HOW_TO_USE_REASON = (
	"This section gives rules for writing and maintaining Human Guidance. "
	"It does not specify PLE product or code behavior."
)
RUNTIME_REQUIRED_IDENTITIES = {
	# Authorization denials and access restrictions.
	"Course membership determines which private Course records an Instructor may use.",
	"**Instructors** can browse the content of Public and Archived **Blueprint Courses**.",
	"Removing a **Student** from a Course revokes future Course access but does not immediately delete the Student's Course records or Student Work.",
	"An **Instructor** can deactivate a Student's access to their Course.",
	"Deactivating Course access does not delete the Student Account or Student Work.",
	"An **Instructor** can restore the Student's Course access later.",
	"**Sysadmins** have full platform-administration capability but do not automatically have access to FERPA Course records.",
	"A Sysadmin may access Course or Student records when needed to resolve a specific support problem.",
	"Sysadmin support access should be limited to that support task and recorded for audit.",
	"**Students** access Question content through their Coursework rather than through the Question Library.",
	"Student navigation and pages should contain only Student interfaces and capabilities.",
	"Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.",
	"Public data should stay separate from private, answer-bearing, identifying, or radioactive FERPA data.",
	"FERPA-sensitive Student data should not become ordinary logs, analytics, URLs, or long-lived browser storage.",
	"FERPA access should be scoped through exact Course membership and **Student** ownership.",
	"**Sysadmins** receive only the FERPA access required for a specific administrative task.",
	"Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.",
	"PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.",
	"WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.",
	"H5P owns its runtime, interactions, state, and scoring.",
	"iMathAS owns its rendering and evaluation.",
	"A Question Backend returns an immutable credit fraction for each complete response it evaluates.",
	"Assessment Attempt submission and grading are fully automatic and require no **Instructor** action.",
	"Automatic grading does not require a separate Student or **Instructor** grading workflow.",
	"The **Student** does not see the grading outcome until the Assessment Attempt is submitted.",
	# Responsive, keyboard, and banner behavior.
	"Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.",
	"Every Student browser action should be usable with the keyboard alone.",
	"PLE responsively scales Course banners while preserving their aspect ratio.",
	# Backend rendering and grading, outcomes, and scoring.
	"Student Work includes Assessment Attempts, saved Question responses, grading outcomes, and the evidence needed to interpret that work after an Attempt is submitted.",
	"Changes to Question point values recalculate scores from the stored grading outcome without changing the outcome.",
	"PLE stores the immutable credit fraction as the grading outcome.",
	"When PLE requests a grading outcome, the Question Backend returns it without a deferred grading state.",
	"Assessment scores are calculated from stored credit fractions and current Question point values.",
	"Changing Question point values recalculates scores without another Question Backend interaction.",
	"The server owns the Attempt start and expiration times.",
	"Attempt time continues while the **Student** is disconnected or the browser is closed.",
	"A **Student** may reconnect, reload, or use another browser session to resume the same active Attempt.",
	"Attempt expiration is checked whenever a **Student** interacts with the Attempt.",
	"Background processing ensures expired Attempts are submitted even when the **Student** is no longer connected.",
	"When an Attempt expires, PLE submits the whole Attempt, finalizing its saved responses. Other Questions remain visibly unanswered, receive zero credit, and count as incorrect without being sent to the Question Backend.",
	"Resuming an Attempt does not reset, pause, or extend its time limit.",
	"Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.",
	"Course retention should follow Course Instance dates and its six-month Active lifetime rather than a fixed academic calendar.",
	"The latest Assessment deadline ends normal teaching and starts the Course Instance's FERPA retention clock.",
	"Creating or extending a later Assessment deadline may move those dates, but not beyond the six-month Active lifetime.",
	"Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.",
	"The configured FERPA retention policy determines the later notice, archive, recovery, and permanent deletion transitions.",
	"PLE warns the **Instructors** before the Course Instance becomes Inactive six months after creation.",
	"The six-month Active limit prevents Course reuse or deadline extensions from indefinitely delaying FERPA retention and deletion.",
	"Course inactivity and FERPA deletion are separate transitions; becoming Inactive does not itself delete Student records.",
	"Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.",
	"PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.",
	"Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.",
	"FERPA-sensitive Student data should be permanently deleted when its retention period expires.",
	"Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.",
	"FERPA retention intervals are operational configuration rather than separate product decisions.",
	"A background process should periodically find Course Instances whose retention deadlines have passed.",
	"Retention decisions should come from stored Course dates and the Course Instance creation time.",
	"The background process should execute retention policy rather than define when retention periods begin or end.",
	"Running the retention process late should produce the same retention decision as running it on schedule.",
	"The retention process should be safe to run repeatedly.",
	"PLE stores the credit fraction as the Question grading outcome.",
	"Course Instance Assessment scores are calculated from stored credit fractions and current Question point values.",
	"An unanswered Question contributes zero points to the Assessment score and counts as incorrect.",
	"When an Assessment has multiple submitted Attempts, the highest Assessment Attempt score is the Student's Assessment score.",
	"PLE uses Question point values directly to calculate Assessment scores.",
	"Changing Question point values recalculates affected Assessment scores.",
	"Score recalculation does not require another Question Backend interaction.",
	"Score recalculation does not change the stored Question grading outcome.",
}


#============================================
def parse_args() -> argparse.Namespace:
	"""Parse one checklist operation and an optional trusted part name."""
	parser = argparse.ArgumentParser(description="Build and validate the Human Guidance checklist.")
	operations = parser.add_mutually_exclusive_group(required=True)
	operations.add_argument("-b", "--build", dest="operation", action="store_const", const="build")
	operations.add_argument("-d", "--diff", dest="operation", action="store_const", const="diff")
	operations.add_argument("-c", "--consistency", dest="operation", action="store_const", const="consistency")
	operations.add_argument("-g", "--gate", dest="operation", metavar="PART")
	operations.add_argument("-s", "--splice", dest="splice_part", metavar="PART")
	return parser.parse_args()


#============================================
def remove_vendored_header(lines: list[str]) -> list[str]:
	"""Remove only Human Guidance's propagated header block."""
	clean_lines: list[str] = []
	in_header = False
	for line in lines:
		if line == "<!-- VENDORED HEADER: START -->":
			in_header = True
			continue
		if line == "<!-- VENDORED HEADER: END -->":
			in_header = False
			continue
		if not in_header:
			clean_lines.append(line)
	return clean_lines


#============================================
def source_lines() -> list[str]:
	"""Return authoritative Human Guidance lines without the vendored header."""
	return remove_vendored_header(HUMAN_GUIDANCE_PATH.read_text(encoding="utf-8").splitlines())


#============================================
def build_checklist_body() -> str:
	"""Convert Human Guidance to the verbatim status-bearing checklist form."""
	body: list[str] = []
	in_how_to_use = False
	for line in source_lines():
		heading = HEADING_PATTERN.match(line)
		if heading is not None:
			in_how_to_use = heading.group(2) == HOW_TO_USE_HEADING
			body.append(line)
			if in_how_to_use:
				body.extend(("", "Implementation status: N/A", f"Reason: {HOW_TO_USE_REASON}"))
			continue
		bullet = SOURCE_BULLET_PATTERN.match(line)
		if bullet is None:
			body.append(line)
		else:
			marker = "N/A" if in_how_to_use else "[ ]"
			body.append(f"{bullet.group(1)}- {marker} {bullet.group(2)}")
	return "\n".join(body).strip() + "\n"


#============================================
def checklist_header() -> str:
	"""Return the fixed status legend."""
	return ("# Human Guidance implementation compliance checklist\n\n"
		"Source: `docs/HUMAN_GUIDANCE.md`. Human Guidance remains authoritative. This file records\n"
		"implementation status only.\n\n"
		"- [x] Verified: implemented behavior matches the bullet. Evidence follows.\n"
		"- [ ] Unverified, or implementation differs. Mismatch follows.\n"
		"- N/A: audited and not an implementation requirement. Reason follows.\n\n")


#============================================
def logical_records(lines: list[str]) -> tuple[list[str], list[str]]:
	"""Extract headings and full logical bullets from source or checklist lines."""
	headings: list[str] = []
	bullets: list[str] = []
	current: str | None = None
	for line in lines:
		if HEADING_PATTERN.match(line) is not None:
			if current is not None:
				bullets.append(current)
				current = None
			headings.append(line)
			continue
		if STATUS_LINE_PATTERN.match(line) is not None or line.startswith("Implementation status:") or line.startswith("Reason:"):
			continue
		check = CHECKLIST_BULLET_PATTERN.match(line)
		not_applicable = NOT_APPLICABLE_PATTERN.match(line)
		source = SOURCE_BULLET_PATTERN.match(line) if check is None and not_applicable is None else None
		text = source.group(2) if source is not None else (check.group(3) if check is not None else (not_applicable.group(2) if not_applicable is not None else None))
		if text is not None:
			if current is not None:
				bullets.append(current)
			current = text
			continue
		if current is not None and line.startswith((" ", "\t")) and line.strip():
			current += " " + line.strip()
		elif line and current is not None:
			bullets.append(current)
			current = None
	if current is not None:
		bullets.append(current)
	return headings, bullets


#============================================
def source_records() -> tuple[list[str], list[str]]:
	"""Read authoritative heading and bullet identities in document order."""
	return logical_records(source_lines())


#============================================
def checklist_records(checklist_path: pathlib.Path) -> tuple[list[str], list[str]]:
	"""Read status-free Human Guidance identities from a checklist or audit part."""
	lines = checklist_path.read_text(encoding="utf-8").splitlines()
	source_headings, _source_bullets = source_records()
	first = next((index for index, line in enumerate(lines) if line in source_headings), len(lines))
	return logical_records(lines[first:])


#============================================
def part_records(part_name: str) -> tuple[list[str], list[str]]:
	"""Return the exact ordered Human Guidance subtrees assigned to an audit part."""
	lines = source_lines()
	selected: list[str] = []
	for root_name, root_level in zip(PART_MANIFEST[part_name], PART_ROOT_LEVELS[part_name], strict=True):
		start = next((i for i, line in enumerate(lines)
			if HEADING_PATTERN.match(line) is not None and
			len(HEADING_PATTERN.match(line).group(1)) == root_level and
			HEADING_PATTERN.match(line).group(2) == root_name), None)
		if start is None:
			raise ValueError(f"Human Guidance heading missing from manifest: {root_name}")
		level = len(HEADING_PATTERN.match(lines[start]).group(1))
		end = start + 1
		while end < len(lines):
			next_heading = HEADING_PATTERN.match(lines[end])
			if next_heading is not None and len(next_heading.group(1)) <= level:
				break
			end += 1
		selected.extend(lines[start:end])
	headings, bullets = logical_records(selected)
	return list(PART_CONTEXT_HEADINGS.get(part_name, ())) + headings, bullets


#============================================
def print_counts() -> None:
	"""Print source and checklist bullet counts."""
	_source_headings, source_bullets = source_records()
	_checklist_headings, checklist_bullets = checklist_records(CHECKLIST_PATH)
	print(f"Human Guidance bullets: {len(source_bullets)}")
	print(f"Checklist bullets: {len(checklist_bullets)}")


#============================================
def build() -> None:
	"""Create the initial checklist only when evidence has not been recorded."""
	if CHECKLIST_PATH.exists():
		print(CHECKLIST_PATH)
		raise SystemExit(1)
	CHECKLIST_PATH.parent.mkdir(parents=True, exist_ok=True)
	CHECKLIST_PATH.write_text(checklist_header() + build_checklist_body(), encoding="utf-8")
	print(CHECKLIST_PATH)
	print_counts()


#============================================
def print_differences(expected: list[str], actual: list[str], noun: str) -> bool:
	"""Print ordered list drift and return whether it exists."""
	if expected == actual:
		return False
	print(f"Changed {noun} sequence:")
	for line in difflib.unified_diff(expected, actual, lineterm=""):
		print(line)
	return True


#============================================
def diff() -> None:
	"""Compare generated identities against current Human Guidance."""
	if not CHECKLIST_PATH.exists():
		print(f"Checklist is missing: {CHECKLIST_PATH}")
		raise SystemExit(1)
	source_headings, source_bullets = source_records()
	checklist_headings, checklist_bullets = checklist_records(CHECKLIST_PATH)
	drift = print_differences(source_headings, checklist_headings, "heading")
	drift = print_differences(source_bullets, checklist_bullets, "bullet") or drift
	print_counts()
	if drift:
		raise SystemExit(1)


#============================================
def checklist_statuses(checklist_path: pathlib.Path) -> list[tuple[str, str, list[str]]]:
	"""Read each status-bearing bullet and its following status lines."""
	lines = checklist_path.read_text(encoding="utf-8").splitlines()
	source_headings, _source_bullets = source_records()
	first = next((index for index, line in enumerate(lines) if line in source_headings), len(lines))
	return checklist_statuses_for_lines(lines[first:])


#============================================
def needs_runtime_or_test(bullet_text: str) -> bool:
	"""Return whether this exact Human Guidance behavior needs observed evidence."""
	return bullet_text in RUNTIME_REQUIRED_IDENTITIES


#============================================
def evidence_has_locator(evidence_line: str) -> bool:
	"""Validate a stable symbol located in a real, in-repository source file."""
	# ASVS 2.2.1/2.2.2: accept only a literal path immediately followed by its locator.
	values = list(re.finditer(r"`([^`]+)`", evidence_line))
	for index, path_match in enumerate(values[:-1]):
		value = path_match.group(1)
		path_value = pathlib.PurePosixPath(value)
		if path_value.is_absolute() or ".." in path_value.parts:
			continue
		candidate = REPO_ROOT.joinpath(*path_value.parts)
		path_component = REPO_ROOT
		if any((path_component := path_component / part).is_symlink() for part in path_value.parts):
			continue
		# ASVS 5.3.2: resolve and contain the ultimate target before accepting evidence.
		try:
			resolved = candidate.resolve(strict=True)
			resolved.relative_to(REPO_ROOT.resolve())
		except (OSError, ValueError):
			continue
		if not resolved.is_file():
			continue
		# A numeric :line suffix is useful to readers but is not part of the symbol.
		symbol = values[index + 1].group(1)
		symbol = re.sub(r":[0-9]+\Z", "", symbol).strip()
		if not symbol:
			continue
		try:
			contents = resolved.read_text(encoding="utf-8")
		except (OSError, UnicodeDecodeError):
			continue
		if symbol in contents:
			return True
	return False


#============================================
def trusted_part_path(part_name: str) -> pathlib.Path:
	"""Resolve one allowlisted audit part under the fixed parts directory."""
	# ASVS 2.2.1/2.2.2: allowlist manifest names, never caller-selected paths.
	if PART_NAME_PATTERN.fullmatch(part_name) is None or part_name not in PART_MANIFEST:
		raise ValueError("PART must be a declared Human Guidance audit part")
	candidate = PARTS_DIRECTORY / part_name
	# ASVS 5.3.2: refuse a symlink escape from the repository-owned audit directory.
	if candidate.exists():
		try:
			candidate.resolve().relative_to(PARTS_DIRECTORY.resolve())
		except (OSError, ValueError):
			raise ValueError("Audit part resolves outside the fixed parts directory") from None
	return candidate


#============================================
def how_to_use_bullets() -> tuple[str, ...]:
	"""Return the five exact source records permitted to inherit the section reason."""
	lines = source_lines()
	start = next((index for index, line in enumerate(lines) if line == f"## {HOW_TO_USE_HEADING}"), None)
	if start is None:
		return ()
	end = next((index for index in range(start + 1, len(lines)) if lines[index].startswith("## ")), len(lines))
	return tuple(logical_records(lines[start:end])[1])


#============================================
def how_to_use_is_valid(
	part_path: pathlib.Path,
	statuses: list[tuple[str, str, list[str]]],
) -> bool:
	"""Validate the one explicit meta-guidance N/A convention."""
	lines = part_path.read_text(encoding="utf-8").splitlines()
	start = next((index for index, line in enumerate(lines) if line == "## How to use this guidance"), None)
	if start is None:
		return True
	end = next((index for index in range(start + 1, len(lines)) if lines[index].startswith("## ")), len(lines))
	section = lines[start:end]
	section_statuses = checklist_statuses_for_lines(section)
	reason = section_reason(section)
	return ("Implementation status: N/A" in section and
		normalize_whitespace(reason) == normalize_whitespace(HOW_TO_USE_REASON) and
		len(section_statuses) == len(how_to_use_bullets()) and
		all(status == "N/A" for status, _text, _lines in section_statuses))


#============================================
def normalize_whitespace(value: str) -> str:
	"""Compare prose without making Markdown line wrapping significant."""
	return " ".join(value.split())


#============================================
def section_reason(lines: list[str]) -> str:
	"""Return a section-level Reason value, including wrapped continuation lines."""
	for index, line in enumerate(lines):
		if not line.startswith("Reason:"):
			continue
		parts = [line.partition(":")[2].strip()]
		for continuation in lines[index + 1:]:
			if not continuation.strip() or HEADING_PATTERN.match(continuation) is not None:
				break
			if (CHECKLIST_BULLET_PATTERN.match(continuation) is not None or
					NOT_APPLICABLE_PATTERN.match(continuation) is not None or
					STATUS_LINE_PATTERN.match(continuation) is not None):
				break
			parts.append(continuation.strip())
		return " ".join(parts)
	return ""


#============================================
def checklist_statuses_for_lines(lines: list[str]) -> list[tuple[str, str, list[str]]]:
	"""Read status records from already-loaded checklist lines."""
	statuses: list[tuple[str, str, list[str]]] = []
	status: str | None = None
	text: str | None = None
	status_lines: list[str] = []
	for line in lines:
		match = CHECKLIST_BULLET_PATTERN.match(line)
		not_applicable = NOT_APPLICABLE_PATTERN.match(line)
		if match is not None or not_applicable is not None:
			if status is not None and text is not None:
				statuses.append((status, text, status_lines))
			status = match.group(2) if match is not None else "N/A"
			text = match.group(3) if match is not None else not_applicable.group(2)
			status_lines = []
			continue
		if status is not None and STATUS_LINE_PATTERN.match(line) is not None:
			status_lines.append(line.strip())
			continue
		if text is not None and line.startswith((" ", "\t")) and line.strip():
			text += " " + line.strip()
	if status is not None and text is not None:
		statuses.append((status, text, status_lines))
	return statuses


#============================================
def spliced_part_statuses() -> list[tuple[int, str, str, list[str]]]:
	"""Return available audit-part records tagged with their global source position."""
	result: list[tuple[int, str, str, list[str]]] = []
	position = 0
	for part_name in PART_MANIFEST:
		expected_bullets = part_records(part_name)[1]
		part_path = PARTS_DIRECTORY / part_name
		if part_path.exists() and part_path.is_file() and not part_path.is_symlink():
			statuses = checklist_statuses(part_path)
			if len(statuses) == len(expected_bullets):
				result.extend((position + index, status, bullet, lines)
					for index, (status, bullet, lines) in enumerate(statuses))
		position += len(expected_bullets)
	return result


#============================================
def duplicate_problems(records: list[tuple[int, str, str, list[str]]]) -> list[str]:
	"""Require each later global duplicate to name its first occurrence and status."""
	first_by_text: dict[str, tuple[str, int]] = {}
	problems: list[str] = []
	for position, status, bullet, status_lines in records:
		first = first_by_text.get(bullet)
		if first is None:
			first_by_text[bullet] = (status, position)
			continue
		first_status, _first_position = first
		if status != first_status:
			problems.append(f"Inconsistent duplicate statuses: {bullet}")
		if not any(line.startswith("- Owner:") and line[8:].strip() for line in status_lines):
			problems.append(f"Duplicate bullet lacks Owner: {bullet}")
	return problems


#============================================
def final_checklist_problems() -> list[str]:
	"""Validate final checklist order and duplicate ownership without audit parts."""
	source_bullets = source_records()[1]
	statuses = checklist_statuses(CHECKLIST_PATH)
	checklist_bullets = [bullet for _status, bullet, _status_lines in statuses]
	problems: list[str] = []
	if checklist_bullets != source_bullets:
		problems.append("Final checklist status records differ from Human Guidance bullet order.")
		return problems
	records = [
		(index, status, bullet, status_lines)
		for index, (status, bullet, status_lines) in enumerate(statuses)
	]
	return duplicate_problems(records)


#============================================
def gate(part_name: str) -> None:
	"""Validate one exact audit part and its status/evidence contract."""
	part_path = trusted_part_path(part_name)
	if not part_path.exists() or not part_path.is_file():
		print(f"Audit part is missing: {part_path}")
		raise SystemExit(1)
	expected_headings, expected_bullets = part_records(part_name)
	headings, bullets = checklist_records(part_path)
	problems: list[str] = []
	if headings != expected_headings:
		problems.append("Audit part headings differ from its exact manifest subtree.")
	if bullets != expected_bullets:
		problems.append("Audit part bullets differ from its exact manifest subtree.")
	statuses = checklist_statuses(part_path)
	if len(statuses) != len(expected_bullets):
		problems.append("Every Human Guidance bullet must carry exactly one [x], [ ], or N/A status.")
	is_how_to_use = part_name == "01_development.md" and any(heading.endswith(HOW_TO_USE_HEADING) for heading in headings)
	if is_how_to_use and not how_to_use_is_valid(part_path, statuses):
		problems.append("How to use this guidance requires section-level N/A status and Reason with only N/A bullets.")
	inherited_how_to = how_to_use_bullets()
	for index, (status, bullet_text, status_lines) in enumerate(statuses):
		# The exception belongs only to the source records at this assigned position.
		inherits_reason = (part_name == "01_development.md" and index < len(inherited_how_to) and
			expected_bullets[index:index + 1] == [inherited_how_to[index]])
		if status == "x":
			evidence = [line for line in status_lines if EVIDENCE_PATTERN.match(line)]
			if not evidence:
				problems.append(f"Verified bullet lacks Evidence (kind): {bullet_text}")
			elif any(not evidence_has_locator(line) for line in evidence):
				problems.append(f"Evidence lacks a real backticked repository path and stable symbol: {bullet_text}")
			if needs_runtime_or_test(bullet_text) and not any(EVIDENCE_PATTERN.match(line).group(1) in ("runtime", "test") for line in evidence):
				problems.append(f"Verified runtime-required bullet lacks runtime or test evidence: {bullet_text}")
		if status == " " and not any(line.startswith("- Mismatch:") for line in status_lines):
			problems.append(f"Unverified bullet lacks Mismatch: {bullet_text}")
		if status == "N/A" and not inherits_reason and not any(line.startswith("- Reason:") and line[9:].strip() for line in status_lines):
			problems.append(f"N/A bullet lacks Reason: {bullet_text}")
	part_offset = sum(len(part_records(name)[1]) for name in PART_MANIFEST if name != part_name and
		list(PART_MANIFEST).index(name) < list(PART_MANIFEST).index(part_name))
	this_records = [(part_offset + index, status, bullet, status_lines)
		for index, (status, bullet, status_lines) in enumerate(statuses)]
	# Compare this part's later duplicates with all available earlier part occurrences.
	all_records = spliced_part_statuses()
	all_by_position = {position: (status, bullet, lines) for position, status, bullet, lines in all_records}
	for position, status, bullet, status_lines in this_records:
		first_position = next((index for index, text in enumerate(source_records()[1]) if text == bullet), position)
		if position != first_position:
			first = all_by_position.get(first_position)
			if first is not None and first[0] != status:
				problems.append(f"Inconsistent duplicate statuses: {bullet}")
			if not any(line.startswith("- Owner:") and line[8:].strip() for line in status_lines):
				problems.append(f"Duplicate bullet lacks Owner: {bullet}")
	for problem in problems:
		print(problem)
	if problems:
		raise SystemExit(1)
	print(f"Audit part passes: {part_path}")


#============================================
def consistency() -> None:
	"""Reject duplicate source bullets assigned inconsistent implementation statuses."""
	if not CHECKLIST_PATH.exists():
		print(f"Checklist is missing: {CHECKLIST_PATH}")
		raise SystemExit(1)
	by_bullet: dict[str, set[str]] = collections.defaultdict(set)
	for status, bullet, _status_lines in checklist_statuses(CHECKLIST_PATH):
		by_bullet[bullet].add(status)
	bad = False
	for bullet, statuses in by_bullet.items():
		if len(statuses) > 1:
			bad = True
			print(f"Inconsistent duplicate statuses ({', '.join(sorted(statuses))}): {bullet}")
	for problem in final_checklist_problems():
		bad = True
		print(problem)
	spliced = spliced_part_statuses()
	source_bullets = source_records()[1]
	if [bullet for _position, _status, bullet, _lines in spliced] != source_bullets:
		bad = True
		print("Audit-part splice differs from the global Human Guidance bullet order.")
	for problem in duplicate_problems(spliced):
		bad = True
		print(problem)
	print_counts()
	if bad:
		raise SystemExit(1)


#============================================
def splice_part(part_name: str) -> None:
	"""Regenerate one checklist subtree from its audited part input."""
	part_path = trusted_part_path(part_name)
	if not CHECKLIST_PATH.exists():
		print(f"Checklist is missing: {CHECKLIST_PATH}")
		raise SystemExit(1)
	if not part_path.exists() or not part_path.is_file():
		print(f"Audit part is missing: {part_path}")
		raise SystemExit(1)
	gate(part_name)
	part_lines = part_path.read_text(encoding="utf-8").splitlines()
	context = PART_CONTEXT_HEADINGS.get(part_name, ())
	start_marker = context[0] if context else f"{'#' * PART_ROOT_LEVELS[part_name][0]} {PART_MANIFEST[part_name][0]}"
	part_names = list(PART_MANIFEST)
	part_index = part_names.index(part_name)
	end_marker: str | None = None
	if part_index + 1 < len(part_names):
		next_part = part_names[part_index + 1]
		next_context = PART_CONTEXT_HEADINGS.get(next_part, ())
		end_marker = (next_context[0] if next_context else
			f"{'#' * PART_ROOT_LEVELS[next_part][0]} {PART_MANIFEST[next_part][0]}")
	checklist_lines = CHECKLIST_PATH.read_text(encoding="utf-8").splitlines()
	try:
		start = next(index for index, line in enumerate(checklist_lines)
			if line == start_marker)
	except StopIteration:
		print(f"Checklist subtree start is missing: {start_marker}")
		raise SystemExit(1)
	end = (next((index for index in range(start + 1, len(checklist_lines))
		if end_marker is not None and checklist_lines[index] == end_marker), len(checklist_lines)))
	CHECKLIST_PATH.write_text("\n".join(checklist_lines[:start] + part_lines + checklist_lines[end:]) + "\n", encoding="utf-8")
	print(f"Spliced audit part: {part_path}")


#============================================
def main() -> None:
	"""Run the selected checklist lifecycle operation."""
	args = parse_args()
	if args.splice_part is not None:
		splice_part(args.splice_part)
	elif args.operation == "build":
		build()
	elif args.operation == "diff":
		diff()
	elif args.operation == "consistency":
		consistency()
	else:
		gate(args.operation)


if __name__ == "__main__":
	main()
