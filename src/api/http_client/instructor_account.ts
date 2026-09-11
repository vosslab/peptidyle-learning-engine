// Strict same-origin transport for the Sysadmin Instructor Account lifecycle.

import type { AccountReference } from "../../../generated/api/AccountReference";
import type { ApiClient } from "../client";
import type { InstructorAccountClient } from "../instructor_account";
import {
  decodeCreateInstructorAccountInput,
  decodeDeactivateInstructorAccountInput,
  decodeInstructorAccount,
  decodeInstructorAccountList,
} from "../decoders/instructor_account";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function accountPath(reference: AccountReference): string {
  if (!/^U-[1-9][0-9]{0,9}$/u.test(reference) || Number(reference.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Instructor Account reference must be canonical");
  }
  return `/api/instructor-accounts/${encodeURIComponent(reference)}`;
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
    createInstructorAccount: (input) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        "/api/instructor-accounts",
        decodeInstructorAccount,
        { method: "POST", body: decodeCreateInstructorAccountInput(input), status: 201 },
      ),
    deactivateInstructorAccount: (reference, input) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        `${accountPath(reference)}/deactivate`,
        decodeInstructorAccount,
        { method: "POST", body: decodeDeactivateInstructorAccountInput(input), status: 200 },
      ),
    reactivateInstructorAccount: (reference) =>
      instructorAccountJson(
        fetchImplementation,
        basePath,
        `${accountPath(reference)}/reactivate`,
        decodeInstructorAccount,
        { method: "POST", status: 200 },
      ),
  };
}
