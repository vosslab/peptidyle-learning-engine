// Strict browser decoder for the exact Revision-owned Bloom Classification.

import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type { BloomCognitiveProcess } from "../../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../../generated/api/BloomKnowledgeDimension";
import { decodeNullable, decodeRecord, decodeStringEnum } from "../decoder";
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

/** ASVS 1.5.2/2.2.1: decodes two independently nullable Bloom dimensions. */
export function decodeBloomClassificationView(
  value: unknown,
  path: string,
): BloomClassificationView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["cognitiveProcess", "knowledgeDimension"]);
  return {
    cognitiveProcess: decodeNullable(
      field(record, "cognitiveProcess", path),
      `${path}.cognitiveProcess`,
      (value, valuePath) => decodeStringEnum(value, valuePath, BLOOM_COGNITIVE_PROCESSES),
    ),
    knowledgeDimension: decodeNullable(
      field(record, "knowledgeDimension", path),
      `${path}.knowledgeDimension`,
      (value, valuePath) => decodeStringEnum(value, valuePath, BLOOM_KNOWLEDGE_DIMENSIONS),
    ),
  };
}
