#!/usr/bin/env node
/**
 * Proves that a normal learner session survives loss of the API replica that
 * issued it.  This is intentionally a host-side, container-dependent test:
 * every learner request below goes through Caddy and seed data comes from the
 * host-only xtask command, never a product fixture route.
 *
 * `--static-check` is the permanent, no-container gate.  The default command
 * is deliberately strict: missing Podman is BLOCKED and exits non-zero rather
 * than turning a deployment prerequisite into a passing skip.
 */

import assert from "node:assert/strict";
import { createHash, randomBytes, randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const REPO_ROOT = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const COMPOSE_FILES = ["containers/compose.yaml", "tests/e2e/compose.replica-e2e.yaml"];
const POLL_TIMEOUT_MS = 45_000;
const REPLICA_REFRESH_MS = 2_500;
const REPLICA_HEADER_PREFIX = "ple-replica-e2e-api-";
const POSTGRES_USER = "ple_e2e";
const POSTGRES_DATABASE = "ple_e2e";
const POSTGRES_PASSWORD = "ple-e2e-local-only";

function fail(message) {
  throw new Error(message);
}

function requireUuid(value, label) {
  assert.match(
    value,
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    label,
  );
  return value;
}

function parseManifest(text) {
  let manifest;
  try {
    manifest = JSON.parse(text.trim());
  } catch {
    fail("e2e seed did not return JSON");
  }
  assert.deepEqual(Object.keys(manifest).sort(), [
    "assignmentId",
    "enrollmentId",
    "problemId",
    "versionId",
  ]);
  for (const [name, value] of Object.entries(manifest)) requireUuid(value, `seed manifest ${name}`);
  return manifest;
}

function validVisibleResponse(response) {
  assert.equal(
    typeof response?.kind,
    "string",
    "issued envelope needs a visible response definition",
  );
  switch (response.kind) {
    case "numeric":
      return { kind: "numeric", value: 0 };
    case "multipleChoice": {
      assert.equal(
        response.selection?.kind,
        "exactlyOne",
        "E2E multiple choice must require one visible choice",
      );
      assert.equal(typeof response.choices?.[0]?.id, "string", "E2E choice needs a visible id");
      return { kind: "multipleChoice", selected: [response.choices[0].id] };
    }
    case "shortText":
      return { kind: "shortText", text: "e2e-visible-response" };
    case "ordering": {
      assert.ok(
        Array.isArray(response.items) && response.items.length > 0,
        "ordering needs visible items",
      );
      return { kind: "ordering", order: response.items.map((item) => item.id) };
    }
    case "externalTool":
      return { kind: "externalTool" };
    default:
      fail(`unsupported visible E2E response kind ${response.kind}`);
  }
}

function parseReplica(value) {
  assert.match(
    value ?? "",
    /^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$/,
    "missing or malformed X-PLE-E2E-Replica header",
  );
  return value;
}

function composeArguments(project, envPath, tail) {
  return [
    "compose",
    "-p",
    project,
    "--env-file",
    envPath,
    ...COMPOSE_FILES.flatMap((path) => ["-f", path]),
    ...tail,
  ];
}

function cleanupArguments(project, envPath) {
  // This is generated here rather than accepted from a caller.  The guard
  // makes the only destructive operation unable to target a developer stack.
  assert.match(project, /^ple-replica-e2e-[a-f0-9]{10}$/);
  return composeArguments(project, envPath, ["down", "--volumes", "--remove-orphans"]);
}

async function withProjectCleanup(project, envPath, operation, cleanup) {
  // Validate the generated project before the callback can make its first
  // Compose call, not only when failure later reaches cleanup.
  const cleanupArgs = cleanupArguments(project, envPath);
  let composeAttempted = false;
  const armCleanup = () => {
    composeAttempted = true;
  };
  try {
    return await operation(armCleanup);
  } finally {
    if (composeAttempted) await cleanup(cleanupArgs);
  }
}

async function command(file, args, label, { allowFailure = false } = {}) {
  try {
    return await execFileAsync(file, args, {
      cwd: REPO_ROOT,
      encoding: "utf8",
      maxBuffer: 4 * 1024 * 1024,
    });
  } catch (error) {
    if (allowFailure)
      return {
        failed: true,
        code: error.code,
        stdout: error.stdout ?? "",
        stderr: error.stderr ?? "",
      };
    // Do not include argv/stdout/stderr here: the seed command receives a DB
    // URL and the test owns a local identity credential.
    fail(`${label} failed (${String(error.code ?? "unknown error")})`);
  }
}

async function waitFor(label, operation, timeoutMs = POLL_TIMEOUT_MS) {
  const deadline = Date.now() + timeoutMs;
  let lastError = "not ready";
  while (Date.now() < deadline) {
    try {
      return await operation();
    } catch (error) {
      lastError = error instanceof Error ? error.message : "not ready";
      await new Promise((resolveWait) => setTimeout(resolveWait, 400));
    }
  }
  fail(`${label} did not become ready: ${lastError}`);
}

async function fetchJson(url, options = {}) {
  const response = await fetch(url, { ...options, signal: AbortSignal.timeout(5_000) });
  const text = await response.text();
  if (!response.ok)
    fail(`${options.method ?? "GET"} ${new URL(url).pathname} returned ${response.status}`);
  try {
    return { response, body: JSON.parse(text) };
  } catch {
    fail(`${new URL(url).pathname} did not return JSON`);
  }
}

async function unusedLoopbackPort() {
  const { createServer } = await import("node:net");
  const server = createServer();
  await new Promise((resolveListen, reject) => {
    server.once("error", reject);
    server.listen({ host: "127.0.0.1", port: 0 }, resolveListen);
  });
  const address = server.address();
  assert.ok(address && typeof address === "object");
  const port = address.port;
  await new Promise((resolveClose, reject) =>
    server.close((error) => (error ? reject(error) : resolveClose())),
  );
  return port;
}

function identityFile(tenantId, studentId) {
  const credential = randomBytes(32).toString("base64url");
  assert.equal(credential.length, 43, "local credential must be canonical 32-byte base64url");
  return {
    credential,
    body: JSON.stringify({
      credentials: [
        {
          credentialSha256: createHash("sha256").update(credential).digest("hex"),
          tenantId,
          userId: studentId,
          displayName: "Replica E2E learner",
          roles: ["student"],
        },
      ],
    }),
  };
}

async function inspectReplicaContainer(project, envPath, replica) {
  assert.ok(replica.startsWith(REPLICA_HEADER_PREFIX), "unexpected replica attribution prefix");
  const suffix = replica.slice(REPLICA_HEADER_PREFIX.length);
  assert.match(suffix, /^[a-f0-9]{12}$/, "replica attribution must carry a container short ID");
  const ids = (
    await command(
      "podman",
      composeArguments(project, envPath, ["ps", "-q", "api"]),
      "listing API replicas",
    )
  ).stdout
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  assert.ok(ids.length >= 2, "expected two API replicas");
  for (const id of ids) {
    // The server accepts only the twelve hex characters from its container
    // hostname and stamps a fixed prefix. Match that suffix against IDs from
    // this Compose project, never a global container listing.
    if (id.startsWith(suffix)) return id;
  }
  fail("issued replica header did not map to this Compose project's API containers");
}

async function postgresCounts(project, envPath, tenantId, attemptId) {
  const sql = [
    "SELECT",
    "(SELECT count(*) FROM question_attempt WHERE tenant_id = :'tenant'::uuid AND attempt_id = :'attempt'::uuid),",
    "(SELECT count(*) FROM submission WHERE tenant_id = :'tenant'::uuid AND attempt_id = :'attempt'::uuid),",
    "(SELECT count(*) FROM submission_idempotency WHERE tenant_id = :'tenant'::uuid AND attempt_id = :'attempt'::uuid),",
    "(SELECT count(*) FROM submission_evaluation WHERE tenant_id = :'tenant'::uuid AND attempt_id = :'attempt'::uuid),",
    "(SELECT count(*) FROM attempt_score_current WHERE tenant_id = :'tenant'::uuid AND attempt_id = :'attempt'::uuid);",
  ].join(" ");
  const result = await command(
    "podman",
    composeArguments(project, envPath, [
      "exec",
      "-T",
      "postgres",
      "psql",
      "-U",
      POSTGRES_USER,
      "-d",
      POSTGRES_DATABASE,
      "-v",
      `tenant=${tenantId}`,
      "-v",
      `attempt=${attemptId}`,
      "-tA",
      "-F",
      "|",
      "-c",
      sql,
    ]),
    "checking durable submission rows",
  );
  assert.equal(
    result.stdout.trim(),
    "1|1|1|1|1",
    "expected one scoped attempt, submission, idempotency, evaluation, and current score record",
  );
}

async function staticCheck() {
  assert.equal(
    parseReplica("ple-replica-e2e-api-0123456789ab"),
    "ple-replica-e2e-api-0123456789ab",
  );
  assert.throws(() => parseReplica("bad header"));
  assert.deepEqual(validVisibleResponse({ kind: "numeric" }), { kind: "numeric", value: 0 });
  assert.deepEqual(
    validVisibleResponse({
      kind: "multipleChoice",
      selection: { kind: "exactlyOne" },
      choices: [{ id: "shown-choice" }],
    }),
    { kind: "multipleChoice", selected: ["shown-choice"] },
  );
  // These UUIDv5-shaped values are the actual `xtask e2e-seed` manifest for
  // tenant 0198e000-0000-7000-8000-000000000001. The seeder deliberately
  // derives deterministic IDs, so v4-only validation would reject live data.
  const manifest = parseManifest(
    JSON.stringify({
      assignmentId: "a8a0b288-690e-51d9-ae1c-e6a553473070",
      enrollmentId: "931d6323-ed71-5bec-ab6f-861f5c55cbc2",
      problemId: "00e324e2-7505-558d-b025-6f57fd5d3aca",
      versionId: "0c2813cc-bb38-5c02-b3c5-b4361d19976d",
    }),
  );
  assert.equal(manifest.versionId, "0c2813cc-bb38-5c02-b3c5-b4361d19976d");
  const args = cleanupArguments("ple-replica-e2e-0123456789", "/tmp/identities.env");
  assert.deepEqual(args.slice(0, 6), [
    "compose",
    "-p",
    "ple-replica-e2e-0123456789",
    "--env-file",
    "/tmp/identities.env",
    "-f",
  ]);
  assert.ok(args.includes("--volumes"), "cleanup uses volumes only with its explicit project name");
  assert.throws(() => cleanupArguments("ple", "/tmp/identities.env"));
  let observedCleanup;
  await assert.rejects(
    withProjectCleanup(
      "ple-replica-e2e-0123456789",
      "/tmp/identities.env",
      async (armCleanup) => {
        armCleanup();
        throw new Error("simulated first compose up failure");
      },
      async (cleanup) => {
        observedCleanup = cleanup;
      },
    ),
    /simulated first compose up failure/,
  );
  assert.deepEqual(observedCleanup, args, "a failed first compose up must still clean the project");
  console.log("replica_restart static check passed");
}

async function runLive() {
  const podman = await command("podman", ["info", "--format", "{{.Host.OS}}"], "checking Podman", {
    allowFailure: true,
  });
  if (podman.failed) {
    console.error("BLOCKED: Podman machine is unavailable; replica restart E2E was not run.");
    process.exitCode = 2;
    return;
  }
  const gatewayDigest = process.env.PLE_E2E_GATEWAY_IMAGE_SHA256;
  if (!/^[a-f0-9]{64}$/.test(gatewayDigest ?? "")) {
    fail(
      "PLE_E2E_GATEWAY_IMAGE_SHA256 must be a 64-character pinned Caddy digest for the live E2E",
    );
  }

  const project = `ple-replica-e2e-${randomBytes(5).toString("hex")}`;
  const tempDirectory = await mkdtemp(join(tmpdir(), "ple-replica-e2e-"));
  const envPath = join(tempDirectory, "compose.env");
  const identityPath = join(tempDirectory, "local-identities.json");
  try {
    const [postgresPort, minioPort, minioConsolePort, gatewayPort] = await Promise.all([
      unusedLoopbackPort(),
      unusedLoopbackPort(),
      unusedLoopbackPort(),
      unusedLoopbackPort(),
    ]);
    const tenantId = randomUUID();
    const instructorId = randomUUID();
    const studentId = randomUUID();
    const identity = identityFile(tenantId, studentId);
    await writeFile(identityPath, identity.body, { mode: 0o600 });
    await writeFile(
      envPath,
      [
        `POSTGRES_USER=${POSTGRES_USER}`,
        `POSTGRES_PASSWORD=${POSTGRES_PASSWORD}`,
        `POSTGRES_DB=${POSTGRES_DATABASE}`,
        "MINIO_ROOT_USER=ple-e2e-minio",
        "MINIO_ROOT_PASSWORD=ple-e2e-minio-password",
        `PLE_POSTGRES_HOST_PORT=${postgresPort}`,
        `PLE_MINIO_API_HOST_PORT=${minioPort}`,
        `PLE_MINIO_CONSOLE_HOST_PORT=${minioConsolePort}`,
        `PLE_GATEWAY_HOST_PORT=${gatewayPort}`,
        `PLE_GATEWAY_IMAGE_SHA256=${gatewayDigest}`,
        `PLE_LOCAL_AUTH_HOST_FILE=${identityPath}`,
        `PLE_PUBLIC_ASSET_BASE_URL=http://127.0.0.1:${minioPort}/content`,
        "PLE_WEBWORK_RENDERER_IMAGE_REPOSITORY=example.invalid/unused-renderer",
        `PLE_WEBWORK_RENDERER_IMAGE_SHA256=${"0".repeat(64)}`,
        "PLE_WEBWORK_RENDERER_BASE_URL=http://webwork-renderer:8080",
        "PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS=15",
        "PLE_WEBWORK_MAX_RESPONSE_BYTES=1048576",
        "PLE_WEBWORK_RENDERER_HEALTHCHECK=true",
      ].join("\n") + "\n",
      { mode: 0o600 },
    );

    await withProjectCleanup(
      project,
      envPath,
      async (armCleanup) => {
        // Arm before invoking Compose: a partially-created initial stack still
        // owns project-scoped containers/volumes that must be removed.
        armCleanup();
        await command(
          "podman",
          composeArguments(project, envPath, ["up", "-d", "postgres", "minio", "createbuckets"]),
          "starting E2E backing services",
        );
        const seeded = await command(
          "cargo",
          [
            "run",
            "-q",
            "-p",
            "xtask",
            "--",
            "e2e-seed",
            "--database-url",
            `postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:${postgresPort}/${POSTGRES_DATABASE}`,
            "--tenant",
            tenantId,
            "--instructor",
            instructorId,
            "--student",
            studentId,
          ],
          "seeding E2E data",
        );
        const manifest = parseManifest(seeded.stdout);

        await command(
          "podman",
          composeArguments(project, envPath, ["up", "-d", "--scale", "api=2", "api", "gateway"]),
          "starting API replicas and gateway",
        );
        const baseUrl = `http://127.0.0.1:${gatewayPort}`;
        await waitFor("gateway", async () => {
          const response = await fetch(`${baseUrl}/health`, { signal: AbortSignal.timeout(2_000) });
          if (!response.ok) fail(`gateway health returned ${response.status}`);
        });

        const login = await fetchJson(`${baseUrl}/api/auth/login`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ credential: identity.credential }),
        });
        const cookie = login.response.headers.get("set-cookie")?.split(";", 1)[0];
        assert.match(
          cookie ?? "",
          /^ple_session=/,
          "local login did not establish a session cookie",
        );
        const studentHeaders = { cookie, "content-type": "application/json" };
        const startedRun = await fetchJson(`${baseUrl}/api/runs`, {
          method: "POST",
          headers: studentHeaders,
          body: JSON.stringify({ assignmentId: manifest.assignmentId }),
        });
        assert.equal(typeof startedRun.body.id, "string", "start run did not return a run id");
        const attempts = await fetchJson(
          `${baseUrl}/api/runs/${encodeURIComponent(startedRun.body.id)}/attempts`,
          {
            headers: { cookie },
          },
        );
        assert.ok(
          Array.isArray(attempts.body.items) && attempts.body.items.length === 1,
          "new E2E run needs exactly one initial attempt",
        );
        const attemptId = attempts.body.items[0]?.id;
        assert.equal(typeof attemptId, "string", "attempt id is missing");

        const initialQuestion = await fetchJson(
          `${baseUrl}/api/attempts/${encodeURIComponent(attemptId)}/question`,
          {
            headers: { cookie },
          },
        );
        const initialReplica = parseReplica(
          initialQuestion.response.headers.get("x-ple-e2e-replica"),
        );
        const initialEnvelope = initialQuestion.body;
        const stoppedContainer = await inspectReplicaContainer(project, envPath, initialReplica);
        await command(
          "podman",
          ["stop", stoppedContainer],
          "stopping the replica that issued the question",
        );
        await new Promise((resolveWait) => setTimeout(resolveWait, REPLICA_REFRESH_MS));

        const resumedQuestion = await waitFor("a distinct API replica after restart", async () => {
          const question = await fetchJson(
            `${baseUrl}/api/attempts/${encodeURIComponent(attemptId)}/question`,
            { headers: { cookie } },
          );
          const replica = parseReplica(question.response.headers.get("x-ple-e2e-replica"));
          if (replica === initialReplica) fail("Caddy still selected the stopped API replica");
          return { question, replica };
        });
        assert.notEqual(
          resumedQuestion.replica,
          initialReplica,
          "replay must cross an API replica boundary",
        );
        assert.deepEqual(
          resumedQuestion.question.body,
          initialEnvelope,
          "issued question envelope changed across replicas",
        );

        const response = validVisibleResponse(initialEnvelope.response);
        const idempotencyKey = `replica-e2e-${randomUUID()}`;
        const submissionOptions = {
          method: "POST",
          headers: { ...studentHeaders, "idempotency-key": idempotencyKey },
          body: JSON.stringify({ response }),
        };
        const firstReceipt = await fetchJson(
          `${baseUrl}/api/submissions/${encodeURIComponent(attemptId)}`,
          submissionOptions,
        );
        const replayReceipt = await fetchJson(
          `${baseUrl}/api/submissions/${encodeURIComponent(attemptId)}`,
          submissionOptions,
        );
        assert.deepEqual(
          replayReceipt.body,
          firstReceipt.body,
          "same idempotency key did not return the original durable receipt",
        );
        await postgresCounts(project, envPath, tenantId, attemptId);
        console.log(
          `replica restart E2E passed: ${initialReplica} -> ${resumedQuestion.replica}; exact envelope and durable replay verified.`,
        );
      },
      async (cleanup) => {
        await command("podman", cleanup, "cleaning E2E project", { allowFailure: true });
      },
    );
  } finally {
    await rm(tempDirectory, { recursive: true, force: true });
  }
}

if (process.argv.includes("--static-check")) {
  await staticCheck();
} else {
  await runLive();
}
