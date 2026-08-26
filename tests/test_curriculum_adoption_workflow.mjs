import assert from "node:assert/strict";
import test from "node:test";

import {
  curriculumAdoptionNextInstruction,
  curriculumAdoptionOperationPresentation,
  withCurriculumAdoptionOperation,
  withCurriculumAdoptionSource,
} from "../src/features/curriculum_adoption/curriculum_adoption_model.ts";

test("curriculum adoption keeps the selected source while the Instructor changes operation", () => {
  const source = { kind: "blueprint", reference: "BP-42" };
  const next = withCurriculumAdoptionOperation({ operation: "blueprint", source }, "rollover");

  assert.equal(next.operation, "rollover");
  assert.equal(next.source, source);
});

test("curriculum adoption source selection retains target choices through a recovery", () => {
  const targetTerm = {
    startDate: "2027-01-11",
    endDate: "2027-05-07",
    timeZone: "America/Chicago",
  };
  const replacements = [
    {
      position: { moduleIndex: null, assignmentIndex: 0, entryIndex: 0, candidateIndex: null },
      question: "AAA-BBBB",
    },
  ];
  const next = withCurriculumAdoptionSource(
    { source: undefined, targetTerm, replacements },
    { kind: "alpha", reference: "AC-7" },
  );

  assert.deepEqual(next.targetTerm, targetTerm);
  assert.deepEqual(next.replacements, replacements);
  assert.deepEqual(next.source, { kind: "alpha", reference: "AC-7" });
});

test("curriculum adoption instructions name the next action at every major stage", () => {
  for (const stage of ["choose", "previewing", "preview", "applying", "receipt", "recovery"]) {
    assert.match(curriculumAdoptionNextInstruction(stage, "blueprint"), /.+/u);
  }
  assert.equal(curriculumAdoptionOperationPresentation("termShift").requiresSource, false);
});
