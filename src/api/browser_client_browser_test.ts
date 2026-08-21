//! Browser transport for the isolated browser-test artifact.
//!
//! The build selects this module only for `dist_browser_test/`. Test setup may
//! select HTTP explicitly to exercise intercepted same-origin requests.

import type { ApiClient } from "./client";
import { createHttpApiClient } from "./http_client";
import { createMockApiClient } from "./mock/client";

declare global {
  interface Window {
    __PLE_USE_MOCK_API__?: boolean;
    __PLE_MOCK_TEACHING_OPERATIONS_INSTRUCTOR__?: boolean;
    __PLE_MOCK_TEACHING_MODIFIER_CONFLICT_ONCE__?: boolean;
    __PLE_MOCK_TEACHING_RETENTION_CONFLICT_ONCE__?: boolean;
    __PLE_MOCK_TEACHING_GROUP_DELETE_CONFLICT_ONCE__?: boolean;
    __PLE_MOCK_TEACHING_RETENTION_ARCHIVE_FORBIDDEN_ONCE__?: boolean;
    __PLE_MOCK_TEACHING_RETENTION_DELETE_UNAVAILABLE_ONCE__?: boolean;
    __PLE_MOCK_ACCOUNT_PENDING_INVITATION__?: boolean;
  }
}

/** Creates the deterministic test client, or HTTP for an explicit interception test. */
export function createBrowserApiClient(): ApiClient {
  if (window.__PLE_USE_MOCK_API__ === false) {
    return createHttpApiClient();
  }
  return createMockApiClient({
    teachingOperationsAuthoring: window.__PLE_MOCK_TEACHING_OPERATIONS_INSTRUCTOR__ === true,
    assignmentAuthoring: window.__PLE_MOCK_TEACHING_OPERATIONS_INSTRUCTOR__ === true,
    teachingAccountPendingInvitation: window.__PLE_MOCK_ACCOUNT_PENDING_INVITATION__ === true,
    teachingModifierConflictOnce: window.__PLE_MOCK_TEACHING_MODIFIER_CONFLICT_ONCE__ === true,
    teachingRetentionConflictOnce: window.__PLE_MOCK_TEACHING_RETENTION_CONFLICT_ONCE__ === true,
    teachingGroupDeleteConflictOnce:
      window.__PLE_MOCK_TEACHING_GROUP_DELETE_CONFLICT_ONCE__ === true,
    teachingRetentionArchiveForbiddenOnce:
      window.__PLE_MOCK_TEACHING_RETENTION_ARCHIVE_FORBIDDEN_ONCE__ === true,
    teachingRetentionDeleteUnavailableOnce:
      window.__PLE_MOCK_TEACHING_RETENTION_DELETE_UNAVAILABLE_ONCE__ === true,
  });
}
