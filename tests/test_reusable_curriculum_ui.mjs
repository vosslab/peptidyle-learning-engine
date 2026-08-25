import assert from "node:assert/strict";
import test from "node:test";

import { emptyReusableDefinition } from "../src/features/reusable_curriculum/reusable_curriculum_model.ts";
import { createBlueprintWhenReady } from "../src/features/reusable_curriculum/reusable_curriculum_creation.ts";

test("empty create drafts remain local until reusable meaning is complete", async () => {
  let createCalls = 0;
  const client = {
    async createBlueprint() {
      createCalls += 1;
      return { blueprint: { reference: "BP-created" }, etag: "etag" };
    },
  };

  const result = await createBlueprintWhenReady(client, {
    definition: emptyReusableDefinition("Local draft"),
  });

  assert.equal(result.kind, "invalid");
  assert.equal(createCalls, 0);
});

test("complete reusable meaning invokes the live create capability once", async () => {
  let createCalls = 0;
  const client = {
    async createBlueprint() {
      createCalls += 1;
      return { blueprint: { reference: "BP-created" }, etag: "etag" };
    },
  };
  const definition = emptyReusableDefinition("Ready draft");
  const result = await createBlueprintWhenReady(client, {
    definition: {
      ...definition,
      entries: [
        { kind: "fixed", questionId: "AAA-BBBB", pointsPossible: "1", scoringMode: "normal" },
      ],
    },
  });

  assert.equal(result.kind, "created");
  assert.equal(createCalls, 1);
});
