import assert from "node:assert/strict";
import test from "node:test";

import {
  addQuestionPoolMembers,
  createQuestionPoolMembersDraft,
  mayEditQuestionPoolMembers,
  questionPoolMembersAreDirty,
  questionPoolMembersDraftAfterReload,
  removeQuestionPoolMember,
  saveQuestionPoolMembersDraft,
} from "../src/components/question_pool_members_model.ts";

const poolId = "3S8B-24DZ";
const firstId = "7K3M-79QP";
const secondId = "2R5X-E7YA";
const thirdId = "4T6V-J9NW";

function member(questionId, revisionNumber, title) {
  return {
    publishedQuestionRevisionTuple: { publishedQuestionId: questionId, revisionNumber },
    question: {
      question_library: {
        summary: { metadata: { questionTitle: title, questionLicense: "CC0-1.0" } },
      },
    },
  };
}

function pool(members, editNumber = 4) {
  return {
    questionPoolId: poolId,
    questionPoolEditNumber: editNumber,
    members,
  };
}

test("Pool member draft retains exact revisions and treats additions as an unordered set", () => {
  const draft = createQuestionPoolMembersDraft(pool([member(firstId, 3, "First question")]));
  const updated = addQuestionPoolMembers(draft, [
    {
      publishedQuestionRevisionTuple: { publishedQuestionId: secondId, revisionNumber: 8 },
      questionTitle: "Second question",
      questionLicense: null,
    },
  ]);

  assert.deepEqual(
    updated.members.map((item) => item.publishedQuestionRevisionTuple),
    [
      { publishedQuestionId: firstId, revisionNumber: 3 },
      { publishedQuestionId: secondId, revisionNumber: 8 },
    ],
  );
  assert.equal(updated.acknowledgedEditNumber, 4);
  assert.equal(questionPoolMembersAreDirty(updated), true);
});

test("Pool member draft cancel restores the acknowledged tuple set", () => {
  const draft = createQuestionPoolMembersDraft(pool([member(firstId, 3, "First question")]));
  const changed = addQuestionPoolMembers(draft, [
    {
      publishedQuestionRevisionTuple: { publishedQuestionId: secondId, revisionNumber: 8 },
      questionTitle: "Second question",
    },
  ]);
  const cancelled = removeQuestionPoolMember(changed, secondId);
  assert.equal(questionPoolMembersAreDirty(cancelled), false);
});

test("Pool member save advances its receipt and a failure preserves the complete draft", async () => {
  const original = createQuestionPoolMembersDraft(pool([member(firstId, 3, "First question")]));
  const changed = addQuestionPoolMembers(original, [
    {
      publishedQuestionRevisionTuple: { publishedQuestionId: secondId, revisionNumber: 8 },
      questionTitle: "Second question",
    },
  ]);
  const saved = await saveQuestionPoolMembersDraft(changed, async (request) => {
    assert.equal(request.expectedQuestionPoolEditNumber, 4);
    assert.equal(request.members.length, 2);
    return { questionPoolId: poolId, questionPoolEditNumber: 5 };
  });
  assert.equal(saved.kind, "saved");
  assert.equal(saved.draft.acknowledgedEditNumber, 5);
  assert.equal(questionPoolMembersAreDirty(saved.draft), false);

  const failed = await saveQuestionPoolMembersDraft(changed, async () => {
    throw new Error("stale Edit Number");
  });
  assert.deepEqual(failed, { kind: "failed", draft: changed });
  assert.equal(questionPoolMembersAreDirty(failed.draft), true);
});

test("reload finishing after a pending Add preserves exact tuples and the draft Edit Number", async () => {
  const original = createQuestionPoolMembersDraft(pool([member(firstId, 3, "First question")]));
  const pendingAdd = Promise.resolve().then(() =>
    addQuestionPoolMembers(original, [
      {
        publishedQuestionRevisionTuple: { publishedQuestionId: secondId, revisionNumber: 8 },
        questionTitle: "Second question",
        questionLicense: null,
      },
    ]),
  );
  const newerDraft = await pendingAdd;
  const reload = questionPoolMembersDraftAfterReload(
    newerDraft,
    original,
    pool([member(thirdId, 9, "Reloaded question")], 10),
  );

  assert.equal(reload.kind, "stale");
  assert.deepEqual(
    reload.draft.members.map((item) => item.publishedQuestionRevisionTuple),
    [
      { publishedQuestionId: firstId, revisionNumber: 3 },
      { publishedQuestionId: secondId, revisionNumber: 8 },
    ],
  );
  assert.equal(reload.draft.acknowledgedEditNumber, 4);
});

test("reload finishing after a pending Remove preserves the newer exact tuple set and Edit Number", () => {
  const original = createQuestionPoolMembersDraft(
    pool([member(firstId, 3, "First question"), member(secondId, 8, "Second question")]),
  );
  const changedDraft = removeQuestionPoolMember(original, secondId);
  const reload = questionPoolMembersDraftAfterReload(
    changedDraft,
    original,
    pool([member(thirdId, 9, "Reloaded question")], 10),
  );

  assert.equal(reload.kind, "stale");
  assert.deepEqual(
    reload.draft.members.map((item) => item.publishedQuestionRevisionTuple),
    [{ publishedQuestionId: firstId, revisionNumber: 3 }],
  );
  assert.equal(reload.draft.acknowledgedEditNumber, 4);
});

test("pending Add finishing after reload appends to refreshed exact tuples and Edit Number", async () => {
  const original = createQuestionPoolMembersDraft(pool([member(firstId, 3, "First question")]));
  let resolveAdd;
  const pendingAdd = new Promise((resolve) => {
    resolveAdd = resolve;
  });
  const reload = questionPoolMembersDraftAfterReload(
    original,
    original,
    pool([member(thirdId, 9, "Reloaded question")], 10),
  );
  assert.equal(reload.kind, "reloaded");

  resolveAdd([
    {
      publishedQuestionRevisionTuple: { publishedQuestionId: secondId, revisionNumber: 8 },
      questionTitle: "Second question",
      questionLicense: null,
    },
  ]);
  const additions = await pendingAdd;
  const completedDraft = addQuestionPoolMembers(reload.draft, additions);

  assert.deepEqual(
    completedDraft.members.map((item) => item.publishedQuestionRevisionTuple),
    [
      { publishedQuestionId: thirdId, revisionNumber: 9 },
      { publishedQuestionId: secondId, revisionNumber: 8 },
    ],
  );
  assert.equal(completedDraft.acknowledgedEditNumber, 10);
});

test("Pool member draft enforces one Revision per Question and preserves one required member", () => {
  const draft = createQuestionPoolMembersDraft(pool([member(firstId, 3, "First question")]));
  assert.throws(
    () =>
      addQuestionPoolMembers(draft, [
        {
          publishedQuestionRevisionTuple: { publishedQuestionId: firstId, revisionNumber: 4 },
          questionTitle: "Another revision",
        },
      ]),
    /one Revision per Published Question/,
  );
  assert.equal(removeQuestionPoolMember(draft, firstId), draft);
});

test("Pool member controls show for the Owner and Sysadmin but not another Instructor", () => {
  assert.equal(mayEditQuestionPoolMembers("instructor", "owner-account", "owner-account"), true);
  assert.equal(mayEditQuestionPoolMembers("instructor", "other-account", "owner-account"), false);
  assert.equal(mayEditQuestionPoolMembers("sysadmin", "admin-account", "owner-account"), true);
  assert.equal(mayEditQuestionPoolMembers("student", "owner-account", "owner-account"), false);
});
