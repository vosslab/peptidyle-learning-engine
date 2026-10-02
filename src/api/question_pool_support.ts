// Optional PLE-managed Hint, Question Feedback, and Worked Solution for one Question Pool.

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { QuestionPoolMetadataEditNumber } from "../../generated/api/QuestionPoolMetadataEditNumber";

export interface QuestionPoolPleManagedSupport {
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
  readonly hint: string | null;
  readonly generalFeedback: string | null;
  readonly workedSolution: string | null;
}

export interface QuestionPoolSupportSave {
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
  readonly hint: string | null;
  readonly generalFeedback: string | null;
  readonly workedSolution: string | null;
}

export interface QuestionPoolSupportClient {
  readonly readQuestionPoolSupport: (
    questionPoolId: QuestionPoolId,
  ) => Promise<QuestionPoolPleManagedSupport>;
  readonly saveQuestionPoolSupport: (
    input: QuestionPoolSupportSave,
  ) => Promise<QuestionPoolPleManagedSupport>;
}
