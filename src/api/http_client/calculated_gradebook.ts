// Same-origin transport for the calculated Gradebook and audited Student work.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseMembershipReference } from "../../../generated/api/CourseMembershipReference";
import type { GradingOperationReference } from "../../../generated/api/GradingOperationReference";
import type { RunReference } from "../../../generated/api/RunReference";
import type { ApiClient } from "../client";
import {
  decodeCalculatedGradebookResult,
  decodeInspectedStudentWorkDetail,
  type CalculatedGradebookQuery,
  type CalculatedGradebookResult,
  type InspectedStudentWorkDetail,
} from "../decoders/calculated_gradebook";
import {
  decodeGradebookSelectionResult,
  decodeSubmittedRunChoicesPage,
  type GradebookSelectionFilter,
  type GradebookSelectionQuery,
  type GradebookSelectionResult,
  type SubmittedRunChoicesPage,
  type SubmittedRunChoicesQuery,
} from "../decoders/gradebook_selection";
import { decodeGradingOperationReference } from "../decoders/grading_operations";
import { decodeCursor, decodeIdentifier } from "../decoders/shared";
import {
  parseAssignmentReference,
  parseCourseMembershipReference,
  parseRunReference,
} from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;

function canonicalReference(
  value: string,
  label: string,
  parse: (candidate: string) => string | null,
): string {
  const decoded = parse(value);
  if (decoded === null) throw new ApiProtocolError(`${label} must be a canonical public reference`);
  return decoded;
}

function gradebookPath(courseId: CourseId, request: CalculatedGradebookQuery): string {
  const course = decodeIdentifier(courseId, "course");
  const query = new URLSearchParams();
  if (request.cursor !== undefined) query.set("cursor", decodeCursor(request.cursor, "cursor"));
  if (request.pageSize !== undefined) {
    if (
      !Number.isSafeInteger(request.pageSize) ||
      request.pageSize < 1 ||
      request.pageSize > MAX_PAGE_SIZE
    ) {
      throw new ApiProtocolError("Gradebook page size must be an integer from 1 through 100");
    }
    query.set("pageSize", String(request.pageSize));
  }
  const filter = request.filter ?? { kind: "all" };
  if (filter.kind === "assignment") {
    query.set(
      "assignmentRef",
      canonicalReference(filter.assignment, "assignment", parseAssignmentReference),
    );
  } else if (filter.kind === "student") {
    query.set(
      "membershipRef",
      canonicalReference(filter.membership, "membership", parseCourseMembershipReference),
    );
  } else if (filter.kind === "operation") {
    query.set("operationRef", decodeGradingOperationReference(filter.operation));
  } else if (filter.kind !== "all") {
    throw new ApiProtocolError("Gradebook filter must use one known public reference");
  }
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `/api/courses/${encodedId(course)}/gradebook${suffix}`;
}

function pageQuery(cursor: string | undefined, pageSize: number | undefined): URLSearchParams {
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("cursor", decodeCursor(cursor, "cursor"));
  if (pageSize !== undefined) {
    if (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE) {
      throw new ApiProtocolError("Gradebook page size must be an integer from 1 through 100");
    }
    query.set("pageSize", String(pageSize));
  }
  return query;
}

function addSelectionFilter(query: URLSearchParams, filter: GradebookSelectionFilter): void {
  if (filter.kind === "assignment") {
    query.set(
      "assignmentRef",
      canonicalReference(filter.assignment, "assignment", parseAssignmentReference),
    );
    return;
  }
  if (filter.kind === "operation") {
    query.set("operationRef", decodeGradingOperationReference(filter.operation));
    return;
  }
  throw new ApiProtocolError("Gradebook selection needs an assignment or operation reference");
}

function gradebookSelectionPath(courseId: CourseId, request: GradebookSelectionQuery): string {
  const course = decodeIdentifier(courseId, "course");
  const query = pageQuery(request.cursor, request.pageSize);
  addSelectionFilter(query, request.filter);
  return `/api/courses/${encodedId(course)}/gradebook/selection?${query.toString()}`;
}

function submittedRunChoicesPath(
  courseId: CourseId,
  membership: CourseMembershipReference,
  assignment: AssignmentReference,
  request: SubmittedRunChoicesQuery,
): string {
  const course = decodeIdentifier(courseId, "course");
  const checkedMembership = canonicalReference(
    membership,
    "membership",
    parseCourseMembershipReference,
  );
  const checkedAssignment = canonicalReference(assignment, "assignment", parseAssignmentReference);
  const query = pageQuery(request.cursor, request.pageSize);
  if (request.operationRef !== undefined) {
    query.set("operationRef", decodeGradingOperationReference(request.operationRef));
  }
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `/api/courses/${encodedId(course)}/gradebook/students/${encodedId(checkedMembership)}/assignments/${encodedId(checkedAssignment)}/runs${suffix}`;
}

function inspectionPath(
  courseId: CourseId,
  membership: CourseMembershipReference,
  assignment: AssignmentReference,
  run: RunReference,
  operationRef: GradingOperationReference | undefined,
): string {
  const course = decodeIdentifier(courseId, "course");
  const checkedMembership = canonicalReference(
    membership,
    "membership",
    parseCourseMembershipReference,
  );
  const checkedAssignment = canonicalReference(assignment, "assignment", parseAssignmentReference);
  const checkedRun = canonicalReference(run, "run", parseRunReference);
  const query =
    operationRef === undefined
      ? ""
      : `?${new URLSearchParams({
          operationRef: decodeGradingOperationReference(operationRef),
        }).toString()}`;
  return `/api/courses/${encodedId(course)}/gradebook/students/${encodedId(checkedMembership)}/assignments/${encodedId(checkedAssignment)}/runs/${encodedId(checkedRun)}${query}`;
}

async function noStoreJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, decoderPath?: string) => T,
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 200) {
    throw new ApiProtocolError(`API response ${path} must use status 200`);
  }
  return decoder(await boundedResponseJson(response, path), "response");
}

function verifySelectionIdentity(
  result: GradebookSelectionResult,
  filter: GradebookSelectionFilter,
): GradebookSelectionResult {
  if (filter.kind !== "assignment") return result;
  const matches =
    result.kind === "singleStudent"
      ? result.assignment === filter.assignment
      : result.rows.every((row) => row.assignment === filter.assignment);
  if (!matches) {
    throw new ApiProtocolError("Gradebook selection does not match its requested assignment");
  }
  return result;
}

function verifyInspectionIdentity(
  detail: InspectedStudentWorkDetail,
  membership: CourseMembershipReference,
  assignment: AssignmentReference,
  run: RunReference,
  operationRef: GradingOperationReference | undefined,
): InspectedStudentWorkDetail {
  if (detail.membership !== membership || detail.assignment !== assignment || detail.run !== run) {
    throw new ApiProtocolError("Inspected Student work does not match its requested route");
  }
  const context = detail.returnContext;
  if (operationRef === undefined && context.kind !== "gradebook") {
    throw new ApiProtocolError("Inspected Student work returned an unexpected operation context");
  }
  if (
    operationRef !== undefined &&
    (context.kind !== "gradingOperation" || context.operation !== operationRef)
  ) {
    throw new ApiProtocolError("Inspected Student work does not match its requested operation");
  }
  return detail;
}

/** Creates the browser capability without coupling transport to the Solid page state. */
export function createCalculatedGradebookClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "getCalculatedGradebook"
  | "getGradebookSelection"
  | "getSubmittedRunChoices"
  | "getInspectedStudentWork"
> {
  return {
    getCalculatedGradebook: (
      courseId,
      query: CalculatedGradebookQuery = {},
    ): Promise<CalculatedGradebookResult> =>
      noStoreJson(
        fetchImplementation,
        basePath,
        gradebookPath(courseId, query),
        decodeCalculatedGradebookResult,
      ),
    getGradebookSelection: async (
      courseId,
      query: GradebookSelectionQuery,
    ): Promise<GradebookSelectionResult> => {
      const path = gradebookSelectionPath(courseId, query);
      const result = await noStoreJson(
        fetchImplementation,
        basePath,
        path,
        decodeGradebookSelectionResult,
      );
      return verifySelectionIdentity(result, query.filter);
    },
    getSubmittedRunChoices: (
      courseId,
      membership,
      assignment,
      query: SubmittedRunChoicesQuery = {},
    ): Promise<SubmittedRunChoicesPage> => {
      const path = submittedRunChoicesPath(courseId, membership, assignment, query);
      return noStoreJson(fetchImplementation, basePath, path, decodeSubmittedRunChoicesPage);
    },
    getInspectedStudentWork: async (
      courseId,
      membership,
      assignment,
      run,
      operationRef,
    ): Promise<InspectedStudentWorkDetail> => {
      const path = inspectionPath(courseId, membership, assignment, run, operationRef);
      const detail = await noStoreJson(
        fetchImplementation,
        basePath,
        path,
        decodeInspectedStudentWorkDetail,
      );
      return verifyInspectionIdentity(detail, membership, assignment, run, operationRef);
    },
  };
}
