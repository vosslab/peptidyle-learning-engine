// Focused strict same-origin HTTP client and decoder behavior tests.

import assert from "node:assert/strict";
import test from "node:test";

import { MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS } from "../generated/api/MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS.ts";
import { MAX_QUESTION_TITLE_UNICODE_SCALARS } from "../generated/api/MAX_QUESTION_TITLE_UNICODE_SCALARS.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeQuestionPage,
  decodeIssuedQuestionPresentation,
  decodeStudentQuestionAttemptView,
} from "../src/api/decoders.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch, jsonResponse } from "./http_client_test_support.mjs";

test("asset URLs require and retain the exact Question Revision identity", () => {
  const client = createHttpApiClient({ basePath: "/live" });
  const assetUrl = client.assetUrl(
    { questionId: "7K3M-X9QP", revisionNumber: 2 },
    "00000000-0000-0000-0000-000000000001",
  );
  assert.equal(
    assetUrl,
    "/live/api/questions/7K3M-X9QP/revisions/2/assets/00000000-0000-0000-0000-000000000001",
  );
});

test("an issued iMathAS Question Backend Question Presentation accepts only its public marker", () => {
  const presentation = {
    questionRevision: { questionId: "7K3M-X9QP", revisionNumber: 1 },
    presentationNonce: "0123456789abcdef0123456789abcdef",
    questionTitle: "iMathAS Question Backend practice item",
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

test("Question Library pages remain bounded and do not disclose an Answer Key", () => {
  const question = {
    ...publishedQuestionFixture.publishedQuestion,
    questionFormat: "pleQuestionJson",
  };
  const page = { items: [question], nextCursor: null };
  assert.deepEqual(decodeQuestionPage(page), page);
  assert.throws(() => decodeQuestionPage({ ...page, answerKey: "secret" }), DecodeError);
  assert.throws(
    () => decodeQuestionPage({ ...page, items: Array.from({ length: 101 }, () => page.items[0]) }),
    DecodeError,
  );
});

test("Question Title and Question Description remain bounded at the strict Question Library boundary", () => {
  const summary = {
    ...publishedQuestionFixture.publishedQuestion,
    questionFormat: "pleQuestionJson",
  };
  const pageWithMetadata = (metadata) => ({
    items: [
      {
        ...summary,
        metadata,
      },
    ],
    nextCursor: null,
  });
  assert.throws(
    () =>
      decodeQuestionPage(
        pageWithMetadata({
          ...summary.metadata,
          questionTitle: "x".repeat(MAX_QUESTION_TITLE_UNICODE_SCALARS + 1),
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionPage(
        pageWithMetadata({
          ...summary.metadata,
          questionDescription: "x".repeat(MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS + 1),
        }),
      ),
    DecodeError,
  );
});

test("Student Question Attempt decoding accepts every generated issued capability and rejects retired values", () => {
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const {
    questionPoolSelectionPosition: _position,
    reproduction: _reproduction,
    ...attemptView
  } = attempt;
  for (const issuedCapability of [
    "questionPresentation",
    "pleQuestionJsonPresentation",
    "webworkPresentation",
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

// Permanent browser-boundary test: native source-generation evidence must not
// return through a Student attempt view. A failure means keep this wire
// contract closed rather than add a legacy-field compatibility path.
test("Student Question Attempt decoding rejects legacy reproduction fields", () => {
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const {
    questionPoolSelectionPosition: _position,
    reproduction: _reproduction,
    ...attemptView
  } = attempt;
  for (const field of ["reproduction", "question_seed", "generated_parameter_sha256"]) {
    assert.throws(
      () => decodeStudentQuestionAttemptView({ ...attemptView, [field]: "server-only" }),
      DecodeError,
      field,
    );
  }
});

test("iMathAS Question Backend launch returns its strict same-origin Assessment route", async () => {
  const course = publishedQuestionFixture.course;
  const assessment = publishedQuestionFixture.assignment;
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const launchUrl = `/api/courses/${course.id}/assessments/${assessment.id}/attempts/${attempt.id}/imathas-question-backend/launch`;
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    jsonResponse({ launchUrl }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(
    await client.beginImathasQuestionBackendLaunch(course.id, assessment.id, attempt.id),
    {
      launchUrl,
    },
  );
  assert.equal(requests[0]?.method, "POST");
  assert.equal(requests[0]?.url, `https://client.example.test${launchUrl}`);
  assert.equal(await requests[0]?.text(), "");
});

test("iMathAS Question Backend launch rejects noncanonical Assessment routes", async () => {
  const course = publishedQuestionFixture.course;
  const assessment = publishedQuestionFixture.assignment;
  const attempt = publishedQuestionFixture.attempts[0];
  assert.ok(attempt);
  const expected = `/api/courses/${course.id}/assessments/${assessment.id}/attempts/${attempt.id}/imathas-question-backend/launch`;
  const routes = [
    `https://client.example.test${expected}`,
    `https://foreign.example${expected}`,
    `//foreign.example${expected}`,
    expected.replace(course.id, "other-course"),
    expected.replace(assessment.id, "other-assessment"),
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
      client.beginImathasQuestionBackendLaunch(course.id, assessment.id, attempt.id),
      DecodeError,
      launchUrl,
    );
  }
});
