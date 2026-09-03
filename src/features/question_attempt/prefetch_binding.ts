// prefetch_binding.ts - immutable receipt-to-prefetch binding policy.

import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { NextIssuedAttempt, PrefetchedNextQuestion } from "../../api/contracts";

type PrefetchBinding = Pick<
  PrefetchedNextQuestion,
  "predecessor" | "issuedQuestion" | "question_seed" | "renderedQuestionSha256"
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
    cached.issuedQuestion.id === issued.issuedQuestion.id &&
    cached.question_seed === issued.question_seed &&
    cached.renderedQuestionSha256 === issued.renderedQuestionSha256
  );
}
