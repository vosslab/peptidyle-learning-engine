// Pure route, session-state, and issued-question contract evidence.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import { createBrowserSessionBoundary } from "../src/auth/browser_session_boundary.ts";
import { createSessionBootstrap, sessionFailureState } from "../src/auth/session_context.tsx";
import { prefetchMatchesIssuedSuccessor } from "../src/features/question_attempt/prefetch_binding.ts";
import { assignmentWorkspacePath } from "../src/pages/assignment_workspace/assignment_workspace_paths.ts";
import { accountRoleMayAccessRoute, routeContractForPathname } from "../src/route_contract.ts";

test("route contracts fail closed and reserve authoring routes for teaching roles", () => {
  assert.equal(routeContractForPathname("/library/7K3-M9QP")?.id, "questionDetail");
  assert.equal(routeContractForPathname("/library/7K3-M9QP/extra"), undefined);
  assert.equal(routeContractForPathname("/blueprint-courses")?.id, "blueprintCourses");
  assert.equal(routeContractForPathname("/blueprint-courses/BP-7")?.id, "blueprintCourseDetail");
  assert.equal(routeContractForPathname("/blueprint-courses/BP-7/extra"), undefined);
  assert.equal(routeContractForPathname("/sysadmin/instructor-approval"), undefined);
  assert.equal(
    routeContractForPathname("/instructor/courses/C-1/assignments/A-1")?.id,
    "assignmentWorkspaceOverview",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/C-1/assignments/A-1/questions")?.id,
    "assignmentWorkspaceQuestions",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/C-1/assignments/A-1/policies")?.id,
    "assignmentWorkspacePolicies",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/C-1/assignments/A-1/student-view")?.id,
    "assignmentWorkspaceStudentView",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/C-1/assignments/A-1/grading-operations")?.id,
    "assignmentWorkspaceGradingOperations",
  );
  assert.equal(routeContractForPathname("/instructor/courses/C-1/assignments/A-1/edit"), undefined);
  assert.equal(accountRoleMayAccessRoute("assignmentOverview", "student"), true);
  assert.equal(accountRoleMayAccessRoute("assignmentOverview", "instructor"), false);
  assert.equal(accountRoleMayAccessRoute("assignmentWorkspaceOverview", "student"), false);
  assert.equal(accountRoleMayAccessRoute("assignmentWorkspaceOverview", "instructor"), true);
  assert.equal(accountRoleMayAccessRoute("assignmentWorkspaceGradingOperations", "student"), false);
  assert.equal(
    accountRoleMayAccessRoute("assignmentWorkspaceGradingOperations", "instructor"),
    true,
  );
  assert.equal(
    accountRoleMayAccessRoute("assignmentWorkspaceGradingOperations", "sysadmin"),
    false,
  );
  assert.equal(accountRoleMayAccessRoute("workspaceEditor", "student"), false);
  assert.equal(accountRoleMayAccessRoute("workspaceEditor", "instructor"), true);
  assert.equal(accountRoleMayAccessRoute("workspaceEditor", "sysadmin"), false);
  assert.equal(accountRoleMayAccessRoute("teachingOperations", "sysadmin"), false);
  assert.equal(accountRoleMayAccessRoute("blueprintCourses", "student"), false);
  assert.equal(accountRoleMayAccessRoute("blueprintCourses", "sysadmin"), false);
  assert.equal(accountRoleMayAccessRoute("blueprintCourses", "instructor"), true);
});

test("assignment workspace paths use the declared grading-operations route", () => {
  assert.equal(
    assignmentWorkspacePath("C-1", "A-1", "gradingOperations"),
    "/instructor/courses/C-1/assignments/A-1/grading-operations",
  );
});

test("session bootstrap retains only safe session state with direct narrow dependencies", async () => {
  const session = {
    authenticated: true,
    account: { id: "account-a", role: "student" },
  };
  const boundaryStates = [];
  let bootstrap;
  bootstrap = createSessionBootstrap(
    async () => session,
    async () => undefined,
    () => boundaryStates.push(bootstrap.state().kind),
  );
  await bootstrap.retry();
  assert.equal(bootstrap.state().kind, "authenticated");
  assert.equal("credential" in bootstrap.state(), false);
  assert.equal(await bootstrap.signOut(), true);
  await bootstrap.retry();
  assert.deepEqual(boundaryStates, ["authenticated", "loading"]);
  assert.deepEqual(sessionFailureState({ status: 401 }), { kind: "expired" });
});

test("browser session generations abort old requests and clear cached projections", async () => {
  const signals = [];
  let cacheClears = 0;
  const boundary = createBrowserSessionBoundary(
    async (_input, init) => {
      signals.push(init?.signal);
      return new Response(null, { status: 204 });
    },
    () => {
      cacheClears += 1;
    },
  );

  await boundary.fetch("/first");
  const firstSignal = signals.at(-1);
  assert.equal(firstSignal?.aborted, false);
  boundary.advance();
  assert.equal(firstSignal?.aborted, true);
  assert.equal(cacheClears, 1);

  await boundary.fetch("/second");
  const secondSignal = signals.at(-1);
  assert.notEqual(secondSignal, firstSignal);
  assert.equal(secondSignal?.aborted, false);

  const request = new AbortController();
  await boundary.fetch("/third", { signal: request.signal });
  const combinedSignal = signals.at(-1);
  request.abort();
  assert.equal(combinedSignal?.aborted, true);
  assert.equal(secondSignal?.aborted, false);
});

test("a stale session lookup cannot overwrite a newer authenticated generation", async () => {
  let releaseFirst;
  let calls = 0;
  const firstSession = new Promise((resolve) => {
    releaseFirst = resolve;
  });
  const newerSession = {
    authenticated: true,
    account: { id: "account-new", role: "student" },
  };
  let advances = 0;
  const bootstrap = createSessionBootstrap(
    async () => {
      calls += 1;
      return calls === 1 ? firstSession : newerSession;
    },
    async () => undefined,
    () => {
      advances += 1;
    },
  );

  const stale = bootstrap.retry();
  await bootstrap.retry();
  assert.deepEqual(bootstrap.state(), { kind: "authenticated", session: newerSession });
  releaseFirst({
    authenticated: true,
    account: { id: "account-old", role: "student" },
  });
  await stale;
  assert.deepEqual(bootstrap.state(), { kind: "authenticated", session: newerSession });
  assert.equal(advances, 1);
});

test("the generated browser surface excludes answer-bearing type names", () => {
  const apiDirectory = path.resolve("generated/api");
  const forbidden = /\b(?:AnswerKey|CorrectResponse|SolutionKey)\b/;
  for (const filename of fs.readdirSync(apiDirectory).filter((name) => name.endsWith(".ts"))) {
    assert.doesNotMatch(
      fs.readFileSync(path.join(apiDirectory, filename), "utf8"),
      forbidden,
      filename,
    );
  }
});

test("prefetched successors require the committed receipt binding", () => {
  const successor = {
    predecessor: "attempt-a",
    issuedQuestion: { id: "issued-question-b" },
    seed: 2,
    renderedQuestionSha256: "a".repeat(64),
  };
  const issued = {
    id: "attempt-b",
    issuedQuestion: successor.issuedQuestion,
    seed: successor.seed,
    deadline: null,
    renderedQuestionSha256: successor.renderedQuestionSha256,
  };
  assert.equal(prefetchMatchesIssuedSuccessor(successor, issued, "attempt-a"), true);
  assert.equal(
    prefetchMatchesIssuedSuccessor({ ...successor, seed: 3 }, issued, "attempt-a"),
    false,
  );
});
