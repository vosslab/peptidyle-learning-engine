// Browser contract for the protected Course Invitation mailer export.

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";

/** Same-origin download boundary for pending Course Invitation mailer input. */
export interface LiveInvitationExportClient {
  /**
   * Downloads the protected attachment without interpreting its recipient data in the browser.
   */
  readonly downloadLiveInvitationExport: (courseInstanceId: CourseInstanceId) => Promise<Blob>;
}
