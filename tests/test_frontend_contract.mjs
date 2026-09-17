// Pure route, session-state, and issued-question contract evidence.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import { createBrowserSessionBoundary } from "../src/auth/browser_session_boundary.ts";
import { createSessionBootstrap, sessionFailureState } from "../src/auth/session_context.tsx";
import { productRoleMayAccessRoute, routeContractForPathname } from "../src/route_contract.ts";

test("route contracts fail closed and reserve declared teaching routes for instructors", () => {
  assert.equal(routeContractForPathname("/library/7K3M-79QP")?.id, "questionDetail");
  assert.equal(routeContractForPathname("/library/7K3M-79QP/extra"), undefined);
  assert.equal(routeContractForPathname("/blueprint-courses")?.id, "blueprintCourses");
  assert.equal(
    routeContractForPathname("/blueprint-courses/search/public")?.id,
    "publicBlueprintSearch",
  );
  assert.equal(productRoleMayAccessRoute("publicBlueprintSearch", "instructor"), true);
  assert.equal(productRoleMayAccessRoute("publicBlueprintSearch", "student"), false);
  assert.equal(productRoleMayAccessRoute("publicBlueprintSearch", "sysadmin"), false);
  assert.equal(routeContractForPathname("/blueprint-courses/BP-7")?.id, "blueprintCourseDetail");
  assert.equal(routeContractForPathname("/blueprint-courses/BP-7/extra"), undefined);
  assert.equal(routeContractForPathname("/sysadmin/instructor-approval"), undefined);
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF")?.id,
    "assessmentWorkspaceOverview",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/questions")?.id,
    "assessmentWorkspaceQuestions",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/properties")?.id,
    "assessmentWorkspacePolicies",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/student-view")
      ?.id,
    "assessmentWorkspaceStudentView",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/delivery-check"),
    undefined,
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/release"),
    undefined,
  );
  assert.equal(
    routeContractForPathname(
      "/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/presentations/0123456789abcdef0123456789abcdef",
    ),
    undefined,
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/edit"),
    undefined,
  );
  assert.equal(productRoleMayAccessRoute("assessmentOverview", "student"), true);
  assert.equal(productRoleMayAccessRoute("assessmentOverview", "instructor"), false);
  assert.equal(productRoleMayAccessRoute("courseAssessments", "student"), false);
  assert.equal(productRoleMayAccessRoute("courseAssessments", "instructor"), true);
  assert.equal(productRoleMayAccessRoute("assessmentWorkspaceOverview", "student"), false);
  assert.equal(productRoleMayAccessRoute("assessmentWorkspaceOverview", "instructor"), true);
  assert.equal(routeContractForPathname("/workspace"), undefined);
  assert.equal(routeContractForPathname("/workspace/draft-1"), undefined);
  assert.equal(routeContractForPathname("/instructor/courses/C-1/grade-settings"), undefined);
  assert.equal(routeContractForPathname("/instructor/courses/C-1/teaching-operations"), undefined);
  assert.equal(productRoleMayAccessRoute("blueprintCourses", "student"), false);
  assert.equal(productRoleMayAccessRoute("blueprintCourses", "sysadmin"), false);
  assert.equal(productRoleMayAccessRoute("blueprintCourses", "instructor"), true);
  assert.equal(routeContractForPathname("/account-settings")?.id, "accountSettings");
  assert.equal(productRoleMayAccessRoute("accountSettings", "student"), true);
  assert.equal(productRoleMayAccessRoute("accountSettings", "instructor"), true);
  assert.equal(productRoleMayAccessRoute("accountSettings", "sysadmin"), true);
  assert.equal(routeContractForPathname("/account-settings/credentials"), undefined);
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
