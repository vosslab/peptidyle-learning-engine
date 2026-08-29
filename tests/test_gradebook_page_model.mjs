import assert from "node:assert/strict";
import test from "node:test";

import { GradebookPageSession } from "../src/pages/gradebook_page_model.ts";

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function gradebookPage(membership, nextCursor = null) {
  const assignment = "A-1";
  return {
    kind: "page",
    schemeRevision: 1,
    rosterRevision: 1,
    mode: "totalPoints",
    rounding: "fourDecimalPlacesHalfAwayFromZero",
    observationTime: 1,
    scoringWitnesses: [{ assignment, generation: 1, status: "current" }],
    nextCursor,
    rows: [
      {
        membership,
        displayLabel: `Student ${membership}`,
        outcome: {
          status: "available",
          score: 0.8,
          letter: "B",
          droppedAssignments: [],
          totalEarned: 8,
          totalPossible: 10,
        },
        assignmentCells: [
          {
            assignment,
            title: "Protein folding",
            included: true,
            category: null,
            availability: "available",
            selectedScore: 0.8,
            scoringStatus: "current",
            inspectionChoice: { kind: "noSubmittedRun" },
          },
        ],
      },
    ],
  };
}

function gradebookPageWithRows(memberships, nextCursor = null) {
  return {
    ...gradebookPage(memberships[0], nextCursor),
    rows: memberships.map((membership) => gradebookPage(membership).rows[0]),
  };
}

function selectedStudent(membership, label, nextCursor = null) {
  return {
    kind: "studentSelection",
    rows: [
      {
        membership,
        displayLabel: label,
        assignment: "A-1",
        inspectionChoice: { kind: "noSubmittedRun" },
      },
    ],
    nextCursor,
  };
}

function selectedStudents(memberships, nextCursor = null) {
  return {
    kind: "studentSelection",
    rows: memberships.map((membership) => ({
      membership,
      displayLabel: `Student ${membership}`,
      assignment: "A-1",
      inspectionChoice: { kind: "noSubmittedRun" },
    })),
    nextCursor,
  };
}

async function settle() {
  await Promise.resolve();
  await Promise.resolve();
}

test("an earlier Gradebook continuation cannot replace a newer route", async () => {
  const initial = deferred();
  const continuation = deferred();
  const replacement = deferred();
  const gradebookRequests = [initial, continuation, replacement];
  const repository = {
    getCalculatedGradebook: async () => gradebookRequests.shift().promise,
    getGradebookSelection: async () => selectedStudent("M-9", "Not used"),
  };
  const session = new GradebookPageSession("C-1", repository, () => {});

  session.reset({ kind: "valid", filter: undefined });
  initial.resolve(gradebookPage("M-1", "older-cursor"));
  await settle();
  session.loadMoreGradebook();
  session.reset({ kind: "valid", filter: { kind: "student", membership: "M-2" } });
  replacement.resolve(gradebookPage("M-2"));
  await settle();
  continuation.resolve({ kind: "reloadRequired", reason: "rosterChanged" });
  await settle();

  assert.equal(session.state.gradebook.kind, "ready");
  assert.equal(session.state.gradebook.rows[0]?.displayLabel, "Student M-2");
});

test("an earlier operation continuation cannot replace a newer operation choice", async () => {
  const firstGradebook = deferred();
  const secondGradebook = deferred();
  const firstSelection = deferred();
  const firstContinuation = deferred();
  const secondSelection = deferred();
  const gradebookRequests = [firstGradebook, secondGradebook];
  const selectionRequests = [firstSelection, firstContinuation, secondSelection];
  const repository = {
    getCalculatedGradebook: async () => gradebookRequests.shift().promise,
    getGradebookSelection: async () => selectionRequests.shift().promise,
  };
  const session = new GradebookPageSession("C-1", repository, () => {});

  session.reset({ kind: "valid", filter: { kind: "operation", operation: "GO-1" } });
  firstGradebook.resolve(gradebookPage("M-1"));
  firstSelection.resolve(selectedStudent("M-1", "Earlier Student", "older-cursor"));
  await settle();
  session.loadMoreSelection();
  session.reset({ kind: "valid", filter: { kind: "operation", operation: "GO-2" } });
  secondGradebook.resolve(gradebookPage("M-2"));
  secondSelection.resolve(selectedStudent("M-2", "Current Student"));
  await settle();
  firstContinuation.reject(new Error("old request failed"));
  await settle();

  assert.equal(session.state.operationSelection.kind, "studentSelection");
  assert.equal(session.state.operationSelection.rows[0]?.displayLabel, "Current Student");
});

test("stale Gradebook continuation success and error cannot interfere with reloads", async () => {
  const initial = deferred();
  const staleSuccess = deferred();
  const firstReload = deferred();
  const staleError = deferred();
  const secondReload = deferred();
  const requests = [initial, staleSuccess, firstReload, staleError, secondReload];
  const session = new GradebookPageSession(
    "C-1",
    {
      getCalculatedGradebook: async () => requests.shift().promise,
      getGradebookSelection: async () => selectedStudent("M-9", "Not used"),
    },
    () => {},
  );

  session.reset({ kind: "valid", filter: undefined });
  initial.resolve(gradebookPage("M-1", "cursor-1"));
  await settle();
  session.loadMoreGradebook();
  session.reload();
  staleSuccess.resolve(gradebookPage("M-stale"));
  firstReload.resolve(gradebookPage("M-2", "cursor-2"));
  await settle();
  session.loadMoreGradebook();
  session.reload();
  staleError.reject(new Error("obsolete continuation failed"));
  secondReload.resolve(gradebookPage("M-3"));
  await settle();

  assert.equal(session.state.gradebook.kind, "ready");
  assert.equal(session.state.gradebook.rows[0]?.membership, "M-3");
});

test("stale operation continuation success cannot replace a reloaded operation choice", async () => {
  const firstGradebook = deferred();
  const secondGradebook = deferred();
  const firstSelection = deferred();
  const staleContinuation = deferred();
  const secondSelection = deferred();
  const gradebookRequests = [firstGradebook, secondGradebook];
  const selectionRequests = [firstSelection, staleContinuation, secondSelection];
  const session = new GradebookPageSession(
    "C-1",
    {
      getCalculatedGradebook: async () => gradebookRequests.shift().promise,
      getGradebookSelection: async () => selectionRequests.shift().promise,
    },
    () => {},
  );

  session.reset({ kind: "valid", filter: { kind: "operation", operation: "GO-1" } });
  firstGradebook.resolve(gradebookPage("M-1"));
  firstSelection.resolve(selectedStudent("M-1", "Earlier Student", "cursor"));
  await settle();
  session.loadMoreSelection();
  session.reload();
  staleContinuation.resolve(selectedStudent("M-stale", "Stale Student"));
  secondGradebook.resolve(gradebookPage("M-2"));
  secondSelection.resolve(selectedStudent("M-2", "Current Student"));
  await settle();

  assert.equal(session.state.operationSelection.kind, "studentSelection");
  assert.equal(session.state.operationSelection.rows[0]?.displayLabel, "Current Student");
});

test("current continuation errors retain Gradebook and operation-selection rows", async () => {
  const gradebookInitial = deferred();
  const gradebookContinuation = deferred();
  const gradebookSession = new GradebookPageSession(
    "C-1",
    {
      getCalculatedGradebook: async () =>
        (gradebookSession.state.gradebook.kind === "loading"
          ? gradebookInitial
          : gradebookContinuation
        ).promise,
      getGradebookSelection: async () => selectedStudent("M-9", "Not used"),
    },
    () => {},
  );
  gradebookSession.reset({ kind: "valid", filter: undefined });
  gradebookInitial.resolve(gradebookPage("M-1", "cursor"));
  await settle();
  gradebookSession.loadMoreGradebook();
  gradebookContinuation.reject(new Error("gradebook unavailable"));
  await settle();
  assert.deepEqual(
    gradebookSession.state.gradebook.kind === "ready"
      ? gradebookSession.state.gradebook.rows.map((row) => row.membership)
      : [],
    ["M-1"],
  );
  assert.equal(
    gradebookSession.state.gradebook.kind === "ready"
      ? gradebookSession.state.gradebook.moreError
      : false,
    true,
  );

  const selectionInitial = deferred();
  const selectionContinuation = deferred();
  const selectionSession = new GradebookPageSession(
    "C-1",
    {
      getCalculatedGradebook: async () => gradebookPage("M-1"),
      getGradebookSelection: async (_courseId, request) =>
        request.cursor === undefined ? selectionInitial.promise : selectionContinuation.promise,
    },
    () => {},
  );
  selectionSession.reset({ kind: "valid", filter: { kind: "operation", operation: "GO-1" } });
  selectionInitial.resolve(selectedStudents(["M-1"], "cursor"));
  await settle();
  selectionSession.loadMoreSelection();
  selectionContinuation.reject(new Error("selection unavailable"));
  await settle();
  assert.deepEqual(
    selectionSession.state.operationSelection.kind === "studentSelection"
      ? selectionSession.state.operationSelection.rows.map((row) => row.membership)
      : [],
    ["M-1"],
  );
  assert.equal(
    selectionSession.state.operationSelection.kind === "studentSelection"
      ? selectionSession.state.operationSelection.moreError
      : false,
    true,
  );
});

test("Gradebook and operation-selection continuations reject duplicate memberships atomically", async () => {
  const gradebookInitial = deferred();
  const gradebookContinuation = deferred();
  let gradebookRequest = 0;
  const gradebookSession = new GradebookPageSession(
    "C-1",
    {
      getCalculatedGradebook: async () => {
        gradebookRequest += 1;
        return (gradebookRequest === 1 ? gradebookInitial : gradebookContinuation).promise;
      },
      getGradebookSelection: async () => selectedStudent("M-9", "Not used"),
    },
    () => {},
  );
  gradebookSession.reset({ kind: "valid", filter: undefined });
  gradebookInitial.resolve(gradebookPage("M-1", "cursor"));
  await settle();
  gradebookSession.loadMoreGradebook();
  gradebookContinuation.resolve(gradebookPageWithRows(["M-1", "M-2"]));
  await settle();
  assert.deepEqual(
    gradebookSession.state.gradebook.kind === "ready"
      ? gradebookSession.state.gradebook.rows.map((row) => row.membership)
      : [],
    ["M-1"],
  );
  assert.equal(
    gradebookSession.state.gradebook.kind === "ready"
      ? gradebookSession.state.gradebook.moreError
      : false,
    true,
  );

  const selectionInitial = deferred();
  const selectionContinuation = deferred();
  let selectionRequest = 0;
  const selectionSession = new GradebookPageSession(
    "C-1",
    {
      getCalculatedGradebook: async () => gradebookPage("M-1"),
      getGradebookSelection: async () => {
        selectionRequest += 1;
        return (selectionRequest === 1 ? selectionInitial : selectionContinuation).promise;
      },
    },
    () => {},
  );
  selectionSession.reset({ kind: "valid", filter: { kind: "operation", operation: "GO-1" } });
  selectionInitial.resolve(selectedStudents(["M-1"], "cursor"));
  await settle();
  selectionSession.loadMoreSelection();
  selectionContinuation.resolve(selectedStudents(["M-2", "M-2"]));
  await settle();
  assert.deepEqual(
    selectionSession.state.operationSelection.kind === "studentSelection"
      ? selectionSession.state.operationSelection.rows.map((row) => row.membership)
      : [],
    ["M-1"],
  );
  assert.equal(
    selectionSession.state.operationSelection.kind === "studentSelection"
      ? selectionSession.state.operationSelection.moreError
      : false,
    true,
  );
});

test("initial Gradebook and operation-selection pages reject duplicate memberships", async () => {
  const gradebookStates = [];
  const gradebookSession = new GradebookPageSession(
    "C-1",
    {
      getCalculatedGradebook: async () => gradebookPageWithRows(["M-1", "M-1"]),
      getGradebookSelection: async () => selectedStudent("M-9", "Not used"),
    },
    (state) => gradebookStates.push(state),
  );
  gradebookSession.reset({ kind: "valid", filter: undefined });
  await settle();
  assert.equal(gradebookSession.state.gradebook.kind, "error");
  assert.equal(gradebookStates.at(-1)?.gradebook.kind, "error");

  const selectionStates = [];
  const selectionSession = new GradebookPageSession(
    "C-1",
    {
      getCalculatedGradebook: async () => gradebookPage("M-1"),
      getGradebookSelection: async () => selectedStudents(["M-1", "M-1"]),
    },
    (state) => selectionStates.push(state),
  );
  selectionSession.reset({ kind: "valid", filter: { kind: "operation", operation: "GO-1" } });
  await settle();
  assert.equal(selectionSession.state.operationSelection.kind, "error");
  assert.equal(selectionStates.at(-1)?.operationSelection.kind, "error");
});
