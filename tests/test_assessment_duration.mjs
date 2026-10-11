import assert from "node:assert/strict";
import test from "node:test";

import {
  ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES,
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
