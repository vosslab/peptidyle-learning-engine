import assert from "node:assert/strict";
import test from "node:test";

import { createQuestionDraftAutosave } from "../src/features/ple_question_json_authoring/question_draft_autosave.ts";

test("debounce coalesces edits and acknowledges only the newest snapshot", async () => {
  const writes = [];
  const states = [];
  const autosave = createQuestionDraftAutosave({
    debounceMs: 15,
    save: async (snapshot) => writes.push(snapshot),
    onStateChange: (state) => states.push(state),
  });
  autosave.edit("incomplete");
  autosave.edit("broken but bounded source");
  await autosave.flush();

  assert.deepEqual(writes, ["broken but bounded source"]);
  assert.equal(autosave.state().status, "saved");
  assert.equal(autosave.state().generation, autosave.state().acknowledgedGeneration);
  assert.ok(states.some((state) => state.status === "unsaved"));
  assert.ok(states.some((state) => state.status === "saving"));
  autosave.dispose();
});

test("writes stay serialized and the latest edit is acknowledged after an in-flight save", async () => {
  let releaseFirst;
  let markFirstStarted;
  const firstStarted = new Promise((resolve) => (markFirstStarted = resolve));
  const writes = [];
  let activeWrites = 0;
  let maximumActiveWrites = 0;
  const autosave = createQuestionDraftAutosave({
    debounceMs: 20,
    save: async (snapshot) => {
      activeWrites += 1;
      maximumActiveWrites = Math.max(maximumActiveWrites, activeWrites);
      writes.push(snapshot);
      if (snapshot === "first") {
        markFirstStarted();
        await new Promise((resolve) => (releaseFirst = resolve));
      }
      activeWrites -= 1;
    },
  });

  autosave.edit("first");
  const firstFlush = autosave.flush();
  await firstStarted;
  autosave.edit("latest");
  const latestFlush = autosave.flush();
  releaseFirst();
  await Promise.all([firstFlush, latestFlush]);

  assert.deepEqual(writes, ["first", "latest"]);
  assert.equal(maximumActiveWrites, 1);
  assert.equal(autosave.state().status, "saved");
  autosave.dispose();
});

test("a failed write retains the newest snapshot for an explicit retry", async () => {
  const writes = [];
  let fail = true;
  const autosave = createQuestionDraftAutosave({
    debounceMs: 60_000,
    save: async (snapshot) => {
      writes.push(snapshot);
      if (fail) throw new Error("offline");
    },
  });
  autosave.edit("unfinished source");
  await assert.rejects(autosave.flush(), /offline/u);
  assert.equal(autosave.state().status, "error");
  assert.equal(autosave.state().generation, 1);
  assert.equal(autosave.state().acknowledgedGeneration, 0);

  fail = false;
  await autosave.flush();
  assert.deepEqual(writes, ["unfinished source", "unfinished source"]);
  assert.equal(autosave.state().status, "saved");
  autosave.dispose();
});

test("a navigation flush persists the pending snapshot before disposal", async () => {
  const writes = [];
  const autosave = createQuestionDraftAutosave({
    debounceMs: 60_000,
    save: async (snapshot) => writes.push(snapshot),
  });
  autosave.edit("the last edit before leaving");

  await autosave.flush();
  autosave.dispose();

  assert.deepEqual(writes, ["the last edit before leaving"]);
  assert.equal(autosave.state().status, "saved");
});
