// Strict same-origin transport for the Sysadmin Instructor Account lifecycle.

import type { AccountId } from "../../../generated/api/AccountId";
import type { ApiClient } from "../client";
import type { InstructorAccountBrowse, InstructorAccountClient } from "../instructor_account";
import {
  decodeCreatedInstructorAccount,
  decodeCreateInstructorAccountInput,
  decodeDeactivateInstructorAccountInput,
  decodeInstructorAccount,
  decodeInstructorAccountList,
  decodeInstructorSetupEmailResponse,
  isCanonicalAccountId,
} from "../decoders/instructor_account";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function accountPath(accountId: AccountId): string {
  if (!isCanonicalAccountId(accountId)) {
    throw new ApiProtocolError("Instructor Account ID must be canonical");
  }
  return `/api/instructor-accounts/${encodeURIComponent(accountId)}`;
}

async function instructorAccountJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST";
    readonly body?: unknown;
    readonly status?: 200 | 201;
  } = {},
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    body: options.body,
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.status !== undefined && response.status !== options.status) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.status}`);
  }
  return decoder(await boundedResponseJson(response, path), "response");
}

const instructorAccountPageSizes: ReadonlyArray<number> = [50, 100, 250];

/** ASVS 2.2.1: allow-list the list body before posting it. The server repeats the check. */
function instructorAccountBrowseBody(browse: InstructorAccountBrowse): {
  readonly query: string;
  readonly state: InstructorAccountBrowse["state"];
  readonly pageSize: InstructorAccountBrowse["pageSize"];
  readonly afterAccountId: InstructorAccountBrowse["afterAccountId"];
} {
  const trimmed = browse.query.trim();
  if ([...trimmed].length > 320 || /[\p{Cc}]/u.test(trimmed)) {
    throw new ApiProtocolError("Instructor Account search is invalid");
  }
  if (!instructorAccountPageSizes.includes(browse.pageSize)) {
    throw new ApiProtocolError("Instructor Account list is invalid");
  }
  if (browse.afterAccountId !== null && !isCanonicalAccountId(browse.afterAccountId)) {
    throw new ApiProtocolError("Instructor Account list is invalid");
  }
  return {
    query: trimmed,
    state: browse.state,
    pageSize: browse.pageSize,
    afterAccountId: browse.afterAccountId,
  };
}

/** Composes this capability independently from ordinary Course-account surfaces. */
export function createInstructorAccountClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof InstructorAccountClient> {
  return {
    listInstructorAccounts: () =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        "/api/instructor-accounts",
        decodeInstructorAccountList,
      ),
    findInstructorAccounts: (browse) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        "/api/instructor-accounts/find",
        decodeInstructorAccountList,
        { method: "POST", body: instructorAccountBrowseBody(browse), status: 200 },
      ),
    createInstructorAccount: (input) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        "/api/instructor-accounts",
        decodeCreatedInstructorAccount,
        { method: "POST", body: decodeCreateInstructorAccountInput(input), status: 201 },
      ),
    sendInstructorSetupEmail: (accountId) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        `${accountPath(accountId)}/send-setup-email`,
        decodeInstructorSetupEmailResponse,
        { method: "POST", status: 200 },
      ),
    deactivateInstructorAccount: (accountId, input) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        `${accountPath(accountId)}/deactivate`,
        decodeInstructorAccount,
        { method: "POST", body: decodeDeactivateInstructorAccountInput(input), status: 200 },
      ),
    reactivateInstructorAccount: (accountId) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        `${accountPath(accountId)}/reactivate`,
        decodeInstructorAccount,
        { method: "POST", status: 200 },
      ),
  };
}
