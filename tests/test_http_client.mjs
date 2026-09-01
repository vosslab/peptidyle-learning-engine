// Focused strict same-origin HTTP client and decoder behavior tests.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeQuestionPage,
  decodeDraftQuestionContent,
  decodeQuestionSubmissionAcknowledgement,
  decodeQuestionPresentation,
  decodeAssignmentAttempt,
} from "../src/api/decoders.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import {
  createRecordingFetch,
  issuedQuestionWireFixture,
  jsonResponse,
} from "./http_client_test_support.mjs";

test("question decoders reject answer-bearing and provider-secret fields", () => {
  const draft = publishedProblemFixture.draft;
  assert.throws(() => decodeDraftQuestionContent({ ...draft, answer: "secret" }), DecodeError);
  assert.throws(
    () =>
      decodeDraftQuestionContent({
        ...draft,
        backendLocator: {
          backend: "imathas",
          provider: "self-hosted",
          itemRef: "42",
          token: "secret",
        },
      }),
    DecodeError,
  );
});

test("issued external-tool envelopes accept only their public marker", () => {
  const envelope = {
    variation: {
      questionRevision: { questionId: "7K3-M9QP", revisionNumber: 1 },
      seed: 2,
    },
    title: "External practice item",
    prompt: [],
    response: { kind: "externalTool" },
  };
  assert.deepEqual(decodeQuestionPresentation(envelope).response, { kind: "externalTool" });
  assert.throws(
    () =>
      decodeQuestionPresentation({
        ...envelope,
        variation: {
          ...envelope.variation,
          generator: { id: "secret", version: "1" },
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionPresentation({
        ...envelope,
        response: { kind: "externalTool", token: "secret" },
      }),
    DecodeError,
  );
});

test("Question Submission acknowledgement separates its answer-free receipt and grading state", () => {
  const attempt = publishedProblemFixture.attempts[0];
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
    "pending acknowledgement cannot mix detailed receipt material at its outer boundary",
  );
});

test("Question Library pages remain bounded and do not disclose answer material", () => {
  const page = { items: [publishedProblemFixture.publishedQuestion], nextCursor: null };
  assert.deepEqual(decodeQuestionPage(page), page);
  assert.throws(() => decodeQuestionPage({ ...page, answerKey: "secret" }), DecodeError);
  assert.throws(
    () => decodeQuestionPage({ ...page, items: Array.from({ length: 101 }, () => page.items[0]) }),
    DecodeError,
  );
});

test("prefetch is a body-free same-origin no-store request", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
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
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const assignmentAttempt = publishedProblemFixture.runs[0];
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
  const assignmentAttempt = publishedProblemFixture.runs[0];
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

test("prefetch rejects a descriptor with a mismatched issued identity", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const predecessor = publishedProblemFixture.attempts[0];
  assert.ok(predecessor);
  const envelope = {
    ...issuedQuestionWireFixture(predecessor, publishedProblemFixture.publishedProblem),
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
          ...publishedProblemFixture.issuedQuestions[1],
          reference: {
            ...publishedProblemFixture.issuedQuestions[1].reference,
            version: "0198e000-0000-7000-8000-000000000099",
          },
        },
        seed: predecessor.seed,
        renderedQuestionSha256: "a".repeat(64),
        questionPoolSelectionPosition: null,
        envelope,
      }),
  });
  await assert.rejects(
    client.prefetchNextQuestion(course.id, assignment.id, predecessor.id),
    DecodeError,
  );
});

test("prefetch preserves safe Question Pool selection for the cache-hit successor", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const predecessor = publishedProblemFixture.attempts[0];
  assert.ok(predecessor);
  const envelope = issuedQuestionWireFixture(predecessor, publishedProblemFixture.publishedProblem);
  const client = createHttpApiClient({
    fetch: async () =>
      jsonResponse({
        predecessor: predecessor.id,
        issuedQuestion: publishedProblemFixture.issuedQuestions[1],
        seed: envelope.seed,
        renderedQuestionSha256: "b".repeat(64),
        questionPoolSelectionPosition: { itemNumber: 1, itemCount: 2 },
        envelope,
      }),
  });
  const prefetched = await client.prefetchNextQuestion(course.id, assignment.id, predecessor.id);
  assert.deepEqual(prefetched?.questionPoolSelectionPosition, { itemNumber: 1, itemCount: 2 });
});

test("external-tool launch is a strict same-origin route projection", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const launchUrl = `/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/external-tool/launch`;
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    jsonResponse({ launchUrl }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(await client.beginExternalToolLaunch(course.id, assignment.id, attempt.id), {
    launchUrl,
  });
  assert.equal(requests[0]?.method, "POST");
  assert.equal(requests[0]?.url, `https://client.example.test${launchUrl}`);
  assert.equal(await requests[0]?.text(), "");
});

test("external-tool launch rejects absolute, foreign, and decorated routes", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
  assert.ok(attempt);
  const expected = `/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/external-tool/launch`;
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
      client.beginExternalToolLaunch(course.id, assignment.id, attempt.id),
      DecodeError,
      launchUrl,
    );
  }
});

test("ordinary submission uses the explicit nested binding and answer-only body", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
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
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
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

test("external-tool submission sends only the marker with its caller idempotency key", async () => {
  const course = publishedProblemFixture.course;
  const assignment = publishedProblemFixture.assignment;
  const attempt = publishedProblemFixture.attempts[0];
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
          response: { kind: "externalTool" },
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
    { kind: "externalTool" },
    "external-tool-once",
  );
  const request = requests[0];
  assert.ok(request);
  assert.equal(
    request.url,
    `https://client.example.test/api/courses/${course.id}/assignments/${assignment.id}/attempts/${attempt.id}/external-tool/launch/submission`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("idempotency-key"), "external-tool-once");
  assert.deepEqual(await request.json(), { response: { kind: "externalTool" } });
});
