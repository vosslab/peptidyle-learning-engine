// Gradebook cursor paging: exact opaque cursors, stable row identity, and safe recovery.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { CursorPageSession } from "../src/pages/cursor_page_session.ts";
import { loadGradebookPage } from "../src/pages/gradebook_page_model.ts";

function row(number) {
  const identifier = `0198e000-0000-7000-8000-${String(number).padStart(12, "0")}`;
  return {
    ...publishedProblemFixture.gradebook[0],
    assignmentId: identifier,
    enrollmentId: `0198e000-0000-7001-8000-${String(number).padStart(12, "0")}`,
    assignmentTitle: `Visible assignment ${number}`,
  };
}

function rowKey(value) {
  return `${value.assignmentId}:${value.enrollmentId}`;
}

test("gradebook paging keeps the opaque cursor and appends a 51st visible row exactly once", async () => {
  const firstPage = Array.from({ length: 50 }, (_, index) => row(index + 1));
  const target = row(51);
  const cursor = "gradebook opaque cursor + /?=";
  const calls = [];
  const client = {
    listGradebook: async (...arguments_) => {
      calls.push(arguments_);
      return arguments_[1] === undefined
        ? { items: firstPage, nextCursor: cursor }
        : { items: [firstPage[49], target], nextCursor: null };
    },
  };

  const initial = await loadGradebookPage(client, publishedProblemFixture.course.id);
  const session = new CursorPageSession(
    initial,
    (nextCursor) => loadGradebookPage(client, publishedProblemFixture.course.id, nextCursor),
    rowKey,
  );

  assert.equal(
    session.state.items.some((value) => value.assignmentTitle === target.assignmentTitle),
    false,
  );
  const appended = await session.loadMore();

  assert.deepEqual(calls, [
    [publishedProblemFixture.course.id],
    [publishedProblemFixture.course.id, cursor],
  ]);
  assert.deepEqual(appended, [target]);
  assert.equal(session.state.items.length, 51);
  assert.equal(
    session.state.items.filter((value) => value.assignmentTitle === target.assignmentTitle).length,
    1,
  );
  assert.equal(session.state.nextCursor, null);
});

test("gradebook transport retry keeps prior rows and retries the exact failed cursor", async () => {
  const initial = { items: [row(1)], nextCursor: "retry-this-exact-cursor" };
  const requested = [];
  let fail = true;
  const session = new CursorPageSession(
    initial,
    async (cursor) => {
      requested.push(cursor);
      if (fail) {
        fail = false;
        throw new Error("temporary offline");
      }
      return { items: [row(2)], nextCursor: null };
    },
    rowKey,
  );

  assert.deepEqual(await session.loadMore(), []);
  assert.equal(session.state.items.length, 1);
  assert.equal(session.state.error?.kind, "transport");
  assert.equal(session.state.nextCursor, "retry-this-exact-cursor");

  assert.deepEqual(await session.retry(), [row(2)]);
  assert.deepEqual(requested, ["retry-this-exact-cursor", "retry-this-exact-cursor"]);
  assert.equal(session.state.items.length, 2);
  assert.equal(session.state.error, null);
});

test("gradebook pagination fails closed when a next cursor contributes no new record", async () => {
  const session = new CursorPageSession(
    { items: [row(1)], nextCursor: "repeated-page" },
    async () => ({ items: [row(1)], nextCursor: "different-but-empty" }),
    rowKey,
  );

  assert.deepEqual(await session.loadMore(), []);
  assert.equal(session.state.error?.kind, "protocol");
  assert.equal(session.state.nextCursor, null);
  assert.deepEqual(await session.retry(), []);
});
