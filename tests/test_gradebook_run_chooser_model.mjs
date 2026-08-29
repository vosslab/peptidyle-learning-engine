import assert from "node:assert/strict";
import test from "node:test";

import { GradebookRunChooserSession } from "../src/pages/gradebook_run_chooser_model.ts";

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function page(run) {
  return {
    rosterRevision: 1,
    nextCursor: null,
    rows: [{ run, submittedAt: 1_700_000_000_000, scoreSelected: true }],
  };
}

function pageWithRuns(runs, nextCursor = null) {
  return {
    ...page(runs[0]),
    nextCursor,
    rows: runs.map((run) => page(run).rows[0]),
  };
}

function scope() {
  return { courseId: "course-1", membership: "M-1", assignment: "A-1" };
}

test("chooser retry keeps the newer exact run when an older initial request finishes last", async () => {
  const initial = deferred();
  const retry = deferred();
  const requests = [initial, retry];
  let requestIndex = 0;
  const session = new GradebookRunChooserSession(
    scope(),
    {
      getSubmittedRunChoices: () => requests[requestIndex++].promise,
    },
    () => undefined,
  );

  const firstLoad = session.start();
  const retryLoad = session.retry();
  retry.resolve(page("R-2"));
  await retryLoad;
  initial.resolve(page("R-1"));
  await firstLoad;

  assert.equal(session.state.kind === "ready" ? session.state.rows[0]?.run : undefined, "R-2");
});

test("chooser retry replaces a pending continuation and leaves the new page pageable", async () => {
  const initial = deferred();
  const staleContinuation = deferred();
  const retry = deferred();
  const currentContinuation = deferred();
  const requests = [initial, staleContinuation, retry, currentContinuation];
  const session = new GradebookRunChooserSession(
    scope(),
    { getSubmittedRunChoices: () => requests.shift().promise },
    () => undefined,
  );

  const initialLoad = session.start();
  initial.resolve(pageWithRuns(["R-1"], "initial-cursor"));
  await initialLoad;
  const staleLoad = session.loadMore();
  const retryLoad = session.retry();
  retry.resolve(pageWithRuns(["R-2"], "retry-cursor"));
  await retryLoad;
  const currentLoad = session.loadMore();
  currentContinuation.resolve(pageWithRuns(["R-3"]));
  await currentLoad;
  staleContinuation.resolve(pageWithRuns(["R-stale"]));
  await staleLoad;

  assert.deepEqual(session.state.kind === "ready" ? session.state.rows.map((row) => row.run) : [], [
    "R-2",
    "R-3",
  ]);
});

test("chooser rejects mixed and incoming duplicate continuation runs atomically", async () => {
  for (const incomingRuns of [
    ["R-1", "R-2"],
    ["R-2", "R-2"],
  ]) {
    const initial = deferred();
    const continuation = deferred();
    const requests = [initial, continuation];
    const session = new GradebookRunChooserSession(
      scope(),
      { getSubmittedRunChoices: () => requests.shift().promise },
      () => undefined,
    );
    const initialLoad = session.start();
    initial.resolve(pageWithRuns(["R-1"], "cursor"));
    await initialLoad;
    const continuationLoad = session.loadMore();
    continuation.resolve(pageWithRuns(incomingRuns));
    await continuationLoad;

    assert.deepEqual(
      session.state.kind === "ready" ? session.state.rows.map((row) => row.run) : [],
      ["R-1"],
    );
    assert.equal(session.state.kind === "ready" ? session.state.moreError : false, true);
  }
});

test("chooser rejects duplicate runs on its initial page before ready publication", async () => {
  const states = [];
  const session = new GradebookRunChooserSession(
    scope(),
    { getSubmittedRunChoices: async () => pageWithRuns(["R-1", "R-1"]) },
    (state) => states.push(state),
  );

  await session.start();

  assert.equal(session.state.kind, "error");
  assert.deepEqual(
    states.map((state) => state.kind),
    ["loading", "error"],
  );
});

test("chooser retains rows on a current continuation error and disposes pending retry or continuation", async () => {
  const initial = deferred();
  const continuation = deferred();
  const states = [];
  let requestNumber = 0;
  const session = new GradebookRunChooserSession(
    scope(),
    {
      getSubmittedRunChoices: () => {
        requestNumber += 1;
        return (requestNumber === 1 ? initial : continuation).promise;
      },
    },
    (state) => states.push(state),
  );
  const initialLoad = session.start();
  initial.resolve(pageWithRuns(["R-1"], "cursor"));
  await initialLoad;
  const continuationLoad = session.loadMore();
  continuation.reject(new Error("continuation failed"));
  await continuationLoad;
  assert.deepEqual(session.state.kind === "ready" ? session.state.rows.map((row) => row.run) : [], [
    "R-1",
  ]);
  assert.equal(session.state.kind === "ready" ? session.state.moreError : false, true);

  const pendingRetry = deferred();
  const retryStates = [];
  let retryRequest = 0;
  const retrySession = new GradebookRunChooserSession(
    scope(),
    {
      getSubmittedRunChoices: () => {
        retryRequest += 1;
        return retryRequest === 1 ? Promise.resolve(page("R-1")) : pendingRetry.promise;
      },
    },
    (state) => retryStates.push(state),
  );
  await retrySession.start();
  const retryLoad = retrySession.retry();
  retrySession.dispose();
  const stateCount = retryStates.length;
  pendingRetry.resolve(page("R-retry"));
  await retryLoad;
  assert.equal(retryStates.length, stateCount);

  const pendingInitial = Promise.resolve(pageWithRuns(["R-1"], "cursor"));
  const continuationRequest = deferred();
  const continuationSessionWithPending = new GradebookRunChooserSession(
    scope(),
    {
      getSubmittedRunChoices: (...[, , , query]) =>
        query?.cursor === undefined ? pendingInitial : continuationRequest.promise,
    },
    () => undefined,
  );
  await continuationSessionWithPending.start();
  const pendingLoadMore = continuationSessionWithPending.loadMore();
  continuationSessionWithPending.dispose();
  continuationRequest.resolve(page("R-2"));
  await pendingLoadMore;
  assert.deepEqual(
    continuationSessionWithPending.state.kind === "ready"
      ? continuationSessionWithPending.state.rows.map((row) => row.run)
      : [],
    ["R-1"],
  );
});

test("disposed chooser completions cannot publish into a later chooser instance", async () => {
  const pending = deferred();
  const firstStates = [];
  const first = new GradebookRunChooserSession(
    scope(),
    { getSubmittedRunChoices: () => pending.promise },
    (state) => firstStates.push(state),
  );
  const firstLoad = first.start();
  first.dispose();

  const second = new GradebookRunChooserSession(
    { ...scope(), membership: "M-2" },
    { getSubmittedRunChoices: async () => page("R-2") },
    () => undefined,
  );
  await second.start();
  pending.resolve(page("R-1"));
  await firstLoad;

  assert.equal(
    firstStates.some((state) => state.kind === "ready"),
    false,
  );
  assert.equal(second.state.kind === "ready" ? second.state.rows[0]?.run : undefined, "R-2");
});
