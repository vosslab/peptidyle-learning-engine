import assert from "node:assert/strict";
import test from "node:test";

import { createBloomClassificationCorrectionClient } from "../src/api/http_client/bloom_classification.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  ApiProtocolError,
  BloomClassificationConflictError,
} from "../src/api/http_client/error.ts";

const question = { questionId: "7K3M-79QP", revisionNumber: 3 };
const poolId = "3S8B-24DZ";
const request = {
  cognitiveProcess: "Analyze",
  knowledgeDimension: "Conceptual Knowledge",
  expectedClassificationEditNumber: "9223372036854775807",
};
const bloom = {
  cognitiveProcess: "Analyze",
  knowledgeDimension: "Conceptual Knowledge",
  classificationEditNumber: "9223372036854775807",
};

function jsonResponse(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "cache-control": "private, no-store",
      "content-type": "application/json",
    },
  });
}

test("Bloom correction posts one complete exact-Revision CAS command", async () => {
  const calls = [];
  const client = createBloomClassificationCorrectionClient(async (input, init) => {
    calls.push({ input, init });
    if (String(input).includes("/question-pools/")) {
      return jsonResponse({ questionPoolId: poolId, bloom });
    }
    return jsonResponse({ questionRevisionTuple: question, bloom });
  }, "/ple");

  assert.deepEqual(await client.correctQuestionBloom(question, request), {
    questionRevisionTuple: question,
    bloom,
  });
  assert.deepEqual(await client.correctQuestionPoolBloom(poolId, request), {
    questionPoolId: poolId,
    bloom,
  });
  assert.equal(calls.length, 2);
  assert.equal(String(calls[0].input), "/ple/api/questions/by-id/7K3M-79QP/revisions/3/bloom");
  assert.equal(String(calls[1].input), "/ple/api/question-pools/3S8B-24DZ/bloom");
  for (const call of calls) {
    assert.equal(call.init.method, "POST");
    assert.equal(call.init.credentials, "same-origin");
    assert.equal(call.init.cache, "no-store");
    assert.equal(call.init.headers["content-type"], "application/json");
    assert.deepEqual(JSON.parse(call.init.body), request);
  }
});

test("Bloom correction surfaces stale state once and never retries", async () => {
  let calls = 0;
  const client = createBloomClassificationCorrectionClient(async () => {
    calls += 1;
    return jsonResponse({ error: "changed" }, 412);
  }, "");

  await assert.rejects(
    () => client.correctQuestionBloom(question, request),
    BloomClassificationConflictError,
  );
  assert.equal(calls, 1);
});

test("Bloom correction rejects response target drift and open receipts", async () => {
  const responses = [
    { questionRevisionTuple: { ...question, revisionNumber: 4 }, bloom },
    { questionRevisionTuple: question, bloom, actor: "Instructor" },
  ];
  const client = createBloomClassificationCorrectionClient(
    async () => jsonResponse(responses.shift()),
    "",
  );

  await assert.rejects(() => client.correctQuestionBloom(question, request), ApiProtocolError);
  await assert.rejects(() => client.correctQuestionBloom(question, request), DecodeError);
});
