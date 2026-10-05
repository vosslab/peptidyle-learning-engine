// Stable request ordering, recovery, and selection contracts. On failure, repair the session;
// do not accept stale rows/facets or reuse a cursor for a different query.
import assert from "node:assert/strict";
import test from "node:test";

import { SearchSession } from "../src/features/search/search_session.ts";

function deferred() {
  let resolve;
  const promise = new Promise((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function definition(fetchPage, selection = 3) {
  return {
    initialQuery: { text: "", filter: "all", sort: "title" },
    cleanup: (query) => ({ ...query, text: query.text.trim() }),
    getText: (query) => query.text,
    setText: (query, text) => ({ ...query, text }),
    fetchPage,
    rowId: (row) => row.id,
    content: (row) => ({ title: row.id, details: [], actions: [] }),
    selection: { maximum: selection },
  };
}

function page(items, nextCursor = null, filterCounts = [items.length]) {
  return { items: items.map((id) => ({ id })), nextCursor, filterCounts };
}

test("outdated responses including filter counts are ignored", async () => {
  const first = deferred();
  const second = deferred();
  const session = new SearchSession(
    definition((query) => (query.text === "first" ? first.promise : second.promise)),
  );

  const firstRequest = session.apply({ ...session.query, text: "first" });
  const secondRequest = session.apply({ ...session.query, text: "second" });
  second.resolve(page(["new"], null, [1]));
  await secondRequest;
  first.resolve(page(["old"], null, [99]));
  await firstRequest;
  assert.deepEqual(session.state, {
    kind: "ready",
    rows: [{ id: "new" }],
    filterCounts: [1],
    nextCursor: null,
  });
});

test("a replacement search clears rows and filter counts while loading", async () => {
  const replacement = deferred();
  const session = new SearchSession(
    definition((query) =>
      query.text === "replacement"
        ? replacement.promise
        : Promise.resolve(page(["old"], null, [4])),
    ),
  );
  await session.apply(session.query);
  const pending = session.apply({ ...session.query, text: "replacement" });
  assert.deepEqual(session.state, {
    kind: "loading",
    rows: [],
    filterCounts: undefined,
    nextCursor: null,
  });
  replacement.resolve(page(["new"], null, [2]));
  await pending;
});

test("a failed next page keeps rows and counts, and retry repeats that page", async () => {
  let nextAttempts = 0;
  const requests = [];
  const session = new SearchSession(
    definition(async (query, cursor, pageSize) => {
      requests.push({ query, cursor, pageSize });
      if (cursor === "next") {
        nextAttempts += 1;
        if (nextAttempts === 1) throw new Error("network");
        return page(["second"], null, [7]);
      }
      return page(["first"], "next", [5]);
    }),
  );
  await session.apply(session.query);
  await session.next();
  assert.equal(session.state.kind, "error");
  assert.deepEqual(session.state.rows, [{ id: "first" }]);
  assert.deepEqual(session.state.filterCounts, [5]);
  await session.retry();
  assert.deepEqual(session.state, {
    kind: "ready",
    rows: [{ id: "second" }],
    filterCounts: [7],
    nextCursor: null,
  });
  assert.equal(requests.filter((request) => request.cursor === "next").length, 2);
});

test("a changed query returns to page one and clears selection", async () => {
  const calls = [];
  const session = new SearchSession(
    definition(async (query, cursor) => {
      calls.push({ query, cursor });
      return page([`${query.filter}-${query.sort}`], "next", [1]);
    }),
  );
  await session.apply(session.query);
  session.select("all-title");
  await session.next();
  await session.apply({ ...session.query, filter: "mine" });
  assert.equal(calls.at(-1).cursor, null);
  assert.equal(session.selectedIds.size, 0);
});

test("page-size changes preserve selection and the configured maximum is enforced", async () => {
  const sizes = [];
  const session = new SearchSession(
    definition(async (_query, _cursor, pageSize) => {
      sizes.push(pageSize);
      return page(["one", "two", "three", "four"]);
    }, 2),
  );
  await session.apply(session.query);
  assert.equal(session.select("one"), true);
  assert.equal(session.select("two"), true);
  assert.equal(session.select("three"), false);
  await session.setPageSize(100);
  assert.deepEqual([...session.selectedIds], ["one", "two"]);
  assert.deepEqual(sizes, [50, 100]);
  session.clearSelection();
  assert.equal(session.selectLoaded(), 2);
  assert.deepEqual([...session.selectedIds], ["one", "two"]);
});

test("selective deselection retains an opposite-kind selection across result pages", async () => {
  const session = new SearchSession(
    definition(async (_query, cursor) =>
      cursor === null ? page(["question-one", "pool-one"], "next") : page(["question-two"]),
    ),
  );
  await session.apply(session.query);
  session.select("question-one");
  session.select("pool-one");
  await session.next();
  session.select("question-two");
  session.deselectIds(["question-one", "question-two"]);
  assert.deepEqual([...session.selectedIds], ["pool-one"]);
});

test("a failed page-size change cannot reuse a cursor from the old page sequence", async () => {
  const requests = [];
  const session = new SearchSession(
    definition(async (_query, cursor, pageSize) => {
      requests.push({ cursor, pageSize });
      if (pageSize === 100) throw new Error("page-size request failed");
      return page([cursor === null ? "first" : "second"], cursor === null ? "next" : null, [1]);
    }),
  );
  await session.apply(session.query);
  await session.next();
  await session.setPageSize(100);
  assert.equal(session.state.kind, "error");
  assert.equal(session.hasPrevious, false);
  await session.previous();
  assert.deepEqual(requests, [
    { cursor: null, pageSize: 50 },
    { cursor: "next", pageSize: 50 },
    { cursor: null, pageSize: 100 },
  ]);
});

test("Clear invalidates an in-flight search and restores the starting query", async () => {
  const pending = deferred();
  const session = new SearchSession(definition(() => pending.promise));
  const request = session.apply({ ...session.query, text: "typed", filter: "mine" });
  session.select("one");
  session.reset();
  pending.resolve(page(["late"], "next", [3]));
  await request;
  assert.equal(session.state.kind, "initial");
  assert.deepEqual(session.state.rows, []);
  assert.equal(session.state.filterCounts, undefined);
  assert.equal(session.query.text, "");
  assert.equal(session.query.filter, "all");
  assert.equal(session.selectedIds.size, 0);
});
