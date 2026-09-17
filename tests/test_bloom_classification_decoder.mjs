import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeBloomClassificationCorrectionRequest,
  decodeBloomClassificationView,
} from "../src/api/decoders/bloom_classification.ts";
import {
  decodeQuestionBloomCorrectionReceipt,
  decodeQuestionPoolBloomCorrectionReceipt,
} from "../src/api/decoders/bloom_correction.ts";

const bloom = {
  cognitiveProcess: "Evaluate",
  knowledgeDimension: "Procedural Knowledge",
  classificationEditNumber: "9223372036854775807",
};

test("Bloom Classification decoder preserves the closed pair and exact Edit Number", () => {
  assert.deepEqual(decodeBloomClassificationView(bloom, "bloom"), bloom);
});

test("Bloom Classification decoder rejects missing, unknown, or unsafe values", () => {
  for (const value of [
    { ...bloom, cognitiveProcess: undefined },
    { ...bloom, cognitiveProcess: "Synthesize" },
    { ...bloom, knowledgeDimension: "Strategic Knowledge" },
    { ...bloom, classificationEditNumber: 1 },
    { ...bloom, classificationEditNumber: "01" },
    { ...bloom, classificationEditNumber: "9223372036854775808" },
    { ...bloom, difficulty: "Hard" },
  ]) {
    assert.throws(() => decodeBloomClassificationView(value, "bloom"), DecodeError);
  }
});

test("Bloom correction request is complete, closed, and precision-safe", () => {
  const request = {
    cognitiveProcess: "Analyze",
    knowledgeDimension: "Conceptual Knowledge",
    expectedClassificationEditNumber: "9223372036854775807",
  };
  assert.deepEqual(decodeBloomClassificationCorrectionRequest(request, "request"), request);
  for (const value of [
    { ...request, cognitiveProcess: undefined },
    { ...request, expectedClassificationEditNumber: "0" },
    { ...request, questionOwner: "caller-authority" },
  ]) {
    assert.throws(() => decodeBloomClassificationCorrectionRequest(value, "request"), DecodeError);
  }
});

test("Bloom correction receipts are exact-target closed DTOs", () => {
  const question = {
    questionRevision: { questionId: "7K3M-79QP", revisionNumber: 3 },
    bloom,
  };
  const pool = {
    questionPoolRevision: { questionPoolId: "3S8B-24DZ", revisionNumber: 5 },
    bloom,
  };
  assert.deepEqual(decodeQuestionBloomCorrectionReceipt(question, "response"), question);
  assert.deepEqual(decodeQuestionPoolBloomCorrectionReceipt(pool, "response"), pool);
  assert.throws(
    () => decodeQuestionBloomCorrectionReceipt({ ...question, changed: true }, "response"),
    DecodeError,
  );
  assert.throws(
    () => decodeQuestionPoolBloomCorrectionReceipt({ ...pool, actor: "Instructor" }, "response"),
    DecodeError,
  );
});
