// Local unordered exact tuple-set draft for ordinary Question Pool editing.

import type { QuestionPoolEditNumber } from "../../generated/api/QuestionPoolEditNumber";
import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import type { UserRole } from "../../generated/api/UserRole";
import {
  sortQuestionPoolMemberRows,
  type QuestionPoolMemberSort,
} from "./record_list/question_pool_member_sort";
import type {
  SaveQuestionPoolMembersInput,
  SavedQuestionPoolMembers,
} from "../api/question_pool_members";

export interface QuestionPoolMemberDraft {
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly questionTitle: string;
  readonly questionLicense: string | null;
}

export interface QuestionPoolMemberDraftDisplayRow {
  readonly draftMember: QuestionPoolMemberDraft;
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly questionTitle: string;
  readonly questionLicense: string | null;
}

export interface QuestionPoolMembersDraft {
  readonly questionPoolId: QuestionPoolView["questionPoolId"];
  readonly acknowledgedEditNumber: QuestionPoolEditNumber;
  readonly acknowledgedMembers: ReadonlyArray<PublishedQuestionRevisionTuple>;
  readonly members: ReadonlyArray<QuestionPoolMemberDraft>;
}

export function mayEditQuestionPoolMembers(
  role: UserRole,
  viewerAccountId: string,
  ownerAccountId: string,
): boolean {
  return role === "sysadmin" || (role === "instructor" && viewerAccountId === ownerAccountId);
}

function tupleKey(tuple: PublishedQuestionRevisionTuple): string {
  return `${tuple.publishedQuestionId}:${tuple.revisionNumber}`;
}

/** Sort a display-only representation of draft members with the shared Pool sort policy. */
export function sortQuestionPoolMemberDraftDisplay(
  members: ReadonlyArray<QuestionPoolMemberDraft>,
  sort: QuestionPoolMemberSort,
): ReadonlyArray<QuestionPoolMemberDraftDisplayRow> {
  const displayRows: QuestionPoolMemberDraftDisplayRow[] = members.map((draftMember) => ({
    draftMember,
    publishedQuestionRevisionTuple: draftMember.publishedQuestionRevisionTuple,
    questionTitle: draftMember.questionTitle,
    questionLicense: draftMember.questionLicense,
  }));
  return sortQuestionPoolMemberRows(displayRows, sort);
}

function checkedMembers(
  members: ReadonlyArray<QuestionPoolMemberDraft>,
): QuestionPoolMemberDraft[] {
  const questionIds = new Set<string>();
  for (const member of members) {
    const questionId = member.publishedQuestionRevisionTuple.publishedQuestionId;
    if (questionIds.has(questionId)) {
      throw new Error("A Question Pool may contain one Revision per Published Question.");
    }
    questionIds.add(questionId);
  }
  return [...members];
}

export function createQuestionPoolMembersDraft(pool: QuestionPoolView): QuestionPoolMembersDraft {
  const members = checkedMembers(
    pool.members.map((member) => ({
      publishedQuestionRevisionTuple: member.publishedQuestionRevisionTuple,
      questionTitle: member.question.question_library.summary.metadata.questionTitle,
      questionLicense: member.question.question_library.summary.metadata.questionLicense,
    })),
  );
  return {
    questionPoolId: pool.questionPoolId,
    acknowledgedEditNumber: pool.questionPoolEditNumber,
    acknowledgedMembers: members.map((member) => member.publishedQuestionRevisionTuple),
    members,
  };
}

export type QuestionPoolMembersReloadResult =
  | { readonly kind: "reloaded"; readonly draft: QuestionPoolMembersDraft }
  | { readonly kind: "stale"; readonly draft: QuestionPoolMembersDraft };

/** Keep a newer local draft when a Pool reload finishes after the draft changed. */
export function questionPoolMembersDraftAfterReload(
  currentDraft: QuestionPoolMembersDraft,
  draftAtReloadStart: QuestionPoolMembersDraft,
  latestPool: QuestionPoolView,
): QuestionPoolMembersReloadResult {
  if (currentDraft !== draftAtReloadStart) return { kind: "stale", draft: currentDraft };
  return { kind: "reloaded", draft: createQuestionPoolMembersDraft(latestPool) };
}

export function addQuestionPoolMembers(
  draft: QuestionPoolMembersDraft,
  additions: ReadonlyArray<QuestionPoolMemberDraft>,
): QuestionPoolMembersDraft {
  return { ...draft, members: checkedMembers([...draft.members, ...additions]) };
}

export function removeQuestionPoolMember(
  draft: QuestionPoolMembersDraft,
  publishedQuestionId: string,
): QuestionPoolMembersDraft {
  if (draft.members.length <= 1) return draft;
  return {
    ...draft,
    members: draft.members.filter(
      (member) => member.publishedQuestionRevisionTuple.publishedQuestionId !== publishedQuestionId,
    ),
  };
}

export function questionPoolMembersAreDirty(draft: QuestionPoolMembersDraft): boolean {
  const acknowledged = new Set(draft.acknowledgedMembers.map(tupleKey));
  const current = new Set(
    draft.members.map((member) => tupleKey(member.publishedQuestionRevisionTuple)),
  );
  return acknowledged.size !== current.size || [...acknowledged].some((key) => !current.has(key));
}

export function savedQuestionPoolMembersDraft(
  draft: QuestionPoolMembersDraft,
  questionPoolEditNumber: QuestionPoolEditNumber,
): QuestionPoolMembersDraft {
  const members = checkedMembers(draft.members);
  return {
    ...draft,
    acknowledgedEditNumber: questionPoolEditNumber,
    acknowledgedMembers: members.map((member) => member.publishedQuestionRevisionTuple),
    members,
  };
}

export type QuestionPoolMembersSaveResult =
  | { readonly kind: "saved"; readonly draft: QuestionPoolMembersDraft }
  | { readonly kind: "failed"; readonly draft: QuestionPoolMembersDraft };

export async function saveQuestionPoolMembersDraft(
  draft: QuestionPoolMembersDraft,
  save: (input: SaveQuestionPoolMembersInput) => Promise<SavedQuestionPoolMembers>,
): Promise<QuestionPoolMembersSaveResult> {
  try {
    const receipt = await save({
      questionPoolId: draft.questionPoolId,
      expectedQuestionPoolEditNumber: draft.acknowledgedEditNumber,
      members: draft.members.map((member) => member.publishedQuestionRevisionTuple),
    });
    return {
      kind: "saved",
      draft: savedQuestionPoolMembersDraft(draft, receipt.questionPoolEditNumber),
    };
  } catch {
    return { kind: "failed", draft };
  }
}
