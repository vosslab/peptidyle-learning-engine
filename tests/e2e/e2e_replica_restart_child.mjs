#!/usr/bin/env node
/** Assertion-only replica durability oracle for one owner-created live-demo target. */

import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { randomUUID } from "node:crypto";
import { constants } from "node:fs";
import { lstat, open } from "node:fs/promises";
import { Agent, request as httpsRequest } from "node:https";
import { dirname, isAbsolute, normalize, resolve } from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const REPO_ROOT = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const INPUT_ENVIRONMENT_NAME = "PLE_LIVE_DEMO_SERVICE_ORACLE_INPUT_FILE";
const INPUT_KEYS = [
  "schemaVersion",
  "oracle",
  "baseUrl",
  "manifestPath",
  "seedManifestPath",
  "workspacePath",
];
const SEED_KEYS = [
  "assignmentId",
  "courseId",
  "enrollmentId",
  "problemId",
  "questionId",
  "versionId",
];
const INPUT_MAXIMUM_BYTES = 16_384;
const RESPONSE_MAXIMUM_BYTES = 1_048_576;
const POLL_TIMEOUT_MS = 45_000;
const REPLICA_REFRESH_MS = 2_500;
const REPLICA_HEADER_PREFIX = "ple-replica-e2e-api-";
const TENANT_ID = "00000000-0000-0000-0000-000000000100";

function fail(message) {
  throw new Error(message);
}

function requireExactKeys(value, keys, label) {
  if (value === null || Array.isArray(value) || typeof value !== "object") {
    fail(`${label} has an invalid shape`);
  }
  assert.deepEqual(Object.keys(value).sort(), [...keys].sort(), `${label} has an invalid shape`);
  return value;
}

function requireUuid(value, label) {
  assert.match(
    value,
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u,
    label,
  );
  return value;
}

function requireQuestionId(value) {
  assert.match(value, /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u);
  return value;
}

function requireAbsolutePath(value, label) {
  if (typeof value !== "string" || value === "" || value.includes("\0")) {
    fail(`service-oracle input ${label} is invalid`);
  }
  if (!isAbsolute(value) || normalize(value) !== value) {
    fail(`service-oracle input ${label} is invalid`);
  }
  return value;
}

export function decodeServiceInput(value) {
  // ASVS 1.5.2 and 15.3.3: only the exact V1 field allowlist is assignable.
  const record = requireExactKeys(value, INPUT_KEYS, "service-oracle input");
  if (record.schemaVersion !== 1 || record.oracle !== "replica_restart") {
    fail("service-oracle input version or oracle is invalid");
  }
  const baseUrl = record.baseUrl;
  const originMatch =
    typeof baseUrl === "string" ? /^https:\/\/localhost:([1-9][0-9]{0,4})\/$/u.exec(baseUrl) : null;
  if (originMatch === null || Number(originMatch[1]) > 65_535) {
    fail("service-oracle input baseUrl is invalid");
  }
  const workspacePath = requireAbsolutePath(record.workspacePath, "workspacePath");
  const manifestPath = requireAbsolutePath(record.manifestPath, "manifestPath");
  const seedManifestPath = requireAbsolutePath(record.seedManifestPath, "seedManifestPath");
  if (dirname(manifestPath) !== workspacePath || dirname(seedManifestPath) !== workspacePath) {
    fail("service-oracle input paths leave the fixed workspace");
  }
  return {
    schemaVersion: 1,
    oracle: "replica_restart",
    baseUrl,
    manifestPath,
    seedManifestPath,
    workspacePath,
  };
}

async function readBoundedPrivateAscii(path, maximumBytes, label) {
  // ASVS 5.3.2 and 15.4.2: validate the path and opened descriptor identities together.
  let before;
  let file;
  try {
    before = await lstat(path);
    file = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  } catch {
    fail(`${label} file is unsafe`);
  }
  try {
    const metadata = await file.stat();
    const uid = typeof process.getuid === "function" ? process.getuid() : undefined;
    if (
      before.isSymbolicLink() ||
      !metadata.isFile() ||
      uid === undefined ||
      before.uid !== uid ||
      metadata.uid !== uid ||
      (before.mode & 0o777) !== 0o600 ||
      (metadata.mode & 0o777) !== 0o600 ||
      before.dev !== metadata.dev ||
      before.ino !== metadata.ino ||
      metadata.size < 1 ||
      metadata.size > maximumBytes
    ) {
      fail(`${label} file is unsafe`);
    }
    const buffer = Buffer.alloc(maximumBytes + 1);
    let offset = 0;
    while (offset < buffer.length) {
      const result = await file.read(buffer, offset, buffer.length - offset, offset);
      if (result.bytesRead === 0) break;
      offset += result.bytesRead;
    }
    if (offset > maximumBytes) fail(`${label} file is unsafe`);
    return buffer.subarray(0, offset).toString("ascii");
  } finally {
    await file.close();
  }
}

export async function readServiceInput(path) {
  const text = await readBoundedPrivateAscii(path, INPUT_MAXIMUM_BYTES, "service-oracle input");
  let value;
  try {
    value = JSON.parse(text);
  } catch {
    fail("service-oracle input is not canonical JSON");
  }
  const decoded = decodeServiceInput(value);
  if (JSON.stringify(decoded) !== text) fail("service-oracle input is not canonical JSON");
  if (dirname(path) !== decoded.workspacePath) {
    fail("service-oracle input path leaves the fixed workspace");
  }
  return decoded;
}

export function decodeSeedManifest(value) {
  const manifest = requireExactKeys(value, SEED_KEYS, "service-oracle seed manifest");
  requireUuid(manifest.assignmentId, "seed manifest assignmentId");
  requireUuid(manifest.courseId, "seed manifest courseId");
  requireUuid(manifest.enrollmentId, "seed manifest enrollmentId");
  requireUuid(manifest.problemId, "seed manifest problemId");
  requireQuestionId(manifest.questionId);
  requireUuid(manifest.versionId, "seed manifest versionId");
  return manifest;
}

async function readSeedManifest(path) {
  const text = await readBoundedPrivateAscii(path, INPUT_MAXIMUM_BYTES, "seed manifest");
  let value;
  try {
    value = JSON.parse(text);
  } catch {
    fail("service-oracle seed manifest is not JSON");
  }
  return decodeSeedManifest(value);
}

export class LocalhostHttpsDispatcher {
  constructor(baseUrl) {
    const origin = new URL(baseUrl);
    if (
      origin.protocol !== "https:" ||
      origin.hostname !== "localhost" ||
      origin.pathname !== "/"
    ) {
      fail("localhost HTTPS dispatcher requires the exact owner origin");
    }
    this.origin = origin.origin;
    // ASVS 12.3.4: the ephemeral Caddy CA has no safe host handoff in V1, so the
    // permitted fallback is confined to this assertion child's request-local agent.
    this.agent = new Agent({ keepAlive: true, rejectUnauthorized: false });
  }

  async request(path, { method = "GET", headers = {}, body = undefined } = {}) {
    const url = new URL(path, `${this.origin}/`);
    if (url.origin !== this.origin || url.protocol !== "https:" || url.hostname !== "localhost") {
      fail("localhost HTTPS dispatcher refused a foreign request");
    }
    const payload = body === undefined ? undefined : Buffer.from(body, "utf8");
    const requestHeaders = { accept: "application/json", ...headers };
    if (payload !== undefined) requestHeaders["content-length"] = String(payload.length);
    return await new Promise((resolveRequest, rejectRequest) => {
      const request = httpsRequest(
        url,
        { method, headers: requestHeaders, agent: this.agent, signal: AbortSignal.timeout(5_000) },
        (response) => {
          const chunks = [];
          let received = 0;
          response.on("data", (chunk) => {
            received += chunk.length;
            if (received > RESPONSE_MAXIMUM_BYTES) {
              response.destroy(new Error("HTTPS response exceeded its bounded size"));
              return;
            }
            chunks.push(chunk);
          });
          response.on("error", rejectRequest);
          response.on("end", () => {
            resolveRequest({
              status: response.statusCode ?? 0,
              headers: response.headers,
              body: Buffer.concat(chunks).toString("utf8"),
            });
          });
        },
      );
      request.on("error", rejectRequest);
      if (payload !== undefined) request.write(payload);
      request.end();
    });
  }

  close() {
    this.agent.destroy();
  }
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
    Object.keys(body).length !== 1 ||
    typeof body.error !== "string" ||
    body.error.length < 1 ||
    body.error.length > 300 ||
    !/^[\x20-\x7e]+$/u.test(body.error)
  ) {
    return "";
  }
  return body.error;
}

async function requestJson(dispatcher, path, options = {}) {
  const response = await dispatcher.request(path, options);
  if (response.status < 200 || response.status > 299) {
    const detail = safeHttpError(response.body);
    fail(
      `${options.method ?? "GET"} ${path} returned ${response.status}${detail ? `: ${detail}` : ""}`,
    );
  }
  try {
    return { ...response, json: JSON.parse(response.body) };
  } catch {
    fail(`${path} did not return JSON`);
  }
}

function secureHostCookie(response, name) {
  const header = response.headers["set-cookie"];
  const values = Array.isArray(header) ? header : header === undefined ? [] : [header];
  const matches = values.filter((value) => value.startsWith(`${name}=`));
  assert.equal(matches.length, 1, `${name} was not issued exactly once`);
  const parts = matches[0].split(";").map((part) => part.trim());
  const pair = parts[0];
  assert.match(pair, new RegExp(`^${name}=[A-Za-z0-9_-]{20,}$`, "u"), `${name} is malformed`);
  const attributes = new Set(parts.slice(1).map((part) => part.toLowerCase()));
  assert.ok(attributes.has("secure"), `${name} must be Secure`);
  assert.ok(attributes.has("httponly"), `${name} must be HttpOnly`);
  assert.ok(attributes.has("path=/"), `${name} must be host-wide`);
  assert.ok(
    !parts.some((part) => part.toLowerCase().startsWith("domain=")),
    `${name} must be host-only`,
  );
  return pair;
}

export async function authenticateMary(dispatcher, baseUrl, courseId) {
  const origin = new URL(baseUrl).origin;
  const account = await requestJson(dispatcher, "/api/auth/live-demo/accounts", {
    method: "POST",
    headers: { "content-type": "application/json", origin },
    body: JSON.stringify({ persona: "maryStudent" }),
  });
  assert.deepEqual(account.json, { authenticated: true });
  const accountCookie = secureHostCookie(account, "__Host-ple_account_session");
  const courses = await requestJson(dispatcher, "/api/auth/account/courses", {
    headers: { cookie: accountCookie },
  });
  const page = requireExactKeys(courses.json, ["courses", "nextCursor"], "account course list");
  assert.ok(Array.isArray(page.courses), "account course list needs courses");
  const selected = page.courses.filter((course) => course?.courseId === courseId);
  assert.equal(selected.length, 1, "Mary account did not expose the exact seeded course");
  assert.equal(selected[0].role, "student", "Mary seeded course role changed");
  const session = await requestJson(dispatcher, "/api/auth/account/course-session", {
    method: "POST",
    headers: { cookie: accountCookie, "content-type": "application/json", origin },
    body: JSON.stringify({ courseId }),
  });
  assert.deepEqual(session.json, { authenticated: true, courseId, role: "student" });
  const courseCookie = secureHostCookie(session, "__Host-ple_session");
  return `${accountCookie}; ${courseCookie}`;
}

async function adapterCommand(action, manifestPath, actionArguments, label) {
  // ASVS 1.2.5: the action and interpreter are closed; no shell or Compose tail exists here.
  try {
    return await execFileAsync(
      "python3",
      [
        "-m",
        "local_stack_control._consumer_cli",
        action,
        "--manifest",
        manifestPath,
        ...actionArguments,
      ],
      { cwd: REPO_ROOT, env: process.env, encoding: "utf8", maxBuffer: 64 * 1024 },
    );
  } catch (error) {
    fail(`${label} failed (${String(error.code ?? "unknown error")})`);
  }
}

function parseReplica(value) {
  assert.match(
    value ?? "",
    /^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$/u,
    "missing or malformed X-PLE-E2E-Replica header",
  );
  return value;
}

function replicaIdPrefix(replica) {
  assert.ok(replica.startsWith(REPLICA_HEADER_PREFIX), "unexpected replica attribution prefix");
  const suffix = replica.slice(REPLICA_HEADER_PREFIX.length);
  assert.match(suffix, /^[a-f0-9]{12}$/u, "replica attribution needs a container short ID");
  return suffix;
}

async function stopIssuingReplica(manifestPath, replica) {
  await adapterCommand(
    "stop-instance",
    manifestPath,
    ["--service", "api", "--id-prefix", replicaIdPrefix(replica)],
    "stopping the replica that issued the question",
  );
}

async function requirePostgresqlCounts(manifestPath, attemptId) {
  const result = await adapterCommand(
    "postgresql-count",
    manifestPath,
    ["--tenant-id", TENANT_ID, "--attempt-id", requireUuid(attemptId, "attempt id")],
    "checking durable submission rows",
  );
  assert.equal(
    result.stdout.trim(),
    "1|1|1|1|1",
    "expected one scoped attempt, submission, idempotency, evaluation, and current score record",
  );
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

export async function runReplicaOracle(input) {
  const manifest = await readSeedManifest(input.seedManifestPath);
  const dispatcher = new LocalhostHttpsDispatcher(input.baseUrl);
  try {
    const cookie = await authenticateMary(dispatcher, input.baseUrl, manifest.courseId);
    const origin = new URL(input.baseUrl).origin;
    const studentHeaders = { cookie, "content-type": "application/json", origin };
    const startedRun = await requestJson(dispatcher, "/api/runs", {
      method: "POST",
      headers: studentHeaders,
      body: JSON.stringify({ assignmentId: manifest.assignmentId }),
    });
    assert.equal(typeof startedRun.json.id, "string", "start run did not return a run id");
    const runId = encodeURIComponent(startedRun.json.id);
    const attempts = await requestJson(dispatcher, `/api/runs/${runId}/attempts`, {
      headers: { cookie },
    });
    assert.ok(Array.isArray(attempts.json.items) && attempts.json.items.length === 1);
    const attemptId = attempts.json.items[0]?.id;
    assert.equal(typeof attemptId, "string", "attempt id is missing");
    const attemptPath = encodeURIComponent(attemptId);
    const initialQuestion = await requestJson(dispatcher, `/api/attempts/${attemptPath}/question`, {
      headers: { cookie },
    });
    const initialReplica = parseReplica(initialQuestion.headers["x-ple-e2e-replica"]);
    const initialEnvelope = initialQuestion.json;
    await stopIssuingReplica(input.manifestPath, initialReplica);
    await new Promise((resolveWait) => setTimeout(resolveWait, REPLICA_REFRESH_MS));
    const resumed = await waitFor("a distinct API replica after restart", async () => {
      const question = await requestJson(dispatcher, `/api/attempts/${attemptPath}/question`, {
        headers: { cookie },
      });
      const replica = parseReplica(question.headers["x-ple-e2e-replica"]);
      if (replica === initialReplica) fail("Caddy still selected the stopped API replica");
      return { question, replica };
    });
    assert.deepEqual(
      resumed.question.json,
      initialEnvelope,
      "issued envelope changed across replicas",
    );
    const response = validVisibleResponse(initialEnvelope.response);
    const idempotencyKey = `replica-e2e-${randomUUID()}`;
    const submissionOptions = {
      method: "POST",
      headers: { ...studentHeaders, "idempotency-key": idempotencyKey },
      body: JSON.stringify({ response }),
    };
    const firstReceipt = await requestJson(
      dispatcher,
      `/api/submissions/${attemptPath}`,
      submissionOptions,
    );
    const replayReceipt = await requestJson(
      dispatcher,
      `/api/submissions/${attemptPath}`,
      submissionOptions,
    );
    assert.deepEqual(replayReceipt.json, firstReceipt.json, "idempotency replay changed receipt");
    await requirePostgresqlCounts(input.manifestPath, attemptId);
    console.log(
      `replica restart E2E passed: ${initialReplica} -> ${resumed.replica}; exact envelope and durable replay verified.`,
    );
  } finally {
    dispatcher.close();
  }
}

export async function main() {
  if (process.argv.length !== 2) fail("replica assertion child accepts no arguments");
  const inputPath = process.env[INPUT_ENVIRONMENT_NAME];
  if (inputPath === undefined) fail("replica assertion child input is unavailable");
  const input = await readServiceInput(requireAbsolutePath(inputPath, "input file"));
  await runReplicaOracle(input);
}

const executedPath = process.argv[1] === undefined ? "" : resolve(process.argv[1]);
if (executedPath === fileURLToPath(import.meta.url)) await main();
