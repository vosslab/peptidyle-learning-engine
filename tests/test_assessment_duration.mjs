import assert from "node:assert/strict";
import test from "node:test";

import {
  ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES,
  assessmentDurationDisplay,
  assessmentDurationOverrideMinutesDraft,
  assessmentDurationOverrideMinutesError,
  assessmentDurationOverrideSecondsFromMinutesDraft,
  calculatedAssessmentDurationMinutes,
  calculatedAssessmentDurationSeconds,
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

test("Assessment duration hydrates only exact stored minutes and reports legacy seconds", () => {
  assert.equal(assessmentDurationOverrideMinutesDraft(null), "");
  assert.equal(assessmentDurationOverrideMinutesDraft(60), "1");
  assert.equal(assessmentDurationOverrideMinutesDraft(43_200), "720");
  assert.equal(assessmentDurationOverrideMinutesDraft(90), "");
  assert.match(
    assessmentDurationOverrideMinutesError("", 90) ?? "",
    /Stored duration 90 seconds \(stored duration is not a whole number of minutes\)/u,
  );
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
