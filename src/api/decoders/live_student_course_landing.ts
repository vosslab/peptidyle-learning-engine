// Strict decoding for the minimal current Student Course Landing projection.

import type {
  LiveStudentAssignmentLandingSummary,
  LiveStudentCourseInvitationSummary,
  LiveStudentCourseLandingSummary,
} from "../live_student_course_landing";
import { decodeArray, decodeRecord } from "../decoder";
import {
  decodeAssignmentReference,
  decodeAssignmentTitle,
  decodeCourseInstanceReference,
  decodeCourseTitle,
  field,
  requireOnlyFields,
} from "./shared";

function decodeCourseSummary(value: unknown, path: string): LiveStudentCourseLandingSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "title"]);
  return {
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    title: decodeCourseTitle(field(record, "title", path), `${path}.title`),
  };
}

function decodeInvitationSummary(value: unknown, path: string): LiveStudentCourseInvitationSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "title"]);
  return {
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    title: decodeCourseTitle(field(record, "title", path), `${path}.title`),
  };
}

function decodeAssignmentSummary(
  value: unknown,
  path: string,
): LiveStudentAssignmentLandingSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "title"]);
  return {
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
  };
}

/** Rejects anything beyond the current Student's minimal Course landing projection. */
export function decodeLiveStudentCourseLandings(
  value: unknown,
  path = "response",
): ReadonlyArray<LiveStudentCourseLandingSummary> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courses"]);
  return decodeArray(field(record, "courses", path), `${path}.courses`, decodeCourseSummary);
}

/** Rejects anything beyond the current Student's minimal pending-invitation projection. */
export function decodeLiveStudentCourseInvitations(
  value: unknown,
  path = "response",
): ReadonlyArray<LiveStudentCourseInvitationSummary> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["invitations"]);
  return decodeArray(
    field(record, "invitations", path),
    `${path}.invitations`,
    decodeInvitationSummary,
  );
}

/** Rejects anything beyond the current Student's minimal Assignment landing projection. */
export function decodeLiveStudentAssignmentLandings(
  value: unknown,
  path = "response",
): ReadonlyArray<LiveStudentAssignmentLandingSummary> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assignments"]);
  return decodeArray(
    field(record, "assignments", path),
    `${path}.assignments`,
    decodeAssignmentSummary,
  );
}
