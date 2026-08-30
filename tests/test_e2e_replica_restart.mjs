import assert from "node:assert/strict";
import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import {
  LocalhostHttpsDispatcher,
  authenticateMary,
  decodeSeedManifest,
  decodeServiceInput,
  readServiceInput,
} from "./e2e/e2e_replica_restart_child.mjs";

const COURSE_ID = "00000000-0000-4000-8000-000000000201";

function inputValue(workspacePath) {
  return {
    schemaVersion: 1,
    oracle: "replica_restart",
    baseUrl: "https://localhost:55001/",
    manifestPath: join(workspacePath, "disposable.manifest"),
    seedManifestPath: join(workspacePath, "service-oracle-seed-manifest.json"),
    workspacePath,
  };
}

test("replica child accepts only canonical mode-0600 V1 input", async () => {
  const workspace = await mkdtemp(join(tmpdir(), "ple-replica-child-"));
  try {
    const value = inputValue(workspace);
    const path = join(workspace, "service-oracle-input.json");
    await writeFile(path, JSON.stringify(value), { encoding: "ascii", mode: 0o600 });
    assert.deepEqual(await readServiceInput(path), value);
    await writeFile(path, `${JSON.stringify(value)}\n`, { encoding: "ascii", mode: 0o600 });
    await assert.rejects(() => readServiceInput(path), /canonical JSON/u);
    await writeFile(path, JSON.stringify(value), { encoding: "ascii" });
    await chmod(path, 0o644);
    await assert.rejects(() => readServiceInput(path), /file is unsafe/u);
  } finally {
    await rm(workspace, { recursive: true });
  }
});

test("replica child rejects ABI extension fields, foreign origins, and path escape", () => {
  const value = inputValue("/private/workspace");
  assert.throws(() => decodeServiceInput({ ...value, project: "foreign" }), /invalid shape/u);
  assert.throws(
    () => decodeServiceInput({ ...value, baseUrl: "https://example.test:55001/" }),
    /baseUrl/u,
  );
  assert.throws(
    () => decodeServiceInput({ ...value, manifestPath: "/private/other/manifest" }),
    /fixed workspace/u,
  );
});

test("replica seed manifest requires the exact scoped course and assignment shape", () => {
  const value = {
    assignmentId: "00000000-0000-4000-8000-000000000202",
    courseId: COURSE_ID,
    enrollmentId: "00000000-0000-4000-8000-000000000203",
    problemId: "00000000-0000-4000-8000-000000000204",
    questionId: "ABC-DEFG",
    versionId: "00000000-0000-4000-8000-000000000205",
  };
  assert.deepEqual(decodeSeedManifest(value), value);
  assert.throws(() => decodeSeedManifest({ ...value, privateScope: "private" }), /invalid shape/u);
});

test("localhost TLS dispatcher is request-local and refuses foreign requests", async () => {
  const dispatcher = new LocalhostHttpsDispatcher("https://localhost:55001/");
  try {
    assert.equal(dispatcher.origin, "https://localhost:55001");
    assert.equal(dispatcher.agent.options.rejectUnauthorized, false);
    await assert.rejects(() => dispatcher.request("https://example.test/api"), /foreign request/u);
  } finally {
    dispatcher.close();
  }
});

test("Mary production auth uses exact Origin, exact seeded course, and two secure cookies", async () => {
  const calls = [];
  const accountCookie = "__Host-ple_account_session=" + "a".repeat(32);
  const courseCookie = "__Host-ple_session=" + "b".repeat(32);
  const responses = [
    {
      status: 200,
      headers: { "set-cookie": [`${accountCookie}; Path=/; HttpOnly; Secure; SameSite=Strict`] },
      body: '{"authenticated":true}',
    },
    {
      status: 200,
      headers: {},
      body: JSON.stringify({
        courses: [
          {
            courseId: COURSE_ID,
            courseReference: "C-ABC-DEFG",
            title: "PLE replica E2E course",
            role: "student",
          },
        ],
        nextCursor: null,
      }),
    },
    {
      status: 200,
      headers: { "set-cookie": [`${courseCookie}; Path=/; HttpOnly; Secure; SameSite=Strict`] },
      body: JSON.stringify({ authenticated: true, courseId: COURSE_ID, role: "student" }),
    },
  ];
  const dispatcher = {
    async request(path, options) {
      calls.push({ path, options });
      return responses.shift();
    },
  };
  const cookie = await authenticateMary(dispatcher, "https://localhost:55001/", COURSE_ID);
  assert.equal(cookie, `${accountCookie}; ${courseCookie}`);
  assert.deepEqual(
    calls.map((call) => [call.path, call.options.method ?? "GET"]),
    [
      ["/api/auth/live-demo/accounts", "POST"],
      ["/api/auth/account/courses", "GET"],
      ["/api/auth/account/course-session", "POST"],
    ],
  );
  assert.equal(calls[0].options.headers.origin, "https://localhost:55001");
  assert.equal(calls[0].options.body, '{"persona":"maryStudent"}');
  assert.equal(calls[1].options.headers.cookie, accountCookie);
  assert.equal(calls[2].options.headers.cookie, accountCookie);
  assert.equal(calls[2].options.headers.origin, "https://localhost:55001");
});
