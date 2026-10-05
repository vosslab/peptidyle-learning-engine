import assert from "node:assert/strict";
import test from "node:test";

import { assessmentContentPickerSelection } from "../src/features/assessment_content_picker/assessment_content_picker_model.ts";

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
