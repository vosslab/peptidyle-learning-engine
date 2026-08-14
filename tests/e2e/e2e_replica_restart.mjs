#!/usr/bin/env node
/**
 * Proves that a normal learner session survives loss of the API replica that
 * issued it.  This is intentionally a host-side, container-dependent test:
 * every learner request below goes through Caddy and seed data comes from the
 * host-only project-tools command, never a product fixture route.
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
const LOCAL_STACK_CONSUMER = resolve(REPO_ROOT, "local_stack_consumer.py");
const POLL_TIMEOUT_MS = 45_000;
const REPLICA_REFRESH_MS = 2_500;
const REPLICA_HEADER_PREFIX = "ple-replica-e2e-api-";
const POSTGRES_USER = "ple_e2e";
const POSTGRES_DATABASE = "ple_e2e";
const POSTGRES_PASSWORD = "ple-e2e-local-only";
const POSTGRES_IMAGE_SHA256 = "7958605b474b3d264a969cb3a123d6aa00ad1e1fe9da8a69984dabb704d93317";
const MINIO_IMAGE_SHA256 = "14cea493d9a34af32f524e538b8346cf79f3321eff8e708c1e2960462bd8936e";
const MINIO_MC_IMAGE_SHA256 = "a7fe349ef4bd8521fb8497f55c6042871b2ae640607cf99d9bede5e9bdf11727";
const SECRET_INIT_IMAGE_SHA256 = "48b0309ca019d89d40f670aa1bc06e426dc0931948452e8491e3d65087abc07d";
const WEBWORK_RENDERER_IMAGE =
  "localhost/pg-renderer@sha256:d606c4b5d82d425729643c4f36d093d549759a416d0527f0340ae0a7319a8456";

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

function safeComposeDiagnostic(stderr, privateValues = []) {
  let redacted = stderr;
  for (const privateValue of privateValues) {
    redacted = redacted.replaceAll(privateValue, "[redacted]");
  }
  return redacted
    .replaceAll(POSTGRES_PASSWORD, "[redacted]")
    .replaceAll("ple-e2e-minio-password", "[redacted]")
    .replace(/postgres:\/\/[^@\s]+@/gu, "postgres://[redacted]@")
    .slice(-2_000);
}

function adapterArguments(action, manifestPath, actionArguments = []) {
  return [LOCAL_STACK_CONSUMER, action, "--manifest", manifestPath, ...actionArguments];
}

async function adapterCommand(action, manifestPath, actionArguments, label, options = {}) {
  // Adapter failures do not echo raw Compose output. The only deployment
  // detail exposed by this runner comes from its bounded, redacted diagnostics
  // action below.
  const adapterOptions = { ...options, safeDiagnostics: false };
  return command(
    "python3",
    adapterArguments(action, manifestPath, actionArguments),
    label,
    adapterOptions,
  );
}

async function adapterCompose(manifestPath, tail, label, options = {}) {
  // The private adapter owns provider selection, the fixed Compose files,
  // target project, and environment isolation. The sentinel keeps a Compose
  // option such as `-d` in the structured tail rather than this adapter argv.
  return adapterCommand("compose", manifestPath, ["--", ...tail], label, options);
}

async function deploymentDiagnostics(manifestPath) {
  const result = await adapterCommand(
    "diagnostics",
    manifestPath,
    ["--service", "api", "--service", "gateway"],
    "reading E2E deployment diagnostics",
    { allowFailure: true },
  );
  return result.stdout ?? result.stderr ?? "";
}

function replicaIdPrefix(replica) {
  assert.ok(replica.startsWith(REPLICA_HEADER_PREFIX), "unexpected replica attribution prefix");
  const suffix = replica.slice(REPLICA_HEADER_PREFIX.length);
  assert.match(suffix, /^[a-f0-9]{12}$/, "replica attribution must carry a container short ID");
  return suffix;
}

async function stopIssuingReplica(manifestPath, replica) {
  const idPrefix = replicaIdPrefix(replica);
  await adapterCommand(
    "stop-instance",
    manifestPath,
    ["--service", "api", "--id-prefix", idPrefix],
    "stopping the replica that issued the question",
  );
}

async function command(
  file,
  args,
  label,
  {
    allowFailure = false,
    environment = {},
    privateValues = [],
    safeDiagnostics = false,
    timeoutMs,
  } = {},
) {
  try {
    return await execFileAsync(file, args, {
      cwd: REPO_ROOT,
      env: { ...process.env, ...environment },
      encoding: "utf8",
      maxBuffer: 4 * 1024 * 1024,
      timeout: timeoutMs,
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
    const diagnostic = safeDiagnostics
      ? safeComposeDiagnostic(error.stderr ?? "", privateValues)
      : "";
    fail(
      `${label} failed (${String(error.code ?? "unknown error")})${diagnostic ? `\n${diagnostic}` : ""}`,
    );
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

function safeHttpError(text) {
  if (Buffer.byteLength(text, "utf8") > 512) return "";
  let body;
  try {
    body = JSON.parse(text);
  } catch {
    return "";
  }
  if (
    body === null ||
    Array.isArray(body) ||
    typeof body !== "object" ||
    !Object.hasOwn(body, "error") ||
    Object.keys(body).length !== 1 ||
    typeof body.error !== "string" ||
    body.error.length === 0 ||
    body.error.length > 300 ||
    !/^[\x20-\x7e]+$/u.test(body.error)
  ) {
    return "";
  }
  return body.error;
}

async function fetchJson(url, options = {}) {
  const response = await fetch(url, { ...options, signal: AbortSignal.timeout(5_000) });
  const text = await response.text();
  if (!response.ok) {
    const detail = safeHttpError(text);
    fail(
      `${options.method ?? "GET"} ${new URL(url).pathname} returned ${response.status}${detail ? `: ${detail}` : ""}`,
    );
  }
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
  const credentialBytes = randomBytes(32);
  const credential = credentialBytes.toString("base64url");
  assert.equal(credential.length, 43, "local credential must be canonical 32-byte base64url");
  return {
    credential,
    body: JSON.stringify({
      credentials: [
        {
          credential_sha256: createHash("sha256").update(credentialBytes).digest("hex"),
          tenant_id: tenantId,
          user_id: studentId,
          display_name: "Replica E2E learner",
          roles: ["student"],
        },
      ],
    }),
  };
}

function canonicalSecret32() {
  const secret = randomBytes(32).toString("base64url");
  assert.equal(secret.length, 43, "secret must be canonical 32-byte base64url");
  return `${secret}\n`;
}

async function writePrivateSecret32(path) {
  await writeFile(path, canonicalSecret32(), { mode: 0o600 });
}

async function provisionGradingReader(manifestPath, password) {
  const sql = `ALTER ROLE ple_grading_reader PASSWORD '${password}';`;
  await adapterCompose(
    manifestPath,
    [
      "exec",
      "-T",
      "postgres",
      "psql",
      "-v",
      "ON_ERROR_STOP=1",
      "-U",
      POSTGRES_USER,
      "-d",
      POSTGRES_DATABASE,
      "-c",
      sql,
    ],
    "provisioning the E2E grading reader",
    { privateValues: [password], safeDiagnostics: true },
  );
}

async function postgresCounts(manifestPath, tenantId, attemptId) {
  const tenant = requireUuid(tenantId, "database count tenant");
  const attempt = requireUuid(attemptId, "database count attempt");
  const sql = [
    "SELECT",
    `(SELECT count(*) FROM question_attempt WHERE tenant_id = '${tenant}'::uuid AND attempt_id = '${attempt}'::uuid),`,
    `(SELECT count(*) FROM submission WHERE tenant_id = '${tenant}'::uuid AND attempt_id = '${attempt}'::uuid),`,
    `(SELECT count(*) FROM submission_idempotency WHERE tenant_id = '${tenant}'::uuid AND attempt_id = '${attempt}'::uuid),`,
    `(SELECT count(*) FROM submission_evaluation WHERE tenant_id = '${tenant}'::uuid AND attempt_id = '${attempt}'::uuid),`,
    `(SELECT count(*) FROM attempt_score_current WHERE tenant_id = '${tenant}'::uuid AND attempt_id = '${attempt}'::uuid);`,
  ].join(" ");
  const result = await adapterCompose(
    manifestPath,
    [
      "exec",
      "-T",
      "postgres",
      "psql",
      "-U",
      POSTGRES_USER,
      "-d",
      POSTGRES_DATABASE,
      "-tA",
      "-F",
      "|",
      "-c",
      sql,
    ],
    "checking durable submission rows",
    { safeDiagnostics: true },
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
  const localIdentity = identityFile(
    "0198e000-0000-7000-8000-000000000001",
    "0198e000-0000-7000-8000-000000000002",
  );
  const localIdentityRecord = JSON.parse(localIdentity.body).credentials[0];
  assert.deepEqual(Object.keys(localIdentityRecord).sort(), [
    "credential_sha256",
    "display_name",
    "roles",
    "tenant_id",
    "user_id",
  ]);
  assert.equal(
    localIdentityRecord.credential_sha256,
    createHash("sha256").update(Buffer.from(localIdentity.credential, "base64url")).digest("hex"),
  );
  assert.deepEqual(
    validVisibleResponse({
      kind: "multipleChoice",
      selection: { kind: "exactlyOne" },
      choices: [{ id: "shown-choice" }],
    }),
    { kind: "multipleChoice", selected: ["shown-choice"] },
  );
  // These UUIDv5-shaped values are the actual `project-tools e2e-seed` manifest for
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
  const secret = canonicalSecret32().trim();
  assert.match(secret, /^[A-Za-z0-9_-]{43}$/u);
  const diagnostic = safeComposeDiagnostic(
    `postgres://ple_e2e:${POSTGRES_PASSWORD}@postgres/ple_e2e ${secret}`,
    [secret],
  );
  assert.doesNotMatch(diagnostic, new RegExp(POSTGRES_PASSWORD, "u"));
  assert.doesNotMatch(diagnostic, new RegExp(secret, "u"));
  assert.match(diagnostic, /\[redacted\]/u);
  assert.deepEqual(adapterArguments("compose", "/tmp/target.manifest", ["--", "up", "-d", "api"]), [
    LOCAL_STACK_CONSUMER,
    "compose",
    "--manifest",
    "/tmp/target.manifest",
    "--",
    "up",
    "-d",
    "api",
  ]);
  assert.equal(replicaIdPrefix("ple-replica-e2e-api-0123456789ab"), "0123456789ab");
  assert.throws(() => replicaIdPrefix("ple-replica-e2e-api-not-a-container"));
  assert.equal(safeHttpError('{"error":"assignment is closed"}'), "assignment is closed");
  assert.equal(safeHttpError('{"error":"safe","request":"credential"}'), "");
  assert.equal(safeHttpError('{"error":"line\\nbreak"}'), "");
  assert.equal(safeHttpError(JSON.stringify({ error: "x".repeat(301) })), "");
  assert.equal(safeHttpError("not JSON"), "");
  console.log("replica_restart static check passed");
}

async function runLive() {
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
  const invitationSecretPath = join(tempDirectory, "invitation_token_secret");
  const questionIdSecretPath = join(tempDirectory, "question_id_secret");
  const capabilityPath = join(tempDirectory, "disposable_capability");
  const manifestPath = join(tempDirectory, "target.manifest");
  const retainEvidence = process.env.PLE_E2E_KEEP === "1";
  let preserveEvidence = retainEvidence;
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
    const gradingReaderPassword = randomBytes(24).toString("hex");
    const capability = randomBytes(32);
    const capabilitySha256 = createHash("sha256").update(capability).digest("hex");
    // The containing temporary directory remains 0700. The mounted file holds
    // only a high-entropy credential hash plus fixture IDs, so 0644 lets the
    // image's non-root UID read it without exposing the bearer credential.
    await writeFile(identityPath, identity.body, { mode: 0o644 });
    await writePrivateSecret32(invitationSecretPath);
    await writePrivateSecret32(questionIdSecretPath);
    await writeFile(capabilityPath, capability, { mode: 0o600 });
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
        `PLE_POSTGRES_IMAGE_SHA256=${POSTGRES_IMAGE_SHA256}`,
        `PLE_MINIO_IMAGE_SHA256=${MINIO_IMAGE_SHA256}`,
        `PLE_MINIO_MC_IMAGE_SHA256=${MINIO_MC_IMAGE_SHA256}`,
        `PLE_SECRET_INIT_IMAGE_SHA256=${SECRET_INIT_IMAGE_SHA256}`,
        `PLE_LOCAL_AUTH_HOST_FILE=${identityPath}`,
        `PLE_INVITATION_TOKEN_SECRET_HOST_FILE=${invitationSecretPath}`,
        `PLE_QUESTION_ID_SECRET_HOST_FILE=${questionIdSecretPath}`,
        `PLE_LOCAL_GRADER_PASSWORD=${gradingReaderPassword}`,
        `PLE_PUBLIC_ASSET_BASE_URL=http://127.0.0.1:${minioPort}/public-assets`,
        "PLE_WEBAUTHN_RP_ID=localhost",
        `PLE_WEBAUTHN_ORIGIN=http://localhost:${gatewayPort}`,
        "PLE_WEBAUTHN_RP_NAME=PLE replica E2E",
        `PLE_WEBWORK_RENDERER_IMAGE=${WEBWORK_RENDERER_IMAGE}`,
        "PLE_WEBWORK_RENDERER_ID=vosslab-webwork-pg-renderer",
        `PLE_WEBWORK_PROBLEM_JWT_SECRET=${randomBytes(32).toString("hex")}`,
        `PLE_WEBWORK_SESSION_JWT_SECRET=${randomBytes(32).toString("hex")}`,
        "PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS=15",
        "PLE_WEBWORK_MAX_RESPONSE_BYTES=1048576",
        `PLE_DISPOSABLE_CAPABILITY_SHA256=${capabilitySha256}`,
      ].join("\n") + "\n",
      { mode: 0o600 },
    );
    await writeFile(
      manifestPath,
      [
        "OWNER=replica-restart",
        `PROJECT=${project}`,
        `ENV_FILE=${envPath}`,
        `CAPABILITY_FILE=${capabilityPath}`,
      ].join("\n") + "\n",
      { mode: 0o600 },
    );

    let cleanupArmed = false;
    let operationFailure;
    let cleanupFailure;
    try {
      // Arm before the first adapter Compose request: a partially-created
      // stack remains target-owned and needs label-derived cleanup.
      cleanupArmed = true;
      await adapterCompose(
        manifestPath,
        ["up", "-d", "postgres", "minio", "createbuckets"],
        "starting E2E backing services",
        { safeDiagnostics: true, timeoutMs: 10 * 60_000 },
      );
      const seeded = await command(
        "cargo",
        [
          "run",
          "-q",
          "-p",
          "project-tools",
          "--",
          "e2e-seed",
          "--database-url",
          `postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:${postgresPort}/${POSTGRES_DATABASE}`,
          "--apply-migrations",
          "--tenant",
          tenantId,
          "--instructor",
          instructorId,
          "--student",
          studentId,
        ],
        "seeding E2E data",
        { environment: { PLE_QUESTION_ID_SECRET_FILE: questionIdSecretPath } },
      );
      const manifest = parseManifest(seeded.stdout);
      await provisionGradingReader(manifestPath, gradingReaderPassword);

      await adapterCompose(
        manifestPath,
        ["up", "-d", "--scale", "api=2", "api", "gateway"],
        "starting API replicas and gateway",
        { safeDiagnostics: true, timeoutMs: 10 * 60_000 },
      );
      const baseUrl = `http://127.0.0.1:${gatewayPort}`;
      try {
        await waitFor("gateway", async () => {
          const response = await fetch(`${baseUrl}/health`, {
            signal: AbortSignal.timeout(2_000),
          });
          if (!response.ok) fail(`gateway health returned ${response.status}`);
        });
      } catch (error) {
        const diagnostic = await deploymentDiagnostics(manifestPath);
        fail(`${error instanceof Error ? error.message : "gateway failed"}\n${diagnostic}`);
      }

      const login = await fetchJson(`${baseUrl}/api/auth/login`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ credential: identity.credential }),
      });
      const cookie = login.response.headers.get("set-cookie")?.split(";", 1)[0];
      assert.match(cookie ?? "", /^ple_session=/, "local login did not establish a session cookie");
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
      await stopIssuingReplica(manifestPath, initialReplica);
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
      await postgresCounts(manifestPath, tenantId, attemptId);
      console.log(
        `replica restart E2E passed: ${initialReplica} -> ${resumedQuestion.replica}; exact envelope and durable replay verified.`,
      );
    } catch (error) {
      operationFailure = error;
    } finally {
      if (cleanupArmed && !retainEvidence) {
        try {
          await adapterCommand("cleanup", manifestPath, [], "cleaning E2E project");
        } catch (error) {
          preserveEvidence = true;
          cleanupFailure = error;
        }
      }
    }
    if (operationFailure !== undefined && cleanupFailure !== undefined) {
      throw new AggregateError(
        [operationFailure, cleanupFailure],
        "replica E2E operation and owned cleanup both failed",
      );
    }
    if (operationFailure !== undefined) throw operationFailure;
    if (cleanupFailure !== undefined) throw cleanupFailure;
  } finally {
    if (preserveEvidence) {
      const reason = retainEvidence ? "PLE_E2E_KEEP=1" : "cleanup failed";
      console.log(`${reason} retained replica E2E evidence in ${tempDirectory}`);
    } else {
      await rm(tempDirectory, { recursive: true, force: true });
    }
  }
}

if (process.argv.includes("--static-check")) {
  await staticCheck();
} else {
  await runLive();
}
