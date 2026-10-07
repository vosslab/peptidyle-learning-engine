// Strict decoding for the Course Instance creation and Teaching Team boundary.

import type { AccountId } from "../../../generated/api/AccountId";
import type { CourseInstanceRouteSummary } from "../../../generated/api/CourseInstanceRouteSummary";
import type { CreateBlueprintFromCourseInstanceInput } from "../../../generated/api/CreateBlueprintFromCourseInstanceInput";
import { THEME_VALUES } from "../../../generated/api/Theme";
import type {
  CourseCreationInstructor,
  CourseInstanceSummary,
  CourseInstanceView,
  CourseInstanceCreationSource,
  CreateCourseInstanceInput,
  CreatedCourseInstance,
  InstallationCourseInspection,
  InstallationCoursePage,
} from "../course_instance";
import type { CourseEditNumber } from "../../../generated/api/CourseEditNumber";
import {
  DecodeError,
  decodeArray,
  decodeNonemptyString,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { blueprintCourseRevisionTuple } from "./blueprint_course";
import { isCanonicalAccountId } from "./instructor_account";
import { decodeCourseTerm } from "./course_term";
import { decodeCourseClassification } from "./course_classification";
import { decodeCourseInstanceId, decodeCourseName, field, requireOnlyFields } from "./shared";

function courseEditNumber(value: unknown, path: string): CourseEditNumber {
  const decoded = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a positive Course Edit Number");
  }
  return decoded;
}

function accountId(value: unknown, path: string): AccountId {
  const decoded = decodeString(value, path);
  if (!isCanonicalAccountId(decoded)) {
    throw new DecodeError(path, "a canonical Account ID");
  }
  return decoded;
}

function summary(value: unknown, path: string): CourseInstanceSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "shortName",
    "longName",
    "term",
    "theme",
    "classification",
    "lifecycleState",
    "courseEditNumber",
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
    courseEditNumber: courseEditNumber(
      field(record, "courseEditNumber", path),
      `${path}.courseEditNumber`,
    ),
    id: decodeCourseInstanceId(field(record, "id", path), `${path}.id`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    theme: decodeStringEnum(field(record, "theme", path), `${path}.theme`, THEME_VALUES),
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
    "id",
    "shortName",
    "longName",
    "term",
    "role",
    "classification",
    "lifecycleState",
  ]);
  return {
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    id: decodeCourseInstanceId(field(record, "id", path), `${path}.id`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    role: decodeStringEnum(field(record, "role", path), `${path}.role`, ["student", "instructor"]),
    lifecycleState: decodeStringEnum(
      field(record, "lifecycleState", path),
      `${path}.lifecycleState`,
      ["active", "inactive"] as const,
    ),
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
    "assignedInstructorAccountId",
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
    requireOnlyFields(sourceRecord, sourcePath, ["kind", "blueprintCourseRevisionTuple"]);
    source = {
      kind,
      blueprintCourseRevisionTuple: blueprintCourseRevisionTuple(
        field(sourceRecord, "blueprintCourseRevisionTuple", sourcePath),
        `${sourcePath}.blueprintCourseRevisionTuple`,
      ),
    };
  }
  const assignedInstructorAccountId = record["assignedInstructorAccountId"];
  const decoded = {
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    source,
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    ...(assignedInstructorAccountId === undefined
      ? {}
      : {
          assignedInstructorAccountId: accountId(
            assignedInstructorAccountId,
            `${path}.assignedInstructorAccountId`,
          ),
        }),
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
  requireOnlyFields(record, path, ["courseInstance", "activeInstructorCount", "blueprintOrigin"]);
  const originValue = field(record, "blueprintOrigin", path);
  let blueprintOrigin: CourseInstanceView["blueprintOrigin"] = null;
  if (originValue !== null) {
    const originPath = `${path}.blueprintOrigin`;
    const origin = decodeRecord(originValue, originPath);
    requireOnlyFields(origin, originPath, [
      "adoptedBlueprintCourseRevisionTuple",
      "currentBlueprintCourseRevisionTuple",
    ]);
    blueprintOrigin = {
      adoptedBlueprintCourseRevisionTuple: blueprintCourseRevisionTuple(
        field(origin, "adoptedBlueprintCourseRevisionTuple", originPath),
        `${originPath}.adoptedBlueprintCourseRevisionTuple`,
      ),
      currentBlueprintCourseRevisionTuple: blueprintCourseRevisionTuple(
        field(origin, "currentBlueprintCourseRevisionTuple", originPath),
        `${originPath}.currentBlueprintCourseRevisionTuple`,
      ),
    };
    // ASVS 2.2.3: a current source head cannot precede its original adoption.
    if (
      blueprintOrigin.currentBlueprintCourseRevisionTuple.blueprintCourseId !==
        blueprintOrigin.adoptedBlueprintCourseRevisionTuple.blueprintCourseId ||
      BigInt(blueprintOrigin.currentBlueprintCourseRevisionTuple.revisionNumber) <
        BigInt(blueprintOrigin.adoptedBlueprintCourseRevisionTuple.revisionNumber)
    ) {
      throw new DecodeError(originPath, "a current Revision at or after the adopted Revision");
    }
  }
  return {
    courseInstance: summary(field(record, "courseInstance", path), `${path}.courseInstance`),
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
  requireOnlyFields(record, path, ["courseInstance"]);
  return {
    courseInstance: summary(field(record, "courseInstance", path), `${path}.courseInstance`),
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
    requireOnlyFields(instructor, entryPath, ["accountId"]);
    return {
      accountId: accountId(field(instructor, "accountId", entryPath), `${entryPath}.accountId`),
    };
  });
}

function instructorDisplayName(value: unknown, path: string): string {
  const name = decodeNonemptyString(value, path);
  if (name !== name.trim() || Array.from(name).length > 200 || /\p{Cc}/u.test(name)) {
    throw new DecodeError(path, "an Instructor display name");
  }
  return name;
}

/** Strict decoder for one Sysadmin Course inspection. ASVS 1.5.2/2.2.1/8.2.3. */
export function decodeInstallationCourse(
  value: unknown,
  path = "response",
): InstallationCourseInspection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "shortName",
    "longName",
    "term",
    "lifecycleState",
    "retentionLifecycleState",
    "instructorDisplayNames",
  ]);
  const instructorDisplayNames = decodeArray(
    field(record, "instructorDisplayNames", path),
    `${path}.instructorDisplayNames`,
    instructorDisplayName,
  );
  if (instructorDisplayNames.length > 200) {
    throw new DecodeError(`${path}.instructorDisplayNames`, "at most 200 Instructor display names");
  }
  return {
    id: decodeCourseInstanceId(field(record, "id", path), `${path}.id`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    lifecycleState: decodeStringEnum(
      field(record, "lifecycleState", path),
      `${path}.lifecycleState`,
      ["active", "inactive"] as const,
    ),
    retentionLifecycleState: decodeStringEnum(
      field(record, "retentionLifecycleState", path),
      `${path}.retentionLifecycleState`,
      ["active", "archived", "deleted"] as const,
    ),
    instructorDisplayNames,
  };
}

/** Strict decoder for one bounded installation Course page. */
export function decodeInstallationCoursePage(
  value: unknown,
  path = "response",
): InstallationCoursePage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courses", "nextCursor"]);
  const courses = decodeArray(
    field(record, "courses", path),
    `${path}.courses`,
    decodeInstallationCourse,
  );
  if (courses.length > 250) {
    throw new DecodeError(`${path}.courses`, "at most 250 Courses");
  }
  const nextCursor = decodeNullable(
    field(record, "nextCursor", path),
    `${path}.nextCursor`,
    decodeNonemptyString,
  );
  if (nextCursor !== null && nextCursor.length > 4096) {
    throw new DecodeError(`${path}.nextCursor`, "a bounded installation Course cursor");
  }
  return {
    courses,
    nextCursor,
  };
}
