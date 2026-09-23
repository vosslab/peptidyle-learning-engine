// Opens exactly one registered current-source fixture for manual Chromium inspection.

import { once } from "node:events";

import { fastUiCase } from "./fast_ui_case_registry.mjs";
import { openProvidedAvatarPickerHarness } from "./helper_provided_avatar_picker_harness.mjs";
import { openRecordListHarness } from "./helper_record_list_harness.mjs";
import { openStudentCourseEntryHarness } from "./helper_student_course_entry_harness.mjs";
import { openRibbonShellEvidencePage } from "./ribbon_shell_helpers.mjs";

function requestedCase(argv) {
  if (argv.length !== 2 || argv[0] !== "--case") {
    throw new Error("Usage: fast_ui_headed_case.mjs --case <name>");
  }
  return argv[1];
}

const caseName = requestedCase(process.argv.slice(2));
const selectedCase = fastUiCase(caseName);

const opened =
  selectedCase.entrypoint === "provided-avatar-picker-harness"
    ? await openProvidedAvatarPickerHarness({ headless: false })
    : selectedCase.entrypoint === "student-course-entry-m6-harness"
      ? await openStudentCourseEntryHarness({ mode: selectedCase.mode, headless: false })
      : selectedCase.kind === "primitive"
        ? await openRecordListHarness({ headless: false })
        : await openRibbonShellEvidencePage({ headless: false });
const { browser, consoleErrors, harnessServer, page, pageErrors } = opened;
try {
  if (selectedCase.entrypoint === "current-production-app") {
    await page.evaluate((pathname) => {
      window.ribbonShell.releaseSession();
      window.ribbonShell.currentNavigate(pathname);
    }, selectedCase.pathname);
  }
  const fixture =
    selectedCase.entrypoint === "provided-avatar-picker-harness"
      ? page.getByRole("group", { name: "Choose an avatar" })
      : selectedCase.entrypoint === "student-course-entry-m6-harness"
        ? page.getByRole("heading", {
            name: "Biochemistry 301: Proteins and Peptides",
            exact: true,
          })
        : selectedCase.kind === "primitive"
          ? page.locator(`[data-record-list-case="${selectedCase.fixture}"]`)
          : page.locator('[data-m10-case="current-production"]');
  await fixture.scrollIntoViewIfNeeded();
  await fixture.waitFor({ state: "visible" });
  process.stdout.write(`Inspecting ${caseName}. Press Ctrl-C to close Chromium.\n`);
  await Promise.race([once(process, "SIGINT"), once(process, "SIGTERM")]);
  if (pageErrors.length > 0 || consoleErrors.length > 0) {
    throw new Error([...pageErrors, ...consoleErrors].join(" | "));
  }
} finally {
  await browser.close();
  await harnessServer.close();
}
