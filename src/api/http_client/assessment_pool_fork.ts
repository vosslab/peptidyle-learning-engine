// Strict same-origin transport for Assessment-owned Question Pool fork commands.

import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { AssessmentQuestionPoolSelectionCountReceipt } from "../../../generated/api/AssessmentQuestionPoolSelectionCountReceipt";
import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type {
  AppendAssessmentQuestionPoolForkMembersInput,
  AppendedAssessmentQuestionPoolForkMembers,
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
import {
  assertResponseMatchesPositiveNumber,
  ifMatchHeaderForPositiveNumber,
} from "./conditional_request";
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

function assessmentPath(courseInstanceId: CourseInstanceId, assessmentId: AssessmentId): string {
  if (
    parseCourseInstanceId(courseInstanceId) === null ||
    parseAssessmentId(assessmentId) === null
  ) {
    throw new ApiProtocolError("Assessment Pool route IDs must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(courseInstanceId)}/assessments/${encodeURIComponent(assessmentId)}`;
}

function forkPath(
  courseInstanceId: CourseInstanceId,
  assessmentId: AssessmentId,
  entry?: AssessmentEntryId,
): string {
  const base = `${assessmentPath(courseInstanceId, assessmentId)}/question-pool-forks`;
  return entry === undefined ? base : `${base}/${encodeURIComponent(entry)}`;
}

function requireMatchingAssessmentEditNumber(
  response: Response,
  editNumber: string,
  path: string,
): void {
  assertResponseMatchesPositiveNumber(response, editNumber, path, "Assessment Edit Number");
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
    readonly expectedAssessmentEditNumber?: string;
  } = {},
): Promise<T> {
  const headers: Record<string, string> =
    options.expectedAssessmentEditNumber === undefined
      ? {}
      : {
          "if-match": ifMatchHeaderForPositiveNumber(
            options.expectedAssessmentEditNumber,
            path,
            "Assessment Edit Number",
          ),
        };
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

function appendBody(input: AppendAssessmentQuestionPoolForkMembersInput): object {
  if (!input.interchangeabilityAttested || input.members.length === 0) {
    throw new ApiProtocolError(
      "Assessment Pool fork membership requires attested nonempty members",
    );
  }
  if (
    !Number.isSafeInteger(input.expectedQuestionPoolEditNumber) ||
    input.expectedQuestionPoolEditNumber < 1
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
): AppendedAssessmentQuestionPoolForkMembers {
  const record = decodeRecord(value, path);
  const allowed = ["assessmentEntryId", "questionPoolEditNumber", "assessmentEditNumber"];
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
  return {
    assessmentEntryId: decodeUuid(record.assessmentEntryId, `${path}.assessmentEntryId`),
    questionPoolEditNumber: decodePositiveInteger(
      record.questionPoolEditNumber,
      `${path}.questionPoolEditNumber`,
    ),
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
      courseInstanceId,
      assessmentId,
      entry,
    ): Promise<AssessmentQuestionPoolForkView> => {
      const path = forkPath(courseInstanceId, assessmentId, entry);
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
      courseInstanceId,
      assessmentId,
      input,
      expectedAssessmentEditNumber,
    ): Promise<ImportedAssessmentQuestionPoolFork> => {
      const path = forkPath(courseInstanceId, assessmentId);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
        headers: {
          "if-match": ifMatchHeaderForPositiveNumber(
            expectedAssessmentEditNumber,
            path,
            "Assessment Edit Number",
          ),
        },
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
      requireMatchingAssessmentEditNumber(response, receipt.assessmentEditNumber, path);
      return receipt;
    },
    appendAssessmentQuestionPoolForkMembers: async (
      courseInstanceId,
      assessmentId,
      entry,
      input,
      expectedAssessmentEditNumber,
    ): Promise<AppendedAssessmentQuestionPoolForkMembers> => {
      const path = forkPath(courseInstanceId, assessmentId, entry);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        headers: {
          "if-match": ifMatchHeaderForPositiveNumber(
            expectedAssessmentEditNumber,
            path,
            "Assessment Edit Number",
          ),
        },
        body: appendBody(input),
      });
      requireNoStore(response, path);
      if (response.status === 412) throw new AssessmentPoolForkConflictError(path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const receipt = decodeAppendReceipt(await boundedResponseJson(response, path), "response");
      requireEntryReceipt(receipt.assessmentEntryId, entry, path);
      requireMatchingAssessmentEditNumber(response, receipt.assessmentEditNumber, path);
      return receipt;
    },
    updateAssessmentQuestionPoolSelectionCount: async (
      courseInstanceId,
      assessmentId,
      entry,
      selectionCount,
      expectedAssessmentEditNumber,
    ): Promise<AssessmentQuestionPoolSelectionCountReceipt> => {
      if (!Number.isSafeInteger(selectionCount) || selectionCount < 1) {
        throw new ApiProtocolError("Assessment Pool selection count must be positive");
      }
      const path = `${forkPath(courseInstanceId, assessmentId, entry)}/selection-count`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        headers: {
          "if-match": ifMatchHeaderForPositiveNumber(
            expectedAssessmentEditNumber,
            path,
            "Assessment Edit Number",
          ),
        },
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
      requireMatchingAssessmentEditNumber(response, receipt.assessmentEditNumber, path);
      return receipt;
    },
  };
}
