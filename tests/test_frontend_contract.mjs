// WP-C9 contract tests for routes, typed mock client, and answer secrecy.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";
import { createMockLocalCredentialLogin } from "../src/api/mock/local_development_auth.ts";
import { createSessionBootstrap, sessionFailureState } from "../src/auth/session_context.tsx";
import { prefetchMatchesIssuedSuccessor } from "../src/features/attempt/prefetch_binding.ts";
import {
  rolesMayAccessRoute,
  routeContractForPathname,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";

const EXPECTED_ROUTE_PATHS = [
  "/",
  "/sign-in",
  "/auth/email/complete",
  "/auth/account/email/complete",
  "/course-invitations/redeem",
  "/account/security",
  "/courses/:courseRef",
  "/courses/:courseRef/assignments/:assignmentRef",
  "/runs/:runRef",
  "/runs/:runRef/summary",
  "/library",
  "/library/:problemRef",
  "/workspace",
  "/workspace/:workspaceRef",
  "/instructor/courses/:courseRef/assignments/new",
  "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
  "/instructor/courses/:courseRef/gradebook",
  "/instructor/courses/:courseRef/appearance",
  "/instructor/courses/:courseRef/students",
];

const INSTRUCTOR_ROUTE_IDS = [
  "library",
  "problemDetail",
  "workspaceList",
  "workspaceEditor",
  "assignmentCreate",
  "assignmentEditor",
  "gradebook",
  "courseAppearance",
  "courseRoster",
];

function materializeRoutePath(routePath) {
  return routePath
    .replaceAll(":courseRef", "C-1")
    .replaceAll(":assignmentRef", "A-1")
    .replaceAll(":runRef", "R-1")
    .replaceAll(":problemRef", "7K3-M9QP")
    .replaceAll(":workspaceRef", "W-1");
}

test("the product route data matches the frozen route contract", () => {
  assert.deepEqual(
    ROUTE_CONTRACT.map((route) => route.path),
    EXPECTED_ROUTE_PATHS,
  );
  assert.equal(new Set(ROUTE_CONTRACT.map((route) => route.id)).size, ROUTE_CONTRACT.length);
});

test("the route contract is the strict single source for instructor-only browser access", () => {
  const instructorRouteIds = ROUTE_CONTRACT.filter((route) => route.requiredRoles.length > 0).map(
    (route) => route.id,
  );

  assert.deepEqual(instructorRouteIds, INSTRUCTOR_ROUTE_IDS);
  for (const route of ROUTE_CONTRACT) {
    const restricted = INSTRUCTOR_ROUTE_IDS.includes(route.id);
    assert.equal(Object.hasOwn(route, "requiredRoles"), true);
    assert.deepEqual(route.requiredRoles, restricted ? ["instructor", "sysadmin"] : []);
    assert.equal(rolesMayAccessRoute(route.id, ["student"]), !restricted);
    assert.equal(rolesMayAccessRoute(route.id, ["instructor"]), true);
    assert.equal(rolesMayAccessRoute(route.id, ["sysadmin"]), true);
  }
});

test("declared path matching is derived from the route contract and fails closed", () => {
  for (const route of ROUTE_CONTRACT) {
    const pathname = materializeRoutePath(route.path);
    assert.equal(routeContractForPathname(pathname)?.id, route.id, pathname);
  }

  for (const pathname of [
    "",
    "library",
    "/library/",
    "/library/7K3-M9QP/extra",
    "/runs/R-1?source=unknown",
    "/instructor/courses/C-1/gradebook/extra",
    "/unknown-learning-space",
  ]) {
    assert.equal(routeContractForPathname(pathname), undefined, pathname);
  }
  assert.equal(rolesMayAccessRoute("unknownRoute", ["instructor", "sysadmin"]), false);
});

test("session bootstrap exposes only safe loading, authenticated, signed-out, and recovery states", async () => {
  const client = createMockApiClient();
  const bootstrap = createSessionBootstrap(
    client.getSession,
    createMockLocalCredentialLogin(),
    client.logout,
  );

  assert.deepEqual(bootstrap.state(), { kind: "loading" });
  await bootstrap.retry();
  assert.equal(bootstrap.state().kind, "authenticated");
  assert.equal("credential" in bootstrap.state(), false);
  assert.equal("answer" in bootstrap.state(), false);

  assert.equal(await bootstrap.signOut(), true);
  assert.deepEqual(bootstrap.state(), { kind: "signedOut" });

  assert.deepEqual(sessionFailureState({ status: 401 }), { kind: "expired" });
  assert.deepEqual(sessionFailureState({ status: 403 }), { kind: "expired" });
  assert.deepEqual(sessionFailureState(new Error("offline")), { kind: "error" });
});

test("local sign-in exchanges the credential once and retains only the safe session", async () => {
  const client = createMockApiClient();
  const bootstrap = createSessionBootstrap(client.getSession, createMockLocalCredentialLogin());

  assert.equal(bootstrap.localCredentialSignInAvailable, true);
  assert.equal(await bootstrap.signInWithLocalCredential("local-only-token"), true);
  assert.equal(bootstrap.state().kind, "authenticated");
  assert.equal("credential" in bootstrap.state(), false);
});

test("production session bootstrap cannot expose or invoke local credential sign-in", async () => {
  const client = createMockApiClient();
  const bootstrap = createSessionBootstrap(client.getSession);

  assert.equal(bootstrap.localCredentialSignInAvailable, false);
  assert.equal(await bootstrap.signInWithLocalCredential("local-only-token"), false);
  assert.deepEqual(bootstrap.state(), { kind: "loading" });
});

test("failed sign-out keeps the authenticated session visible for a safe retry", async () => {
  const client = createMockApiClient();
  const bootstrap = createSessionBootstrap(
    client.getSession,
    createMockLocalCredentialLogin(),
    () => Promise.reject(new Error("server unavailable")),
  );
  await bootstrap.retry();

  assert.equal(await bootstrap.signOut(), false);
  assert.equal(bootstrap.state().kind, "authenticated");
});

test("the shell keeps client navigation role-aware and never echoes route exceptions", () => {
  const source = fs.readFileSync("src/app.tsx", "utf8");

  assert.match(source, /useSessionBootstrap/);
  assert.match(source, /canUseAuthoringTools/);
  assert.match(source, /queueMicrotask\(focusMainContent\)/);
  assert.match(source, /<Show when={location\.pathname} keyed>/);
  assert.doesNotMatch(source, /String\(error\)/);
  assert.doesNotMatch(source, /error\.message/);
  assert.doesNotMatch(source, /localStorage/);
});

test("pagination skip links bypass router interception while retaining same-context fragments", () => {
  for (const sourcePath of [
    "src/pages/course_assignments_page.tsx",
    "src/pages/gradebook_page.tsx",
  ]) {
    const source = fs.readFileSync(sourcePath, "utf8");
    assert.match(
      source,
      /class="skip-link" href="#(?:assignment|gradebook)-pagination" target="_self"/u,
    );
  }
});

test("the typed mock client loads a complete run screen with no backend", async () => {
  const client = createMockApiClient();
  const activeRun = await client.startRun(publishedProblemFixture.assignment.id);
  const screen = await client.getRunScreen(activeRun.id);

  assert.equal(screen.run.completedAt, null);
  assert.equal(screen.attempt.run, activeRun.id);
  assert.equal(screen.attempt.seed, 1004);
  assert.equal(screen.issuedQuestion.version, screen.attempt.questionVersion);
  assert.equal(screen.issuedQuestion.seed, screen.attempt.seed);
  assert.equal(screen.issuedQuestion.response.kind, "multipleChoice");
  assert.deepEqual(Object.keys(screen.issuedQuestion).sort(), [
    "prompt",
    "response",
    "seed",
    "title",
    "version",
  ]);
  assert.equal(
    screen.issuedQuestion.title,
    publishedProblemFixture.publishedProblem.metadata.title,
  );
  assert.equal("question" in screen, false);
  assert.equal("source" in screen.issuedQuestion, false);
  assert.equal("grading" in screen.issuedQuestion, false);
  assert.equal("answer" in screen.issuedQuestion, false);
  assert.equal("answerKey" in screen.issuedQuestion, false);
});

test("the typed mock client exposes cursor-ready assignment run history", async () => {
  const client = createMockApiClient();
  const history = await client.listRuns(publishedProblemFixture.enrollment.id);

  assert.deepEqual(history.items, publishedProblemFixture.runs);
  assert.equal(history.nextCursor, null);
});

test("activity wire fields keep authenticated identity and assignment position explicit", () => {
  assert.notEqual(
    publishedProblemFixture.enrollment.user,
    publishedProblemFixture.enrollment.student,
  );
  assert.ok(publishedProblemFixture.attempts.every((attempt) => attempt.assignmentPosition === 0));
});

test("catalog browse returns hot metadata and cursor-paged taxonomy", async () => {
  const client = createMockApiClient();
  const catalog = await client.listProblems();
  const summary = catalog.items[0];

  assert.notEqual(summary, undefined);
  assert.equal(summary.questionId, publishedProblemFixture.catalogProblem.questionId);
  assert.equal(summary.backend, "native");
  assert.equal("prompt" in summary, false);
  assert.equal("response" in summary, false);
  assert.equal(catalog.nextCursor, null);

  const taxonomy = await client.listTaxonomy();
  assert.deepEqual(taxonomy.items, publishedProblemFixture.publishedProblem.metadata.taxonomy);
  assert.equal(taxonomy.nextCursor, null);
});

test("course browse carries membership role and safe Question ID assignment summaries", async () => {
  const client = createMockApiClient();
  const courses = await client.listCourses();
  const course = courses.items[0];

  assert.notEqual(course, undefined);
  assert.equal(course.role, "student");

  const assignments = await client.listAssignments(course.id);
  const assignment = assignments.items[0];
  assert.notEqual(assignment, undefined);
  assert.equal("courseId" in assignment, false);
  assert.equal("tenant" in assignment, false);
  assert.equal("disclosurePolicy" in assignment, false);
  assert.deepEqual(
    assignment.items.map((item) => item.questionId),
    [publishedProblemFixture.catalogProblem.questionId],
  );
  assert.equal("prompt" in assignment, false);
  assert.equal("response" in assignment, false);
});

test("mock fallback validation reports shape only and never correctness", async () => {
  const client = createMockApiClient();
  const definition = publishedProblemFixture.publishedProblem.response;
  assert.equal(definition.kind, "multipleChoice");
  if (definition.kind !== "multipleChoice") {
    return;
  }

  const empty = await client.validateResponseFormatOnServer(definition, {
    kind: "multipleChoice",
    selected: [],
  });
  assert.deepEqual(empty.violations, [
    { kind: "selectionCount", expected: { kind: "exactlyOne" }, actual: 0 },
  ]);

  const structurallyValid = await client.validateResponseFormatOnServer(definition, {
    kind: "multipleChoice",
    selected: ["carbonyl"],
  });
  assert.deepEqual(structurallyValid, { violations: [] });
  assert.equal("correct" in structurallyValid, false);
});

test("mock timer fallback follows inclusive grace and pause boundaries", async () => {
  const client = createMockApiClient();
  const policy = { kind: "perQuestion", seconds: 9, graceSeconds: 2 };
  const cases = [
    {
      name: "pause extension keeps a submission on time",
      submittedAt: 11_500,
      evaluatedAt: 11_500,
      pauseExtensionMillis: 2_000,
      expected: "submittedOnTime",
    },
    {
      name: "inclusive grace boundary remains acceptable",
      submittedAt: 12_000,
      evaluatedAt: 12_000,
      pauseExtensionMillis: 0,
      expected: "submittedWithinGrace",
    },
    {
      name: "one millisecond after grace times out",
      submittedAt: 12_001,
      evaluatedAt: 12_001,
      pauseExtensionMillis: 0,
      expected: "timedOut",
    },
  ];

  for (const timerCase of cases) {
    const verdict = await client.timerVerdictOnServer({
      policy,
      timer: {
        issuedAt: 1_000,
        deadline: 10_000,
        submittedAt: timerCase.submittedAt,
      },
      evaluatedAt: timerCase.evaluatedAt,
      pauseExtensionMillis: timerCase.pauseExtensionMillis,
    });
    assert.equal(verdict, timerCase.expected, timerCase.name);
  }
});

test("mock capability fallback names every missing requirement", async () => {
  const client = createMockApiClient();
  const violations = await client.validateAssignmentConfigOnServer({
    questions: [
      {
        question: publishedProblemFixture.publishedProblem,
        backendCapabilities: [],
      },
    ],
    requiredCapabilities: ["printExport"],
  });

  assert.deepEqual(
    violations.map((violation) => violation.capability),
    ["algorithmicGeneration", "serverGrading", "printExport"],
  );
});

test("the generated browser surface contains no answer-bearing type", () => {
  const apiDirectory = path.resolve("generated/api");
  const files = fs.readdirSync(apiDirectory).filter((filename) => filename.endsWith(".ts"));
  const forbiddenFilename = /answer[_-]?key|correct[_-]?response|solution[_-]?key/i;
  const forbiddenType = /\b(?:AnswerKey|CorrectResponse|SolutionKey)\b/;
  const forbiddenBoundary = /crates\/grading|\.\.\/grading|grading::/;

  assert.ok(files.length > 0, "generated API surface is empty");
  for (const filename of files) {
    assert.doesNotMatch(filename, forbiddenFilename);
    const source = fs.readFileSync(path.join(apiDirectory, filename), "utf8");
    assert.doesNotMatch(source, forbiddenType, filename);
    assert.doesNotMatch(source, forbiddenBoundary, filename);
  }
});

test("the learner route renders only its issued envelope through question-agnostic boundaries", () => {
  const source = fs.readFileSync("src/pages/run_page.tsx", "utf8");

  assert.match(source, /<QuestionRenderer/);
  assert.match(source, /<ResponseWidget/);
  assert.match(source, /createAttemptStateMachine/);
  assert.match(source, /useCourseThemeRouteData/);
  assert.doesNotMatch(source, /runtime\.queries\.runScreen/);
  assert.doesNotMatch(source, /MultipleChoiceResponse/);
  assert.doesNotMatch(source, /QuestionDefinition/);
  assert.doesNotMatch(source, /answerKey|correctResponse|grading/iu);
});

test("the learner route binds stable recovery rather than creating a key for each click", () => {
  const source = fs.readFileSync("src/pages/run_page.tsx", "utf8");

  assert.match(source, /attemptStorage\(\)/);
  assert.match(source, /retryWhenOnline/);
  assert.match(source, /ApiRequestError/);
  assert.match(source, /status === 401/);
  assert.match(source, /Start another practice/);
  assert.doesNotMatch(source, /onSubmit={[\s\S]{0,300}crypto\.randomUUID/);
});

test("a prefetched successor is usable only when the committed receipt binds it", () => {
  const predecessor = "0198e000-0000-7000-8000-000000000030";
  const cached = {
    predecessor,
    run: "0198e000-0000-7000-8000-000000000023",
    assignmentPosition: 2,
    questionVersion: "0198e000-0000-7000-8000-000000000041",
    seed: 42,
    renderedQuestionSha256: "a".repeat(64),
  };
  const issued = {
    id: "0198e000-0000-7000-8000-000000000031",
    run: cached.run,
    assignmentPosition: cached.assignmentPosition,
    questionVersion: cached.questionVersion,
    seed: cached.seed,
    deadline: null,
    renderedQuestionSha256: cached.renderedQuestionSha256,
  };
  const mismatches = [
    { ...cached, predecessor: "0198e000-0000-7000-8000-000000000099" },
    { ...cached, run: "0198e000-0000-7000-8000-000000000099" },
    { ...cached, assignmentPosition: 3 },
    { ...cached, questionVersion: "0198e000-0000-7000-8000-000000000099" },
    { ...cached, seed: 43 },
    { ...cached, renderedQuestionSha256: "b".repeat(64) },
  ];

  assert.equal(prefetchMatchesIssuedSuccessor(cached, issued, predecessor), true);
  assert.equal(
    mismatches.every(
      (candidate) => !prefetchMatchesIssuedSuccessor(candidate, issued, predecessor),
    ),
    true,
  );
});

test("a receipt with no successor completes locally without loading an active run screen", () => {
  const source = fs.readFileSync("src/pages/run_page.tsx", "utf8");

  assert.match(
    source,
    /if \(receiptNext === null\) \{\s+machine\.complete\(\);\s+setSummaryVisible\(true\);\s+void loadSummary\(\);\s+return;/,
  );
  assert.match(source, /function matchesIssuedSuccessor\(attempt: QuestionAttempt/);
  assert.match(source, /attempt\.id === receipt\.id/);
  assert.match(source, /attempt\.run === receipt\.run/);
  assert.match(source, /attempt\.assignmentPosition === receipt\.assignmentPosition/);
  assert.match(source, /attempt\.questionVersion === receipt\.questionVersion/);
  assert.match(source, /attempt\.seed === receipt\.seed/);
  assert.match(source, /attempt\.timer\.deadline === receipt\.deadline/);
  assert.match(
    source,
    /attempt\.provenance\.renderedQuestionSha256 === receipt\.renderedQuestionSha256/,
  );
  assert.match(source, /if \(!matchesIssuedSuccessor\(next\.attempt, receiptNext\)\)/);
  assert.match(
    source,
    /if \(advancedScreen === null \|\| advancedAttemptId === null\) return;\s+setScreen\(advancedScreen\);/,
  );
});

test("a cache-hit successor keeps summary projection bound to the advanced attempt context", () => {
  const source = fs.readFileSync("src/pages/run_page.tsx", "utf8");

  assert.match(
    source,
    /const currentAttemptId = \(\): string =>\s*currentState\(\)\?\.context\.attemptId \?\? screen\(\)\.attempt\.id/,
  );
  assert.match(source, /outcome\.attempt === currentAttemptId\(\)/);
  assert.doesNotMatch(source, /outcome\.attempt === screen\(\)\.attempt\.id/);
});

test("the learner remounts response entry only for a distinct server-issued attempt", () => {
  const source = fs.readFileSync("src/pages/run_page.tsx", "utf8");

  assert.match(source, /when={currentState\(\)\?\.context\.attemptId}\s+keyed/);
  assert.match(source, /attemptId={attemptId}/);
  assert.match(source, /initialResponse={currentState\(\)\?\.response \?\? undefined}/);
});

test("a prefetch response cannot warm assets unless its transport and page bindings hold", () => {
  const source = fs.readFileSync("src/pages/run_page.tsx", "utf8");

  assert.match(source, /value\.run !== machine\.state\(\)\.context\.runId/);
  assert.match(source, /const MAX_PREFETCH_ASSETS = 12/);
  assert.match(source, /new Set\(/);
  assert.match(source, /\.slice\(0, MAX_PREFETCH_ASSETS\)/);
});
