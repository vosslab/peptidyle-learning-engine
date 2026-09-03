// Focused strict same-origin HTTP client and decoder behavior tests.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedQuestionFixture } from "./fixtures/published_question.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeQuestionPage,
  decodeDraftQuestionContent,
  decodeQuestionSubmissionAcknowledgement,
  decodeIssuedQuestionPresentation,
  decodeAssignmentAttempt,
  decodeStudentQuestionAttemptView,
} from "../src/api/decoders.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import {
  createRecordingFetch,
  issuedQuestionWireFixture,
  jsonResponse,
} from "./http_client_test_support.mjs";

test("question decoders reject answer-bearing and iMathAS-secret fields", () => {
  const draft = publishedQuestionFixture.draft;
  assert.throws(() => decodeDraftQuestionContent({ ...draft, answer: "secret" }), DecodeError);
  assert.throws(
    () =>
      decodeDraftQuestionContent({
        ...draft,
        questionBackend: "imathas",
        draftImathasQuestionBackendBinding: {
          deploymentReference: "self-hosted",
          itemReference: "42",
          backendSecret: "secret",
        },
      }),
    DecodeError,
  );
});

test("an issued iMathAS Question Backend Question Presentation accepts only its public marker", () => {
  const presentation = {
    questionRevision: { questionId: "7K3-M9QP", revisionNumber: 1 },
    question_seed: 2,
    presentationNonce: "0123456789abcdef0123456789abcdef",
    title: "iMathAS Question Backend practice item",
    prompt: [],
    response: { kind: "imathasQuestionBackend" },
  };
  assert.deepEqual(decodeIssuedQuestionPresentation(presentation).response, {
    kind: "imathasQuestionBackend",
  });
  assert.throws(
    () =>
      decodeIssuedQuestionPresentation({
        ...presentation,
        imathasQuestionBackendBinding: { itemReference: "secret" },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeIssuedQuestionPresentation({
        ...presentation,
        response: { kind: "imathasQuestionBackend", token: "secret" },
      }),
    DecodeError,
  );
});

test("Question Submission acknowledgement separates its answer-free receipt and grading state", () => {
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const pending = {
    receipt: { accepted: true, attemptId: attempt.id },
    gradingState: "pending",
    nextAction: "check_status",
  };
  assert.deepEqual(decodeQuestionSubmissionAcknowledgement(pending), pending);
  assert.throws(
    () => decodeQuestionSubmissionAcknowledgement({ ...pending, gradingState: "unknown" }),
    DecodeError,
  );
  for (const forbidden of [
    "response",
    "feedback",
    "result",
    "score",
    "nextIssued",
    "nextPending",
  ]) {
    assert.throws(
      () => decodeQuestionSubmissionAcknowledgement({ ...pending, [forbidden]: "private" }),
      DecodeError,
      forbidden,
    );
  }
  assert.throws(
    () => decodeQuestionSubmissionAcknowledgement({ ...pending, attempt: {} }),
    DecodeError,
    "pending acknowledgement cannot mix detailed receipt data at its outer boundary",
  );
});

test("Question Library pages remain bounded and do not disclose an Answer Key", () => {
  const page = { items: [publishedQuestionFixture.publishedQuestion], nextCursor: null };
  assert.deepEqual(decodeQuestionPage(page), page);
  assert.throws(() => decodeQuestionPage({ ...page, answerKey: "secret" }), DecodeError);
  assert.throws(
    () => decodeQuestionPage({ ...page, items: Array.from({ length: 101 }, () => page.items[0]) }),
    DecodeError,
  );
});

test("prefetch is a body-free same-origin no-store request", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(String(input), "https://client.example.test"), init);
      requests.push(request);
      return new Response(null, { status: 204 });
    },
  });
  assert.equal(await client.prefetchNextQuestion(course.id, assignment.id, attempt.id), null);
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/prefetch-next`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.cache, "no-store");
  assert.equal(await request.text(), "");
});

test("Assignment Attempt start uses the explicit nested course and assignment route without a body", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const assignmentAttempt = publishedQuestionFixture.assignment_attempts[0];
  assert.ok(assignmentAttempt);
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    jsonResponse(assignmentAttempt),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(
    await client.startAssignmentAttempt(course.id, assignment.id),
    assignmentAttempt,
  );
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/assignment-attempts`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("content-type"), null);
  assert.equal(await request.text(), "");
});

test("Assignment Attempt transport preserves its exact Released Assignment Revision", () => {
  const assignmentAttempt = publishedQuestionFixture.assignment_attempts[0];
  assert.ok(assignmentAttempt);
  assert.deepEqual(decodeAssignmentAttempt(assignmentAttempt).assignmentRevision, {
    assignment: "A-1",
    revision_number: "1",
  });
  assert.throws(() => {
    const { assignmentRevision: _revision, ...withoutRevision } = assignmentAttempt;
    return decodeAssignmentAttempt(withoutRevision);
  }, DecodeError);
});

test("Student Question Attempt decoding accepts every generated issued capability and rejects retired values", () => {
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const { questionPoolSelectionPosition: _position, ...attemptView } = attempt;
  for (const issuedCapability of [
    "questionPresentation",
    "pleQuestionJsonPresentation",
    "webworkPresentation",
    "qtiPresentation",
    "notApplicable",
  ]) {
    assert.equal(
      decodeStudentQuestionAttemptView({ ...attemptView, issuedCapability }).issuedCapability,
      issuedCapability,
    );
  }
  assert.throws(
    () =>
      decodeStudentQuestionAttemptView({
        ...attemptView,
        issuedCapability: "presentationEnvelope",
      }),
    DecodeError,
  );
});

test("prefetch rejects a descriptor with a mismatched issued identity", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const predecessor = publishedQuestionFixture.attempts[0];
  assert.ok(predecessor);
  const questionPresentation = {
    ...issuedQuestionWireFixture(predecessor, publishedQuestionFixture.publishedQuestionRevision),
    questionRevision: {
      questionId: "BCDEFGH",
      revisionNumber: "99",
    },
  };
  const client = createHttpApiClient({
    fetch: async () =>
      jsonResponse({
        predecessor: predecessor.id,
        issuedQuestion: {
          ...publishedQuestionFixture.issuedQuestions[1],
          reference: {
            ...publishedQuestionFixture.issuedQuestions[1].reference,
            version: "0198e000-0000-7000-8000-000000000099",
          },
        },
        question_seed: predecessor.question_seed,
        renderedQuestionSha256: "a".repeat(64),
        questionPoolSelectionPosition: null,
        presentation: questionPresentation,
      }),
  });
  await assert.rejects(
    client.prefetchNextQuestion(course.id, assignment.id, predecessor.id),
    DecodeError,
  );
});

test("prefetch preserves safe Question Pool selection for the cache-hit successor", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const predecessor = publishedQuestionFixture.attempts[0];
  assert.ok(predecessor);
  const questionPresentation = issuedQuestionWireFixture(
    predecessor,
    publishedQuestionFixture.publishedQuestionRevision,
  );
  const client = createHttpApiClient({
    fetch: async () =>
      jsonResponse({
        predecessor: predecessor.id,
        issuedQuestion: publishedQuestionFixture.issuedQuestions[1],
        question_seed: questionPresentation.question_seed,
        renderedQuestionSha256: "b".repeat(64),
        questionPoolSelectionPosition: { selectedQuestionNumber: 1, selectedQuestionCount: 2 },
        presentation: questionPresentation,
      }),
  });
  const prefetched = await client.prefetchNextQuestion(course.id, assignment.id, predecessor.id);
  assert.deepEqual(prefetched?.questionPoolSelectionPosition, {
    selectedQuestionNumber: 1,
    selectedQuestionCount: 2,
  });
});

test("iMathAS Question Backend launch returns its strict same-origin launch route", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const launchUrl = `/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/imathas-question-backend/launch`;
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    jsonResponse({ launchUrl }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(
    await client.beginImathasQuestionBackendLaunch(course.id, assignment.id, attempt.id),
    {
      launchUrl,
    },
  );
  assert.equal(requests[0]?.method, "POST");
  assert.equal(requests[0]?.url, `https://client.example.test${launchUrl}`);
  assert.equal(await requests[0]?.text(), "");
});

test("iMathAS Question Backend launch rejects absolute, foreign, and decorated routes", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const expected = `/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/imathas-question-backend/launch`;
  const routes = [
    `https://client.example.test${expected}`,
    `https://foreign.example${expected}`,
    `//foreign.example${expected}`,
    expected.replace(course.id, "other-course"),
    expected.replace(assignment.id, "other-assignment"),
    expected.replace(attempt.id, "other-attempt"),
    "/api/health",
    `${expected}?token=secret`,
    `${expected}#fragment`,
  ];
  for (const launchUrl of routes) {
    const client = createHttpApiClient({
      fetch: async () => jsonResponse({ launchUrl }),
    });
    await assert.rejects(
      client.beginImathasQuestionBackendLaunch(course.id, assignment.id, attempt.id),
      DecodeError,
      launchUrl,
    );
  }
});

test("ordinary submission uses the explicit nested binding and answer-only body", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const { questionPoolSelectionPosition: _questionPoolSelectionPosition, ...receiptAttempt } =
    attempt;
  const response = { kind: "numeric", value: 18 };
  const receipt = {
    receipt: {
      accepted: true,
      attempt: {
        ...receiptAttempt,
        submission: { ...receiptAttempt.submission, response, gradingResult: null },
      },
      feedback: null,
      assignmentScoringState: "current",
      assignmentAttemptCompletion: "inProgress",
      nextIssued: null,
      nextPending: false,
    },
    gradingState: "graded",
  };
  const { recordingFetch, requests } = createRecordingFetch(async () => jsonResponse(receipt));
  const client = createHttpApiClient({ fetch: recordingFetch });

  await client.submitResponse(course.id, assignment.id, attempt.id, response, "nested-once");
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/submissions`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("idempotency-key"), "nested-once");
  assert.deepEqual(await request.json(), { response });
});

test("submission status uses its route-bound same-origin no-store GET", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const pending = {
    receipt: { accepted: true, attemptId: attempt.id },
    gradingState: "pending",
    nextAction: "check_status",
  };
  const { recordingFetch, requests } = createRecordingFetch(async () => jsonResponse(pending, 202));
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(await client.getSubmissionStatus(course.id, assignment.id, attempt.id), pending);
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/submission-status`,
  );
  assert.equal(request.method, "GET");
  assert.equal(request.credentials, "same-origin");
  assert.equal(request.cache, "no-store");
  assert.equal(await request.text(), "");
});

test("iMathAS Question Backend submission sends only the marker with its caller idempotency key", async () => {
  const course = publishedQuestionFixture.course;
  const assignment = publishedQuestionFixture.assignment;
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const { questionPoolSelectionPosition: _questionPoolSelectionPosition, ...receiptAttempt } =
    attempt;
  const receipt = {
    receipt: {
      accepted: true,
      attempt: {
        ...receiptAttempt,
        submission: {
          ...receiptAttempt.submission,
          response: { kind: "imathasQuestionBackend" },
          gradingResult: null,
        },
      },
      feedback: null,
      assignmentScoringState: "current",
      assignmentAttemptCompletion: "inProgress",
      nextIssued: null,
      nextPending: false,
    },
    gradingState: "graded",
  };
  const { recordingFetch, requests } = createRecordingFetch(async () => jsonResponse(receipt));
  const client = createHttpApiClient({ fetch: recordingFetch });

  await client.submitResponse(
    course.id,
    assignment.id,
    attempt.id,
    { kind: "imathasQuestionBackend" },
    "imathas-question-backend-once",
  );
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/imathas-question-backend/launch/submission`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("idempotency-key"), "imathas-question-backend-once");
  assert.deepEqual(await request.json(), { response: { kind: "imathasQuestionBackend" } });
});
