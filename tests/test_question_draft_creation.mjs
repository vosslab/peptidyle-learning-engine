import assert from "node:assert/strict";
import test from "node:test";

import {
  createQuestionDraftCreationClient,
  isAllowedWebworkPgPath,
  QUESTION_DRAFT_CREATION_FORMATS,
} from "../src/api/question_draft_creation.ts";

const createdDraft = {
  draftQuestionId: "018f0000-0000-7000-8000-000000000001",
  draftQuestionEditNumber: "1",
};

test("ordinary Draft creation offers Native JSON, PG, and PGML", async () => {
  const requests = [];
  const client = createQuestionDraftCreationClient({
    fetch: async (input, init) => {
      requests.push({ input, init });
      return new Response(JSON.stringify(createdDraft), {
        status: 201,
        headers: { "content-type": "application/json" },
      });
    },
  });
  const options = QUESTION_DRAFT_CREATION_FORMATS.map(({ format }) => format);
  assert.deepEqual(options, ["pleQuestionJson", "webworkPg", "webworkPgml"]);

  for (const format of options) {
    const webworkPgPath =
      format === "pleQuestionJson" ? undefined : `Library/Genetics/${format}.pg`;
    const created = await client.createDraft({ format, webworkPgPath });
    assert.deepEqual(created, createdDraft);
  }

  assert.equal(requests.length, 3);
  for (const [index, format] of options.entries()) {
    const request = requests[index];
    assert.equal(request.input, "/api/authoring/drafts");
    assert.equal(request.init.method, "POST");
    assert.equal(request.init.credentials, "same-origin");
    assert.equal(request.init.cache, "no-store");
    assert.equal(request.init.headers["content-type"], "application/json");
    const body = JSON.parse(request.init.body);
    assert.deepEqual(Object.keys(body).sort(), [
      "metadata",
      "questionBackend",
      "questionFormat",
      "source",
      "webworkPgPath",
    ]);
    assert.equal(body.questionFormat, format);
    assert.equal(body.questionBackend, format === "pleQuestionJson" ? "ple" : "webwork");
    assert.equal(
      body.webworkPgPath,
      format === "pleQuestionJson" ? null : `Library/Genetics/${format}.pg`,
    );
    assert.equal(Object.hasOwn(body, "questionType"), false);
    assert.equal(typeof body.source, "string");
    assert.equal(typeof body.metadata, "object");
    if (format === "pleQuestionJson") {
      assert.equal(JSON.parse(body.source).format, "pleQuestionJson");
      assert.equal(body.metadata.questionTitle, "");
    } else {
      assert.equal(body.source, "");
      assert.equal(body.metadata.questionTitle, "");
    }
  }
});

test("Native JSON creation sends blank and malformed source text unchanged", async () => {
  const requests = [];
  const client = createQuestionDraftCreationClient({
    fetch: async (input, init) => {
      requests.push({ input, init });
      return new Response(JSON.stringify(createdDraft), { status: 201 });
    },
  });
  const sources = ["", "{ malformed Native JSON"];

  for (const source of sources) {
    await client.createDraft({ format: "pleQuestionJson", source });
  }

  assert.deepEqual(
    requests.map(({ init }) => JSON.parse(init.body).source),
    sources,
  );
  for (const { init } of requests) {
    const body = JSON.parse(init.body);
    assert.equal(body.questionBackend, "ple");
    assert.equal(body.questionFormat, "pleQuestionJson");
    assert.equal(body.webworkPgPath, null);
  }
});

test("registered WebWork paths follow the relative path persistence contract", () => {
  assert.equal(isAllowedWebworkPgPath("Library/Genetics/question.pg"), true);
  assert.equal(isAllowedWebworkPgPath(""), false);
  assert.equal(isAllowedWebworkPgPath("/Library/question.pg"), false);
  assert.equal(isAllowedWebworkPgPath("Library/../question.pg"), false);
  assert.equal(isAllowedWebworkPgPath("Library//question.pg"), false);
  assert.equal(isAllowedWebworkPgPath("Library\\question.pg"), false);
  assert.equal(isAllowedWebworkPgPath("Library/\0question.pg"), false);
  assert.equal(isAllowedWebworkPgPath(`Library/${"x".repeat(1_020)}`), false);
});

test("invalid PG path stops before the ordinary creation request", async () => {
  let requestCount = 0;
  const client = createQuestionDraftCreationClient({
    fetch: async () => {
      requestCount += 1;
      return new Response(JSON.stringify(createdDraft), { status: 201 });
    },
  });

  await assert.rejects(
    client.createDraft({ format: "webworkPgml", webworkPgPath: "../outside.pg" }),
    /allowed relative WebWork PG path/u,
  );
  assert.equal(requestCount, 0);
});
