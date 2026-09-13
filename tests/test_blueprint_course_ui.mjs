import assert from "node:assert/strict";
import test from "node:test";

import { createBlueprintCourseWhenReady } from "../src/features/blueprint_course/blueprint_course_creation.ts";
import { emptyReusableContent } from "../src/features/blueprint_course/blueprint_course_model.ts";

function blueprint(content) {
  return {
    short_name: "Local Blueprint",
    long_name: "Local Blueprint Course",
    modules: [{ label: "Module 1", assignments: [content] }],
  };
}

test("incomplete Blueprint Course authoring remains local", async () => {
  let createCalls = 0;
  const client = {
    async createBlueprintCourse() {
      createCalls += 1;
      return { blueprintCourse: { reference: "BP-created" }, etag: "etag" };
    },
  };

  const result = await createBlueprintCourseWhenReady(
    client,
    blueprint(emptyReusableContent("Local working state")),
    "create-local",
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
    blueprint({
      ...content,
      entries: [
        { kind: "fixed", question_id: "AAA-BBBB", points_possible: "1", scoring_rule: "normal" },
      ],
    }),
    "create-ready",
  );

  assert.equal(result.kind, "created");
  assert.equal(createCalls, 1);
});
