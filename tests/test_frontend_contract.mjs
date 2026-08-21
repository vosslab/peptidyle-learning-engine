// Pure route, session-state, and issued-question contract evidence.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import { createSessionBootstrap, sessionFailureState } from "../src/auth/session_context.tsx";
import { prefetchMatchesIssuedSuccessor } from "../src/features/attempt/prefetch_binding.ts";
import {
  rolesMayAccessRoute,
  routeContractForPathname,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";

test("route contracts fail closed and reserve authoring routes for teaching roles", () => {
  assert.equal(routeContractForPathname("/library/7K3-M9QP")?.id, "problemDetail");
  assert.equal(routeContractForPathname("/library/7K3-M9QP/extra"), undefined);
  assert.equal(rolesMayAccessRoute("workspaceEditor", ["student"]), false);
  assert.equal(rolesMayAccessRoute("workspaceEditor", ["instructor"]), true);
  assert.ok(ROUTE_CONTRACT.length > 0);
});

test("session bootstrap retains only safe session state with direct narrow dependencies", async () => {
  const session = {
    authenticated: true,
    tenant: "tenant-a",
    user: { id: "user-a", displayName: "Ada", roles: ["student"] },
  };
  const bootstrap = createSessionBootstrap(
    async () => session,
    async () => session,
    async () => undefined,
  );
  await bootstrap.retry();
  assert.equal(bootstrap.state().kind, "authenticated");
  assert.equal("credential" in bootstrap.state(), false);
  assert.equal(await bootstrap.signOut(), true);
  assert.deepEqual(sessionFailureState({ status: 401 }), { kind: "expired" });
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
    run: "run-a",
    assignmentPosition: 1,
    questionVersion: "version-b",
    seed: 2,
    renderedQuestionSha256: "a".repeat(64),
  };
  const issued = { ...successor };
  assert.equal(prefetchMatchesIssuedSuccessor(successor, issued, "attempt-a"), true);
  assert.equal(
    prefetchMatchesIssuedSuccessor({ ...successor, seed: 3 }, issued, "attempt-a"),
    false,
  );
});
