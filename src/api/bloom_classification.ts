// Browser capability for correcting one exact Revision-owned Bloom pair.

import type { BloomClassificationCorrectionRequest } from "../../generated/api/BloomClassificationCorrectionRequest";
import type { QuestionBloomCorrectionReceipt } from "../../generated/api/QuestionBloomCorrectionReceipt";
import type { QuestionPoolBloomCorrectionReceipt } from "../../generated/api/QuestionPoolBloomCorrectionReceipt";
import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";

export interface BloomClassificationCorrectionClient {
  readonly correctQuestionBloom: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
    request: BloomClassificationCorrectionRequest,
  ) => Promise<QuestionBloomCorrectionReceipt>;
  readonly correctQuestionPoolBloom: (
    questionPoolId: QuestionPoolId,
    request: BloomClassificationCorrectionRequest,
  ) => Promise<QuestionPoolBloomCorrectionReceipt>;
}
