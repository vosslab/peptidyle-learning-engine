import assert from "node:assert/strict";
import test from "node:test";

import {
  blueprintDetailCollectionLink,
  blueprintSearchReturnPath,
  saveBlueprintSearchReturnState,
  takeBlueprintSearchReturnState,
} from "../src/pages/blueprint_course_search_return_state.ts";

const token = "8f5e7d01-b6c7-4c14-8a0b-4bfef6390d6d";
const session = { accountId: "acct", sessionId: "sess" };

test("an owner returning from public search keeps that result page", () => {
  const link = blueprintDetailCollectionLink("blueprint_course_owner", token);
  assert.equal(link.label, "Return to Public Blueprint Courses");
  assert.equal(link.href, blueprintSearchReturnPath(token));
});

test("detail without a search token follows read access", () => {
  assert.deepEqual(blueprintDetailCollectionLink("blueprint_course_owner", null), {
    href: "/blueprint-courses",
    label: "Return to My Blueprint Courses",
  });
  assert.deepEqual(blueprintDetailCollectionLink("active_instructor", "not-a-token"), {
    href: "/blueprint-courses/search/public",
    label: "Return to Public Blueprint Courses",
  });
});

test("return state restores one cursor for the same session and is consumed once", () => {
  const state = {
    draft: {
      query: "peptide",
      promotedOnly: false,
      classification: {
        disciplineUuid: null,
        subjectUuid: null,
        topicUuid: null,
        subtopicUuid: null,
        crossDiscipline: false,
      },
      classificationDescription: "",
      sort: "name",
    },
    submitted: {
      query: "peptide",
      promotedOnly: true,
      classification: {
        disciplineUuid: null,
        subjectUuid: null,
        topicUuid: null,
        subtopicUuid: null,
        crossDiscipline: false,
      },
      classificationDescription: "",
      sort: "name",
    },
    currentCursor: "cursor-2",
    previousCursors: [undefined, "cursor-1"],
    pageSize: 100,
    scrollY: 40,
    linkKey: "open:BP7K3M2QAF",
  };
  saveBlueprintSearchReturnState(session, token, state);
  assert.equal(
    takeBlueprintSearchReturnState({ accountId: "other", sessionId: "sess" }, token),
    null,
  );
  assert.equal(takeBlueprintSearchReturnState(session, token), null);
  saveBlueprintSearchReturnState(session, token, state);
  assert.deepEqual(takeBlueprintSearchReturnState(session, token), state);
  assert.equal(takeBlueprintSearchReturnState(session, token), null);
  assert.equal(takeBlueprintSearchReturnState(session, token), null);
});
