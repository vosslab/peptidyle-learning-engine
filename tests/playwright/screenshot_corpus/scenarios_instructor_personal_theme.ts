// scenarios_instructor_personal_theme.ts - M6 personal Theme evidence on global Instructor pages.

import type { Page } from "playwright";
import type { Theme } from "../../../generated/api/Theme";

import type { ScenarioDefinition } from "./scenario_types";
import { assertDocumentAppearance, persistDisplayMode } from "./theme_capture_workflow";
import { choosePersona, followCaptureLink, scrollTop } from "./visible_workflows";

const PERSONAL_THEME = "magma";
const PUBLISHED_QUESTION_TITLE = "Biochemistry Chapter 1: Charged functional groups";

interface SavedPersonalAppearance {
  readonly theme: Theme;
  readonly followsBrowser: boolean;
  readonly displayedMode: "light" | "dark";
}

async function openProfile(page: Page): Promise<void> {
  await choosePersona(page, "Elena Rivera");
  await openProfileFromCurrentSession(page);
}

async function openProfileFromCurrentSession(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Profile", exact: true }).click();
  await page.getByRole("menuitem", { name: "Profile settings", exact: true }).click();
  await page.locator('[data-route-surface="profile"]').waitFor();
  await page.getByRole("heading", { level: 2, name: "Appearance", exact: true }).waitFor();
}

async function savedPersonalAppearance(page: Page): Promise<SavedPersonalAppearance> {
  const theme = await page.locator('input[name="personal-theme"]:checked').inputValue();
  if (theme.length === 0) throw new Error("Instructor personal Theme has no selected value.");
  const displayModePreference = await page.evaluate(async () => {
    const response = await fetch("/api/account/settings");
    if (!response.ok) throw new Error("Could not read saved appearance settings.");
    return ((await response.json()) as { readonly displayModePreference?: unknown })
      .displayModePreference;
  });
  if (
    displayModePreference !== null &&
    displayModePreference !== "light" &&
    displayModePreference !== "dark"
  ) {
    throw new Error("Instructor display mode preference is not nullable Light or Dark.");
  }
  const displayedMode = await page.locator("html").getAttribute("data-display-mode");
  if (displayedMode !== "light" && displayedMode !== "dark") {
    throw new Error("Instructor display mode is not Light or Dark.");
  }
  return {
    theme: theme as Theme,
    followsBrowser: displayModePreference === null,
    displayedMode,
  };
}

async function restorePersonalAppearance(
  page: Page,
  saved: SavedPersonalAppearance,
): Promise<void> {
  await openProfileFromCurrentSession(page);
  const form = page.locator(".profile-appearance-form");
  const personalTheme = form.locator(`[data-theme-option="${saved.theme}"]`).getByRole("radio");
  if (!(await personalTheme.isChecked())) {
    const savedThemeResponse = page.waitForResponse((response) => {
      const request = response.request();
      return (
        request.method() === "PUT" &&
        new URL(response.url()).pathname === "/api/instructor/personal-theme"
      );
    });
    await personalTheme.check();
    await form.getByRole("button", { name: "Save personal theme", exact: true }).click();
    if (!(await savedThemeResponse).ok())
      throw new Error("Could not restore Instructor personal Theme.");
  }
  const followBrowser = page.getByRole("button", { name: "Follow browser setting", exact: true });
  if (saved.followsBrowser) {
    if (await followBrowser.isVisible()) {
      const clearedPreference = page.waitForResponse((response) => {
        const request = response.request();
        return (
          request.method() === "PUT" &&
          new URL(response.url()).pathname === "/api/account/appearance/display-mode-preference"
        );
      });
      await followBrowser.click();
      const response = await clearedPreference;
      if (!response.ok()) throw new Error("Could not restore browser-following display mode.");
      const settings = (await response.json()) as { readonly displayModePreference?: unknown };
      if (settings.displayModePreference !== null) {
        throw new Error("Display mode preference did not clear back to browser following.");
      }
    }
  } else {
    await persistDisplayMode(page, saved.displayedMode);
  }
}

async function persistPersonalTheme(page: Page): Promise<void> {
  await persistDisplayMode(page, "light");
  const form = page.locator(".profile-appearance-form");
  const personalTheme = form.locator(`[data-theme-option="${PERSONAL_THEME}"]`).getByRole("radio");
  if (!(await personalTheme.isChecked())) {
    const savedThemeResponse = page.waitForResponse((response) => {
      const request = response.request();
      return (
        request.method() === "PUT" &&
        new URL(response.url()).pathname === "/api/instructor/personal-theme"
      );
    });
    await personalTheme.check();
    await form.getByRole("button", { name: "Save personal theme", exact: true }).click();
    const saved = await savedThemeResponse;
    if (!saved.ok())
      throw new Error(`Instructor personal Theme ${PERSONAL_THEME} did not persist.`);
    const settings = (await saved.json()) as { readonly personalTheme?: string | null };
    if (settings.personalTheme !== PERSONAL_THEME) {
      throw new Error(
        `Instructor personal Theme saved ${String(settings.personalTheme)} instead of ${PERSONAL_THEME}.`,
      );
    }
  }
  await page.reload({ waitUntil: "commit" });
  await page.locator('[data-route-surface="profile"]').waitFor();
  if (
    !(await page.locator(`[data-theme-option="${PERSONAL_THEME}"]`).getByRole("radio").isChecked())
  ) {
    throw new Error(`Instructor personal Theme ${PERSONAL_THEME} did not persist across reload.`);
  }
  await assertDocumentAppearance(page, { theme: PERSONAL_THEME, mode: "light" });
}

async function openFilteredLibrary(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Questions", exact: true })
    .click();
  await page.getByRole("heading", { name: "Search Question Library", exact: true }).waitFor();
  await page.getByLabel("Search Question Library").fill("charged functional");
  await page.getByLabel("Search Question Library").press("Enter");
  await page.getByRole("heading", { name: PUBLISHED_QUESTION_TITLE, exact: true }).waitFor();
}

export const INSTRUCTOR_PERSONAL_THEME_SCENARIO: ScenarioDefinition = {
  id: "instructor_personal_theme_samples",
  role: "instructor",
  captures: [
    {
      checkpoint: "personal_theme_library_light",
      filenameStem: "theme-personal-library-light",
      area: "question library",
      workflow: "Instructor personal Theme comparison",
      state: "Magma Light filtered library",
      viewport: "laptop",
      privacyProfile: "instructor_answer_free",
      displayMode: "light",
      expectedTheme: PERSONAL_THEME,
      caption: "Instructor personal Magma Theme on a filtered Question Library in Light mode",
    },
    {
      checkpoint: "personal_theme_library_dark",
      filenameStem: "theme-personal-library-dark",
      area: "question library",
      workflow: "Instructor personal Theme comparison",
      state: "Magma Dark filtered library",
      viewport: "laptop",
      privacyProfile: "instructor_answer_free",
      displayMode: "dark",
      expectedTheme: PERSONAL_THEME,
      caption: "Instructor personal Magma Theme on a filtered Question Library in Dark mode",
    },
    {
      checkpoint: "personal_theme_question_dark",
      filenameStem: "theme-personal-question-detail-dark",
      area: "question library",
      workflow: "Instructor personal Theme comparison",
      state: "Magma Dark published Question detail",
      viewport: "laptop",
      privacyProfile: "instructor_answer_free",
      displayMode: "dark",
      expectedTheme: PERSONAL_THEME,
      caption: "Published Question detail using the Instructor personal Magma Theme in Dark mode",
    },
  ],
  viewportCoverage: {
    laptop: { status: "captured" },
    tablet: {
      status: "covered_by",
      target: "personal_theme_library_light",
      reason: "Personal Theme evidence uses the canonical Instructor laptop workspace.",
    },
    phone: {
      status: "covered_by",
      target: "personal_theme_library_light",
      reason: "Personal Theme evidence uses the canonical Instructor laptop workspace.",
    },
    square: {
      status: "covered_by",
      target: "personal_theme_library_light",
      reason: "Personal Theme evidence uses the canonical Instructor laptop workspace.",
    },
  },
  run: async (runtime): Promise<void> => {
    const session = await runtime.open("personal_theme_library_light");
    let saved: SavedPersonalAppearance | undefined;
    try {
      await openProfile(session.page);
      saved = await savedPersonalAppearance(session.page);
      await persistPersonalTheme(session.page);
      await openFilteredLibrary(session.page);
      await scrollTop(session.page);
      await runtime.captureCheckpoint(session, "personal_theme_library_light");
      await persistDisplayMode(session.page, "dark");
      await assertDocumentAppearance(session.page, { theme: PERSONAL_THEME, mode: "dark" });
      await openFilteredLibrary(session.page);
      await scrollTop(session.page);
      await runtime.captureCheckpoint(session, "personal_theme_library_dark");
      const result = session.page.locator(".record-list__row").filter({
        has: session.page.getByRole("heading", { name: PUBLISHED_QUESTION_TITLE, exact: true }),
      });
      await followCaptureLink(
        session.page,
        result.getByRole("link", { name: "Open", exact: true }),
      );
      await session.page.getByRole("region", { name: "Question prompt", exact: true }).waitFor();
      await scrollTop(session.page);
      await runtime.captureCheckpoint(session, "personal_theme_question_dark");
    } finally {
      try {
        if (saved !== undefined) await restorePersonalAppearance(session.page, saved);
      } finally {
        await runtime.close(session);
      }
    }
  },
};
