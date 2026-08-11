"""Fast source-policy gate for the real UI walkthrough harness."""

import pathlib
import re


REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parent.parent
SIMULATOR_DIRECTORY = REPOSITORY_ROOT / "tests" / "playwright" / "simulator"
E2E_DIRECTORY = REPOSITORY_ROOT / "tests" / "e2e"
KEYBOARD_JOURNEY_GLOB = "ui_walkthrough_keyboard_j*.spec.ts"
PLATFORM_JOURNEYS = {f"ui_walkthrough_keyboard_j{number}.spec.ts" for number in range(1, 6)}
ALLOWED_PLATFORM_KEYS = {"Tab", "Shift+Tab", "Space", "Enter"}


def sources_under(directory: pathlib.Path, suffixes: tuple[str, ...]) -> list[pathlib.Path]:
	"""Return stable harness sources under one owned test directory."""
	paths = []
	for path in directory.rglob("*"):
		if path.is_file() and path.suffix in suffixes:
			paths.append(path)
	return sorted(paths)


def harness_sources(repository_root: pathlib.Path) -> list[pathlib.Path]:
	"""Select the simulator implementation, runner, and visible keyboard journeys."""
	paths = [
		path
		for path in sources_under(
			repository_root / "tests" / "playwright" / "simulator",
			(".ts",),
		)
		if not path.name.endswith(".spec.ts")
	]
	paths.extend(
		sources_under(
			repository_root / "tests" / "walkthrough",
			(".py", ".sh", ".ts"),
		)
	)
	paths.extend(
		[
			repository_root / "tests" / "e2e" / "e2e_ui_walkthrough.py",
			repository_root / "tests" / "e2e" / "e2e_ui_walkthrough.sh",
		]
	)
	paths.extend((repository_root / "tests" / "playwright").glob(KEYBOARD_JOURNEY_GLOB))
	paths.append(repository_root / "tests" / "playwright" / "ui_walkthrough_instructor_setup.spec.ts")
	paths.append(repository_root / "tests" / "playwright" / "ui_walkthrough_live_config.ts")
	return sorted({path for path in paths if path.is_file()})


def matches(pattern: str, source: str) -> bool:
	"""Match one case-insensitive policy pattern across source lines."""
	return re.search(pattern, source, flags=re.IGNORECASE | re.MULTILINE) is not None


def common_violations(source: str) -> list[str]:
	"""Find imports and database setup that bypass the public harness boundary."""
	violations = []
	if matches(r"(?:from|import|require)\s*\(?[^\n]*?(?:^|[/'\"])\.{0,3}(?:src|crates)/", source):
		violations.append("imports-product-internals")
	if matches(r"(?:from|import|require)\s*\(?[^\n]*?(?:generated(?:/|[\"']))", source):
		violations.append("imports-generated-private-data")
	if matches(r"\bimport\s*\(", source):
		violations.append("uses-dynamic-import")
	if matches(
		r"\b(?:psql|postgres(?:ql)?|sqlx|database_url|sqlite|mysql|pragma|with\s+\w+\s+as\s*\(|select\s+.+\s+from|insert\s+into|update\s+\w+\s+set|delete\s+from|create\s+table|alter\s+table|drop\s+table|(?:new\s+)?(?:pool|client)\s*\(|create(?:pool|client)\s*\()\b",
		source,
	):
		violations.append("uses-database-shaped-setup")
	return violations


def residual_member_violations(path: pathlib.Path, source: str) -> list[str]:
	"""Reject every browser-control member left after the narrow contract allowlist."""
	remaining = source
	remaining = remaining.replace("page.goto(\"/\")", "")
	remaining = remaining.replace("page.goto('/')", "")
	remaining = remaining.replace("page.goto(`/`)", "")
	for key in ALLOWED_PLATFORM_KEYS:
		remaining = remaining.replace(f'page.keyboard.press("{key}")', "")
		remaining = remaining.replace(f"page.keyboard.press('{key}')", "")
		remaining = remaining.replace(f"page.keyboard.press(`{key}`)", "")
	remaining = remaining.replace(
		"target.evaluate((element) => element === document.activeElement)",
		"",
	)
	if matches(
		r"\.(?:goto|click|dblclick|hover|tap|press|focus|type|\$eval|\$\$eval|evaluate|check|uncheck|selectOption|setChecked|dispatchEvent)\b",
		remaining,
	):
		return ["uses-residual-browser-control-member"]
	return []


def has_unapproved_keyboard_press(source: str) -> bool:
	"""Require journey key events to spell one platform-contract key literally."""
	for match in re.finditer(r"keyboard\.press\s*\(\s*([^)]*)\)", source):
		argument = match.group(1).strip()
		if len(argument) < 2 or argument[0] not in "\"'`" or argument[-1] != argument[0]:
			return True
		if argument[1:-1] not in ALLOWED_PLATFORM_KEYS:
			return True
	return False


def has_nonroot_goto(source: str) -> bool:
	"""Permit only the intentional root entry route before visible navigation."""
	for match in re.finditer(r"(?:page|\w+)\.goto\s*\(\s*([^)]*)\)", source):
		argument = match.group(1).strip()
		if argument not in {"\"/\"", "'/'", "`/`"}:
			return True
	return False


def keyboard_violations(path: pathlib.Path, source: str, platform_path: bool) -> list[str]:
	"""Reject browser shortcuts and answer-bearing assertions in journey specifications."""
	violations = []
	if matches(r"\b(?:storageState|addCookies|cookies?\s*\(|APIRequest|page\.request|context\.request|request\.(?:get|post|put|patch|delete)|(?:api|client|http)\.(?:get|post|put|patch|delete)|page\.route|fetch\s*\()\b", source):
		violations.append("uses-private-browser-or-api-shortcut")
	if matches(r"[\"'`]/(?:(?:api/)?(?:private|internal)/|(?:api/)?scores?(?:[/?\"'`]))", source):
		violations.append("uses-private-endpoint")
	if matches(r"expect[\s\S]{0,500}?(?:correct\s+(?:answer|choice|response)|incorrect\s+(?:answer|choice|response)|answer\s*(?:key|is|was|:)|expected\s+answer|solution|rationale|score)", source):
		violations.append("asserts-answer-bearing-content")
	if matches(r"\b(?:toContainText|toHaveInnerText|getByText|textContent)\b|locator\s*\(\s*[\"']body[\"']", source):
		violations.append("uses-body-text-assertion")
	if matches(
		r"(?:\bcatch\s*\([^)]*\)\s*\{|\.catch\s*\([^)]*\)\s*=>?\s*\{?|\.then\s*\([^,]+,\s*(?:\([^)]*\)|\w+)\s*=>\s*\{?)[\s\S]{0,600}?\b(?:PASS|passed)\b",
		source,
	):
		violations.append("converts-hidden-failure-to-pass")
	if platform_path:
		if has_unapproved_keyboard_press(source):
			violations.append("uses-unapproved-platform-key")
		if has_nonroot_goto(source):
			violations.append("uses-nonroot-direct-navigation")
		if matches(
			r"(?:\.click\s*\(|\.dblclick\s*\(|\.hover\s*\(|\.tap\s*\(|\bmouse\.|\btouchscreen\.|\.focus\s*\(|\.check\s*\(|\.uncheck\s*\(|\.selectOption\s*\(|\.setChecked\s*\(|\.dispatchEvent\s*\(|\blocator\.press\s*\(|\bgoBack\s*\(|\bgoForward\s*\(|\bhistory\.)",
			source,
		):
			violations.append("uses-non-platform-keyboard-path")
	return violations


def scan_source(path: pathlib.Path, source: str) -> list[str]:
	"""Return path-qualified policy violations for one harness source."""
	violations = common_violations(source)
	violations.extend(residual_member_violations(path, source))
	name = path.name
	if name.startswith("ui_walkthrough_keyboard_j") or name == "ui_walkthrough_instructor_setup.spec.ts":
		violations.extend(keyboard_violations(path, source, name in PLATFORM_JOURNEYS or name == "ui_walkthrough_instructor_setup.spec.ts"))
	if name == "keyboard_walkthrough.ts" and has_unapproved_keyboard_press(source):
		violations.append("uses-unapproved-platform-key")
	return [f"{path.as_posix()}: {violation}" for violation in violations]


def workspace_violations(repository_root: pathlib.Path) -> list[str]:
	"""Scan the owned harness surface without inspecting product implementation files."""
	violations = []
	for path in harness_sources(repository_root):
		source = path.read_text(encoding="utf8")
		violations.extend(scan_source(path.relative_to(repository_root), source))
	return violations


def test_workspace_walkthrough_harness_stays_independent() -> None:
	"""The committed simulator uses only public arrangements and visible browser actions."""
	assert workspace_violations(REPOSITORY_ROOT) == []


def test_scanner_rejects_product_database_and_private_browser_shortcuts() -> None:
	"""Hostile source cannot smuggle product, SQL, or request-state access into a journey."""
	source = """
import { privateThing } from \"../../src/privateThing\";
import { serverThing } from \"../../crates/server\";
import generatedPrivateData from \"./generated/private-data\";
await import(\"./late-bound-helper\");
const row = await page.request.get(\"/private/scores\");
await context.addCookies([]);
await page.goto(\"/api/private/score\");
const sql = \"SELECT secret FROM attempts\";
const cte = \"WITH attempts AS (SELECT 1) SELECT * FROM attempts\";
const pragma = \"PRAGMA table_info(attempts)\";
const client = new Client();
await page.evaluate(() => document.body.textContent);
"""
	violations = scan_source(pathlib.Path("tests/playwright/ui_walkthrough_keyboard_j2.spec.ts"), source)
	assert {
		"imports-product-internals",
		"imports-generated-private-data",
		"uses-dynamic-import",
		"uses-database-shaped-setup",
		"uses-residual-browser-control-member",
	} <= {item.rsplit(": ", 1)[1] for item in violations}


def test_scanner_rejects_nonvisible_platform_and_answer_shortcuts() -> None:
	"""Hostile J1 source cannot replace keyboard evidence with internal or answer evidence."""
	source = """
await page.locator(\"button\").click();
await page.keyboard.press(\"ArrowDown\");
await page.keyboard.press(\"q\");
await page.goto(\"/courses/private-course\");
await page.locator(\"input\").focus().check();
await context.cookies();
await fetch(\"/internal/run\");
await api.get(\"/private/attempts\");
await expect(page).toContainText(
  \"Correct answer is nitrogen\",
);
try { await run(); } catch (error) { return \"PASS\"; }
await Promise.resolve().catch(() => \"PASS\");
"""
	violations = scan_source(pathlib.Path("tests/playwright/ui_walkthrough_keyboard_j1.spec.ts"), source)
	assert {
		"uses-non-platform-keyboard-path",
		"uses-unapproved-platform-key",
		"uses-nonroot-direct-navigation",
		"uses-private-browser-or-api-shortcut",
		"uses-private-endpoint",
		"asserts-answer-bearing-content",
		"converts-hidden-failure-to-pass",
	} <= {item.rsplit(": ", 1)[1] for item in violations}


def test_scanner_rejects_instructor_pointer_history_and_private_body_shortcuts() -> None:
	"""The setup prerequisite has the same keyboard and privacy policy as learner journeys."""
	source = """
await page.getByRole("button").click();
await page.goBack();
await context.request.get("/api/courses");
await context.cookies();
await page.locator("body").textContent();
await page.evaluate(() => localStorage.getItem("private"));
"""
	violations = scan_source(pathlib.Path("tests/playwright/ui_walkthrough_instructor_setup.spec.ts"), source)
	assert {
		"uses-non-platform-keyboard-path",
		"uses-private-browser-or-api-shortcut",
		"uses-body-text-assertion",
		"uses-residual-browser-control-member",
	} <= {item.rsplit(": ", 1)[1] for item in violations}


def test_scanner_rejects_residual_members_aliases_and_body_text() -> None:
	"""Aliases cannot evade the closed list of browser controls or text assertions."""
	source = """
const navigate = page.goto;
const keypress = page.keyboard.press;
const focus = page.locator("input").focus;
const type = page.keyboard.type;
await page.$eval("input", (element) => element);
await page.$$eval("input", (elements) => elements);
await expect(page.locator("body")).toContainText("nitrogen");
"""
	violations = scan_source(pathlib.Path("tests/playwright/ui_walkthrough_keyboard_j3.spec.ts"), source)
	names = {item.rsplit(": ", 1)[1] for item in violations}
	assert "uses-residual-browser-control-member" in names
	assert "uses-body-text-assertion" in names


def test_scanner_allows_only_the_structural_active_element_observation() -> None:
	"""The shared native-tab helper may observe focus but cannot mutate page state."""
	accepted = """
await page.keyboard.press("Tab");
if (await target.evaluate((element) => element === document.activeElement)) return;
"""
	rejected = "await target.evaluate((element) => element.click());"
	accepted_violations = scan_source(pathlib.Path("tests/playwright/simulator/keyboard_walkthrough.ts"), accepted)
	rejected_violations = scan_source(pathlib.Path("tests/playwright/simulator/keyboard_walkthrough.ts"), rejected)
	assert accepted_violations == []
	assert "uses-residual-browser-control-member" in {
		item.rsplit(": ", 1)[1] for item in rejected_violations
	}
