// Browser capability contract for server-owned Blueprint Course adoption.

import type { CurriculumAdoptionApplyIntent } from "../../generated/api/CurriculumAdoptionApplyIntent";
import type { CurriculumAdoptionCompleted } from "../../generated/api/CurriculumAdoptionCompleted";
import type { CurriculumAdoptionPreview } from "../../generated/api/CurriculumAdoptionPreview";
import type { CurriculumAdoptionPreviewRequest } from "../../generated/api/CurriculumAdoptionPreviewRequest";

/**
 * Browser capability for one complete Blueprint Course or Course Instance adoption operation.
 *
 * The server owns current authorization, preview facts, apply records, and receipts. The browser
 * submits only a closed operation request and an idempotent apply intent.
 */
export interface CurriculumAdoptionClient {
  readonly previewCurriculumAdoption: (
    request: CurriculumAdoptionPreviewRequest,
  ) => Promise<CurriculumAdoptionPreview>;
  readonly applyCurriculumAdoption: (
    intent: CurriculumAdoptionApplyIntent,
  ) => Promise<CurriculumAdoptionCompleted>;
}
