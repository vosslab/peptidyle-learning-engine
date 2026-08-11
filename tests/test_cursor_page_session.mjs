// Cursor pagination must stay bounded, preserve visible work, and fail closed on bad chains.

import assert from "node:assert/strict";
import test from "node:test";

import { CursorPageSession } from "../src/pages/cursor_page_session.ts";

function page(items, nextCursor) {
  return { items, nextCursor };
}

test("a 50-row page appends the 51st record exactly once", async () => {
  const initial = Array.from({ length: 50 }, (_, index) => ({ id: `assignment-${index}` }));
  const requests = [];
  const session = new CursorPageSession(
    page(initial, "after-50"),
    async (cursor) => {
      requests.push(cursor);
      return page([{ id: "assignment-50" }], null);
    },
    (item) => item.id,
  );

  const appended = await session.loadMore();
  assert.deepEqual(requests, ["after-50"]);
  assert.deepEqual(appended, [{ id: "assignment-50" }]);
  assert.equal(session.state.items.length, 51);
  assert.equal(session.state.nextCursor, null);
});

test("overlap and duplicate records never duplicate prior visible items", async () => {
  const session = new CursorPageSession(
    page([{ id: "a" }, { id: "a" }, { id: "b" }], "next"),
    async () => page([{ id: "b" }, { id: "c" }, { id: "c" }], null),
    (item) => item.id,
  );

  const appended = await session.loadMore();
  assert.deepEqual(appended, [{ id: "c" }]);
  assert.deepEqual(
    session.state.items.map((item) => item.id),
    ["a", "b", "c"],
  );
});

test("concurrent load-more requests share the one opaque cursor request", async () => {
  let release;
  const pending = new Promise((resolve) => {
    release = resolve;
  });
  const requests = [];
  const session = new CursorPageSession(
    page([{ id: "a" }], "next"),
    async (cursor) => {
      requests.push(cursor);
      return pending;
    },
    (item) => item.id,
  );

  const first = session.loadMore();
  const second = session.loadMore();
  assert.equal(first, second);
  assert.deepEqual(requests, ["next"]);
  release(page([{ id: "b" }], null));
  await first;
  assert.deepEqual(
    session.state.items.map((item) => item.id),
    ["a", "b"],
  );
});

test("a transport failure retains visible records and retries the exact opaque cursor", async () => {
  const requests = [];
  let attempt = 0;
  const session = new CursorPageSession(
    page([{ id: "a" }], "opaque cursor"),
    async (cursor) => {
      requests.push(cursor);
      attempt += 1;
      if (attempt === 1) throw new Error("temporary failure");
      return page([{ id: "b" }], null);
    },
    (item) => item.id,
  );

  await session.loadMore();
  assert.equal(session.state.error?.kind, "transport");
  assert.equal(session.state.nextCursor, "opaque cursor");
  assert.deepEqual(
    session.state.items.map((item) => item.id),
    ["a"],
  );
  await session.retry();
  assert.deepEqual(requests, ["opaque cursor", "opaque cursor"]);
  assert.equal(session.state.error, null);
  assert.deepEqual(
    session.state.items.map((item) => item.id),
    ["a", "b"],
  );
});

test("repeated cursor and zero-new nonterminal pages fail closed without retry", async () => {
  const repeated = new CursorPageSession(
    page([{ id: "a" }], "next"),
    async () => page([{ id: "b" }], "next"),
    (item) => item.id,
  );
  await repeated.loadMore();
  assert.equal(repeated.state.error?.kind, "protocol");
  assert.equal(repeated.state.nextCursor, null);
  assert.deepEqual(await repeated.retry(), []);

  const zeroNew = new CursorPageSession(
    page([{ id: "a" }], "next"),
    async () => page([{ id: "a" }], "later"),
    (item) => item.id,
  );
  await zeroNew.loadMore();
  assert.equal(zeroNew.state.error?.kind, "protocol");
  assert.equal(zeroNew.state.nextCursor, null);
  assert.deepEqual(
    zeroNew.state.items.map((item) => item.id),
    ["a"],
  );
});

test("a terminal page with no newly appended records remains a valid completed list", async () => {
  const session = new CursorPageSession(
    page([{ id: "a" }], "next"),
    async () => page([{ id: "a" }], null),
    (item) => item.id,
  );
  assert.deepEqual(await session.loadMore(), []);
  assert.equal(session.state.error, null);
  assert.equal(session.state.nextCursor, null);
  assert.deepEqual(
    session.state.items.map((item) => item.id),
    ["a"],
  );
});
