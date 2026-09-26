// scenarios_student_profile.ts - Responsive Student account profile captures.

import {
  directViewportCaptures,
  viewportCoverage,
  type ScenarioDefinition,
} from "./scenario_types";
import type { ScenarioRuntime } from "./runtime";
import { choosePersona, scrollTop } from "./visible_workflows";

async function studentProfile(runtime: ScenarioRuntime): Promise<void> {
  for (const checkpoint of [
    "profile_laptop",
    "profile_tablet",
    "profile_phone",
    "profile_square",
  ] as const) {
    const session = await runtime.open(checkpoint);
    try {
      const page = session.page;
      await choosePersona(page, "Mary Okafor");
      await page.getByRole("button", { name: "Profile", exact: true }).click();
      await page.getByRole("menuitem", { name: "Profile", exact: true }).click();
      await page.locator('[data-route-surface="profile"]').waitFor();
      await page.getByRole("heading", { level: 1, name: "Your profile", exact: true }).waitFor();
      await page.getByRole("heading", { level: 2, name: "Avatar Gallery", exact: true }).waitFor();
      await page.getByLabel("Time zone", { exact: true }).waitFor();
      await page
        .getByRole("list", { name: "Available provided avatars", exact: true })
        .getByRole("listitem")
        .first()
        .waitFor();
      await scrollTop(page);
      await runtime.captureCheckpoint(session, checkpoint);
    } finally {
      await runtime.close(session);
    }
  }
}

const profileCaptures = directViewportCaptures({
  checkpoint: "profile_laptop",
  area: "account",
  workflow: "profile preferences",
  state: "default profile",
  viewport: "laptop",
  privacyProfile: "student_self",
  caption: "Student Profile",
});

export const STUDENT_PROFILE_SCENARIO: ScenarioDefinition = {
  id: "student_profile",
  role: "student",
  captures: profileCaptures,
  viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
  run: studentProfile,
};
