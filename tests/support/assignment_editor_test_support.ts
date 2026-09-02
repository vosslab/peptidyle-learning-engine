import type { AssignmentEditorState } from "../../src/pages/assignment_editor_model.ts";

export function createMasteryAssignmentEditorState(courseId: string): AssignmentEditorState {
  return {
    id: "",
    courseId,
    title: "",
    entries: [],
    policies: {
      assignmentCompletionRule: { kind: "allCorrect" },
      assignmentAttemptGradeRule: "highest",
      assignmentAttemptContinuationRule: { kind: "unlimited" },
      questionPoolReuseRule: "reuseSelection",
      questionVariationRule: "newVariation",
      assignmentAttemptResumeRule: "resumable",
      assignmentQuestionDisplayRule: "allQuestions",
      assignmentNavigationRule: "freeNavigation",
      assignmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      question_feedback: "after_submit",
      question_answer: "after_submit",
      question_answer_explanation: "after_submit",
      class_statistics: "never",
    },
    revision: "",
  };
}
