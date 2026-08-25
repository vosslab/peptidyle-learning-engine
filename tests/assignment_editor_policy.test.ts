import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeAssignmentCreateInput,
  decodeAssignmentEditorInput,
  decodeLearnerDisclosurePolicy,
} from "../src/api/decoders/catalog_course";
import {
  assignmentCreateInput,
  assignmentInput,
  createMasteryAssignmentDraft,
} from "../src/pages/assignment_editor_model";

const courseId = "0198e000-0000-7000-8000-000000000014";
const item = {
  id: "0198e000-0000-7000-8000-000000000017",
  questionId: "7K3-M9QP",
  title: "Peptide bond resonance and planarity",
  backend: "native" as const,
  capabilities: [],
  position: 0,
  pointsPossible: "1",
  deliveryState: "active" as const,
  scoringMode: "normal" as const,
};

test("new assignments use direct-cutover disclosure defaults", () => {
  const draft = createMasteryAssignmentDraft(courseId);
  assert.deepEqual(draft.disclosurePolicy, {
    score: "afterSubmit",
    perItemCorrectness: "afterSubmit",
    feedbackText: "afterSubmit",
    solution: "afterSubmit",
    classStatistics: "never",
  });
});

test("assignment disclosure policy round-trips in create and update bodies", () => {
  const draft = {
    ...createMasteryAssignmentDraft(courseId),
    title: "Peptide practice",
    entries: [{ ...item, kind: "fixed" as const }],
    disclosurePolicy: {
      score: "duringAttempt" as const,
      perItemCorrectness: "afterSubmit" as const,
      feedbackText: "afterDue" as const,
      solution: "afterClose" as const,
      classStatistics: "never" as const,
    },
  };

  const create = assignmentCreateInput(draft);
  assert.deepEqual(decodeAssignmentCreateInput(create), create);
  const update = assignmentInput(draft);
  assert.deepEqual(decodeAssignmentEditorInput(update), update);
});

test("assignment disclosure policy rejects omissions and unknown members", () => {
  const valid = assignmentCreateInput({
    ...createMasteryAssignmentDraft(courseId),
    title: "Peptide practice",
    entries: [{ ...item, kind: "fixed" as const }],
  });
  const missing = {
    ...valid,
    disclosurePolicy: { ...valid.disclosurePolicy },
  } as Record<string, unknown>;
  delete (missing.disclosurePolicy as Record<string, unknown>).solution;
  assert.throws(() => decodeAssignmentCreateInput(missing));
  assert.throws(() =>
    decodeAssignmentCreateInput({
      ...valid,
      disclosurePolicy: { ...valid.disclosurePolicy, privateAnswerKey: "no" },
    }),
  );
  assert.throws(() => decodeLearnerDisclosurePolicy({ score: "afterSubmit" }, "response"));
  assert.throws(() =>
    decodeLearnerDisclosurePolicy(
      { ...valid.disclosurePolicy, privateAnswerKey: "not allowed" },
      "response",
    ),
  );
});

test("assignment create request decoder rejects unknown disclosure policy members", () => {
  assert.throws(() =>
    decodeAssignmentCreateInput({
      title: "Disclosure policy",
      entries: [
        {
          kind: "fixed",
          questionId: "7K3-M9QP",
          position: 0,
          pointsPossible: "1",
          deliveryState: "active",
          scoringMode: "normal",
        },
      ],
      policies: {
        completion: { kind: "allCorrect" },
        grade: "highest",
        continuedPractice: { kind: "unlimited" },
        variation: "newSeeds",
      },
      disclosurePolicy: {
        ...createMasteryAssignmentDraft(courseId).disclosurePolicy,
        privateAnswerKey: "not allowed",
      },
    }),
  );
});

test("assignment request decoders reject pool cardinalities beyond the server contract", () => {
  const valid = assignmentCreateInput({
    ...createMasteryAssignmentDraft(courseId),
    title: "Bounded pool",
    entries: [{ ...item, kind: "fixed" as const }],
  });
  assert.throws(() =>
    decodeAssignmentCreateInput({
      ...valid,
      entries: Array.from({ length: 1025 }, (_value, position) => ({
        ...valid.entries[0],
        position,
      })),
    }),
  );
  assert.throws(() =>
    decodeAssignmentCreateInput({
      ...valid,
      entries: [
        {
          kind: "selectionGroup",
          candidateQuestionIds: Array.from(
            { length: 1025 },
            (_value, index) => `${index.toString(16).padStart(3, "0").toUpperCase()}-0000`,
          ),
          position: 0,
          drawCount: 1,
          pointsPerItem: "1",
          ordering: "candidateOrder",
        },
      ],
    }),
  );
});
