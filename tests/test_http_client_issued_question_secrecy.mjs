// Issued-question transport secrecy checks for the strict HTTP client.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch, jsonResponse } from "./http_client_test_support.mjs";

const courseInstanceId = "CI000001AE";
const assessmentId = "A000001AT";
const questionId = "7K3M-79QP";
const questionTuple = { publishedQuestionId: questionId, revisionNumber: 1 };

function questionAttempt() {
  return {
    id: "0198e000-0000-7000-8000-000000000033",
    issuedQuestion: "0198e000-0000-7000-8000-000000000043",
    finalizedResponse: null,
    state: "open",
    timing: { issuedAt: 1_786_000_004_100, deadline: null, finalizedAt: null },
    issuedCapability: "pleQuestionJsonPresentation",
    assessmentScoringState: "current",
    questionPoolSelectionPosition: null,
  };
}

function issuedQuestionPresentation(attempt) {
  return {
    publishedQuestionRevisionTuple: questionTuple,
    presentationNonce: attempt.id.replaceAll("-", "").slice(-32),
    questionTitle: "Peptide bond resonance",
    prompt: [{ kind: "text", markdown: "Which bond has restricted rotation?" }],
    response: {
      kind: "singleChoice",
      choices: [
        { id: "0001", body: [{ kind: "text", markdown: "Carbon-to-nitrogen" }] },
        { id: "0002", body: [{ kind: "text", markdown: "Carbon-to-oxygen" }] },
      ],
    },
  };
}

function clientWithIssuedQuestion(mutator) {
  const attempt = questionAttempt();
  const { recordingFetch, requests } = createRecordingFetch(async (request) => {
    if (new URL(request.url).pathname.endsWith("/question")) {
      const issued = issuedQuestionPresentation(attempt);
      mutator(issued);
      return jsonResponse(issued);
    }
    return jsonResponse(attempt);
  });
  return {
    attempt,
    client: createHttpApiClient({ fetch: recordingFetch }),
    requests,
  };
}

test("issued-question transport uses the explicit nested course and Assessment route", async () => {
  const { attempt, client, requests } = clientWithIssuedQuestion(() => {});
  await client.getIssuedQuestion(courseInstanceId, assessmentId, attempt.id);
  assert.equal(
    requests[1]?.url,
    `https://client.example.test/api/course-instances/${courseInstanceId}/assessments/${assessmentId}/attempts/${attempt.id}/question`,
  );
});

test("issued-question transport preserves a concealed nested-route 404 without a legacy retry", async () => {
  const attempt = questionAttempt();
  const { recordingFetch, requests } = createRecordingFetch(async (request) => {
    if (new URL(request.url).pathname.endsWith("/question")) return jsonResponse({}, 404);
    return jsonResponse(attempt);
  });
  const client = createHttpApiClient({ fetch: recordingFetch });
  await assert.rejects(
    client.getIssuedQuestion(courseInstanceId, assessmentId, attempt.id),
    (error) => error instanceof ApiRequestError && error.status === 404,
  );
  assert.equal(requests.length, 2);
  assert.match(
    requests[1]?.url ?? "",
    /\/api\/course-instances\/.*\/assessments\/.*\/attempts\/.*\/question$/u,
  );
});

test("issued-question transport rejects a response that carries a server-only field", async () => {
  const { attempt, client } = clientWithIssuedQuestion((issued) => {
    issued.grading = { mode: "allOrNothing", points: 1 };
  });
  await assert.rejects(
    client.getIssuedQuestion(courseInstanceId, assessmentId, attempt.id),
    (error) =>
      error instanceof DecodeError &&
      error.message === "response.grading must be a field allowed by this response contract",
  );
});

test("issued-question transport rejects server-only data from a Question Presentation", async () => {
  const hostilePresentations = [
    {
      name: "prompt text",
      mutate: (presentation) => {
        presentation.prompt[0].solution = "carbonyl";
      },
      path: "response.prompt[0].solution",
    },
    {
      name: "math prompt",
      mutate: (presentation) => {
        presentation.prompt = [
          {
            kind: "math",
            latex: "x",
            description: "A variable.",
            grading: { answer: "x" },
          },
        ];
      },
      path: "response.prompt[0].grading",
    },
    {
      name: "code prompt",
      mutate: (presentation) => {
        presentation.prompt = [
          { kind: "code", language: "text", source: "x", checker: "private-checker" },
        ];
      },
      path: "response.prompt[0].checker",
    },
    {
      name: "table prompt",
      mutate: (presentation) => {
        presentation.prompt = [
          {
            kind: "table",
            headers: ["A"],
            rows: [["x"]],
            description: "A table.",
            arbitrary: true,
          },
        ];
      },
      path: "response.prompt[0].arbitrary",
    },
    {
      name: "image asset Tuple",
      mutate: (presentation) => {
        presentation.prompt = [
          {
            kind: "image",
            questionImageAssetTuple: {
              questionImageAssetId: "0198e000-0000-7000-8000-000000000010",
              checksum: "0".repeat(64),
              objectKey: "private/answer-key",
            },
            description: "A diagram.",
          },
        ];
      },
      path: "response.prompt[0].questionImageAssetTuple.objectKey",
    },
    {
      name: "multiple-choice response",
      mutate: (presentation) => {
        presentation.response.correctChoiceId = "carbonyl";
      },
      path: "response.response.correctChoiceId",
    },
    {
      name: "multiple-choice choice",
      mutate: (presentation) => {
        presentation.response.choices[0].answer = true;
      },
      path: "response.response.choices[0].answer",
    },
    {
      name: "multiple-choice choice content",
      mutate: (presentation) => {
        presentation.response.choices[0].body[0].solution = "private";
      },
      path: "response.response.choices[0].body[0].solution",
    },
    {
      name: "selection rule",
      mutate: (presentation) => {
        presentation.response.grading = "allOrNothing";
      },
      path: "response.response.grading",
    },
    {
      name: "numerical response",
      mutate: (presentation) => {
        presentation.response = {
          kind: "numerical",
          maxCharacters: 128,
          displayedUnit: null,
          answer: 4,
        };
      },
      path: "response.response.answer",
    },
    {
      name: "fill-in response",
      mutate: (presentation) => {
        presentation.response = {
          kind: "fillIn",
          maxCharacters: 20,
          checker: "private-checker",
        };
      },
      path: "response.response.checker",
    },
    {
      name: "ordering item",
      mutate: (presentation) => {
        presentation.response = {
          kind: "ordering",
          items: [{ id: "first", body: [], solution: 0 }],
        };
      },
      path: "response.response.items[0].solution",
    },
  ];

  for (const hostile of hostilePresentations) {
    const { attempt, client } = clientWithIssuedQuestion(hostile.mutate);
    await assert.rejects(
      client.getIssuedQuestion(courseInstanceId, assessmentId, attempt.id),
      (error) =>
        error instanceof DecodeError &&
        error.message === `${hostile.path} must be a field allowed by this response contract`,
      hostile.name,
    );
  }
});
