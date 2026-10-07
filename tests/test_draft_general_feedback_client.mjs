import assert from "node:assert/strict";
import test from "node:test";

import {
  createPleQuestionGeneralFeedbackClient,
  questionTypeUpdateForBackend,
} from "../src/features/ple_question_json_authoring/question_general_feedback_client.ts";
import {
  canPublishWebworkDraft,
  createDraftQuestionMutationQueue,
  isDraftSnapshotSaved,
} from "../src/features/ple_question_json_authoring/question_publication_review_model.ts";

const draftQuestion = "0190a0b0-c0d0-7e10-8a20-304050607080";
const metadata = {
  questionTitle: "Amino acid charge",
  questionDescription: "Classify the side chain at the stated pH.",
  tags: ["amino acids", "charge"],
  questionLicense: "CC-BY-4.0",
  questionCitation: null,
  language: "en",
};

function metadataResponse(questionType = null, editNumber = "8") {
  return new Response(
    JSON.stringify({
      metadata,
      questionType,
      generalFeedback: null,
      hint: null,
      workedSolution: null,
      authors: [],
    }),
    {
      status: 200,
      headers: {
        "content-type": "application/json",
        etag: `"${editNumber}"`,
      },
    },
  );
}

function savedResponse(editNumber) {
  return new Response(null, {
    status: 204,
    headers: { etag: `"${editNumber}"` },
  });
}

test("metadata read decodes nullable and selected WebWork Question Types", async () => {
  for (const questionType of [null, "multipleChoice", "hotspot"]) {
    const client = createPleQuestionGeneralFeedbackClient({
      fetch: async () => metadataResponse(questionType),
    });
    const loaded = await client.load(draftQuestion);

    assert.equal(loaded.questionType, questionType);
    assert.equal(loaded.draftQuestionEditNumber, "8");
    assert.deepEqual(loaded.metadata, metadata);
    assert.deepEqual(loaded.authors, []);
  }

  const invalid = createPleQuestionGeneralFeedbackClient({
    fetch: async () => metadataResponse("unsupported"),
  });
  await assert.rejects(invalid.load(draftQuestion), /must contain/u);
});

test("metadata save preserves an omitted Type and advances the shared Edit Number", async () => {
  const requests = [];
  const client = createPleQuestionGeneralFeedbackClient({
    fetch: async (input, init) => {
      requests.push(new Request(new URL(String(input), "https://ple.example"), init));
      return savedResponse("9");
    },
  });

  const saved = await client.save(
    draftQuestion,
    metadata,
    { generalFeedback: null, hint: "Count alleles.", workedSolution: null },
    "8",
  );
  const request = requests[0];
  const body = await request.json();

  assert.equal(saved.draftQuestionEditNumber, "9");
  assert.equal(request.method, "PUT");
  assert.equal(new URL(request.url).pathname, `/api/authoring/drafts/${draftQuestion}/metadata`);
  assert.equal(request.headers.get("if-match"), '"8"');
  assert.equal(Object.hasOwn(body, "questionType"), false);
  assert.deepEqual(body.metadata, metadata);
  assert.equal(body.hint, "Count alleles.");
});

test("metadata save encodes explicit Type clearing and manual WebWork Type values", async () => {
  const requests = [];
  const client = createPleQuestionGeneralFeedbackClient({
    fetch: async (input, init) => {
      requests.push(new Request(new URL(String(input), "https://ple.example"), init));
      return savedResponse(String(requests.length + 8));
    },
  });
  const support = { generalFeedback: null, hint: null, workedSolution: null };

  await client.save(draftQuestion, metadata, support, "9", null);
  await client.save(draftQuestion, metadata, support, "10", "matching");

  const clearBody = await requests[0].json();
  const selectedBody = await requests[1].json();
  assert.equal(Object.hasOwn(clearBody, "questionType"), true);
  assert.equal(clearBody.questionType, null);
  assert.equal(selectedBody.questionType, "matching");
  assert.equal(requests[0].headers.get("if-match"), '"9"');
  assert.equal(requests[1].headers.get("if-match"), '"10"');
});

test("Native saves omit manual Type while WebWork may set or clear it", () => {
  assert.equal(questionTypeUpdateForBackend("ple", "hotspot"), undefined);
  assert.equal(questionTypeUpdateForBackend("webwork", null), null);
  assert.equal(questionTypeUpdateForBackend("webwork", "numeric"), "numeric");
});

test("Saved and first-publication gates require source, metadata, and WebWork Type", () => {
  assert.equal(isDraftSnapshotSaved(true, false, false), true);
  assert.equal(isDraftSnapshotSaved(false, false, false), false);
  assert.equal(isDraftSnapshotSaved(true, true, false), false);
  assert.equal(isDraftSnapshotSaved(true, false, true), false);
  assert.equal(canPublishWebworkDraft(false, "matching"), false);
  assert.equal(canPublishWebworkDraft(true, null), false);
  assert.equal(canPublishWebworkDraft(true, "matching"), true);
});

test("source and metadata saves share one serialized Draft Edit Number", async () => {
  const serialize = createDraftQuestionMutationQueue();
  const events = [];
  let editNumber = "8";
  let finishSourceSave;
  const sourceSaveGate = new Promise((resolve) => {
    finishSourceSave = resolve;
  });

  const sourceSave = serialize(async () => {
    events.push(`source request ${editNumber}`);
    await sourceSaveGate;
    editNumber = "9";
    events.push(`source acknowledgement ${editNumber}`);
  });
  const metadataSave = serialize(async () => {
    events.push(`metadata request ${editNumber}`);
  });

  await Promise.resolve();
  assert.deepEqual(events, ["source request 8"]);
  finishSourceSave();
  await Promise.all([sourceSave, metadataSave]);

  assert.deepEqual(events, ["source request 8", "source acknowledgement 9", "metadata request 9"]);
});
