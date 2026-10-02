// assessment_question_id_batch.ts - paste several Question IDs onto one Assessment.

import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import { normalizeHumanEnteredQuestionId } from "../../question_id";

/** An Assessment delivers at most this many Questions, so one paste stops there. */
export const QUESTION_ID_BATCH_LIMIT = 250;

export interface QuestionIdPin {
  readonly questionId: string;
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly questionTitle: string;
  readonly description: string;
  readonly bloom: BloomClassificationView | null;
}

export type QuestionIdResolution = QuestionIdPin | "missing" | "unavailable";

export type QuestionIdBatchResolution =
  | { readonly kind: "empty" }
  | { readonly kind: "invalid"; readonly invalidTokens: ReadonlyArray<string> }
  | { readonly kind: "overCapacity"; readonly count: number }
  | {
      readonly kind: "resolved";
      readonly pins: ReadonlyArray<QuestionIdPin>;
      readonly missing: ReadonlyArray<string>;
      readonly unavailable: ReadonlyArray<string>;
    };

/** Splits a paste into canonical Question IDs and tokens that are not Question IDs. */
export function parseQuestionIdBatch(value: string): {
  readonly canonicalIds: ReadonlyArray<string>;
  readonly invalidTokens: ReadonlyArray<string>;
} {
  const canonicalIds: string[] = [];
  const invalidTokens: string[] = [];
  const seen = new Set<string>();
  for (const token of value.split(/[\s,;]+/u)) {
    if (token.length === 0) continue;
    const canonical = normalizeHumanEnteredQuestionId(token);
    if (canonical === null) {
      invalidTokens.push(token);
      continue;
    }
    if (seen.has(canonical)) continue;
    seen.add(canonical);
    canonicalIds.push(canonical);
  }
  return { canonicalIds, invalidTokens };
}

/**
 * Resolves a paste only after every token is a canonical Question ID and the
 * paste fits the remaining Assessment capacity.
 */
export async function resolveQuestionIdBatch(
  value: string,
  capacity: number,
  resolve: (questionId: string) => Promise<QuestionIdResolution>,
): Promise<QuestionIdBatchResolution> {
  const parsed = parseQuestionIdBatch(value);
  if (parsed.invalidTokens.length > 0) {
    return { kind: "invalid", invalidTokens: parsed.invalidTokens };
  }
  if (parsed.canonicalIds.length === 0) return { kind: "empty" };
  if (
    parsed.canonicalIds.length > capacity ||
    parsed.canonicalIds.length > QUESTION_ID_BATCH_LIMIT
  ) {
    return { kind: "overCapacity", count: parsed.canonicalIds.length };
  }
  const pins: QuestionIdPin[] = [];
  const missing: string[] = [];
  const unavailable: string[] = [];
  for (const questionId of parsed.canonicalIds) {
    const resolved = await resolve(questionId);
    if (resolved === "missing") missing.push(questionId);
    else if (resolved === "unavailable") unavailable.push(questionId);
    else pins.push(resolved);
  }
  return { kind: "resolved", pins, missing, unavailable };
}
