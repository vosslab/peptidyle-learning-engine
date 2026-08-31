import assert from "node:assert/strict";
import test from "node:test";

import { createBlueprintCourseWhenReady } from "../src/features/reusable_curriculum/reusable_curriculum_creation.ts";
import { emptyReusableDefinition } from "../src/features/reusable_curriculum/reusable_curriculum_model.ts";

function draft(definition) {
  return {
    title: "Local Blueprint Course",
    modules: [{ label: "Module 1", definitions: [definition] }],
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
    draft(emptyReusableDefinition("Local draft")),
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
  const definition = emptyReusableDefinition("Ready assignment");
  const result = await createBlueprintCourseWhenReady(
    client,
    draft({
      ...definition,
      entries: [
        { kind: "fixed", question_id: "AAA-BBBB", points_possible: "1", scoring_mode: "normal" },
      ],
    }),
  );

  assert.equal(result.kind, "created");
  assert.equal(createCalls, 1);
});
