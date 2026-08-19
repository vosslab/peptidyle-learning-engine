#!/usr/bin/env node
/**
 * Proves that a normal learner session survives loss of the API replica that
 * issued it.  This is intentionally a host-side, container-dependent test:
 * every learner request below goes through Caddy and seed data comes from the
 * host-only project-tools command, never a product fixture route.
 *
 * The command is deliberately strict: missing Podman is BLOCKED and exits
 * non-zero rather than turning a deployment prerequisite into a passing skip.
 */

import assert from "node:assert/strict";
import { createHash, randomBytes, randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const REPO_ROOT = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const POLL_TIMEOUT_MS = 45_000;
const REPLICA_REFRESH_MS = 2_500;
const REPLICA_HEADER_PREFIX = "ple-replica-e2e-api-";
const POSTGRES_USER = "ple_e2e";
const POSTGRES_DATABASE = "ple_e2e";
const POSTGRES_PASSWORD = "ple-e2e-local-only";
const CANONICAL_SELECTION_NAMES = [
  "PLE_GATEWAY_IMAGE_SHA256",
  "PLE_POSTGRES_IMAGE_SHA256",
  "PLE_MINIO_IMAGE_SHA256",
  "PLE_MINIO_MC_IMAGE_SHA256",
  "PLE_SECRET_INIT_IMAGE_SHA256",
  "PLE_WEBWORK_RENDERER_IMAGE",
  "PLE_WEBWORK_RENDERER_BASE_URL",
  "PLE_WEBWORK_RENDERER_ID",
  "PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
  "PLE_WEBWORK_MAX_RESPONSE_BYTES",
];

function fail(message) {
  throw new Error(message);
}

async function canonicalStackSelections() {
  const text = await readFile(join(REPO_ROOT, "containers", "env.example"), "utf8");
  const values = new Map();
  for (const [index, line] of text.split(/\r?\n/).entries()) {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("#")) continue;
    const separator = line.indexOf("=");
    if (separator < 1) fail(`canonical environment line ${index + 1} is not NAME=value`);
    const name = line.slice(0, separator);
    const value = line.slice(separator + 1);
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name) || values.has(name)) {
      fail(`canonical environment line ${index + 1} is invalid`);
    }
    values.set(name, value);
  }
  return Object.fromEntries(
    CANONICAL_SELECTION_NAMES.map((name) => {
      const value = values.get(name);
      if (!value) fail(`canonical environment must select ${name}`);
      return [name, value];
    }),
  );
}

function requireUuid(value, label) {
  assert.match(
    value,
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    label,
  );
  return value;
}

function requireQuestionId(value) {
  assert.match(value, /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/, "seed manifest questionId");
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
    "questionId",
    "versionId",
  ]);
  requireUuid(manifest.assignmentId, "seed manifest assignmentId");
  requireUuid(manifest.enrollmentId, "seed manifest enrollmentId");
  requireUuid(manifest.problemId, "seed manifest problemId");
  requireQuestionId(manifest.questionId);
  requireUuid(manifest.versionId, "seed manifest versionId");
  return manifest;
}

function visibleSelectionMinimum(selection) {
  switch (selection?.kind) {
    case "exactlyOne":
    case "atLeastOne":
      return 1;
    case "exactly":
      assert.ok(Number.isInteger(selection.count) && selection.count >= 0);
      return selection.count;
    case "anyNumber":
      return 0;
    default:
      fail("visible response needs a supported selection rule");
  }
}

function visibleHotspotPoints(regions, minimum) {
  assert.ok(Array.isArray(regions), "hotspot needs visible regions");
  assert.ok(Number.isInteger(minimum) && minimum >= 0 && minimum <= regions.length);
  if (minimum === 0) return [];
  const contains = (region, x, y) =>
    x >= region.x && x <= region.x + region.width && y >= region.y && y <= region.y + region.height;
  const points = [];
  for (const target of regions) {
    for (const field of ["x", "y", "width", "height"]) {
      assert.equal(typeof target?.[field], "number", `hotspot region needs visible ${field}`);
    }
    const targetRight = target.x + target.width;
    const targetBottom = target.y + target.height;
    const xCandidates = new Set([target.x, targetRight, Math.floor((target.x + targetRight) / 2)]);
    const yCandidates = new Set([
      target.y,
      targetBottom,
      Math.floor((target.y + targetBottom) / 2),
    ]);
    for (const region of regions) {
      for (const x of [
        region.x - 1,
        region.x,
        region.x + region.width,
        region.x + region.width + 1,
      ]) {
        if (x >= target.x && x <= targetRight) xCandidates.add(x);
      }
      for (const y of [
        region.y - 1,
        region.y,
        region.y + region.height,
        region.y + region.height + 1,
      ]) {
        if (y >= target.y && y <= targetBottom) yCandidates.add(y);
      }
    }
    let point;
    for (const x of xCandidates) {
      for (const y of yCandidates) {
        if (x >= 0 && x <= 10_000 && y >= 0 && y <= 10_000) {
          if (regions.filter((region) => contains(region, x, y)).length === 1) {
            point = { x, y };
            break;
          }
        }
      }
      if (point !== undefined) break;
    }
    if (point !== undefined) points.push(point);
    if (points.length === minimum) return points;
  }
  fail("hotspot presentation has no unambiguous visible response candidate");
}

function validVisibleResponse(response) {
  assert.equal(
    typeof response?.kind,
    "string",
    "issued envelope needs a visible response definition",
  );
  switch (response.kind) {
    case "singleChoice": {
      assert.ok(response.choices?.length > 0, "single choice needs a visible choice");
      assert.equal(typeof response.choices[0]?.id, "string", "E2E choice needs a visible id");
      return { kind: "multipleChoice", selected: [response.choices[0].id] };
    }
    case "multipleAnswer": {
      assert.ok(Array.isArray(response.choices), "multiple answer needs visible choices");
      assert.ok(
        Number.isInteger(response.minimum) &&
          Number.isInteger(response.maximum) &&
          response.minimum >= 0 &&
          response.minimum <= response.maximum &&
          response.maximum <= response.choices.length,
        "multiple answer needs valid visible selection bounds",
      );
      const selected = response.choices.slice(0, response.minimum).map((choice) => {
        assert.equal(typeof choice?.id, "string", "E2E choice needs a visible id");
        return choice.id;
      });
      return { kind: "multipleChoice", selected };
    }
    case "multipleChoice": {
      assert.ok(Array.isArray(response.choices), "multiple choice needs visible choices");
      const minimum = visibleSelectionMinimum(response.selection);
      assert.ok(minimum <= response.choices.length, "multiple choice needs enough visible choices");
      const selected = response.choices.slice(0, minimum).map((choice) => {
        assert.equal(typeof choice?.id, "string", "E2E choice needs a visible id");
        return choice.id;
      });
      return { kind: "multipleChoice", selected };
    }
    case "fillIn":
      assert.ok(response.maxCharacters >= 1, "fill-in needs visible input capacity");
      return { kind: "shortText", text: "" };
    case "shortText":
      assert.ok(response.maxLength >= 0, "short-text response needs visible input capacity");
      return { kind: "shortText", text: "" };
    case "multiFillIn": {
      assert.ok(response.blanks?.length > 0, "multi-fill-in needs visible blanks");
      const answers = response.blanks.map((blank) => {
        assert.equal(typeof blank?.id, "string", "E2E blank needs a visible id");
        assert.ok(blank.maxCharacters >= 1, "E2E blank needs visible input capacity");
        return { slot: blank.id, text: "" };
      });
      return { kind: "multiBlank", answers };
    }
    case "multiBlank": {
      assert.ok(response.blanks?.length > 0, "multi-blank response needs visible blanks");
      const answers = response.blanks.map((blank) => {
        assert.equal(typeof blank?.id, "string", "E2E blank needs a visible id");
        assert.ok(blank.maxLength >= 0, "E2E blank needs visible input capacity");
        return { slot: blank.id, text: "" };
      });
      return { kind: "multiBlank", answers };
    }
    case "numerical":
      assert.ok(response.maxCharacters >= 1, "numerical response needs visible input capacity");
      return { kind: "numeric", value: 0 };
    case "numeric":
      return { kind: "numeric", value: 0 };
    case "matching": {
      assert.ok(response.prompts?.length > 0, "matching needs visible prompts");
      assert.ok(
        response.choices?.length >= (response.reuseChoices === true ? 1 : response.prompts.length),
        "matching needs enough visible choices",
      );
      const matches = response.prompts.map((prompt, index) => {
        assert.equal(typeof prompt?.id, "string", "E2E matching prompt needs a visible id");
        const choice = response.choices[response.reuseChoices === true ? 0 : index];
        assert.equal(typeof choice?.id, "string", "E2E matching choice needs a visible id");
        return { prompt: prompt.id, choice: choice.id };
      });
      return { kind: "matching", matches };
    }
    case "ordering": {
      assert.ok(
        Array.isArray(response.items) && response.items.length > 0,
        "ordering needs visible items",
      );
      const order = response.items.map((item) => {
        assert.equal(typeof item?.id, "string", "E2E ordering item needs a visible id");
        return item.id;
      });
      return { kind: "ordering", order };
    }
    case "hotspot": {
      const regions = response.surface?.regions ?? response.regions;
      assert.ok(Array.isArray(regions), "hotspot needs visible regions");
      const minimum =
        response.minimum === undefined
          ? visibleSelectionMinimum(response.selection)
          : response.minimum;
      if (response.maximum !== undefined) {
        assert.ok(
          Number.isInteger(response.maximum) &&
            minimum <= response.maximum &&
            response.maximum <= regions.length,
          "hotspot needs valid visible selection bounds",
        );
      }
      const points = visibleHotspotPoints(regions, minimum);
      return { kind: "hotspot", points };
    }
    case "externalTool":
      return { kind: "externalTool" };
    case "fileUpload":
      return fail("file-upload response requires a server-issued learner object reference");
    default:
      return fail(`unsupported visible E2E response kind ${response.kind}`);
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

function redactFailureOutput(stdout, stderr, privateValues = []) {
  let redacted = `${stdout}\n${stderr}`;
  for (const privateValue of privateValues) {
    if (privateValue !== "") redacted = redacted.replaceAll(privateValue, "[redacted]");
  }
  return redacted
    .replaceAll(POSTGRES_PASSWORD, "[redacted]")
    .replaceAll("ple-e2e-minio-password", "[redacted]")
    .replace(/postgres:\/\/[^@\s]+@/gu, "postgres://[redacted]@")
    .slice(-2_000);
}

async function createPrivateDirectory() {
  const result = await command(
    "python3",
    ["-m", "local_stack_control._consumer_cli", "prepare-state", "--owner", "replica-restart"],
    "preparing replica E2E private state",
    { safeDiagnostics: false },
  );
  const directory = result.stdout.trim();
  const expectedPrefix = join(REPO_ROOT, "target", "replica-e2e", "run-");
  if (!directory.startsWith(expectedPrefix)) fail("private-state adapter returned an invalid path");
  return directory;
}

async function removePrivateDirectory(directory) {
  await command(
    "python3",
    [
      "-m",
      "local_stack_control._consumer_cli",
      "remove-state",
      "--owner",
      "replica-restart",
      "--directory",
      directory,
    ],
    "removing replica E2E private state",
    { safeDiagnostics: false },
  );
}

function adapterArguments(action, manifestPath, actionArguments = []) {
  return [
    "-m",
    "local_stack_control._consumer_cli",
    action,
    "--manifest",
    manifestPath,
    ...actionArguments,
  ];
}

async function adapterCommand(action, manifestPath, actionArguments, label, options = {}) {
  // The closed adapter emits only bounded, redacted failure output. Keep that
  // receipt available to the runner without exposing the adapter argv.
  const adapterOptions = { ...options };
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
      ? redactFailureOutput(error.stdout ?? "", error.stderr ?? "", privateValues)
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
          learner_alias: "replica-e2e-learner",
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

async function runLive() {
  const selections = await canonicalStackSelections();

  const project = `ple-replica-e2e-${randomBytes(5).toString("hex")}`;
  const applicationImage = `localhost/peptidyle-learning-engine:${project}`;
  const tempDirectory = await createPrivateDirectory();
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
        `PLE_GATEWAY_IMAGE_SHA256=${selections.PLE_GATEWAY_IMAGE_SHA256}`,
        `PLE_APPLICATION_IMAGE=${applicationImage}`,
        `PLE_POSTGRES_IMAGE_SHA256=${selections.PLE_POSTGRES_IMAGE_SHA256}`,
        `PLE_MINIO_IMAGE_SHA256=${selections.PLE_MINIO_IMAGE_SHA256}`,
        `PLE_MINIO_MC_IMAGE_SHA256=${selections.PLE_MINIO_MC_IMAGE_SHA256}`,
        `PLE_SECRET_INIT_IMAGE_SHA256=${selections.PLE_SECRET_INIT_IMAGE_SHA256}`,
        `PLE_LOCAL_AUTH_HOST_FILE=${identityPath}`,
        `PLE_INVITATION_TOKEN_SECRET_HOST_FILE=${invitationSecretPath}`,
        `PLE_QUESTION_ID_SECRET_HOST_FILE=${questionIdSecretPath}`,
        `PLE_LOCAL_GRADER_PASSWORD=${gradingReaderPassword}`,
        `PLE_PUBLIC_ASSET_BASE_URL=http://127.0.0.1:${minioPort}/public-assets`,
        "PLE_WEBAUTHN_RP_ID=localhost",
        `PLE_WEBAUTHN_ORIGIN=http://localhost:${gatewayPort}`,
        "PLE_WEBAUTHN_RP_NAME=PLE replica E2E",
        `PLE_WEBWORK_RENDERER_IMAGE=${selections.PLE_WEBWORK_RENDERER_IMAGE}`,
        `PLE_WEBWORK_RENDERER_BASE_URL=${selections.PLE_WEBWORK_RENDERER_BASE_URL}`,
        `PLE_WEBWORK_RENDERER_ID=${selections.PLE_WEBWORK_RENDERER_ID}`,
        `PLE_WEBWORK_PROBLEM_JWT_SECRET=${randomBytes(32).toString("hex")}`,
        `PLE_WEBWORK_SESSION_JWT_SECRET=${randomBytes(32).toString("hex")}`,
        `PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS=${selections.PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS}`,
        `PLE_WEBWORK_MAX_RESPONSE_BYTES=${selections.PLE_WEBWORK_MAX_RESPONSE_BYTES}`,
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
        ["build", "api", "gateway"],
        "building current API and gateway images",
        { safeDiagnostics: true, timeoutMs: 10 * 60_000 },
      );
      await adapterCompose(
        manifestPath,
        ["up", "-d", "identity-secret-init", "webwork-renderer"],
        "starting API prerequisites",
        { safeDiagnostics: true, timeoutMs: 10 * 60_000 },
      );
      await waitFor(
        "renderer health",
        () =>
          adapterCompose(
            manifestPath,
            [
              "exec",
              "-T",
              "webwork-renderer",
              "curl",
              "--fail",
              "--silent",
              "--show-error",
              "--max-time",
              "5",
              "http://127.0.0.1:3000/health/",
            ],
            "checking renderer health",
          ),
        2 * 60_000,
      );
      await adapterCompose(
        manifestPath,
        ["up", "-d", "--no-deps", "api"],
        "starting the first API replica",
        { safeDiagnostics: true, timeoutMs: 10 * 60_000 },
      );
      await waitFor(
        "first API health",
        () =>
          adapterCompose(
            manifestPath,
            ["exec", "-T", "api", "/usr/local/bin/peptidyle-api", "--health-probe"],
            "checking first API health",
          ),
        2 * 60_000,
      );
      await adapterCompose(
        manifestPath,
        ["up", "-d", "--no-deps", "--scale", "api=2", "api", "gateway"],
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
      await removePrivateDirectory(tempDirectory);
    }
  }
}

if (process.argv.length !== 2) fail("replica restart E2E accepts no command-line arguments");
await runLive();
