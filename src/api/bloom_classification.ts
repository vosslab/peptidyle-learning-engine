// Browser capability for correcting one exact Revision-owned Bloom pair.

import type { BloomClassificationCorrectionRequest } from "../../generated/api/BloomClassificationCorrectionRequest";
import type { QuestionBloomCorrectionReceipt } from "../../generated/api/QuestionBloomCorrectionReceipt";
import type { QuestionPoolBloomCorrectionReceipt } from "../../generated/api/QuestionPoolBloomCorrectionReceipt";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionRevisionTuple } from "../../generated/api/QuestionRevisionTuple";

export interface BloomClassificationCorrectionClient {
  readonly correctQuestionBloom: (
    questionRevisionTuple: QuestionRevisionTuple,
    request: BloomClassificationCorrectionRequest,
  ) => Promise<QuestionBloomCorrectionReceipt>;
  readonly correctQuestionPoolBloom: (
    questionPoolId: QuestionId,
    request: BloomClassificationCorrectionRequest,
  ) => Promise<QuestionPoolBloomCorrectionReceipt>;
}
