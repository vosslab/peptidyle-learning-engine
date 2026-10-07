import assert from "node:assert/strict";
import test from "node:test";

import { createPleQuestionGeneralFeedbackClient } from "../src/features/ple_question_json_authoring/question_general_feedback_client.ts";
import { createPleQuestionJsonClient } from "../src/features/ple_question_json_authoring/question_json_client.ts";
import { createPleQuestionJsonRepository } from "../src/features/ple_question_json_authoring/question_json_repository.ts";
import { PLE_QUESTION_JSON_MEDIA_TYPE } from "../src/features/ple_question_json_authoring/question_json_source.ts";
import { recordMetadata, source } from "./ple_question_json_authoring_support.mjs";

const draftQuestion = "0198e000-0000-7000-8000-000000000001";

function jsonResponse(value, status = 200, etag = '"1"') {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", etag },
  });
}

function noContent(etag = '"1"') {
  return new Response(null, { status: 204, headers: { etag } });
}

test("Draft metadata accepts empty title and description and preserves unset language as null", async () => {
  const emptyMetadata = {
    questionTitle: "",
    questionDescription: "",
    tags: [],
    questionLicense: null,
    questionCitation: null,
    language: null,
  };
  let savedMetadata = emptyMetadata;
  let readCount = 0;
  const client = createPleQuestionGeneralFeedbackClient({
    fetch: async (_path, init) => {
      if (init.method === "PUT") {
        savedMetadata = JSON.parse(init.body).metadata;
        return noContent('"2"');
      }
      assert.equal(init.method, "GET");
      readCount += 1;
      return jsonResponse(
        {
          metadata: savedMetadata,
          questionType: null,
          generalFeedback: null,
          hint: null,
          workedSolution: null,
          authors: [],
        },
        200,
        readCount === 1 ? '"1"' : '"2"',
      );
    },
  });
  const loaded = await client.load(draftQuestion);
  assert.deepEqual(loaded.metadata, emptyMetadata);
  assert.equal(loaded.questionType, null);
  await client.save(
    draftQuestion,
    loaded.metadata,
    { generalFeedback: null, hint: null, workedSolution: null },
    loaded.draftQuestionEditNumber,
  );
  const reloaded = await client.load(draftQuestion);
  assert.deepEqual(reloaded.metadata, emptyMetadata);
  assert.equal(reloaded.questionType, null);
});

test("Draft metadata GET and PUT round-trip record metadata with separately managed support text", async () => {
  let savedBody;
  const client = createPleQuestionGeneralFeedbackClient({
    fetch: async (path, init) => {
      assert.equal(path, "/api/authoring/drafts/0198e000-0000-7000-8000-000000000001/metadata");
      if (init.method === "GET") {
        return jsonResponse(
          {
            metadata: recordMetadata(),
            questionType: null,
            generalFeedback: "Keep the units.",
            hint: "Count alleles.",
            workedSolution: "Show the cross.",
            authors: ["Ada Lovelace"],
          },
          200,
          '"4"',
        );
      }
      assert.equal(init.method, "PUT");
      assert.equal(init.headers["if-match"], '"4"');
      assert.equal(init.headers["content-type"], "application/json");
      savedBody = JSON.parse(init.body);
      return noContent('"5"');
    },
  });
  assert.deepEqual(await client.load(draftQuestion), {
    metadata: recordMetadata(),
    questionType: null,
    generalFeedback: "Keep the units.",
    hint: "Count alleles.",
    workedSolution: "Show the cross.",
    authors: ["Ada Lovelace"],
    draftQuestionEditNumber: "4",
  });
  assert.deepEqual(
    await client.save(
      draftQuestion,
      recordMetadata(),
      { generalFeedback: "Keep the units.", hint: null, workedSolution: "Show the cross." },
      "4",
    ),
    { draftQuestionEditNumber: "5" },
  );
  assert.deepEqual(savedBody, {
    metadata: recordMetadata(),
    generalFeedback: "Keep the units.",
    hint: null,
    workedSolution: "Show the cross.",
  });
  const feedbackOnly = createPleQuestionGeneralFeedbackClient({
    fetch: async () => jsonResponse({ generalFeedback: "Keep the units." }),
  });
  await assert.rejects(() => feedbackOnly.load(draftQuestion));
  const extraField = createPleQuestionGeneralFeedbackClient({
    fetch: async () =>
      jsonResponse({
        metadata: recordMetadata(),
        questionType: null,
        generalFeedback: null,
        hint: null,
        workedSolution: null,
        source: "backend",
      }),
  });
  await assert.rejects(() => extraField.load(draftQuestion));
});

test("Draft metadata GET preserves optional Question citation text", async () => {
  const read = (questionCitation) => {
    const client = createPleQuestionGeneralFeedbackClient({
      fetch: async () =>
        jsonResponse({
          metadata: { ...recordMetadata(), questionCitation },
          questionType: null,
          generalFeedback: null,
          hint: null,
          workedSolution: null,
          authors: [],
        }),
    });
    return client.load(draftQuestion);
  };

  const uncited = await read(null);
  assert.equal(uncited.questionType, null);
  assert.equal(uncited.metadata.questionCitation, null);
  assert.equal(
    (await read("Author et al. (2024). https://example.org/paper")).metadata.questionCitation,
    "Author et al. (2024). https://example.org/paper",
  );
  assert.equal((await read("")).metadata.questionCitation, "");
  await assert.rejects(() => read({ citationText: "Author et al. (2024)" }));
});

test("metadata and Native source saves use separate payloads and share the Draft Edit Number", async () => {
  const requestBodies = [];
  let etag = 1;
  const fetch = async (path, init) => {
    if (init.method === "GET") {
      if (String(path).endsWith("/source")) {
        return new Response(JSON.stringify(source()), {
          headers: { "content-type": PLE_QUESTION_JSON_MEDIA_TYPE, etag: `"${etag}"` },
        });
      }
      return jsonResponse(
        {
          metadata: recordMetadata(),
          questionType: null,
          generalFeedback: null,
          hint: null,
          workedSolution: null,
          authors: [],
        },
        200,
        `"${etag}"`,
      );
    }
    requestBodies.push({
      path: String(path),
      body: JSON.parse(init.body),
      ifMatch: init.headers["if-match"],
    });
    etag += 1;
    return noContent(`"${etag}"`);
  };
  const sourceClient = createPleQuestionJsonClient({ fetch });
  const metadataClient = createPleQuestionGeneralFeedbackClient({ fetch });
  const repository = createPleQuestionJsonRepository(sourceClient);
  await repository.load(draftQuestion);
  const sourceSave = await repository.save(draftQuestion, source());
  assert.equal(sourceSave.draftQuestionEditNumber, "2");
  assert.equal(requestBodies[0].path.endsWith("/source"), true);
  assert.deepEqual(Object.keys(requestBodies[0].body).sort(), [
    "authorScript",
    "externalResources",
    "feedback",
    "format",
    "prompt",
    "questionHint",
    "response",
  ]);
  const metadataSave = await metadataClient.save(
    draftQuestion,
    { ...recordMetadata(), questionTitle: "Updated title", language: null },
    { generalFeedback: null, hint: null, workedSolution: null },
    sourceSave.draftQuestionEditNumber,
  );
  repository.synchronizeDraftQuestionEditNumber(
    draftQuestion,
    metadataSave.draftQuestionEditNumber,
  );
  assert.equal(metadataSave.draftQuestionEditNumber, "3");
  assert.equal(requestBodies[1].path.endsWith("/metadata"), true);
  assert.equal(requestBodies[1].ifMatch, '"2"');
  assert.deepEqual(requestBodies[1].body.metadata, {
    ...recordMetadata(),
    questionTitle: "Updated title",
    language: null,
  });
  assert.equal(Object.hasOwn(requestBodies[1].body, "source"), false);
  assert.equal(Object.hasOwn(requestBodies[0].body, "metadata"), false);
});
