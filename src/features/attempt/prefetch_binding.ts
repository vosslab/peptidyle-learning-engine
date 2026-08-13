// prefetch_binding.ts - immutable receipt-to-prefetch binding policy.

import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { NextIssuedAttempt, PrefetchedNextQuestion } from "../../api/contracts";

type PrefetchBinding = Pick<
  PrefetchedNextQuestion,
  | "predecessor"
  | "run"
  | "assignmentPosition"
  | "questionVersion"
  | "seed"
  | "renderedQuestionSha256"
>;

/**
 * Accept a cached successor only when the committed receipt binds every
 * descriptor the prefetch route could know before submission.
 */
export function prefetchMatchesIssuedSuccessor(
  cached: PrefetchBinding,
  issued: NextIssuedAttempt,
  predecessor: QuestionAttemptId,
): boolean {
  return (
    cached.predecessor === predecessor &&
    cached.run === issued.run &&
    cached.assignmentPosition === issued.assignmentPosition &&
    cached.questionVersion === issued.questionVersion &&
    cached.seed === issued.seed &&
    cached.renderedQuestionSha256 === issued.renderedQuestionSha256
  );
}
