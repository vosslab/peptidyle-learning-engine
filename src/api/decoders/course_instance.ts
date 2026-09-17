// Strict decoding for the Course Instance creation and Teaching Team boundary.

import type { AccountReference } from "../../../generated/api/AccountReference";
import type { CourseInstanceRouteSummary } from "../../../generated/api/CourseInstanceRouteSummary";
import type { CreateBlueprintFromCourseInstanceInput } from "../../../generated/api/CreateBlueprintFromCourseInstanceInput";
import { COURSE_THEME_VALUES } from "../../../generated/api/CourseTheme";
import type {
  CourseCreationInstructor,
  CourseInstanceSummary,
  CourseInstanceView,
  CourseInstanceCreationSource,
  CreateCourseInstanceInput,
  CreatedCourseInstance,
} from "../course_instance";
import {
  DecodeError,
  decodeArray,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeBlueprintCourseReference, decodeBlueprintRevision } from "./blueprint_course";
import { isCanonicalAccountReference } from "./instructor_account";
import { decodeCourseTerm } from "./course_term";
import { decodeCourseClassification } from "./course_classification";
import { decodeUuid } from "../decoder";
import {
  decodeCourseInstanceReference,
  decodeCourseName,
  field,
  requireOnlyFields,
} from "./shared";

function accountReference(value: unknown, path: string): AccountReference {
  const decoded = decodeString(value, path);
  if (!isCanonicalAccountReference(decoded)) {
    throw new DecodeError(path, "a canonical Account public reference");
  }
  return decoded;
}

function summary(value: unknown, path: string): CourseInstanceSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "shortName",
    "longName",
    "term",
    "theme",
    "classification",
    "lifecycleState",
    "metadataEtag",
  ]);
  return {
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    lifecycleState: decodeStringEnum(
      field(record, "lifecycleState", path),
      `${path}.lifecycleState`,
      ["active", "inactive"] as const,
    ),
    metadataEtag: decodeUuid(field(record, "metadataEtag", path), `${path}.metadataEtag`),
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    theme: decodeStringEnum(field(record, "theme", path), `${path}.theme`, COURSE_THEME_VALUES),
  };
}

/** Strictly decodes the closed member-safe Course Instance route projection. */
export function decodeCourseInstanceRouteSummary(
  value: unknown,
  path = "response",
): CourseInstanceRouteSummary {
  // ASVS 1.5.2 and 2.2.1: accept only the generated public response shape.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "shortName",
    "longName",
    "term",
    "role",
    "classification",
  ]);
  return {
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    role: decodeStringEnum(field(record, "role", path), `${path}.role`, ["student", "instructor"]),
  };
}

/** Strictly validates the creation request before it crosses the transport boundary. */
export function decodeCreateCourseInstanceInput(
  value: unknown,
  path = "request",
): CreateCourseInstanceInput {
  const record = decodeRecord(value, path);
  // ASVS 1.5.2 and 2.2.1: exclude flat or mixed source fields before transport.
  requireOnlyFields(record, path, [
    "source",
    "shortName",
    "longName",
    "term",
    "assignedInstructor",
    "classification",
  ]);
  const sourcePath = `${path}.source`;
  const sourceRecord = decodeRecord(field(record, "source", path), sourcePath);
  const kind = decodeStringEnum(field(sourceRecord, "kind", sourcePath), `${sourcePath}.kind`, [
    "empty",
    "adopted",
  ] as const);
  let source: CourseInstanceCreationSource;
  if (kind === "empty") {
    requireOnlyFields(sourceRecord, sourcePath, ["kind"]);
    source = { kind };
  } else {
    requireOnlyFields(sourceRecord, sourcePath, ["kind", "blueprintCourse", "blueprintRevision"]);
    source = {
      kind,
      blueprintCourse: decodeBlueprintCourseReference(
        field(sourceRecord, "blueprintCourse", sourcePath),
        `${sourcePath}.blueprintCourse`,
      ),
      blueprintRevision: decodeBlueprintRevision(
        field(sourceRecord, "blueprintRevision", sourcePath),
        `${sourcePath}.blueprintRevision`,
      ),
    };
  }
  const assignedInstructor = record["assignedInstructor"];
  const decoded = {
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    source,
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    ...(assignedInstructor === undefined
      ? {}
      : { assignedInstructor: accountReference(assignedInstructor, `${path}.assignedInstructor`) }),
  } satisfies CreateCourseInstanceInput;
  return decoded;
}

/** Strictly validates the only Instructor-owned fields for Course-derived Blueprint creation. */
export function decodeCreateBlueprintFromCourseInstanceInput(
  value: unknown,
  path = "request",
): CreateBlueprintFromCourseInstanceInput {
  const record = decodeRecord(value, path);
  // ASVS 1.5.2 and 2.2.1: reusable content and Course delivery state never cross this boundary.
  requireOnlyFields(record, path, ["classification", "shortName", "longName"]);
  return {
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
  };
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
  requireOnlyFields(record, path, ["course", "activeInstructorCount", "blueprintOrigin"]);
  const originValue = field(record, "blueprintOrigin", path);
  let blueprintOrigin: CourseInstanceView["blueprintOrigin"] = null;
  if (originValue !== null) {
    const originPath = `${path}.blueprintOrigin`;
    const origin = decodeRecord(originValue, originPath);
    requireOnlyFields(origin, originPath, ["reference", "adoptedRevision", "currentRevision"]);
    blueprintOrigin = {
      reference: decodeBlueprintCourseReference(
        field(origin, "reference", originPath),
        `${originPath}.reference`,
      ),
      adoptedRevision: decodeBlueprintRevision(
        field(origin, "adoptedRevision", originPath),
        `${originPath}.adoptedRevision`,
      ),
      currentRevision: decodeBlueprintRevision(
        field(origin, "currentRevision", originPath),
        `${originPath}.currentRevision`,
      ),
    };
    // ASVS 2.2.3: a current source head cannot precede its original adoption.
    if (BigInt(blueprintOrigin.currentRevision) < BigInt(blueprintOrigin.adoptedRevision)) {
      throw new DecodeError(originPath, "a current Revision at or after the adopted Revision");
    }
  }
  return {
    course: summary(field(record, "course", path), `${path}.course`),
    activeInstructorCount: decodePositiveInteger(
      field(record, "activeInstructorCount", path),
      `${path}.activeInstructorCount`,
    ),
    blueprintOrigin,
  };
}

export function decodeCreatedCourseInstance(
  value: unknown,
  path = "response",
): CreatedCourseInstance {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course"]);
  return {
    course: summary(field(record, "course", path), `${path}.course`),
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
      reference: accountReference(
        field(instructor, "reference", entryPath),
        `${entryPath}.reference`,
      ),
    };
  });
}
