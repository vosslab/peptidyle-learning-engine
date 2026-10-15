import assert from "node:assert/strict";
import test from "node:test";

import { blueprintLifecyclePresentation } from "../src/features/blueprint_course/blueprint_course_model.ts";

test("Public Blueprint reuse is shared while editing and archive stay with the owner", () => {
  const reader = blueprintLifecyclePresentation("public", "active_instructor");
  const owner = blueprintLifecyclePresentation("public", "blueprint_course_owner");

  assert.equal(reader.canAdopt, true);
  assert.equal(reader.canEdit, false);
  assert.equal(reader.canArchive, false);
  assert.equal(reader.canRestore, false);
  assert.equal(owner.canAdopt, true);
  assert.equal(owner.canEdit, true);
  assert.equal(owner.canArchive, true);
});

test("Archived Blueprints are read-only and unavailable for adoption; only the owner restores", () => {
  for (const access of ["active_instructor", "blueprint_course_owner"]) {
    const presentation = blueprintLifecyclePresentation("archived", access);
    assert.equal(presentation.canAdopt, false);
    assert.equal(presentation.canEdit, false);
    assert.equal(presentation.canArchive, false);
    assert.equal(presentation.canRestore, access === "blueprint_course_owner");
  }
});
