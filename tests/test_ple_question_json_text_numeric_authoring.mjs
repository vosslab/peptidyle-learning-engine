import assert from "node:assert/strict";
import test from "node:test";

import {
  addFillInAnswer,
  normalizedAcceptedAnswer,
  removeFillInAnswer,
  setFillInAnswer,
  validateFillInResponse,
} from "../src/features/ple_question_json_authoring/question_json_fill_in_model.ts";
import {
  numericResponseFromAuthoring,
  parseNumericLiteral,
  setNumericResponseToleranceKind,
  setNumericResponseToleranceValue,
  validateNumericResponse,
} from "../src/features/ple_question_json_authoring/question_json_numeric_model.ts";

function fillIn() {
  return { kind: "fillIn", answers: ["ATP"], matchMode: "caseInsensitive", maxLength: 12 };
}

test("fill-in operations preserve authored text and refuse invalid saved-answer states through validation", () => {
  const original = fillIn();
  const changed = setFillInAnswer(original, 0, "  ATP synthase  ");
  assert.equal(changed.answers[0], "  ATP synthase  ");
  assert.equal(original.answers[0], "ATP");
  assert.equal(normalizedAcceptedAnswer(" ATP\u00a0SYNTHASE "), "atp synthase");
  assert.equal(validateFillInResponse({ ...changed, maxLength: 30 }).valid, true);
  assert.equal(
    validateFillInResponse({ ...changed, answers: ["ATP", " atp "] }).issues["answers.1"],
    "This repeats another accepted answer after ordinary spacing and capitalization are ignored.",
  );
  assert.equal(
    validateFillInResponse({ ...changed, answers: [""] }).issues["answers.0"],
    "Enter an accepted answer or remove this row.",
  );
  assert.equal(
    validateFillInResponse({ ...changed, answers: ["answer longer than the limit"], maxLength: 4 })
      .issues["answers.0"],
    "This answer is longer than the student response limit.",
  );
});

test("fill-in add and remove operations retain one accepted-answer row", () => {
  const added = addFillInAnswer(fillIn());
  assert.equal(added.answers.length, 2);
  assert.equal(added.answers[0], "ATP");
  assert.notEqual(added.answers[1].trim(), "");
  const editedAddedAnswer = setFillInAnswer(added, 1, "ADP");
  assert.equal(editedAddedAnswer.answers[1], "ADP");
  assert.deepEqual(removeFillInAnswer(fillIn(), 0), fillIn());
  const oneRemaining = removeFillInAnswer(editedAddedAnswer, 1);
  assert.equal(oneRemaining.answers.length, 1);
  assert.equal(setFillInAnswer(oneRemaining, 0, "ADP").answers[0], "ADP");
});

test("numeric literals accept complete decimal or scientific notation without rewriting the literal", () => {
  assert.equal(parseNumericLiteral("6.022e23"), 6.022e23);
  assert.equal(parseNumericLiteral("-0.25"), -0.25);
  assert.equal(parseNumericLiteral("6.022e"), null);
  assert.equal(parseNumericLiteral("Infinity"), null);
  assert.equal(parseNumericLiteral("0x10"), null);
});

test("numeric validation exposes only applicable tolerance errors and rejects invalid values", () => {
  assert.equal(validateNumericResponse("6.02e23", { kind: "exact" }, null).valid, true);
  assert.equal(
    validateNumericResponse("NaN", { kind: "exact" }, null).issues.answer,
    "Enter a finite numeric value, such as 6.02e23.",
  );
  assert.equal(
    validateNumericResponse("1", { kind: "absolute", epsilon: -0.1 }, null).issues.epsilon,
    "Absolute tolerance must be a finite value of zero or greater.",
  );
  assert.equal(
    validateNumericResponse("1", { kind: "relative", fraction: Number.POSITIVE_INFINITY }, null)
      .issues.fraction,
    "Relative tolerance must be a finite fraction of zero or greater.",
  );
  assert.equal(
    validateNumericResponse("1", { kind: "significantFigures", digits: 0 }, null).issues.digits,
    "Significant figures must be a whole number from 1 through 255.",
  );
  assert.equal(
    validateNumericResponse("1", { kind: "exact" }, " ").issues.unit,
    "Leave the unit blank or enter a short unit label.",
  );
});

test("numeric operations create one applicable tolerance field and construct only valid source responses", () => {
  const relative = setNumericResponseToleranceValue(
    setNumericResponseToleranceKind("relative"),
    0.01,
  );
  assert.deepEqual(relative, { kind: "relative", fraction: 0.01 });
  const response = { kind: "numeric", answer: 0, tolerance: { kind: "exact" }, unit: null };
  assert.deepEqual(numericResponseFromAuthoring(response, "6.022e23", relative, "mol^-1"), {
    kind: "numeric",
    answer: 6.022e23,
    tolerance: relative,
    unit: "mol^-1",
  });
  assert.equal(numericResponseFromAuthoring(response, "1e", { kind: "exact" }, null), null);
});
