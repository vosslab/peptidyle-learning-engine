import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeSyntheticPreviewAccommodationAdjustmentRequest,
  decodeInstructorCourseInvitationCreateRequest,
  decodeCourseInvitationTargetSearchPage,
  decodeCourseInvitationTargetSearchRequest,
  decodeCourseStudentMembershipsPage,
  decodeInstructorMembershipRemovalRequest,
  decodeTeachingOperationRevisionResponse,
  decodeTeachingPreviewView,
} from "../src/api/decoders.ts";

function membership(reference, display) {
  return { reference, display, role: "student", status: "active" };
}

function baseSource() {
  return { kind: "base", label: "Course policy" };
}

test("allowed preview carries its Assignment Policy Source while denied preview is sealed", () => {
  const source = baseSource();
  const preview = {
    active_student_course_membership: "allowed",
    timeZone: "America/Chicago",
    start: { kind: "mayStart", late: "onTime" },
    availableAt: { value: null, source },
    dueAt: { value: "2026-08-20T09:30:00.000", source },
    closesAt: { value: "2026-08-20T10:30:00.000", source },
    assignmentAttemptTimeLimitSeconds: { value: 3600, source },
    attemptLimit: { value: 2, source },
    lateWorkRule: { value: "accept", source },
    assignmentDeadlineRule: { value: "autoSubmit", source },
  };
  assert.deepEqual(decodeTeachingPreviewView(preview), preview);
  assert.throws(
    () =>
      decodeTeachingPreviewView({
        active_student_course_membership: "denied",
        reason: "activeStudentCourseMembershipRequired",
        dueAt: preview.dueAt,
      }),
    DecodeError,
  );
});

test("policy mutation decoders require every explicit adjustment state", () => {
  const request = {
    mode: "replace",
    adjustment: {
      availableAt: { kind: "inherit" },
      dueAt: { kind: "set", value: "2026-08-20T09:30:00.000" },
      closesAt: { kind: "unrestricted" },
      assignmentAttemptTimeLimitSeconds: { kind: "set", value: 3600 },
      attemptLimit: { kind: "set", value: 2 },
    },
  };
  assert.deepEqual(decodeSyntheticPreviewAccommodationAdjustmentRequest(request), request);
  assert.throws(
    () =>
      decodeSyntheticPreviewAccommodationAdjustmentRequest({
        ...request,
        adjustment: { ...request.adjustment, dueAt: { kind: "set" } },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSyntheticPreviewAccommodationAdjustmentRequest({
        ...request,
        adjustment: { ...request.adjustment, dueAt: { kind: "set", value: 100 } },
      }),
    DecodeError,
  );
  assert.deepEqual(decodeTeachingOperationRevisionResponse({ revision: "42" }), {
    revision: "42",
  });
  assert.throws(() => decodeTeachingOperationRevisionResponse({ revision: "042" }), DecodeError);
});

test("safe-picker pages reject PII and preserve only bounded safe rows", () => {
  const targetPage = {
    targets: [
      {
        account: { reference: "U-7", display: "Ada Lovelace" },
      },
    ],
    nextCursor: "after-7",
  };
  const studentPage = {
    students: [membership("M-9", "Ada Lovelace")],
    nextCursor: null,
  };
  assert.deepEqual(decodeCourseInvitationTargetSearchPage(targetPage), targetPage);
  assert.deepEqual(
    decodeCourseInvitationTargetSearchRequest({ query: "Ada", after: null, size: 20 }),
    {
      query: "Ada",
      after: null,
      size: 20,
    },
  );
  assert.deepEqual(decodeCourseStudentMembershipsPage(studentPage), studentPage);
  assert.throws(
    () =>
      decodeCourseInvitationTargetSearchPage({
        ...targetPage,
        targets: [
          {
            ...targetPage.targets[0],
            account: { ...targetPage.targets[0].account, email: "private" },
          },
        ],
      }),
    DecodeError,
  );
  assert.throws(
    () => decodeCourseInvitationTargetSearchRequest({ query: "A", after: null, size: 20 }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCourseStudentMembershipsPage({
        ...studentPage,
        students: [{ ...studentPage.students[0], userId: "private" }],
      }),
    DecodeError,
  );
});

test("Instructor Course Invitation creation accepts only the Instructor-only operation shape", () => {
  const request = { target: "U-7" };
  assert.deepEqual(decodeInstructorCourseInvitationCreateRequest(request), request);
  assert.throws(
    () =>
      decodeInstructorCourseInvitationCreateRequest({
        target: "U-7",
        membershipRole: "instructor",
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeInstructorCourseInvitationCreateRequest({ target: "U-7", membershipRole: "student" }),
    DecodeError,
  );
});

test("empty Instructor removal contract rejects invented data", () => {
  assert.deepEqual(decodeInstructorMembershipRemovalRequest({}), {});
  assert.throws(() => decodeInstructorMembershipRemovalRequest({ revision: "3" }), DecodeError);
});
