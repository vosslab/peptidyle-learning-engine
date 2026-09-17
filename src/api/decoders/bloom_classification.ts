// Strict browser decoder for the exact Revision-owned Bloom Classification.

import type { BloomClassificationEditNumber } from "../../../generated/api/BloomClassificationEditNumber";
import type { BloomClassificationCorrectionRequest } from "../../../generated/api/BloomClassificationCorrectionRequest";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type { BloomCognitiveProcess } from "../../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../../generated/api/BloomKnowledgeDimension";
import { DecodeError, decodeRecord, decodeString, decodeStringEnum } from "../decoder";
import { field, requireOnlyFields } from "./shared";

export const BLOOM_COGNITIVE_PROCESSES = [
  "Remember",
  "Understand",
  "Apply",
  "Analyze",
  "Evaluate",
  "Create",
] as const satisfies ReadonlyArray<BloomCognitiveProcess>;

export const BLOOM_KNOWLEDGE_DIMENSIONS = [
  "Factual Knowledge",
  "Conceptual Knowledge",
  "Procedural Knowledge",
  "Metacognitive Knowledge",
] as const satisfies ReadonlyArray<BloomKnowledgeDimension>;

export function isBloomCognitiveProcess(value: string): value is BloomCognitiveProcess {
  return BLOOM_COGNITIVE_PROCESSES.some((candidate) => candidate === value);
}

export function isBloomKnowledgeDimension(value: string): value is BloomKnowledgeDimension {
  return BLOOM_KNOWLEDGE_DIMENSIONS.some((candidate) => candidate === value);
}

function decodeClassificationEditNumber(
  value: unknown,
  path: string,
): BloomClassificationEditNumber {
  const editNumber = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(editNumber) || BigInt(editNumber) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL BIGINT decimal");
  }
  return editNumber;
}

/** Closed complete correction command; target identity remains owned by the route. */
export function decodeBloomClassificationCorrectionRequest(
  value: unknown,
  path: string,
): BloomClassificationCorrectionRequest {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "cognitiveProcess",
    "knowledgeDimension",
    "expectedClassificationEditNumber",
  ]);
  return {
    cognitiveProcess: decodeStringEnum(
      field(record, "cognitiveProcess", path),
      `${path}.cognitiveProcess`,
      BLOOM_COGNITIVE_PROCESSES,
    ),
    knowledgeDimension: decodeStringEnum(
      field(record, "knowledgeDimension", path),
      `${path}.knowledgeDimension`,
      BLOOM_KNOWLEDGE_DIMENSIONS,
    ),
    expectedClassificationEditNumber: decodeClassificationEditNumber(
      field(record, "expectedClassificationEditNumber", path),
      `${path}.expectedClassificationEditNumber`,
    ),
  };
}

/** ASVS 1.5.2/2.2.1: accepts only the closed pair and precision-safe Edit Number. */
export function decodeBloomClassificationView(
  value: unknown,
  path: string,
): BloomClassificationView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "cognitiveProcess",
    "knowledgeDimension",
    "classificationEditNumber",
  ]);
  return {
    cognitiveProcess: decodeStringEnum(
      field(record, "cognitiveProcess", path),
      `${path}.cognitiveProcess`,
      BLOOM_COGNITIVE_PROCESSES,
    ),
    knowledgeDimension: decodeStringEnum(
      field(record, "knowledgeDimension", path),
      `${path}.knowledgeDimension`,
      BLOOM_KNOWLEDGE_DIMENSIONS,
    ),
    classificationEditNumber: decodeClassificationEditNumber(
      field(record, "classificationEditNumber", path),
      `${path}.classificationEditNumber`,
    ),
  };
}
