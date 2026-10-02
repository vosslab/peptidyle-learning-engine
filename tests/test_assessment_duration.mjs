import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES,
  assessmentDurationDefaultDescription,
  assessmentDurationDisplay,
  assessmentDurationOverrideMinutesDraft,
  assessmentDurationOverrideMinutesError,
  assessmentDurationOverrideSecondsFromMinutesDraft,
  calculatedAssessmentDurationMinutes,
  calculatedAssessmentDurationSeconds,
  NO_CLOSING_TIME_GUIDANCE,
  NO_OPENING_TIME_GUIDANCE,
  OPTIONAL_DURATION_OVERRIDE_GUIDANCE,
  UNLIMITED_ATTEMPTS_GUIDANCE,
} from "../src/assessment_duration.ts";

test("Assessment duration defaults are calculated in whole minutes before serializing seconds", () => {
  assert.equal(calculatedAssessmentDurationMinutes(1), 2);
  assert.equal(calculatedAssessmentDurationMinutes(2), 3);
  assert.equal(calculatedAssessmentDurationMinutes(250), 375);
  assert.equal(calculatedAssessmentDurationMinutes(0), null);
  assert.equal(calculatedAssessmentDurationSeconds(2), 180);
});

test("Assessment duration overrides accept only blank or whole minutes and serialize seconds", () => {
  assert.equal(assessmentDurationOverrideSecondsFromMinutesDraft(""), null);
  assert.equal(assessmentDurationOverrideSecondsFromMinutesDraft("1"), 60);
  assert.equal(
    assessmentDurationOverrideSecondsFromMinutesDraft(
      ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES.toString(),
    ),
    43_200,
  );
  for (const invalid of ["0", "1.5", "721", " 1", "1e2"]) {
    assert.equal(assessmentDurationOverrideSecondsFromMinutesDraft(invalid), undefined);
  }
});

test("Assessment duration hydration preserves every supplied second", () => {
  assert.equal(assessmentDurationOverrideMinutesDraft(null), "");
  assert.equal(assessmentDurationOverrideMinutesDraft(60), "1");
  assert.equal(assessmentDurationOverrideMinutesDraft(43_200), "720");
  assert.equal(assessmentDurationOverrideMinutesDraft(90), "1.5");
  assert.match(assessmentDurationOverrideMinutesError("1.5") ?? "", /whole number/u);
});

test("timing settings name minutes, a blank override, no closing time, and unlimited Attempts", () => {
  assert.match(assessmentDurationDefaultDescription(1), /2 minutes/u);
  assert.equal(
    OPTIONAL_DURATION_OVERRIDE_GUIDANCE,
    "Leave the override blank to use this calculated default.",
  );
  assert.equal(
    NO_OPENING_TIME_GUIDANCE,
    "Leave the available date and time blank for no opening time.",
  );
  assert.equal(
    NO_CLOSING_TIME_GUIDANCE,
    "Leave the closing date and time blank for no closing time.",
  );
  assert.equal(UNLIMITED_ATTEMPTS_GUIDANCE, "Leave blank for unlimited Attempts.");
  const page = readFileSync(
    new URL(
      "../src/pages/assessment_workspace/assessment_workspace_policies_page.tsx",
      import.meta.url,
    ),
    "utf8",
  );
  for (const name of [
    "AssessmentWorkspacePoliciesPage",
    "OPTIONAL_DURATION_OVERRIDE_GUIDANCE",
    "NO_OPENING_TIME_GUIDANCE",
    "NO_CLOSING_TIME_GUIDANCE",
    "UNLIMITED_ATTEMPTS_GUIDANCE",
  ]) {
    assert.equal(page.includes(name), true, name);
  }
});

test("Assessment duration display uses minutes and hours without rounding legacy seconds", () => {
  assert.equal(assessmentDurationDisplay(60), "1 minute");
  assert.equal(
    assessmentDurationDisplay(90),
    "90 seconds (stored duration is not a whole number of minutes)",
  );
  assert.equal(assessmentDurationDisplay(3_600), "1 hour");
  assert.equal(assessmentDurationDisplay(5_400), "1 hour 30 minutes");
});
