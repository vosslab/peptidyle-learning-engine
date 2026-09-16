import assert from "node:assert/strict";
import test from "node:test";

import {
  saveBlueprintSearchReturnState,
  takeBlueprintSearchReturnState,
} from "../src/pages/blueprint_course_search_return_state.ts";

const classification = {
  disciplineUuid: "biology",
  subjectUuid: "biochemistry",
  topicUuid: "proteins",
  subtopicUuid: "folding",
  crossDiscipline: true,
};
const view = {
  draft: {
    query: "unsent draft",
    promotedOnly: false,
    classification,
    classificationDescription: "Biology / Biochemistry / Proteins / Folding",
  },
  submitted: {
    query: "Biochem",
    promotedOnly: true,
    classification,
    classificationDescription: "Biology / Biochemistry / Proteins / Folding",
  },
  pages: 2,
  scrollY: 720,
  linkKey: "open:BP75",
};
function session() {
  return { authenticated: true, account: { id: "instructor", productRole: "instructor" } };
}

test("Blueprint return preserves separate draft/applied filters and position once", () => {
  const scope = session();
  const token = saveBlueprintSearchReturnState(scope, view);
  assert.deepEqual(takeBlueprintSearchReturnState(scope, token), view);
  assert.equal(takeBlueprintSearchReturnState(scope, token), null);
});

test("Blueprint return cannot cross session refresh or mismatched navigation", () => {
  const scope = session();
  const token = saveBlueprintSearchReturnState(scope, view);
  assert.equal(takeBlueprintSearchReturnState(session(), token), null);
  assert.equal(takeBlueprintSearchReturnState(scope, token), null);
  const next = saveBlueprintSearchReturnState(scope, view);
  assert.equal(takeBlueprintSearchReturnState(scope, "another navigation"), null);
  assert.equal(takeBlueprintSearchReturnState(scope, next), null);
});
