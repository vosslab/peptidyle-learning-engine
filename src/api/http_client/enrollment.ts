// Passwordless account, passkey, roster, and manual-export transport.

import type { CourseRosterClient, ManualGradeExport, RosterImportPreview } from "../enrollment";
import {
  decodeAccountAuthenticated,
  decodeAccountEmailChanged,
  decodeAccountCoursePage,
  decodeClaimedCourseInvitation,
  decodeCourseEnrollmentPolicyResult,
  decodeCourseInvitationAccepted,
  decodeLocalTeachingMemberAccepted,
  decodeCourseRosterPage,
  decodeEmailAuthenticationAccepted,
  decodePasskeyAuthenticated,
  decodePasskeyList,
  decodePasskeySummary,
  decodeRosterImportCommitResult,
  decodeRosterImportPreview,
  decodeRosterRevisionResult,
  decodeSelectedCourseSession,
  decodeWebauthnStart,
} from "../enrollment";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestJson, requestPath, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_ROSTER_CSV_BYTES = 1_048_576;
const MAX_GRADE_EXPORT_BYTES = 1_048_576;

function positiveRevisionHeader(revision: number, name: string): string {
  if (!Number.isSafeInteger(revision) || revision <= 0) {
    throw new ApiProtocolError(`${name} must be a positive safe integer`);
  }
  return `"${revision}"`;
}

function idempotencyHeader(value: string): string {
  if (value.length === 0 || value.length > 128 || /[^\x21-\x7e]/u.test(value)) {
    throw new ApiProtocolError("idempotency key must contain 1 through 128 visible ASCII bytes");
  }
  return value;
}

function verifyNumericEtag(response: Response, expected: number, path: string): void {
  if (response.headers.get("etag") !== `"${expected}"`) {
    throw new ApiProtocolError(`API response ${path} ETag does not match its revision`);
  }
}

async function noContentRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
): Promise<void> {
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "DELETE",
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204) {
    throw new ApiProtocolError(`API response ${path} must use status 204`);
  }
}

async function rosterMutation<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method: "POST" | "PUT" | "DELETE";
    readonly body?: unknown;
    readonly headers?: Readonly<Record<string, string>>;
  },
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {
    accept: "application/json",
    ...options.headers,
  };
  const body = options.body === undefined ? undefined : JSON.stringify(options.body);
  if (body !== undefined) headers["content-type"] = "application/json";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method,
    headers,
    body,
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return { body: decoder(await boundedResponseJson(response, path)), response };
}

function credentialJson(
  credential: Credential | null,
): RegistrationResponseJSON | AuthenticationResponseJSON {
  if (!(credential instanceof PublicKeyCredential)) {
    throw new ApiProtocolError("The authenticator did not return a public-key credential");
  }
  return credential.toJSON();
}

function isRegistrationResponse(
  value: RegistrationResponseJSON | AuthenticationResponseJSON,
): value is RegistrationResponseJSON {
  return "attestationObject" in value.response;
}

function isAuthenticationResponse(
  value: RegistrationResponseJSON | AuthenticationResponseJSON,
): value is AuthenticationResponseJSON {
  return "signature" in value.response;
}

/**
 * The platform parser owns the WebAuthn JSON-to-binary conversion. The cast is
 * isolated at this browser-standard boundary; the server remains the semantic
 * validator for RP ID, origin, challenge, credential, and signature.
 */
function registrationOptions(
  value: Readonly<Record<string, unknown>>,
): PublicKeyCredentialCreationOptions {
  return PublicKeyCredential.parseCreationOptionsFromJSON(
    value as unknown as PublicKeyCredentialCreationOptionsJSON,
  );
}

/** See registrationOptions; this is the matching discoverable-login boundary. */
function authenticationOptions(
  value: Readonly<Record<string, unknown>>,
): PublicKeyCredentialRequestOptions {
  return PublicKeyCredential.parseRequestOptionsFromJSON(
    value as unknown as PublicKeyCredentialRequestOptionsJSON,
  );
}

export async function registerPasskeyWithBrowser(
  client: CourseRosterClient,
  label: string,
): Promise<Awaited<ReturnType<CourseRosterClient["completePasskeyRegistration"]>>> {
  if (!("credentials" in navigator) || !("PublicKeyCredential" in globalThis)) {
    throw new ApiProtocolError("This browser does not support passkeys");
  }
  const started = await client.startPasskeyRegistration();
  const credential = await navigator.credentials.create({
    publicKey: registrationOptions(started.options),
  });
  const json = credentialJson(credential);
  if (!isRegistrationResponse(json)) {
    throw new ApiProtocolError("The authenticator returned an unexpected registration response");
  }
  return client.completePasskeyRegistration(started.ceremonyId, label, json);
}

export async function authenticatePasskeyWithBrowser(client: CourseRosterClient): Promise<void> {
  if (!("credentials" in navigator) || !("PublicKeyCredential" in globalThis)) {
    throw new ApiProtocolError("This browser does not support passkeys");
  }
  const started = await client.startPasskeyAuthentication();
  const credential = await navigator.credentials.get({
    publicKey: authenticationOptions(started.options),
  });
  const json = credentialJson(credential);
  if (!isAuthenticationResponse(json)) {
    throw new ApiProtocolError("The authenticator returned an unexpected sign-in response");
  }
  await client.completePasskeyAuthentication(started.ceremonyId, json);
}

export function createEnrollmentClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): CourseRosterClient {
  return {
    startEmailAuthentication: (email) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/passwordless/email/start",
        decodeEmailAuthenticationAccepted,
        { method: "POST", body: { email } },
      ),
    completeEmailAuthentication: (token, displayName) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/passwordless/email/complete",
        decodeAccountAuthenticated,
        { method: "POST", body: { token, displayName } },
      ),
    startAccountEmailChange: (email) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/account/email/start",
        decodeEmailAuthenticationAccepted,
        { method: "POST", body: { email } },
      ),
    completeAccountEmailChange: (token) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/account/email/complete",
        decodeAccountEmailChanged,
        { method: "POST", body: { token } },
      ),
    listAccountCourses: () =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/account/courses",
        decodeAccountCoursePage,
      ),
    selectAccountCourse: (courseId) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/account/course-session",
        decodeSelectedCourseSession,
        { method: "POST", body: { courseId } },
      ),
    redeemCourseInvitation: (invitationToken) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/course-invitations/redeem",
        decodeClaimedCourseInvitation,
        { method: "POST", body: { invitationToken } },
      ),
    startPasskeyRegistration: () =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/passkeys/registration/start",
        decodeWebauthnStart,
        { method: "POST" },
      ),
    completePasskeyRegistration: (ceremonyId, label, credential) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/passkeys/registration/complete",
        decodePasskeySummary,
        { method: "POST", body: { ceremonyId, label, credential } },
      ),
    startPasskeyAuthentication: () =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/passkeys/authentication/start",
        decodeWebauthnStart,
        { method: "POST" },
      ),
    completePasskeyAuthentication: (ceremonyId, credential) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/auth/passkeys/authentication/complete",
        decodePasskeyAuthenticated,
        { method: "POST", body: { ceremonyId, credential } },
      ),
    listPasskeys: () =>
      requestJson(fetchImplementation, basePath, "/api/auth/passkeys", decodePasskeyList),
    revokePasskey: (passkeyId) =>
      noContentRequest(fetchImplementation, basePath, `/api/auth/passkeys/${encodedId(passkeyId)}`),
    listCourseRoster: (courseId, cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/roster${
          cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`
        }`,
        decodeCourseRosterPage,
      ),
    addLocalTeachingMember: async (
      courseId,
      learnerAlias,
    ): ReturnType<CourseRosterClient["addLocalTeachingMember"]> => {
      const path = `/api/courses/${encodedId(courseId)}/local-teaching-members`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeLocalTeachingMemberAccepted,
        { method: "POST", body: { learnerAlias } },
      );
      verifyNumericEtag(result.response, result.body.rosterRevision, path);
      return result.body;
    },
    inviteCourseMember: async (
      courseId,
      email,
      rosterId,
      idempotencyKey,
    ): ReturnType<CourseRosterClient["inviteCourseMember"]> => {
      const path = `/api/courses/${encodedId(courseId)}/invitations`;
      return (
        await rosterMutation(fetchImplementation, basePath, path, decodeCourseInvitationAccepted, {
          method: "POST",
          headers: { "idempotency-key": idempotencyHeader(idempotencyKey) },
          body: { email, rosterId },
        })
      ).body;
    },
    revokeCourseInvitation: async (
      courseId,
      invitationId,
      rosterRevision,
    ): ReturnType<CourseRosterClient["revokeCourseInvitation"]> => {
      const path = `/api/courses/${encodedId(courseId)}/invitations/${encodedId(invitationId)}`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeRosterRevisionResult,
        {
          method: "DELETE",
          headers: { "if-match": positiveRevisionHeader(rosterRevision, "roster revision") },
        },
      );
      verifyNumericEtag(result.response, result.body.rosterRevision, path);
      return result.body;
    },
    revokeCourseMember: async (
      courseId,
      memberId,
      rosterRevision,
    ): ReturnType<CourseRosterClient["revokeCourseMember"]> => {
      const path = `/api/courses/${encodedId(courseId)}/members/${encodedId(memberId)}`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeRosterRevisionResult,
        {
          method: "DELETE",
          headers: { "if-match": positiveRevisionHeader(rosterRevision, "roster revision") },
        },
      );
      verifyNumericEtag(result.response, result.body.rosterRevision, path);
      return result.body;
    },
    replaceCourseEnrollmentPolicy: async (
      courseId,
      policy,
      rosterRevision,
    ): ReturnType<CourseRosterClient["replaceCourseEnrollmentPolicy"]> => {
      const path = `/api/courses/${encodedId(courseId)}/enrollment-policy`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeCourseEnrollmentPolicyResult,
        {
          method: "PUT",
          headers: { "if-match": positiveRevisionHeader(rosterRevision, "roster revision") },
          body: policy,
        },
      );
      verifyNumericEtag(result.response, result.body.rosterRevision, path);
      return result.body;
    },
    previewRosterImport: async (
      courseId,
      csv,
      rosterRevision,
      idempotencyKey,
    ): ReturnType<CourseRosterClient["previewRosterImport"]> => {
      if (csv.size <= 0) throw new ApiProtocolError("Roster CSV is empty");
      if (csv.size > MAX_ROSTER_CSV_BYTES) throw new ApiProtocolError("Roster CSV exceeds 1 MiB");
      const path = `/api/courses/${encodedId(courseId)}/roster-imports/preview`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: {
          accept: "application/json",
          "content-type": "text/csv; charset=utf-8",
          "idempotency-key": idempotencyHeader(idempotencyKey),
          "if-match": positiveRevisionHeader(rosterRevision, "roster revision"),
        },
        body: csv,
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const preview = decodeRosterImportPreview(await boundedResponseJson(response, path));
      verifyNumericEtag(response, preview.importRevision, path);
      return preview;
    },
    commitRosterImport: async (
      courseId,
      preview,
      rowNumbers,
      idempotencyKey,
    ): ReturnType<CourseRosterClient["commitRosterImport"]> => {
      const path = `/api/courses/${encodedId(courseId)}/roster-imports/${encodedId(preview.importId)}/commit`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeRosterImportCommitResult,
        {
          method: "POST",
          headers: {
            "idempotency-key": idempotencyHeader(idempotencyKey),
            "if-match": positiveRevisionHeader(preview.importRevision, "roster import revision"),
          },
          body: { rowNumbers },
        },
      );
      if (result.body.importId !== preview.importId) {
        throw new ApiProtocolError("Roster import commit does not match its preview");
      }
      verifyNumericEtag(result.response, result.body.importRevision, path);
      return result.body;
    },
    createManualGradeExport: async (courseId, assignmentId): Promise<ManualGradeExport> => {
      const path = `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/grade-export.csv`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: { accept: "text/csv" },
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const contentType = response.headers.get("content-type")?.split(";", 1)[0]?.trim();
      if (contentType !== "text/csv") {
        throw new ApiProtocolError(`API response ${path} must be text/csv`);
      }
      const exportId = response.headers.get("x-ple-export-id");
      if (exportId === null || !/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/iu.test(exportId)) {
        throw new ApiProtocolError(`API response ${path} must include one export ID`);
      }
      const disposition = response.headers.get("content-disposition");
      const filename = disposition?.match(/^attachment; filename=([A-Za-z0-9._-]+)$/u)?.[1];
      if (filename === undefined) {
        throw new ApiProtocolError(`API response ${path} must include a safe filename`);
      }
      const csv = await response.blob();
      if (csv.size > MAX_GRADE_EXPORT_BYTES) {
        throw new ApiProtocolError(`API response ${path} exceeds the grade-export limit`);
      }
      return { assignmentId, exportId, filename, csv };
    },
  };
}

export function readyRosterRows(preview: RosterImportPreview): ReadonlyArray<number> {
  return preview.rows.filter((row) => row.status === "readyToInvite").map((row) => row.rowNumber);
}

export function newIdempotencyKey(): string {
  if (!("randomUUID" in crypto)) {
    throw new ApiProtocolError("This browser cannot create an idempotency key");
  }
  return crypto.randomUUID();
}
