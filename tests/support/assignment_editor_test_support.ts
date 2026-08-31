import type { AssignmentEditorDraft } from "../../src/pages/assignment_editor_model.ts";

export function createMasteryAssignmentDraft(courseId: string): AssignmentEditorDraft {
  return {
    id: "",
    courseId,
    title: "",
    entries: [],
    policies: {
      assignmentCompletionRule: { kind: "allCorrect" },
      assignmentAttemptGradeRule: "highest",
      assignmentAttemptContinuationRule: { kind: "unlimited" },
      questionVariationRule: "reuseQuestionsWithNewSeeds",
      assignmentAttemptResumeRule: "resumable",
      assignmentQuestionDisplayRule: "allQuestions",
      assignmentNavigationRule: "freeNavigation",
      assignmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      feedback_text: "after_submit",
      solution: "after_submit",
      class_statistics: "never",
    },
    revision: "",
  };
}
