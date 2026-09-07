// Strict decoding for the M8 Course Instance creation and teaching-team boundary.

import type { AccountReference } from "../../../generated/api/AccountReference";
import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { CourseTerm } from "../../../generated/api/CourseTerm";
import type {
  CourseCreationInstructor,
  CourseInstanceSummary,
  CourseInstanceView,
  CreateCourseInstanceInput,
  CreatedCourseInstance,
} from "../course_instance";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import { decodeBlueprintCourseReference, decodeBlueprintRevision } from "./blueprint_course";
import { decodeCourseTerm } from "./course_term";
import {
  decodeCourseInstanceReference,
  decodeCourseTitle,
  field,
  requireOnlyFields,
} from "./shared";

function accountReference(value: unknown, path: string): AccountReference {
  const decoded = decodeString(value, path);
  if (!/^U-[1-9][0-9]{0,9}$/u.test(decoded) || Number(decoded.slice(2)) > 2_147_483_647) {
    throw new DecodeError(path, "a canonical Account public reference");
  }
  return decoded;
}

function title(value: unknown, path: string): string {
  const decoded = decodeCourseTitle(value, path);
  if (decoded !== decoded.trim() || Array.from(decoded).length > 200) {
    throw new DecodeError(path, "a trimmed Course Title within its bound");
  }
  return decoded;
}

function summary(value: unknown, path: string): CourseInstanceSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "title", "term"]);
  return {
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    title: title(field(record, "title", path), `${path}.title`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
  };
}

/** Strictly validates the small M8 creation request before it crosses the transport boundary. */
export function decodeCreateCourseInstanceInput(
  value: unknown,
  path = "request",
): CreateCourseInstanceInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "blueprintCourse",
    "blueprintRevision",
    "title",
    "term",
    "assignedInstructor",
  ]);
  const assignedInstructor = record["assignedInstructor"];
  const decoded = {
    blueprintCourse: decodeBlueprintCourseReference(
      field(record, "blueprintCourse", path),
      `${path}.blueprintCourse`,
    ) as BlueprintCourseReference,
    blueprintRevision: decodeBlueprintRevision(
      field(record, "blueprintRevision", path),
      `${path}.blueprintRevision`,
    ),
    title: title(field(record, "title", path), `${path}.title`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`) as CourseTerm,
    ...(assignedInstructor === undefined
      ? {}
      : { assignedInstructor: accountReference(assignedInstructor, `${path}.assignedInstructor`) }),
  } satisfies CreateCourseInstanceInput;
  return decoded;
}

export function decodeCourseInstanceList(
  value: unknown,
  path = "response",
): ReadonlyArray<CourseInstanceSummary> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  if (field(record, "nextCursor", path) !== null) {
    throw new DecodeError(`${path}.nextCursor`, "null for the bounded live Course Instance list");
  }
  return decodeArray(field(record, "items", path), `${path}.items`, summary);
}

export function decodeCourseInstanceView(value: unknown, path = "response"): CourseInstanceView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course", "isAssignedInstructor", "activeInstructorCount"]);
  return {
    course: summary(field(record, "course", path), `${path}.course`),
    isAssignedInstructor: decodeBoolean(
      field(record, "isAssignedInstructor", path),
      `${path}.isAssignedInstructor`,
    ),
    activeInstructorCount: decodePositiveInteger(
      field(record, "activeInstructorCount", path),
      `${path}.activeInstructorCount`,
    ),
  };
}

export function decodeCreatedCourseInstance(
  value: unknown,
  path = "response",
): CreatedCourseInstance {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course", "creatorIsAssignedInstructor"]);
  return {
    course: summary(field(record, "course", path), `${path}.course`),
    creatorIsAssignedInstructor: decodeBoolean(
      field(record, "creatorIsAssignedInstructor", path),
      `${path}.creatorIsAssignedInstructor`,
    ),
  };
}

export function decodeCourseCreationInstructors(
  value: unknown,
  path = "response",
): ReadonlyArray<CourseCreationInstructor> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  if (field(record, "nextCursor", path) !== null) {
    throw new DecodeError(`${path}.nextCursor`, "null for the bounded Instructor selection list");
  }
  return decodeArray(field(record, "items", path), `${path}.items`, (entry, entryPath) => {
    const instructor = decodeRecord(entry, entryPath);
    requireOnlyFields(instructor, entryPath, ["reference"]);
    return {
      reference: accountReference(field(instructor, "reference", entryPath), `${entryPath}.reference`),
    };
  });
}
