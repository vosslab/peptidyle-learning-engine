import assert from "node:assert/strict";
import test from "node:test";

import { createBlueprintCourseWhenReady } from "../src/features/blueprint_course/blueprint_course_creation.ts";
import { emptyReusableContent } from "../src/features/blueprint_course/blueprint_course_model.ts";

function blueprint(content) {
  return {
    short_name: "Local Blueprint",
    classification: {
      disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
      subjectUuid: null,
      topicUuid: null,
      subtopicUuid: null,
      tags: [],
    },
    long_name: "Local Blueprint Course",
    modules: [{ label: "Module 1", assessments: [content] }],
  };
}

test("incomplete Blueprint Course authoring remains local", async () => {
  let createCalls = 0;
  const client = {
    async createBlueprintCourse() {
      createCalls += 1;
      return { blueprintCourse: { id: "BP-created" } };
    },
  };

  const result = await createBlueprintCourseWhenReady(
    client,
    blueprint(emptyReusableContent("practice_question_assignment", "Local working state")),
    "create-local",
  );

  assert.equal(result.kind, "invalid");
  assert.equal(createCalls, 0);
});

test("complete Blueprint Course meaning invokes its one live create capability", async () => {
  let createCalls = 0;
  let createdContent;
  const client = {
    async createBlueprintCourse(content) {
      createCalls += 1;
      createdContent = content;
      return { blueprintCourse: { id: "BP-created" } };
    },
  };
  const content = emptyReusableContent("exam", "Ready exam");
  const result = await createBlueprintCourseWhenReady(
    client,
    blueprint({
      ...content,
      entries: [
        {
          kind: "fixed",
          published_question_revision_tuple: {
            publishedQuestionId: "AAAA-2BBB",
            revisionNumber: 1,
          },
          points_possible: "1",
          scoring_rule: "normal",
        },
      ],
    }),
    "create-ready",
  );

  assert.equal(result.kind, "created");
  assert.equal(createCalls, 1);
  assert.equal(createdContent.modules[0].assessments[0].assessment_type, "exam");
});
