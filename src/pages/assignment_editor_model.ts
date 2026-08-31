// assignment_editor_model.ts - Questions-page browser state for QID-only assignment content.

import type { FixedQuestionAssignmentEntrySummary } from "../../generated/api/FixedQuestionAssignmentEntrySummary";
import type { QuestionPoolAssignmentEntrySummary } from "../../generated/api/QuestionPoolAssignmentEntrySummary";
import type { AssignmentEntrySummary } from "../../generated/api/AssignmentEntrySummary";
import type { Capability } from "../../generated/api/Capability";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import type { AssignmentActivityRules } from "../../generated/api/AssignmentActivityRules";
import { MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL } from "../../generated/api/MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES } from "../../generated/api/MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { QuestionId } from "../../generated/api/QuestionId";
import type {
  AssignmentCapabilityViolation,
  AssignmentContentInput,
  AssignmentEditorEntryInput,
  AssignmentEditorDetail,
} from "../api/contracts";
import { normalizeQuestionIdSyntax } from "../question_id";

export interface AssignmentQuestionRow {
  readonly questionId: QuestionId;
  readonly title: string;
  readonly backend: QuestionBackend;
}

export type AssignmentEditorFixedQuestionEntry = FixedQuestionAssignmentEntrySummary & {
  readonly kind: "fixedQuestion";
};

/**
 * Browser-owned pool state deliberately names only public Question IDs.
 * The server mints and preserves Question Pool entry/candidate identities when it saves this definition.
 */
export interface AssignmentEditorQuestionPoolEntry {
  readonly kind: "questionPool";
  readonly id?: string;
  readonly availability: "available" | "retired";
  readonly scoringRule: "normal" | "fullCredit" | "extraCredit" | "excluded";
  readonly candidates: ReadonlyArray<AssignmentQuestionRow>;
  readonly drawCount: number;
  readonly pointsPerItem: string;
  readonly selectionRule: {
    readonly algorithm: "v1";
    readonly ordering: "candidateOrder" | "randomized";
  };
}

export type AssignmentEditorEntry =
  AssignmentEditorFixedQuestionEntry | AssignmentEditorQuestionPoolEntry;

export interface AssignmentEditorDraft {
  readonly id: string;
  readonly courseId: string;
  readonly title: string;
  /** One ordered definition, shared by fixed questions and Question Pools. */
  readonly entries: ReadonlyArray<AssignmentEditorEntry>;
  readonly policies: AssignmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
  readonly revision: string;
}

export function assignmentEditorDraftFrom(detail: AssignmentEditorDetail): AssignmentEditorDraft {
  return {
    id: detail.id,
    courseId: detail.courseId,
    title: detail.title,
    entries: detail.entries.map(assignmentEditorEntryFrom),
    policies: detail.policies,
    studentFeedbackReleaseRule: detail.studentFeedbackReleaseRule,
    revision: detail.revision,
  };
}

export function fixedQuestionEntry(
  entry: FixedQuestionAssignmentEntrySummary,
): AssignmentEditorFixedQuestionEntry {
  return { ...entry, kind: "fixedQuestion" };
}

export function questionPoolEntry(
  entry: QuestionPoolAssignmentEntrySummary,
): AssignmentEditorQuestionPoolEntry {
  if (entry.selectionRule.algorithm !== "v1") {
    throw new Error("This assignment uses an unsupported pool draw algorithm.");
  }
  return {
    kind: "questionPool",
    id: entry.id,
    availability: entry.availability,
    scoringRule: entry.scoringRule,
    candidates: entry.candidates.map((candidate) => ({
      questionId: candidate.questionId,
      title: candidate.title,
      backend: candidate.backend,
    })),
    drawCount: entry.drawCount,
    pointsPerItem: entry.pointsPerItem,
    selectionRule: entry.selectionRule,
  };
}

export function assignmentEditorEntryFrom(entry: AssignmentEntrySummary): AssignmentEditorEntry {
  if (entry.kind === "fixedQuestion") return fixedQuestionEntry(entry);
  return questionPoolEntry(entry);
}

export function fixedEntries(
  draft: AssignmentEditorDraft,
): ReadonlyArray<AssignmentEditorFixedQuestionEntry> {
  return draft.entries.filter(
    (entry): entry is AssignmentEditorFixedQuestionEntry => entry.kind === "fixedQuestion",
  );
}

export function moveAssignmentEntry(
  draft: AssignmentEditorDraft,
  entryIndex: number,
  direction: -1 | 1,
): AssignmentEditorDraft {
  const nextIndex = entryIndex + direction;
  if (entryIndex < 0 || nextIndex < 0 || nextIndex >= draft.entries.length) return draft;
  const entries = [...draft.entries];
  const current = entries[entryIndex];
  const adjacent = entries[nextIndex];
  if (current === undefined || adjacent === undefined) return draft;
  entries[entryIndex] = adjacent;
  entries[nextIndex] = current;
  return { ...draft, entries };
}

export function appendFixedEntries(
  draft: AssignmentEditorDraft,
  rows: ReadonlyArray<AssignmentQuestionRow>,
): AssignmentEditorDraft {
  const known = new Set(fixedEntries(draft).map((item) => item.questionId));
  const fresh = rows.filter((row) => !known.has(row.questionId));
  if (fresh.length === 0) return draft;
  const entries = [
    ...draft.entries,
    ...fresh.map((row) =>
      fixedQuestionEntry({
        id: `new-${row.questionId}`,
        questionId: row.questionId,
        title: row.title,
        backend: row.backend,
        capabilities: [],
        pointsPossible: "1",
        availability: "available",
        scoringRule: "normal",
      }),
    ),
  ];
  return { ...draft, entries };
}

export function appendQuestionPool(draft: AssignmentEditorDraft): AssignmentEditorDraft {
  const questionPool: AssignmentEditorQuestionPoolEntry = {
    kind: "questionPool",
    candidates: [],
    availability: "available",
    scoringRule: "normal",
    drawCount: 1,
    pointsPerItem: "1",
    selectionRule: { algorithm: "v1", ordering: "candidateOrder" },
  };
  return { ...draft, entries: [...draft.entries, questionPool] };
}

function entryInput(entry: AssignmentEditorEntry): AssignmentEditorEntryInput {
  if (entry.kind === "fixedQuestion") {
    return {
      kind: "fixedQuestion",
      questionId: entry.questionId,
      pointsPossible: entry.pointsPossible,
      availability: entry.availability,
      scoringRule: entry.scoringRule,
    };
  }
  return {
    kind: "questionPool",
    candidateQuestionIds: entry.candidates.map((candidate) => candidate.questionId),
    availability: entry.availability,
    scoringRule: entry.scoringRule,
    drawCount: entry.drawCount,
    pointsPerItem: entry.pointsPerItem,
    selectionRule: entry.selectionRule,
  };
}

/** Questions owns only the visible title and ordered fixed-or-pool definition. */
export function assignmentContentInput(draft: AssignmentEditorDraft): AssignmentContentInput {
  return {
    title: draft.title,
    entries: draft.entries.map(entryInput),
  };
}

export function validateQuestionPoolEntry(entry: AssignmentEditorQuestionPoolEntry): string | null {
  if (entry.candidates.length === 0) return "Add at least one candidate Question ID.";
  if (entry.candidates.length > MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL)
    return `Keep this pool to ${MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL} candidate Question IDs or fewer.`;
  if (entry.drawCount < 1) return "Draw count must be at least one.";
  if (entry.drawCount > entry.candidates.length)
    return "Draw count cannot exceed the number of candidate Question IDs.";
  if (
    new Set(entry.candidates.map((candidate) => candidate.questionId)).size !==
    entry.candidates.length
  )
    return "Each candidate Question ID can appear only once in a pool.";
  return null;
}

/**
 * Mirrors the server-owned pool bounds so the Instructor receives a recovery
 * path before a save request. Rust and PostgreSQL remain authoritative.
 */
export function validateAssignmentEditorDraft(draft: AssignmentEditorDraft): string | null {
  if (draft.entries.length > MAX_ASSIGNMENT_ORDERED_ENTRIES)
    return `Keep this assignment to ${MAX_ASSIGNMENT_ORDERED_ENTRIES} ordered entries or fewer.`;
  let totalCandidates = 0;
  for (const entry of draft.entries) {
    if (entry.kind !== "questionPool") continue;
    const entryError = validateQuestionPoolEntry(entry);
    if (entryError !== null)
      return `Question pool ${draft.entries.indexOf(entry) + 1}: ${entryError}`;
    totalCandidates += entry.candidates.length;
    if (totalCandidates > MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES)
      return `Keep all pools to ${MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES} candidate Question IDs or fewer.`;
  }
  return null;
}

export function parseExactProblemDisplayReferences(value: string): ReadonlyArray<string> {
  const values = value
    .split(/[\n,]/u)
    .map((entry) => entry.trim())
    .filter(Boolean);
  if (values.length === 0) throw new Error("Paste at least one Question ID, such as 7K3-M9QP.");
  const normalized = values.map((entry) => normalizeQuestionIdSyntax(entry));
  if (normalized.some((entry) => entry === null))
    throw new Error("Use canonical Question IDs such as 7K3-M9QP.");
  const ids = normalized as string[];
  if (new Set(ids).size !== ids.length)
    throw new Error("Each Question ID can appear once in this operation.");
  return ids;
}

export function capabilityLabel(capability: Capability): string {
  return capability.replace(/([A-Z])/gu, " $1").toLowerCase();
}
export function assignmentProblemLabel(
  row: AssignmentQuestionRow | FixedQuestionAssignmentEntrySummary,
): string {
  return row.questionId;
}
export function questionBackendLabel(backend: QuestionBackend): string {
  return { native: "PLE native", webwork: "WeBWorK", qti: "QTI", h5p: "H5P", imathas: "iMathAS" }[
    backend
  ];
}
export function violationMatchesQuestion(
  violation: AssignmentCapabilityViolation,
  questionId: string,
): boolean {
  return violation.questionId === questionId;
}
