import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeAssignmentPolicyPatchUpdateRequest,
  decodeCourseInvitationTargetSearchPage,
  decodeCourseInvitationTargetSearchRequest,
  decodeCourseStudentMembershipsPage,
  decodeInstructorMembershipRemovalRequest,
  decodeRetentionReadView,
  decodeSysadminInstructorCandidateSearchPage,
  decodeSysadminInstructorCandidateSearchRequest,
  decodeTeachingOperationRevisionResponse,
  decodeTeachingPreviewView,
} from "../src/api/decoders.ts";

function membership(reference, display) {
  return { reference, display, role: "student", status: "active" };
}

function baseSource() {
  return { kind: "base", label: "Course policy" };
}

test("allowed preview carries exact provenance while denied preview is sealed", () => {
  const source = baseSource();
  const preview = {
    entitlement: "allowed",
    timeZone: "America/Chicago",
    start: { kind: "mayStart", late: "onTime" },
    availableAt: { value: null, source },
    dueAt: { value: "2026-08-20T09:30:00.000", source },
    closesAt: { value: "2026-08-20T10:30:00.000", source },
    timeLimitSeconds: { value: 3600, source },
    attemptLimit: { value: 2, source },
    lateSubmission: { value: "accept", source },
    deadlineBehavior: { value: "autoSubmit", source },
  };
  assert.deepEqual(decodeTeachingPreviewView(preview), preview);
  assert.throws(
    () =>
      decodeTeachingPreviewView({
        entitlement: "denied",
        reason: "notEntitled",
        dueAt: preview.dueAt,
      }),
    DecodeError,
  );
});

test("policy mutation decoders require every explicit patch state", () => {
  const request = {
    mode: "override",
    patch: {
      availableAt: { kind: "inherit" },
      dueAt: { kind: "set", value: "2026-08-20T09:30:00.000" },
      closesAt: { kind: "unrestricted" },
      timeLimitSeconds: { kind: "set", value: 3600 },
      attemptLimit: { kind: "set", value: 2 },
    },
  };
  assert.deepEqual(decodeAssignmentPolicyPatchUpdateRequest(request), request);
  assert.throws(
    () =>
      decodeAssignmentPolicyPatchUpdateRequest({
        ...request,
        patch: { ...request.patch, dueAt: { kind: "set" } },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeAssignmentPolicyPatchUpdateRequest({
        ...request,
        patch: { ...request.patch, dueAt: { kind: "set", value: 100 } },
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
        approval: { state: "approved", revision: "2" },
      },
    ],
    nextCursor: "after-7",
  };
  const studentPage = {
    students: [membership("M-9", "Ada Lovelace")],
    nextCursor: null,
  };
  assert.deepEqual(decodeCourseInvitationTargetSearchPage(targetPage), targetPage);
  assert.deepEqual(decodeCourseInvitationTargetSearchRequest({ query: "Ada", after: null, size: 20 }), {
    query: "Ada",
    after: null,
    size: 20,
  });
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

test("Sysadmin candidate search keeps approval eligibility distinct from account authority", () => {
  const candidates = {
    candidates: [
      {
        account: { reference: "U-8", display: "Avery Student" },
        approval: { state: "unapproved", revision: null },
      },
    ],
    nextCursor: null,
  };
  assert.deepEqual(decodeSysadminInstructorCandidateSearchPage(candidates), candidates);
  assert.deepEqual(
    decodeSysadminInstructorCandidateSearchRequest({ query: "Avery", after: null, size: 25 }),
    { query: "Avery", after: null, size: 25 },
  );
  assert.throws(
    () =>
      decodeSysadminInstructorCandidateSearchPage({
        ...candidates,
        candidates: [
          {
            ...candidates.candidates[0],
            account: { ...candidates.candidates[0].account, privateScope: "private" },
          },
        ],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSysadminInstructorCandidateSearchPage({
        ...candidates,
        candidates: [
          {
            ...candidates.candidates[0],
            approval: { state: "approved", revision: null },
          },
        ],
      }),
    DecodeError,
  );
});

test("retention and empty removal contracts reject leaked or invented data", () => {
  const retention = {
    state: "notificationDue",
    assignmentDefinitions: "retain",
    revision: "3",
    notification: { intent: "extend", createdAt: 1, copy: "Retention extended." },
  };
  assert.deepEqual(decodeRetentionReadView(retention), retention);
  assert.deepEqual(decodeInstructorMembershipRemovalRequest({}), {});
  assert.throws(() => decodeInstructorMembershipRemovalRequest({ revision: "3" }), DecodeError);
  assert.throws(
    () => decodeRetentionReadView({ ...retention, recipients: ["private"] }),
    DecodeError,
  );
});
