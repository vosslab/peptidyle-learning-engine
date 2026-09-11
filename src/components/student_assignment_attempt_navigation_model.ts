// student_assignment_attempt_navigation_model.ts - answer-free Question navigation labels.

export type StudentAssignmentAttemptQuestionState = "unanswered" | "saved" | "closed";

export function studentAssignmentAttemptQuestionStateLabel(
  state: StudentAssignmentAttemptQuestionState,
): string {
  switch (state) {
    case "unanswered":
      return "Not answered";
    case "saved":
      return "Saved";
    case "closed":
      return "Closed";
  }
}
