// student_assessment_attempt_navigation_model.ts - answer-free Question navigation labels.

export type StudentAssessmentAttemptQuestionState = "unanswered" | "saved" | "closed";

/**
 * Selects the largest useful contiguous Question window that fits between the
 * first and last positions. The returned indexes describe numbered buttons;
 * omitted ranges are rendered separately by the component.
 */
export function studentAssessmentAttemptVisiblePositionIndexes(
  positionCount: number,
  currentIndex: number,
  numberSlots: number,
): ReadonlyArray<number> {
  if (positionCount <= 0) return [];
  const slots = Math.max(5, Math.floor(numberSlots));
  if (positionCount <= slots) return Array.from({ length: positionCount }, (_, index) => index);

  const middleCount = Math.max(3, slots - 2);
  const safeCurrentIndex = Math.min(
    positionCount - 1,
    Math.max(1, currentIndex < 0 ? 1 : currentIndex),
  );
  const lastMiddleStart = positionCount - middleCount - 1;
  const middleStart = Math.max(
    1,
    Math.min(safeCurrentIndex - Math.floor(middleCount / 2), lastMiddleStart),
  );
  return [
    0,
    ...Array.from({ length: middleCount }, (_, offset) => middleStart + offset),
    positionCount - 1,
  ];
}

export function studentAssessmentAttemptQuestionStateLabel(
  state: StudentAssessmentAttemptQuestionState,
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
