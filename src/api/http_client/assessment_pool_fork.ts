// Strict same-origin transport for Assessment-owned Question Pool fork commands.

import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { AssessmentQuestionPoolSelectionCountReceipt } from "../../../generated/api/AssessmentQuestionPoolSelectionCountReceipt";
import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type {
  AppendAssessmentQuestionPoolForkRevisionInput,
  AppendedAssessmentQuestionPoolForkRevision,
  AssessmentPoolForkClient,
  ImportAssessmentQuestionPoolForkInput,
  ImportedAssessmentQuestionPoolFork,
} from "../assessment_pool_fork";
import {
  decodeAssessmentQuestionPoolForkView,
  decodeAssessmentQuestionPoolSelectionCountReceipt,
  decodeImportedAssessmentQuestionPoolFork,
} from "../decoders/assessment_pool_fork";
import {
  DecodeError,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeUuid,
} from "../decoder";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseAssessmentId, parseCourseInstanceId } from "../../navigation/public_route";
import { validateCanonicalQuestionIdSyntax } from "../../../generated/api/QuestionIdSyntaxContract";

/** A stale Assessment Pool command must reload its complete Assessment workspace. */
export class AssessmentPoolForkConflictError extends ApiRequestError {
  public constructor(path: string) {
    super(412, path);
    this.name = "AssessmentPoolForkConflictError";
  }
}

function assessmentPath(course: CourseInstanceId, assessment: AssessmentId): string {
  if (parseCourseInstanceId(course) === null || parseAssessmentId(assessment) === null) {
    throw new ApiProtocolError("Assessment Pool route references must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/assessments/${encodeURIComponent(assessment)}`;
}

function forkPath(
  course: CourseInstanceId,
  assessment: AssessmentId,
  entry?: AssessmentEntryId,
): string {
  const base = `${assessmentPath(course, assessment)}/question-pool-forks`;
  return entry === undefined ? base : `${base}/${encodeURIComponent(entry)}`;
}

function quotedStrongEtag(etag: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(etag) || BigInt(etag.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(
      `API ${path} If-Match must be one quoted strong Assessment Edit Number`,
    );
  }
  return etag;
}

function requireResponseEtag(response: Response, editNumber: string, path: string): void {
  if (response.headers.get("etag") !== `"${editNumber}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its Assessment Edit Number`);
  }
}

function requireEntryReceipt(
  received: AssessmentEntryId,
  expected: AssessmentEntryId,
  path: string,
): void {
  if (received !== expected) {
    throw new ApiProtocolError(`API response ${path} must retain the requested Assessment Entry`);
  }
}

async function requestJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decode: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: string;
  } = {},
): Promise<T> {
  const headers: Record<string, string> =
    options.etag === undefined ? {} : { "if-match": quotedStrongEtag(options.etag, path) };
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new AssessmentPoolForkConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decode(await boundedResponseJson(response, path), "response");
}

function importBody(input: ImportAssessmentQuestionPoolForkInput): object {
  const sourceQuestionPoolId = validateCanonicalQuestionIdSyntax(input.sourceQuestionPoolId);
  if (sourceQuestionPoolId === null || sourceQuestionPoolId !== input.sourceQuestionPoolId) {
    throw new ApiProtocolError("Question Pool ID must be canonical");
  }
  if (!Number.isSafeInteger(input.authoredPosition) || input.authoredPosition < 0) {
    throw new ApiProtocolError("Assessment Pool authored position must be nonnegative");
  }
  if (!Number.isSafeInteger(input.selectionCount) || input.selectionCount < 1) {
    throw new ApiProtocolError("Assessment Pool selection count must be positive");
  }
  return input;
}

function appendBody(input: AppendAssessmentQuestionPoolForkRevisionInput): object {
  if (!input.interchangeabilityAttested || input.members.length === 0) {
    throw new ApiProtocolError("Assessment Pool fork revision requires attested nonempty members");
  }
  if (
    !/^[1-9][0-9]*$/u.test(input.expectedQuestionPoolEditNumber) ||
    BigInt(input.expectedQuestionPoolEditNumber) > 9_223_372_036_854_775_807n
  ) {
    throw new ApiProtocolError(
      "Assessment Pool expected Question Pool Edit Number must be a positive integer",
    );
  }
  return input;
}

function decodeAppendReceipt(
  value: unknown,
  path = "response",
): AppendedAssessmentQuestionPoolForkRevision {
  const record = decodeRecord(value, path);
  const allowed = [
    "assessmentEntryId",
    "revisionNumber",
    "blueprintEditNumber",
    "assessmentEditNumber",
  ];
  if (
    Object.keys(record).length !== allowed.length ||
    Object.keys(record).some((key) => !allowed.includes(key))
  ) {
    throw new DecodeError(path, "one closed Assessment Pool append receipt");
  }
  const assessmentEditNumber = decodeString(
    record.assessmentEditNumber,
    `${path}.assessmentEditNumber`,
  );
  if (!/^[1-9][0-9]*$/u.test(assessmentEditNumber)) {
    throw new DecodeError(`${path}.assessmentEditNumber`, "a positive Assessment Edit Number");
  }
  const blueprintEditNumber = decodeString(
    record.blueprintEditNumber,
    `${path}.blueprintEditNumber`,
  );
  if (!/^[1-9][0-9]*$/u.test(blueprintEditNumber)) {
    throw new DecodeError(`${path}.blueprintEditNumber`, "a positive Blueprint Edit Number");
  }
  return {
    assessmentEntryId: decodeUuid(record.assessmentEntryId, `${path}.assessmentEntryId`),
    revisionNumber: decodePositiveInteger(record.revisionNumber, `${path}.revisionNumber`),
    blueprintEditNumber,
    assessmentEditNumber,
  };
}

/** Creates the narrow Pool-fork client without weakening the shared request transport. */
export function createAssessmentPoolForkClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): AssessmentPoolForkClient {
  return {
    getAssessmentQuestionPoolFork: async (
      course,
      assessment,
      entry,
    ): Promise<AssessmentQuestionPoolForkView> => {
      const path = forkPath(course, assessment, entry);
      const fork = await requestJson(
        fetchImplementation,
        basePath,
        path,
        decodeAssessmentQuestionPoolForkView,
      );
      requireEntryReceipt(fork.assessmentEntryId, entry, path);
      return fork;
    },
    importAssessmentQuestionPoolFork: async (
      course,
      assessment,
      input,
      etag,
    ): Promise<ImportedAssessmentQuestionPoolFork> => {
      const path = forkPath(course, assessment);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
        headers: { "if-match": quotedStrongEtag(etag, path) },
        body: importBody(input),
      });
      requireNoStore(response, path);
      if (response.status === 412) throw new AssessmentPoolForkConflictError(path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 201) {
        throw new ApiProtocolError(`API response ${path} must use status 201`);
      }
      const receipt = decodeImportedAssessmentQuestionPoolFork(
        await boundedResponseJson(response, path),
        "response",
      );
      requireResponseEtag(response, receipt.assessmentEditNumber, path);
      return receipt;
    },
    appendAssessmentQuestionPoolForkRevision: async (
      course,
      assessment,
      entry,
      input,
      etag,
    ): Promise<AppendedAssessmentQuestionPoolForkRevision> => {
      const path = forkPath(course, assessment, entry);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        headers: { "if-match": quotedStrongEtag(etag, path) },
        body: appendBody(input),
      });
      requireNoStore(response, path);
      if (response.status === 412) throw new AssessmentPoolForkConflictError(path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const receipt = decodeAppendReceipt(await boundedResponseJson(response, path), "response");
      requireEntryReceipt(receipt.assessmentEntryId, entry, path);
      requireResponseEtag(response, receipt.assessmentEditNumber, path);
      return receipt;
    },
    updateAssessmentQuestionPoolSelectionCount: async (
      course,
      assessment,
      entry,
      selectionCount,
      etag,
    ): Promise<AssessmentQuestionPoolSelectionCountReceipt> => {
      if (!Number.isSafeInteger(selectionCount) || selectionCount < 1) {
        throw new ApiProtocolError("Assessment Pool selection count must be positive");
      }
      const path = `${forkPath(course, assessment, entry)}/selection-count`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        headers: { "if-match": quotedStrongEtag(etag, path) },
        body: { selectionCount },
      });
      requireNoStore(response, path);
      if (response.status === 412) throw new AssessmentPoolForkConflictError(path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const receipt = decodeAssessmentQuestionPoolSelectionCountReceipt(
        await boundedResponseJson(response, path),
        "response",
      );
      requireEntryReceipt(receipt.assessmentEntryId, entry, path);
      if (receipt.selectionCount !== selectionCount) {
        throw new ApiProtocolError(
          `API response ${path} must retain the requested selection count`,
        );
      }
      requireResponseEtag(response, receipt.assessmentEditNumber, path);
      return receipt;
    },
  };
}
