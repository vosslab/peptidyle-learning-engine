import assert from "node:assert/strict";
import test from "node:test";

import { loadStudentRibbonNavigation } from "../src/ribbon/student_ribbon_navigation.ts";

const BIOCHEMISTRY_ID = "00000000-0000-4000-8000-000000000001";
const REVIEW_COURSE_ID = "00000000-0000-4000-8000-000000000002";
const BIOCHEMISTRY_ATTEMPT_ID = "00000000-0000-4000-8000-000000000011";
const REVIEW_ATTEMPT_ID = "00000000-0000-4000-8000-000000000012";
const FEEDBACK_ATTEMPT_ID = "00000000-0000-4000-8000-000000000013";

test("Student Ribbon navigation preserves Course order and selects a reliable active Attempt", async () => {
  const courses = [
    { id: REVIEW_COURSE_ID, shortName: "Zebra", longName: "Aardvark Review Course" },
    { id: BIOCHEMISTRY_ID, shortName: "Alpha", longName: "Biochemistry 301" },
  ];
  const client = {
    listLiveStudentCourses: async () => courses,
    getStudentCourseActiveAttempt: async (courseId) => {
      return courseId === BIOCHEMISTRY_ID
        ? {
            assessmentAttemptId: BIOCHEMISTRY_ATTEMPT_ID,
            startedAt: 1_000,
            latestActivityAt: 2_000,
          }
        : {
            assessmentAttemptId: REVIEW_ATTEMPT_ID,
            startedAt: 1_500,
            latestActivityAt: 2_000,
          };
    },
    getStudentLatestFeedback: async () => ({ assessmentAttemptId: FEEDBACK_ATTEMPT_ID }),
  };

  const navigation = await loadStudentRibbonNavigation(client);
  assert.deepEqual(navigation.studentCourses, [
    { id: REVIEW_COURSE_ID, shortName: "Zebra" },
    { id: BIOCHEMISTRY_ID, shortName: "Alpha" },
  ]);
  assert.equal(navigation.activeAttemptId, REVIEW_ATTEMPT_ID);
  assert.equal(navigation.latestFeedbackAttemptId, FEEDBACK_ATTEMPT_ID);
});

test("an unavailable Course lookup does not hide another Course's active Attempt", async () => {
  const client = {
    listLiveStudentCourses: async () => [
      { id: REVIEW_COURSE_ID, shortName: "Review" },
      { id: BIOCHEMISTRY_ID, shortName: "Biochemistry" },
    ],
    getStudentCourseActiveAttempt: async (courseId) => {
      if (courseId === REVIEW_COURSE_ID) throw new Error("temporary lookup failure");
      return {
        assessmentAttemptId: BIOCHEMISTRY_ATTEMPT_ID,
        startedAt: 1_000,
        latestActivityAt: 2_000,
      };
    },
    getStudentLatestFeedback: async () => ({ assessmentAttemptId: null }),
  };

  const navigation = await loadStudentRibbonNavigation(client);
  assert.equal(navigation.activeAttemptId, BIOCHEMISTRY_ATTEMPT_ID);
});
