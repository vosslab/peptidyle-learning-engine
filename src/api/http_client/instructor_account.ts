// Strict same-origin transport for the Sysadmin Instructor Account lifecycle.

import type { AccountId } from "../../../generated/api/AccountId";
import type { ApiClient } from "../client";
import type { InstructorAccountClient } from "../instructor_account";
import {
  decodeCompleteInstructorIdentityVettingInput,
  decodeCreateInstructorAccountInput,
  decodeDeactivateInstructorAccountInput,
  decodeInstructorAccount,
  decodeInstructorAccountList,
  decodeInstructorIdentityVettingReceipt,
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
    completeInstructorIdentityVetting: (input) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        "/api/instructor-identity-vetting-decisions",
        decodeInstructorIdentityVettingReceipt,
        {
          method: "POST",
          body: decodeCompleteInstructorIdentityVettingInput(input),
          status: 201,
        },
      ),
    createInstructorAccount: (input) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        "/api/instructor-accounts",
        decodeInstructorAccount,
        { method: "POST", body: decodeCreateInstructorAccountInput(input), status: 201 },
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
