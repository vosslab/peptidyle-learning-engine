import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeHypotheticalStudentViewScenarioModifiers,
  decodeInstructorCourseInvitationCreateRequest,
  decodeCourseInvitationTargetSearchPage,
  decodeCourseInvitationTargetSearchRequest,
  decodeInstructorMembershipRemovalRequest,
} from "../src/api/decoders.ts";

test("policy mutation decoders require every explicit adjustment state", () => {
  const request = {
    mode: "replace",
    adjustment: {
      available_at: { kind: "inherit" },
      due_at: { kind: "set", value: "2026-08-20T09:30:00.000" },
      closes_at: { kind: "unrestricted" },
      assignment_attempt_time_limit_seconds: { kind: "set", value: 3600 },
      attempt_limit: { kind: "set", value: 2 },
    },
  };
  assert.deepEqual(decodeHypotheticalStudentViewScenarioModifiers(request), request);
  assert.throws(
    () =>
      decodeHypotheticalStudentViewScenarioModifiers({
        ...request,
        adjustment: { ...request.adjustment, due_at: { kind: "set" } },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeHypotheticalStudentViewScenarioModifiers({
        ...request,
        adjustment: { ...request.adjustment, due_at: { kind: "set", value: 100 } },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeHypotheticalStudentViewScenarioModifiers({
        ...request,
        adjustment: {
          ...request.adjustment,
          unexpected_available_at: request.adjustment.available_at,
        },
      }),
    DecodeError,
  );
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
  assert.deepEqual(decodeCourseInvitationTargetSearchPage(targetPage), targetPage);
  assert.deepEqual(
    decodeCourseInvitationTargetSearchRequest({ query: "Ada", after: null, size: 20 }),
    {
      query: "Ada",
      after: null,
      size: 20,
    },
  );
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
