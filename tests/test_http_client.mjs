// Focused strict same-origin HTTP client and decoder behavior tests.

import assert from "node:assert/strict";
import test from "node:test";

import { MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS } from "../generated/api/MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS.ts";
import { MAX_QUESTION_TITLE_UNICODE_SCALARS } from "../generated/api/MAX_QUESTION_TITLE_UNICODE_SCALARS.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeQuestionPage,
  decodeIssuedQuestionPresentation,
  decodeStudentQuestionAttemptView,
} from "../src/api/decoders.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch, jsonResponse } from "./http_client_test_support.mjs";

const courseInstanceId = "CI000001AE";
const assessmentId = "A000001AT";
const attemptId = "0198e000-0000-7000-8000-000000000030";

function questionSummary() {
  return {
    questionId: "7K3M-79QP",
    publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
    parentPublishedQuestionRevisionTuple: null,
    backend: "ple",
    questionFormat: "pleQuestionJson",
    questionType: "multipleChoice",
    capabilities: ["clientRendering", "serverGrading"],
    metadata: {
      questionTitle: "Peptide bond resonance",
      questionDescription: "Identify the bond with partial double-bond character.",
      tags: [],
      questionLicense: null,
      questionCitation: null,
      language: null,
    },
    authorship: { authors: [{ displayName: "Test Author", accountId: null }] },
    availability: { availability: "available" },
    publishedAt: 1_786_000_000_000,
    bloom: null,
  };
}

function questionAttemptView() {
  return {
    id: attemptId,
    issuedQuestion: "0198e000-0000-7000-8000-000000000040",
    finalizedResponse: null,
    state: "open",
    timing: { issuedAt: 1_786_000_001_100, deadline: null, finalizedAt: null },
    issuedCapability: "notApplicable",
  };
}

test("Question image URLs require and retain the exact Question Revision identity", () => {
  const client = createHttpApiClient({ basePath: "/live" });
  const questionImageUrl = client.questionImageUrl(
    { publishedQuestionId: "7K3M-79QP", revisionNumber: 2 },
    "00000000-0000-0000-0000-000000000001",
  );
  assert.equal(
    questionImageUrl,
    "/live/api/questions/7K3M-79QP/revisions/2/images/00000000-0000-0000-0000-000000000001",
  );
});

test("an issued iMathAS Question Backend Question Presentation accepts only its public marker", () => {
  const presentation = {
    publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
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
        imathasQuestionBackendBinding: { itemId: "secret" },
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
  const question = questionSummary();
  const page = { items: [question], nextCursor: null };
  assert.deepEqual(decodeQuestionPage(page), page);
  assert.throws(() => decodeQuestionPage({ ...page, answerKey: "secret" }), DecodeError);
  assert.throws(
    () => decodeQuestionPage({ ...page, items: Array.from({ length: 101 }, () => page.items[0]) }),
    DecodeError,
  );
});

test("Question Title and Question Description remain bounded at the strict Question Library boundary", () => {
  const summary = questionSummary();
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
  const attemptView = questionAttemptView();
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
  const attemptView = questionAttemptView();
  for (const field of ["reproduction", "question_seed", "generated_parameter_sha256"]) {
    assert.throws(
      () => decodeStudentQuestionAttemptView({ ...attemptView, [field]: "server-only" }),
      DecodeError,
      field,
    );
  }
});

test("iMathAS Question Backend launch returns its strict same-origin Assessment route", async () => {
  const launchUrl = `/api/course-instances/${courseInstanceId}/assessments/${assessmentId}/attempts/${attemptId}/imathas-question-backend/launch`;
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    jsonResponse({ launchUrl }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.deepEqual(
    await client.beginImathasQuestionBackendLaunch(courseInstanceId, assessmentId, attemptId),
    {
      launchUrl,
    },
  );
  assert.equal(requests[0]?.method, "POST");
  assert.equal(requests[0]?.url, `https://client.example.test${launchUrl}`);
  assert.equal(await requests[0]?.text(), "");
});

test("iMathAS Question Backend launch rejects noncanonical Assessment routes", async () => {
  const expected = `/api/course-instances/${courseInstanceId}/assessments/${assessmentId}/attempts/${attemptId}/imathas-question-backend/launch`;
  const routes = [
    `https://client.example.test${expected}`,
    `https://foreign.example${expected}`,
    `//foreign.example${expected}`,
    expected.replace(courseInstanceId, "other-course"),
    expected.replace(assessmentId, "other-assessment"),
    expected.replace(attemptId, "other-attempt"),
    "/api/health",
    `${expected}?token=secret`,
    `${expected}#fragment`,
  ];
  for (const launchUrl of routes) {
    const client = createHttpApiClient({
      fetch: async () => jsonResponse({ launchUrl }),
    });
    await assert.rejects(
      client.beginImathasQuestionBackendLaunch(courseInstanceId, assessmentId, attemptId),
      DecodeError,
      launchUrl,
    );
  }
});
