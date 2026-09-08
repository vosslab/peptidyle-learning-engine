// Browser contract for the protected Course Invitation mailer export.

import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";

/** Same-origin download boundary for pending Course Invitation mailer input. */
export interface LiveInvitationExportClient {
  /**
   * Downloads the protected attachment without interpreting its recipient data in the browser.
   */
  readonly downloadLiveInvitationExport: (course: CourseInstanceReference) => Promise<Blob>;
}
