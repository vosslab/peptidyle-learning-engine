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
      score: "after_submit",
      per_item_correctness: "after_submit",
      feedback_text: "after_submit",
      solution: "after_submit",
      class_statistics: "never",
    },
    revision: "",
  };
}
