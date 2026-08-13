// Issued-question transport secrecy checks for the strict HTTP client.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { createFixtureFetch, jsonResponse } from "./http_client_test_support.mjs";

test("issued-question transport rejects a response that carries a server-only field", async () => {
  const { fixtureFetch } = createFixtureFetch();
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const response = await fixtureFetch(input, init);
      if (input.toString().includes("/question")) {
        const value = await response.json();
        value.grading = { mode: "allOrNothing", points: 1 };
        return jsonResponse(value);
      }
      return response;
    },
  });
  await assert.rejects(
    client.getIssuedQuestion(publishedProblemFixture.attempts.at(-1).id),
    (error) =>
      error instanceof DecodeError &&
      error.message === "response.grading must be a field allowed by this response contract",
  );
});

test("issued-question transport rejects answer material at every nested envelope record", async () => {
  const hostileEnvelopes = [
    {
      name: "prompt text",
      mutate: (envelope) => {
        envelope.prompt[0].solution = "carbonyl";
      },
      path: "response.prompt[0].solution",
    },
    {
      name: "math prompt",
      mutate: (envelope) => {
        envelope.prompt = [
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
      mutate: (envelope) => {
        envelope.prompt = [
          { kind: "code", language: "text", source: "x", checker: "private-checker" },
        ];
      },
      path: "response.prompt[0].checker",
    },
    {
      name: "table prompt",
      mutate: (envelope) => {
        envelope.prompt = [
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
      name: "image asset reference",
      mutate: (envelope) => {
        envelope.prompt = [
          {
            kind: "image",
            asset: {
              asset: "0198e000-0000-7000-8000-000000000010",
              checksum: "0".repeat(64),
              objectKey: "private/answer-key",
            },
            description: "A diagram.",
          },
        ];
      },
      path: "response.prompt[0].asset.objectKey",
    },
    {
      name: "multiple-choice response",
      mutate: (envelope) => {
        envelope.response.correctChoiceId = "carbonyl";
      },
      path: "response.response.correctChoiceId",
    },
    {
      name: "multiple-choice choice",
      mutate: (envelope) => {
        envelope.response.choices[0].answer = true;
      },
      path: "response.response.choices[0].answer",
    },
    {
      name: "multiple-choice choice content",
      mutate: (envelope) => {
        envelope.response.choices[0].body[0].solution = "private";
      },
      path: "response.response.choices[0].body[0].solution",
    },
    {
      name: "selection rule",
      mutate: (envelope) => {
        envelope.response.grading = "allOrNothing";
      },
      path: "response.response.grading",
    },
    {
      name: "numerical response",
      mutate: (envelope) => {
        envelope.response = {
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
      mutate: (envelope) => {
        envelope.response = {
          kind: "fillIn",
          maxCharacters: 20,
          checker: "private-checker",
        };
      },
      path: "response.response.checker",
    },
    {
      name: "ordering item",
      mutate: (envelope) => {
        envelope.response = {
          kind: "ordering",
          items: [{ id: "first", body: [], solution: 0 }],
        };
      },
      path: "response.response.items[0].solution",
    },
  ];

  for (const hostile of hostileEnvelopes) {
    const { fixtureFetch } = createFixtureFetch();
    const client = createHttpApiClient({
      fetch: async (input, init) => {
        const response = await fixtureFetch(input, init);
        if (input.toString().includes("/question")) {
          const value = await response.json();
          hostile.mutate(value);
          return jsonResponse(value);
        }
        return response;
      },
    });
    await assert.rejects(
      client.getIssuedQuestion(publishedProblemFixture.attempts.at(-1).id),
      (error) =>
        error instanceof DecodeError &&
        error.message === `${hostile.path} must be a field allowed by this response contract`,
      hostile.name,
    );
  }
});
