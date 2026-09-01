import assert from "node:assert/strict";
import test from "node:test";

import { createBlueprintCourseWhenReady } from "../src/features/blueprint_course/blueprint_course_creation.ts";
import { emptyReusableContent } from "../src/features/blueprint_course/blueprint_course_model.ts";

function draft(content) {
  return {
    title: "Local Blueprint Course",
    modules: [{ label: "Module 1", assignments: [content] }],
  };
}

test("incomplete Blueprint Course drafts remain local", async () => {
  let createCalls = 0;
  const client = {
    async createBlueprintCourse() {
      createCalls += 1;
      return { blueprintCourse: { reference: "BP-created" }, etag: "etag" };
    },
  };

  const result = await createBlueprintCourseWhenReady(
    client,
    draft(emptyReusableContent("Local draft")),
  );

  assert.equal(result.kind, "invalid");
  assert.equal(createCalls, 0);
});

test("complete Blueprint Course meaning invokes its one live create capability", async () => {
  let createCalls = 0;
  const client = {
    async createBlueprintCourse() {
      createCalls += 1;
      return { blueprintCourse: { reference: "BP-created" }, etag: "etag" };
    },
  };
  const content = emptyReusableContent("Ready assignment");
  const result = await createBlueprintCourseWhenReady(
    client,
    draft({
      ...content,
      entries: [
        { kind: "fixed", question_id: "AAA-BBBB", points_possible: "1", scoring_rule: "normal" },
      ],
    }),
  );

  assert.equal(result.kind, "created");
  assert.equal(createCalls, 1);
});
