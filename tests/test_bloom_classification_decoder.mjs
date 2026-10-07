import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  BLOOM_COGNITIVE_PROCESS_MEANINGS,
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSION_MEANINGS,
  BLOOM_KNOWLEDGE_DIMENSIONS,
  decodeBloomClassificationView,
} from "../src/api/decoders/bloom_classification.ts";

const complete = {
  cognitiveProcess: "Evaluate",
  knowledgeDimension: "Procedural Knowledge",
};

test("Bloom teaching meanings cover every guide value and no other", () => {
  assert.deepEqual(Object.keys(BLOOM_COGNITIVE_PROCESS_MEANINGS), [...BLOOM_COGNITIVE_PROCESSES]);
  assert.deepEqual(Object.keys(BLOOM_KNOWLEDGE_DIMENSION_MEANINGS), [
    ...BLOOM_KNOWLEDGE_DIMENSIONS,
  ]);
  for (const meaning of Object.values(BLOOM_COGNITIVE_PROCESS_MEANINGS)) {
    assert.equal(meaning.trim(), meaning);
    assert.ok(meaning.length > 0);
  }
  for (const meaning of Object.values(BLOOM_KNOWLEDGE_DIMENSION_MEANINGS)) {
    assert.equal(meaning.trim(), meaning);
    assert.ok(meaning.length > 0);
  }
});

test("Bloom projection allows either dimension to be absent independently", () => {
  assert.deepEqual(decodeBloomClassificationView(complete, "bloom"), complete);
  assert.deepEqual(
    decodeBloomClassificationView({ cognitiveProcess: "Apply", knowledgeDimension: null }, "bloom"),
    { cognitiveProcess: "Apply", knowledgeDimension: null },
  );
  assert.deepEqual(
    decodeBloomClassificationView(
      { cognitiveProcess: null, knowledgeDimension: "Factual Knowledge" },
      "bloom",
    ),
    { cognitiveProcess: null, knowledgeDimension: "Factual Knowledge" },
  );
  assert.deepEqual(
    decodeBloomClassificationView({ cognitiveProcess: null, knowledgeDimension: null }, "bloom"),
    { cognitiveProcess: null, knowledgeDimension: null },
  );
});

test("Bloom projection rejects unknown, missing, old-counter, and extra fields", () => {
  for (const value of [
    { ...complete, cognitiveProcess: "Synthesize" },
    { ...complete, knowledgeDimension: "Strategic Knowledge" },
    { cognitiveProcess: "Apply" },
    { ...complete, classificationEditNumber: "2" },
    { ...complete, difficulty: "Hard" },
  ]) {
    assert.throws(() => decodeBloomClassificationView(value, "bloom"), DecodeError);
  }
});
