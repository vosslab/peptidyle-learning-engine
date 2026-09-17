// Browser capability for correcting one exact Revision-owned Bloom pair.

import type { BloomClassificationCorrectionRequest } from "../../generated/api/BloomClassificationCorrectionRequest";
import type { QuestionBloomCorrectionReceipt } from "../../generated/api/QuestionBloomCorrectionReceipt";
import type { QuestionPoolBloomCorrectionReceipt } from "../../generated/api/QuestionPoolBloomCorrectionReceipt";
import type { QuestionPoolRevisionReference } from "../../generated/api/QuestionPoolRevisionReference";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";

export interface BloomClassificationCorrectionClient {
  readonly correctQuestionBloom: (
    reference: QuestionRevisionReference,
    request: BloomClassificationCorrectionRequest,
  ) => Promise<QuestionBloomCorrectionReceipt>;
  readonly correctQuestionPoolBloom: (
    reference: QuestionPoolRevisionReference,
    request: BloomClassificationCorrectionRequest,
  ) => Promise<QuestionPoolBloomCorrectionReceipt>;
}
