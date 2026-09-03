// Pure public-reference navigation for the Instructor Gradebook surfaces.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { CourseMembershipReference } from "../../generated/api/CourseMembershipReference";
import type { InstructorGradingOperationReference } from "../../generated/api/InstructorGradingOperationReference";
import type { AssignmentAttemptReference } from "../../generated/api/AssignmentAttemptReference";
import type {
  CalculatedGradebookQuery,
  InspectedStudentWorkReturnContext,
} from "../api/decoders/calculated_gradebook";
import { decodeInstructorGradingOperationReference } from "../api/decoders/grading_operations";
import {
  parseAssignmentReference,
  parseCourseMembershipReference,
  parseCourseInstanceReference,
  parseAssignmentAttemptReference,
  type AssignmentRouteReference,
  type CourseMembershipRouteReference,
  type CourseInstanceRouteReference,
  type AssignmentAttemptRouteReference,
} from "../navigation/public_route";

/** The one allowed Gradebook URL filter. */
export type GradebookRouteFilter =
  | { readonly kind: "assignment"; readonly assignment: AssignmentRouteReference }
  | { readonly kind: "student"; readonly membership: CourseMembershipRouteReference }
  | { readonly kind: "operation"; readonly operation: InstructorGradingOperationReference };

export interface GradebookRouteSearch {
  readonly kind: "valid";
  readonly filter: GradebookRouteFilter | undefined;
}

export type GradebookSearchInvalidReason =
  "malformedSearch" | "unknownKey" | "duplicateKey" | "multipleFilters" | "invalidReference";

export interface InvalidGradebookRouteSearch {
  readonly kind: "invalid";
  readonly reason: GradebookSearchInvalidReason;
  readonly key?: string;
}

export type GradebookRouteSearchResult = GradebookRouteSearch | InvalidGradebookRouteSearch;

/** The one optional detail query preserves an exact grading-operation return context. */
export type InspectedStudentWorkRouteSearch =
  | { readonly kind: "valid"; readonly operation: InstructorGradingOperationReference | undefined }
  | {
      readonly kind: "invalid";
      readonly reason: GradebookSearchInvalidReason;
      readonly key?: string;
    };

const GRADEBOOK_FILTER_KEYS = ["assignmentRef", "membershipRef", "operationRef"] as const;
type GradebookFilterKey = (typeof GRADEBOOK_FILTER_KEYS)[number];

function invalidSearch(
  reason: GradebookSearchInvalidReason,
  key?: string,
): InvalidGradebookRouteSearch {
  return key === undefined ? { kind: "invalid", reason } : { kind: "invalid", reason, key };
}

function searchParamsFrom(value: string | URLSearchParams): URLSearchParams | null {
  if (value instanceof URLSearchParams) return new URLSearchParams(value);
  if (value !== "" && !value.startsWith("?")) return null;
  const query = value.startsWith("?") ? value.slice(1) : value;
  if (query.includes("#")) return null;
  return new URLSearchParams(query);
}

function isGradebookFilterKey(value: string): value is GradebookFilterKey {
  return (GRADEBOOK_FILTER_KEYS as ReadonlyArray<string>).includes(value);
}

/** Parses only the closed Gradebook filter query; malformed input never becomes the all view. */
export function parseGradebookRouteSearch(
  value: string | URLSearchParams,
): GradebookRouteSearchResult {
  const params = searchParamsFrom(value);
  if (params === null) return invalidSearch("malformedSearch");

  const entries = Array.from(params.entries());
  for (const [key] of entries) {
    if (!isGradebookFilterKey(key)) return invalidSearch("unknownKey", key);
    if (params.getAll(key).length !== 1) return invalidSearch("duplicateKey", key);
  }
  if (entries.length === 0) return { kind: "valid", filter: undefined };
  if (entries.length !== 1) return invalidSearch("multipleFilters");

  const [key, valueForKey] = entries[0] as [GradebookFilterKey, string];
  if (key === "assignmentRef") {
    const assignment = parseAssignmentReference(valueForKey);
    return assignment === null
      ? invalidSearch("invalidReference", key)
      : { kind: "valid", filter: { kind: "assignment", assignment } };
  }
  if (key === "membershipRef") {
    const membership = parseCourseMembershipReference(valueForKey);
    return membership === null
      ? invalidSearch("invalidReference", key)
      : { kind: "valid", filter: { kind: "student", membership } };
  }
  try {
    const operation = decodeInstructorGradingOperationReference(valueForKey);
    return { kind: "valid", filter: { kind: "operation", operation } };
  } catch (_error: unknown) {
    return invalidSearch("invalidReference", key);
  }
}

/** Maps the closed route filter to the transport query without introducing page state. */
export function gradebookQueryForFilter(
  filter: GradebookRouteFilter | undefined,
): CalculatedGradebookQuery {
  if (filter === undefined) return {};
  if (filter.kind === "assignment")
    return { filter: { kind: "assignment", assignment: filter.assignment } };
  if (filter.kind === "student")
    return { filter: { kind: "student", membership: filter.membership } };
  return { filter: { kind: "operation", operation: filter.operation } };
}

/**
 * Parses the closed detail query. It is deliberately narrower than Gradebook search:
 * an inspected Assignment Attempt can retain one operation origin, or no extra context at all.
 */
export function parseInspectedStudentWorkRouteSearch(
  value: string | URLSearchParams,
): InspectedStudentWorkRouteSearch {
  const params = searchParamsFrom(value);
  if (params === null) return invalidSearch("malformedSearch");
  const entries = Array.from(params.entries());
  if (entries.length === 0) return { kind: "valid", operation: undefined };
  const [key, operationReference] = entries[0] as [string, string];
  if (key !== "operationRef") return invalidSearch("unknownKey", key);
  if (params.getAll(key).length !== 1) return invalidSearch("duplicateKey", key);
  if (entries.length !== 1) return invalidSearch("multipleFilters");
  try {
    return {
      kind: "valid",
      operation: decodeInstructorGradingOperationReference(operationReference),
    };
  } catch (_error: unknown) {
    return invalidSearch("invalidReference", key);
  }
}

function checkedCourse(value: CourseInstanceReference): CourseInstanceRouteReference {
  const result = parseCourseInstanceReference(value);
  if (result === null) throw new Error("invalid public course reference");
  return result;
}

function checkedAssignment(value: AssignmentReference): AssignmentRouteReference {
  const result = parseAssignmentReference(value);
  if (result === null) throw new Error("invalid public assignment reference");
  return result;
}

function checkedMembership(value: CourseMembershipReference): CourseMembershipRouteReference {
  const result = parseCourseMembershipReference(value);
  if (result === null) throw new Error("invalid public course membership reference");
  return result;
}

function checkedAssignmentAttempt(
  value: AssignmentAttemptReference,
): AssignmentAttemptRouteReference {
  const result = parseAssignmentAttemptReference(value);
  if (result === null) throw new Error("invalid public Assignment Attempt reference");
  return result;
}

/** Returns the stable DOM target for one Student-assignment Gradebook cell. */
export function gradebookCellFocusId(
  membership: CourseMembershipReference,
  assignment: AssignmentReference,
): string {
  const membershipReference = checkedMembership(membership);
  const assignmentReference = checkedAssignment(assignment);
  return `gradebook-cell-${membershipReference}-${assignmentReference}`;
}

function gradebookPath(
  course: CourseInstanceReference,
  filter: GradebookRouteFilter | undefined,
): string {
  const checkedCourseInstanceReference = checkedCourse(course);
  const query = new URLSearchParams();
  if (filter?.kind === "assignment")
    query.set("assignmentRef", checkedAssignment(filter.assignment));
  if (filter?.kind === "student") query.set("membershipRef", checkedMembership(filter.membership));
  if (filter?.kind === "operation") {
    query.set("operationRef", decodeInstructorGradingOperationReference(filter.operation));
  }
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `/instructor/courses/${checkedCourseInstanceReference}/gradebook${suffix}`;
}

/** Returns to Gradebook with the Student filter and the invoking cell ready for focus. */
export function gradebookReturnUrl(
  course: CourseInstanceReference,
  membership: CourseMembershipReference,
  assignment: AssignmentReference,
): string {
  const path = gradebookPath(course, {
    kind: "student",
    membership: checkedMembership(membership),
  });
  return `${path}#${gradebookCellFocusId(membership, assignment)}`;
}

/** Opens Gradebook in the exact operation context that initiated Student selection. */
export function operationGradebookUrl(
  course: CourseInstanceReference,
  operation: InstructorGradingOperationReference,
): string {
  return gradebookPath(course, {
    kind: "operation",
    operation: decodeInstructorGradingOperationReference(operation),
  });
}

/** Returns the stable DOM target for one operation control. */
export function gradingOperationControlFocusId(
  operation: InstructorGradingOperationReference,
): string {
  const checkedOperation = decodeInstructorGradingOperationReference(operation);
  return `grading-operation-control-${checkedOperation}`;
}

function gradingOperationsPath(
  course: CourseInstanceReference,
  assignment: AssignmentReference,
): string {
  const checkedCourseInstanceReference = checkedCourse(course);
  const checkedAssignmentReference = checkedAssignment(assignment);
  return `/instructor/courses/${checkedCourseInstanceReference}/assignments/${checkedAssignmentReference}/grading-operations`;
}

/** Returns to assignment Grading operations and restores the initiating operation control. */
export function gradingOperationReturnUrl(
  course: CourseInstanceReference,
  assignment: AssignmentReference,
  operation: InstructorGradingOperationReference,
): string {
  return `${gradingOperationsPath(course, assignment)}#${gradingOperationControlFocusId(operation)}`;
}

/**
 * Builds the only supported return destinations from the verified detail response.
 * The response contract, rather than browser route state, owns the authority choice.
 */
export function inspectedStudentWorkReturnUrl(context: InspectedStudentWorkReturnContext): string {
  if (context.kind === "gradebook") {
    return gradebookReturnUrl(context.course, context.focus.membership, context.focus.assignment);
  }
  return gradingOperationReturnUrl(context.course, context.assignment, context.focus.operation);
}

/** Builds the verified/audited Student-work detail URL, optionally retaining operation context. */
export function inspectedStudentWorkUrl(
  course: CourseInstanceReference,
  membership: CourseMembershipReference,
  assignment: AssignmentReference,
  assignmentAttempt: AssignmentAttemptReference,
  operation?: InstructorGradingOperationReference,
): string {
  const checkedCourseInstanceReference = checkedCourse(course);
  const checkedMembershipReference = checkedMembership(membership);
  const checkedAssignmentReference = checkedAssignment(assignment);
  const checkedAssignmentAttemptReference = checkedAssignmentAttempt(assignmentAttempt);
  const path = `/instructor/courses/${checkedCourseInstanceReference}/gradebook/students/${checkedMembershipReference}/assignments/${checkedAssignmentReference}/assignment-attempts/${checkedAssignmentAttemptReference}`;
  if (operation === undefined) return path;
  const operationReference = decodeInstructorGradingOperationReference(operation);
  return `${path}?${new URLSearchParams({ operationRef: operationReference }).toString()}`;
}
