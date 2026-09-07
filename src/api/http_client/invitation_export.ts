// Strict same-origin transport for the M18 protected Course Invitation export.

import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type { LiveInvitationExportClient } from "../invitation_export";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { requireNoStore } from "./response";

const INVITATION_EXPORT_FILENAME = "ple-invitations.json";
const MAX_INVITATION_EXPORT_BYTES = 1_048_576;

function invitationExportPath(course: CourseInstanceReference): string {
  if (!/^C-[1-9][0-9]{0,9}$/u.test(course) || Number(course.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/invitation-export`;
}

function requireInvitationExportAttachment(response: Response, path: string): void {
  // ASVS 8.3.1: accept only the fixed download form so a recipient list never
  // becomes a browser-readable document through an altered response contract.
  if (response.headers.get("content-type") !== "application/json") {
    throw new ApiProtocolError(`API response ${path} must use application/json`);
  }
  if (
    response.headers.get("content-disposition") !==
    `attachment; filename=${INVITATION_EXPORT_FILENAME}`
  ) {
    throw new ApiProtocolError(`API response ${path} must use the protected attachment filename`);
  }
}

/** Composes M18 separately from generic Course and CSV-export clients. */
export function createLiveInvitationExportClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveInvitationExportClient> {
  return {
    downloadLiveInvitationExport: async (course): Promise<Blob> => {
      const path = invitationExportPath(course);
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      requireInvitationExportAttachment(response, path);
      const exportBlob = await response.blob();
      if (exportBlob.type !== "application/json" || exportBlob.size > MAX_INVITATION_EXPORT_BYTES) {
        throw new ApiProtocolError(
          `API response ${path} does not contain a bounded invitation export`,
        );
      }
      return exportBlob;
    },
  };
}
