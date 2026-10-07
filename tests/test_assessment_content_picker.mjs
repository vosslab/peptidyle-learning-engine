import assert from "node:assert/strict";
import test from "node:test";

import { assessmentContentPickerSelection } from "../src/features/assessment_content_picker/assessment_content_picker_model.ts";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  NO_QUESTION_LIBRARY_FACET_TRUNCATION,
} from "../src/pages/library_page_model.ts";
import { questionLibrarySearchDefinition } from "../src/pages/question_library_search_definition.ts";

test("Assessment content picker starts from the shared mixed Library search definition", async () => {
  let request;
  const definition = questionLibrarySearchDefinition({
    async search(query, cursor, pageSize) {
      request = { query, cursor, pageSize };
      return {
        items: [],
        nextCursor: null,
        aggregates: [],
        facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
      };
    },
  });
  assert.equal(definition.initialQuery.kind, "both");
  assert.equal(definition.initialQuery.questions, "inNoPool");
  await definition.fetchPage(definition.initialQuery, null, 50);
  assert.equal(request.query.kind, "both");
  assert.equal(request.query.questions, "inNoPool");
  assert.equal(request.cursor, null);
  assert.equal(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.kind, "both");
});

function question(id, revisionNumber, title) {
  return {
    kind: "question",
    displayId: id,
    questionTitle: title,
    publishedQuestionRevisionTuple: { publishedQuestionId: id, revisionNumber },
  };
}

function pool(id, metadataEdit, poolEdit, memberCount, title) {
  return {
    kind: "pool",
    displayId: id,
    questionPoolMetadataEditNumber: metadataEdit,
    questionPoolEditNumber: poolEdit,
    memberCount,
    title,
  };
}

test("Assessment content picker preserves every selected Question exact revision in order", () => {
  assert.deepEqual(
    assessmentContentPickerSelection([
      question("AAAA-1BBBB", 2, "First"),
      question("CCCC-2DDDD", 7, "Second"),
    ]),
    {
      kind: "questions",
      questions: [
        {
          title: "First",
          publishedQuestionRevisionTuple: { publishedQuestionId: "AAAA-1BBBB", revisionNumber: 2 },
        },
        {
          title: "Second",
          publishedQuestionRevisionTuple: { publishedQuestionId: "CCCC-2DDDD", revisionNumber: 7 },
        },
      ],
    },
  );
});

test("Assessment content picker returns the Pool edit token rather than its distinct metadata edit", () => {
  assert.deepEqual(assessmentContentPickerSelection([pool("EEEE-3FFFF", 9, 3, 12, "Pool")]), {
    kind: "pool",
    title: "Pool",
    questionPoolId: "EEEE-3FFFF",
    questionPoolEditNumber: 3,
    memberCount: 12,
  });
});
