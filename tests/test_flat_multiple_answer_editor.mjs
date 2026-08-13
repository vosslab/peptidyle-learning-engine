import assert from "node:assert/strict";
import test from "node:test";

import {
  addMultipleAnswerChoice,
  moveMultipleAnswerChoice,
  removeMultipleAnswerChoice,
  setMultipleAnswerChoiceText,
  setMultipleAnswerCorrect,
  validateMultipleAnswerResponse,
} from "../src/features/flat_question_authoring/flat_multiple_answer_model.ts";

function response() {
  return {
    kind: "multipleAnswer",
    choices: [
      { id: "kinase", text: "Kinase", feedback: null },
      { id: "lipid", text: "Lipid", feedback: null },
      { id: "enzyme", text: "Enzyme", feedback: null },
    ],
    correctChoices: ["kinase", "enzyme"],
  };
}

test("multiple-answer text edits and reordering retain choice IDs and exact correct IDs", () => {
  const renamed = setMultipleAnswerChoiceText(response(), "kinase", "Protein kinase");
  assert.equal(renamed.changed, true);
  assert.deepEqual(
    renamed.response.choices.map((choice) => choice.id),
    ["kinase", "lipid", "enzyme"],
  );
  assert.deepEqual(renamed.response.correctChoices, ["kinase", "enzyme"]);

  const moved = moveMultipleAnswerChoice(renamed.response, "enzyme", "earlier");
  assert.equal(moved.changed, true);
  assert.deepEqual(
    moved.response.choices.map((choice) => choice.id),
    ["kinase", "enzyme", "lipid"],
  );
  assert.deepEqual(moved.response.correctChoices, ["kinase", "enzyme"]);
});

test("multiple-answer correct-set changes are exact and a removed correct choice leaves actionable validation", () => {
  const unchecked = setMultipleAnswerCorrect(response(), "enzyme", false);
  assert.deepEqual(unchecked.response.correctChoices, ["kinase"]);

  const checked = setMultipleAnswerCorrect(unchecked.response, "lipid", true);
  assert.deepEqual(checked.response.correctChoices, ["kinase", "lipid"]);

  const removed = removeMultipleAnswerChoice(checked.response, "kinase");
  assert.equal(removed.changed, true);
  assert.deepEqual(
    removed.response.choices.map((choice) => choice.id),
    ["lipid", "enzyme"],
  );
  assert.deepEqual(removed.response.correctChoices, ["lipid"]);

  const oneCorrect = { ...response(), correctChoices: ["kinase"] };
  const onlyCorrectRemoved = removeMultipleAnswerChoice(oneCorrect, "kinase");
  assert.deepEqual(onlyCorrectRemoved.response.correctChoices, []);
  assert.equal(
    validateMultipleAnswerResponse(onlyCorrectRemoved.response).correctChoices,
    "Mark at least one choice as a correct answer before saving.",
  );
});

test("multiple-answer additions receive a new stable identity and limit operations explain recovery", () => {
  const original = response();
  const originalIds = new Set(original.choices.map((choice) => choice.id));
  const added = addMultipleAnswerChoice(original);
  assert.equal(added.changed, true);
  const newChoice = added.response.choices.at(-1);
  assert.notEqual(newChoice, undefined);
  assert.notEqual(newChoice.id, "");
  assert.equal(originalIds.has(newChoice.id), false);
  assert.equal(
    new Set(added.response.choices.map((choice) => choice.id)).size,
    added.response.choices.length,
  );
  assert.deepEqual(added.response.correctChoices, ["kinase", "enzyme"]);

  const firstCannotMoveEarlier = moveMultipleAnswerChoice(response(), "kinase", "earlier");
  assert.equal(firstCannotMoveEarlier.changed, false);
  assert.equal(firstCannotMoveEarlier.error, "That choice is already at the requested position.");
});
