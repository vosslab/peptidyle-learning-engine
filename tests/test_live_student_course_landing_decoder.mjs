import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeLiveStudentAssessmentLandings,
  decodeLiveStudentCourseLandings,
  decodeLiveStudentCourseInvitations,
} from "../src/api/decoders/live_student_course_landing.ts";

test("Student Course landing carries both Course Instance names", () => {
  const value = {
    courses: [{ id: "CI6F2R8TA0", shortName: "Mol Bio", longName: "Molecular Biology" }],
  };
  assert.deepEqual(decodeLiveStudentCourseLandings(value), value.courses);
});

test("pending invitations require complete verified Instructor and inclusive term context", () => {
  const invitation = {
    id: "CI6F2R8TA0",
    shortName: "Mol Bio",
    longName: "Molecular Biology",
    instructorDisplayName: "Elena Voss",
    term: { startDate: "2026-08-24", endDate: "2026-12-12" },
  };
  assert.deepEqual(decodeLiveStudentCourseInvitations({ invitations: [invitation] }), [invitation]);
  for (const override of [
    { instructorDisplayName: undefined },
    { instructorDisplayName: null },
    { instructorDisplayName: "" },
    { instructorDisplayName: " Elena Voss" },
    { instructorDisplayName: "Elena\nVoss" },
    { instructorDisplayName: "a".repeat(201) },
    { term: undefined },
    { term: { startDate: "2026-08-24" } },
    { term: { startDate: "2026-08-24", endDate: "2026-08-23" } },
    { term: { startDate: "2026-02-30", endDate: "2026-12-12" } },
    { term: { ...invitation.term, timeZone: "UTC" } },
  ]) {
    assert.throws(
      () =>
        decodeLiveStudentCourseInvitations({
          invitations: [{ ...invitation, ...override }],
        }),
      DecodeError,
    );
  }
});

test("pending invitations reject private identities and roster fields", () => {
  const invitation = {
    id: "CI6F2R8TA0",
    shortName: "Mol Bio",
    longName: "Molecular Biology",
    instructorDisplayName: "Elena Voss",
    term: { startDate: "2026-08-24", endDate: "2026-12-12" },
  };
  for (const field of [
    "email",
    "accountId",
    "courseId",
    "invitationId",
    "membershipId",
    "roster",
  ]) {
    assert.throws(
      () =>
        decodeLiveStudentCourseInvitations({
          invitations: [{ ...invitation, [field]: "private" }],
        }),
      DecodeError,
    );
  }
});

function assessment(overrides = {}) {
  return {
    id: "A5D9Q3XAH",
    title: "Peptide practice",
    assessmentType: "regular_assignment",
    decision: {
      availableAt: 1_000,
      dueAt: 2_000,
      closesAt: 3_000,
      timeLimitSeconds: 900,
      attemptLimit: 2,
      lateWorkRule: "reject",
      displayTimeZone: "America/Chicago",
      evaluatedAt: 1_500,
      startDecision: "may_start",
      publicReason: null,
    },
    assessmentAttemptNumber: 1,
    assessmentAttemptCompletion: "inProgress",
    canResumeAssessmentAttempt: true,
    gradedQuestionCount: 1,
    savedQuestionCount: 1,
    questionCount: 4,
    ...overrides,
  };
}

test("Student Course landing accepts an omitted score while disclosure withholds it", () => {
  const value = { assessments: [assessment()] };
  assert.deepEqual(decodeLiveStudentAssessmentLandings(value), value.assessments);
});

test("Student Course landing accepts a disclosed Assessment score independently of latest progress", () => {
  const value = {
    assessments: [assessment({ assessmentScore: { pointsEarned: 1, pointsPossible: 2 } })],
  };
  assert.deepEqual(decodeLiveStudentAssessmentLandings(value), value.assessments);
});

test("Student Course landing accepts complete Bonus and extra-credit contribution pairs", () => {
  for (const assessmentScore of [
    { pointsEarned: 3, pointsPossible: 0 },
    { pointsEarned: 3, pointsPossible: 2 },
  ]) {
    const value = { assessments: [assessment({ assessmentScore })] };
    assert.deepEqual(decodeLiveStudentAssessmentLandings(value), value.assessments);
  }
});

test("Student Course landing requires the closed Assessment Type and resumability fields", () => {
  for (const assessmentType of [
    "regular_assignment",
    "practice_question_assignment",
    "bonus_assignment",
    "quiz",
    "exam",
  ]) {
    const value = { assessments: [assessment({ assessmentType })] };
    assert.deepEqual(decodeLiveStudentAssessmentLandings(value), value.assessments);
  }

  for (const candidate of [
    assessment({ assessmentType: undefined }),
    assessment({ assessmentType: "project" }),
    assessment({ canResumeAssessmentAttempt: undefined }),
    assessment({ canResumeAssessmentAttempt: "yes" }),
  ]) {
    assert.throws(
      () => decodeLiveStudentAssessmentLandings({ assessments: [candidate] }),
      DecodeError,
    );
  }
});

test("Student Course landing rejects malformed contributions and the retired score alias", () => {
  for (const candidate of [
    assessment({ assessmentScore: { pointsEarned: 1 } }),
    assessment({ assessmentScore: null }),
    assessment({ assessmentScore: { pointsEarned: -1, pointsPossible: 2 } }),
    assessment({ assessmentScore: { pointsEarned: 1, pointsPossible: -2 } }),
    assessment({ assessmentScore: { pointsEarned: Number.POSITIVE_INFINITY, pointsPossible: 2 } }),
    assessment({ score: { pointsEarned: 1, pointsPossible: 2 } }),
    assessment({ pointsEarned: 1, pointsPossible: 2 }),
  ]) {
    assert.throws(
      () => decodeLiveStudentAssessmentLandings({ assessments: [candidate] }),
      DecodeError,
    );
  }
});

test("Student Course landing requires the closed server decision summary", () => {
  assert.throws(
    () =>
      decodeLiveStudentAssessmentLandings({
        assessments: [
          assessment({
            decision: {
              ...assessment().decision,
              startDecision: "closed",
              publicReason: "This Assignment is closed for new work.",
              accommodationId: "private",
            },
          }),
        ],
      }),
    DecodeError,
  );
});
