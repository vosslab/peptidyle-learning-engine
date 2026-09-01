// Browser capability contract for server-owned Blueprint operations.

import type { BlueprintOperationApplyIntent } from "../../generated/api/BlueprintOperationApplyIntent";
import type { BlueprintOperationCompleted } from "../../generated/api/BlueprintOperationCompleted";
import type { BlueprintOperationPreview } from "../../generated/api/BlueprintOperationPreview";
import type { BlueprintOperationPreviewRequest } from "../../generated/api/BlueprintOperationPreviewRequest";

/**
 * Browser capability for one complete Blueprint Course or Course Instance operation.
 *
 * The server owns current authorization, preview facts, apply records, and receipts. The browser
 * submits only a closed operation request and an idempotent apply intent.
 */
export interface BlueprintOperationsClient {
  readonly previewBlueprintOperation: (
    request: BlueprintOperationPreviewRequest,
  ) => Promise<BlueprintOperationPreview>;
  readonly applyBlueprintOperation: (
    intent: BlueprintOperationApplyIntent,
  ) => Promise<BlueprintOperationCompleted>;
}
