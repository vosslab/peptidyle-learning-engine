import type { AssignmentEditorDraft } from "../../src/pages/assignment_editor_model.ts";

export function createMasteryAssignmentDraft(courseId: string): AssignmentEditorDraft {
  return {
    id: "",
    courseId,
    title: "",
    entries: [],
    policies: {
      completion: { kind: "allCorrect" },
      grade: "highest",
      continuedPractice: { kind: "unlimited" },
      variation: "newSeeds",
    },
    disclosurePolicy: {
      score: "afterSubmit",
      perItemCorrectness: "afterSubmit",
      feedbackText: "afterSubmit",
      solution: "afterSubmit",
      classStatistics: "never",
    },
    revision: "",
  };
}
