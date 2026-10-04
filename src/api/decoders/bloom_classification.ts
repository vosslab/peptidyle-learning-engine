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

/** Guide meanings for the work a Question requires. AI assignment stays deferred. */
export const BLOOM_COGNITIVE_PROCESS_MEANINGS = {
  Remember: "Retrieve relevant knowledge",
  Understand: "Construct meaning from presented or recalled knowledge",
  Apply: "Use a procedure in a situation",
  Analyze: "Separate material into parts and relate those parts",
  Evaluate: "Make a judgment using stated or appropriate criteria",
  Create: "Assemble elements into a coherent or functional new whole",
} as const satisfies Record<BloomCognitiveProcess, string>;

/** Guide meanings for the knowledge a Question assesses. */
export const BLOOM_KNOWLEDGE_DIMENSION_MEANINGS = {
  "Factual Knowledge": "Terminology, specific details, and discrete elements",
  "Conceptual Knowledge": "Categories, principles, theories, models, and systems",
  "Procedural Knowledge": "Skills, algorithms, techniques, methods, and their use",
  "Metacognitive Knowledge": "Strategies and awareness of one's own cognition",
} as const satisfies Record<BloomKnowledgeDimension, string>;

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
