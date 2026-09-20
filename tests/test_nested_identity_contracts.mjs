import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeApplyAssessmentBlueprintUpdateInput } from "../src/api/decoders/assessment_blueprint_update.ts";
import { decodeLiveAssessmentAccess } from "../src/api/decoders/assessment_attempt_issuance.ts";
import {
  decodeAssessmentQuestionPoolSelectionCountReceipt,
  decodeImportedAssessmentQuestionPoolFork,
} from "../src/api/decoders/assessment_pool_fork.ts";
import { decodeDueSoonAssessments } from "../src/api/decoders/assessment_release.ts";
import { decodeInstructorStudentView } from "../src/api/decoders/assessment_student_view.ts";
import { decodeCourseGradebook } from "../src/api/decoders/live_gradebook.ts";
import { decodeAssessmentSummary } from "../src/api/decoders/question_library.ts";
import { decodeBlueprintChangeProposalCreateRequest } from "../src/api/decoders/blueprint_change_proposal.ts";
import { decodeBlueprintComparisonView } from "../src/api/decoders/blueprint_comparison.ts";
import { decodeKnownBlueprintForks } from "../src/api/decoders/blueprint_course.ts";
import {
  decodeRecoveredAttempt,
  decodeRecoverySelection,
} from "../src/api/decoders/course_student_work_recovery.ts";

const COURSE = "CI6F2R8TA0";
const ATTEMPT = "0198e000-0000-7000-8000-000000000017";
const ENTRY = "0198e000-0000-7000-8000-000000000018";
const QUESTION_TUPLE = { questionId: "ABCD-XEFG", revisionNumber: 1 };
const BLUEPRINT_TUPLE = { blueprintCourseId: "BP7K3M2QXH", revisionNumber: "1" };

test("Recovery selection requires branded Course Instance and Assessment Attempt IDs", () => {
  const decoded = decodeRecoverySelection({
    action: "select",
    courseInstanceId: COURSE,
    nextCursor: null,
    attempts: [
      {
        courseInstanceId: COURSE,
        courseRosterTuple: null,
        assessmentId: "A7K3M2QXF",
        assessmentTitle: "Quiz",
        assessmentAttemptId: ATTEMPT,
        assessmentAttemptNumber: 1,
        startedAt: "2026-09-01T12:00:00Z",
        submittedAt: null,
        studentDataArchivedAt: "2026-09-02T12:00:00Z",
        deleteDueAt: "2026-09-09T12:00:00Z",
      },
    ],
  });
  assert.equal(decoded.courseInstanceId, COURSE);
  assert.equal(decoded.attempts[0]?.assessmentAttemptId, ATTEMPT);
  assert.throws(
    () =>
      decodeRecoverySelection({
        action: "select",
        course: COURSE,
        nextCursor: null,
        attempts: [],
      }),
    DecodeError,
  );
});

test("Recovered Question requires a Question Revision Tuple and rejects split siblings", () => {
  const attempt = {
    courseInstanceId: COURSE,
    courseRosterTuple: null,
    assessmentId: "A7K3M2QXF",
    assessmentTitle: "Quiz",
    assessmentAttemptId: ATTEMPT,
    assessmentAttemptNumber: 1,
    startedAt: "2026-09-01T12:00:00Z",
    studentDataArchivedAt: "2026-09-02T12:00:00Z",
    deleteDueAt: "2026-09-09T12:00:00Z",
    expiresAt: null,
    attemptFactsText: "facts",
    submissionText: null,
    questions: [
      {
        issuedPosition: 0,
        questionRevisionTuple: QUESTION_TUPLE,
        deliveryText: "prompt",
        poolText: null,
        attemptText: null,
        presentationText: null,
        reproductionText: null,
        backendDocumentText: null,
        savedResponseText: null,
        finalizedResponseText: null,
        gradingText: null,
        unavailableEvidence: [],
      },
    ],
  };
  const decoded = decodeRecoveredAttempt({ action: "recover", attempt });
  assert.deepEqual(decoded.questions[0]?.questionRevisionTuple, QUESTION_TUPLE);
  assert.throws(
    () =>
      decodeRecoveredAttempt({
        action: "recover",
        attempt: {
          ...attempt,
          questions: [
            {
              ...attempt.questions[0],
              questionId: QUESTION_TUPLE.questionId,
              revisionNumber: QUESTION_TUPLE.revisionNumber,
            },
          ],
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeRecoveredAttempt({
        action: "recover",
        attempt: {
          ...attempt,
          questions: [
            {
              issuedPosition: 0,
              questionId: QUESTION_TUPLE.questionId,
              revisionNumber: QUESTION_TUPLE.revisionNumber,
              deliveryText: "prompt",
              poolText: null,
              attemptText: null,
              presentationText: null,
              reproductionText: null,
              backendDocumentText: null,
              savedResponseText: null,
              finalizedResponseText: null,
              gradingText: null,
              unavailableEvidence: [],
            },
          ],
        },
      }),
    DecodeError,
  );
});

test("Pool fork and selection-count receipts reject leftover entry identities", () => {
  const imported = {
    assessmentEntryId: ENTRY,
    questionPoolId: QUESTION_TUPLE.questionId,
    questionPoolEditNumber: 1,
    assessmentEditNumber: "2",
  };
  assert.equal(decodeImportedAssessmentQuestionPoolFork(imported).assessmentEntryId, ENTRY);
  assert.throws(
    () => decodeImportedAssessmentQuestionPoolFork({ ...imported, entry: ENTRY }),
    DecodeError,
  );
  const receipt = {
    assessmentEntryId: ENTRY,
    selectionCount: 1,
    assessmentEditNumber: "2",
  };
  assert.equal(decodeAssessmentQuestionPoolSelectionCountReceipt(receipt).assessmentEntryId, ENTRY);
  assert.throws(
    () => decodeAssessmentQuestionPoolSelectionCountReceipt({ ...receipt, entry: ENTRY }),
    DecodeError,
  );
});

test("Attempt issuance rejects leftover attempt identity names", () => {
  assert.throws(
    () =>
      decodeLiveAssessmentAccess({
        decision: { kind: "unavailable" },
        attempt: ATTEMPT,
        title: "Quiz",
        assessmentType: "regular_assignment",
        questionCount: 1,
        pointsPossible: 1,
        previousAttempts: [],
      }),
    DecodeError,
  );
});

test("Student View rejects leftover course identity names", () => {
  assert.throws(
    () =>
      decodeInstructorStudentView({
        course: COURSE,
        editNumber: "1",
        status: "unreleased",
        title: "Quiz",
        instructions: "",
        displayTimeZone: "America/New_York",
        delivery: { kind: "ordered" },
        entries: [],
      }),
    DecodeError,
  );
});

test("Assessment Summary, Gradebook, and Due Soon reject leftover courseId", () => {
  const summary = {
    id: "A8H4N6PA6",
    courseInstanceId: COURSE,
    title: "Protein structure",
    entries: [],
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      submitted_response: "after_submit",
      question_answer: "never",
      question_answer_explanation: "never",
      class_statistics: "never",
    },
    policies: {
      questionVariationRule: "newVariation",
      assessmentQuestionOrderRule: "shuffled",
    },
  };
  assert.equal(decodeAssessmentSummary(summary, "summary", true).courseInstanceId, COURSE);
  assert.throws(
    () => decodeAssessmentSummary({ ...summary, courseId: COURSE }, "summary", true),
    DecodeError,
  );

  const gradebook = {
    courseInstanceId: COURSE,
    studentWork: [],
  };
  assert.equal(decodeCourseGradebook(gradebook).courseInstanceId, COURSE);
  assert.throws(() => decodeCourseGradebook({ ...gradebook, courseId: COURSE }), DecodeError);

  const dueSoon = {
    items: [
      {
        courseInstanceId: COURSE,
        courseLongName: "Molecular Biology",
        assessmentId: "A9J5V7WA3",
        assessmentType: "quiz",
        assessmentTitle: "DNA repair",
        assessmentStatus: "released",
        dueAtMillis: 1790971200125,
      },
    ],
    nextCursor: null,
    displayTimeZone: "America/Chicago",
  };
  assert.equal(decodeDueSoonAssessments(dueSoon).items[0]?.courseInstanceId, COURSE);
  assert.throws(
    () =>
      decodeDueSoonAssessments({
        ...dueSoon,
        items: [{ ...dueSoon.items[0], courseId: COURSE }],
      }),
    DecodeError,
  );
});

test("Change Proposal create rejects leftover target and split Revision fields", () => {
  assert.throws(
    () =>
      decodeBlueprintChangeProposalCreateRequest({
        target: BLUEPRINT_TUPLE.blueprintCourseId,
        sourceRevisionTuple: BLUEPRINT_TUPLE,
        sourceBlueprintEditNumber: "1",
        targetRevisionTuple: BLUEPRINT_TUPLE,
        targetBlueprintEditNumber: "1",
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeBlueprintChangeProposalCreateRequest({
        sourceRevisionNumber: "1",
        sourceBlueprintEditNumber: "1",
        targetRevisionNumber: "1",
        targetBlueprintEditNumber: "1",
      }),
    DecodeError,
  );
});

test("Blueprint comparison rejects leftover scalar left/right Blueprint Course IDs", () => {
  assert.throws(
    () =>
      decodeBlueprintComparisonView({
        left: BLUEPRINT_TUPLE.blueprintCourseId,
        right: "BP6F2R8TA9",
        assessmentRelationships: [],
        sharedQuestionIds: [],
        leftOnlyQuestionIds: [],
        rightOnlyQuestionIds: [],
      }),
    DecodeError,
  );
});

test("Known forks require named current and source Blueprint Revision Tuples", () => {
  const forks = decodeKnownBlueprintForks([
    {
      id: BLUEPRINT_TUPLE.blueprintCourseId,
      shortName: "Fork",
      longName: "Fork Course",
      availability: "private",
      currentRevisionTuple: BLUEPRINT_TUPLE,
      sourceRevisionTuple: BLUEPRINT_TUPLE,
      ownerDisplayName: "Elena",
    },
  ]);
  assert.deepEqual(forks[0]?.currentRevisionTuple, BLUEPRINT_TUPLE);
  assert.throws(
    () =>
      decodeKnownBlueprintForks([
        {
          id: BLUEPRINT_TUPLE.blueprintCourseId,
          shortName: "Fork",
          longName: "Fork Course",
          availability: "private",
          currentRevisionNumber: "2",
          sourceRevisionNumber: "1",
          ownerDisplayName: "Elena",
        },
      ]),
    DecodeError,
  );
});

test("Blueprint update apply rejects leftover expectedEditNumber", () => {
  assert.throws(
    () =>
      decodeApplyAssessmentBlueprintUpdateInput({
        expectedSourceRevisionNumber: "2",
        expectedEditNumber: "3",
      }),
    DecodeError,
  );
  assert.deepEqual(
    decodeApplyAssessmentBlueprintUpdateInput({
      expectedSourceBlueprintRevisionTuple: BLUEPRINT_TUPLE,
      expectedAssessmentEditNumber: "3",
    }),
    {
      expectedSourceBlueprintRevisionTuple: BLUEPRINT_TUPLE,
      expectedAssessmentEditNumber: "3",
    },
  );
});
