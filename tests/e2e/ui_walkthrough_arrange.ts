// ui_walkthrough_arrange.ts - fixed, redacted supported-API setup for the UI walkthrough.

import { lstatSync, readFileSync } from "node:fs";

import { request, type APIRequestContext } from "@playwright/test";

import {
  arrangeSeededCourseAssignments,
  type AssignmentArrangement,
} from "../playwright/simulator/assignment_arrangement";
import { arrangeRetryCorpus } from "../playwright/simulator/retry_corpus";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;
const CREDENTIAL = /^[A-Za-z0-9_-]{32,}$/u;
const MAX_PRIVATE_FILE_BYTES = 4096;

export interface LauncherManifest {
  readonly assignmentId: string;
}

export interface ArrangementRecord {
  readonly label:
    | "launcher-seeded-enrollment"
    | "launcher-baseline-assignment"
    | "api-retry-corpus-publication"
    | "api-mastery-assignment"
    | "api-exam-assignment";
  readonly baselineAssignmentId?: string;
  readonly courseId?: string;
  readonly problemId?: string;
  readonly versionId?: string;
  /** Private runner handoff only; it is stripped before any report is written. */
  readonly catalogSearchTitle?: string;
  readonly masteryAssignmentId?: string;
  readonly examAssignmentId?: string;
}

export interface ArrangementOutput {
  readonly arrangements: readonly ArrangementRecord[];
}

export function instructorSetupArrangementOutput(corpus: {
  readonly problem: string;
  readonly version: string;
  readonly catalogSearchTitle: string;
}): ArrangementOutput {
  return {
    arrangements: [
      {
        label: "api-retry-corpus-publication",
        problemId: corpus.problem,
        versionId: corpus.version,
        catalogSearchTitle: corpus.catalogSearchTitle,
      },
    ],
  };
}

function requiredEnvironment(name: string): string {
  const value = process.env[name];
  if (value === undefined || value.trim() === "") throw new Error("arrangement-input");
  return value.trim();
}

function readPrivateRegularFile(path: string): string {
  const metadata = lstatSync(path);
  if (!metadata.isFile() || metadata.isSymbolicLink() || (metadata.mode & 0o777) !== 0o600) {
    throw new Error("arrangement-input");
  }
  const bytes = readFileSync(path);
  if (bytes.length > MAX_PRIVATE_FILE_BYTES || bytes.some((byte) => byte > 0x7f)) {
    throw new Error("arrangement-input");
  }
  return bytes.toString("ascii");
}

export function instructorCredential(loginContents: string): string {
  const matches = [...loginContents.matchAll(/^instructor=([^\r\n]+)$/gmu)];
  const credential = matches[0]?.[1];
  if (matches.length !== 1 || credential === undefined || !CREDENTIAL.test(credential)) {
    throw new Error("arrangement-input");
  }
  return credential;
}

export function launcherManifest(manifestContents: string): LauncherManifest {
  let parsed: unknown;
  try {
    parsed = JSON.parse(manifestContents);
  } catch {
    throw new Error("arrangement-input");
  }
  const expectedKeys = ["assignmentId", "enrollmentId", "problemId", "versionId"];
  const assignmentId = isRecord(parsed) ? parsed["assignmentId"] : undefined;
  if (
    !isRecord(parsed) ||
    Object.keys(parsed).length !== expectedKeys.length ||
    expectedKeys.some((key) => !Object.prototype.hasOwnProperty.call(parsed, key)) ||
    expectedKeys.some((key) => !isUuid(parsed[key])) ||
    !isUuid(assignmentId)
  ) {
    throw new Error("arrangement-input");
  }
  return { assignmentId };
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isUuid(value: unknown): value is string {
  return typeof value === "string" && UUID.test(value);
}

export function arrangementOutputFor(
  baselineAssignmentId: string,
  corpus: { readonly problem: string; readonly version: string },
  assignments: AssignmentArrangement,
): ArrangementOutput {
  const arrangements: readonly ArrangementRecord[] = [
    { label: "launcher-seeded-enrollment" },
    { label: "launcher-baseline-assignment", baselineAssignmentId },
    {
      label: "api-retry-corpus-publication",
      problemId: corpus.problem,
      versionId: corpus.version,
    },
    {
      label: "api-mastery-assignment",
      courseId: assignments.courseId,
      masteryAssignmentId: assignments.masteryAssignmentId,
    },
    {
      label: "api-exam-assignment",
      courseId: assignments.courseId,
      examAssignmentId: assignments.examAssignmentId,
    },
  ];
  return { arrangements };
}

interface InstructorLoginContext {
  post(
    path: string,
    request: { readonly data: { readonly credential: string } },
  ): Promise<{
    status(): number;
  }>;
  dispose(): Promise<void>;
}

interface RequestFactory<T extends InstructorLoginContext> {
  newContext(options: { readonly baseURL: string }): Promise<T>;
}

export async function authenticatedInstructorContextWithRequest<T extends InstructorLoginContext>(
  requestFactory: RequestFactory<T>,
  baseUrl: string,
  credential: string,
): Promise<T> {
  const context = await requestFactory.newContext({ baseURL: baseUrl });
  try {
    const response = await context.post("/api/auth/login", { data: { credential } });
    if (response.status() === 200) return context;
  } catch {
    // The caller receives one generic stage after the context is disposed.
  }
  await context.dispose();
  throw new Error("assignment-login");
}

async function authenticatedInstructorContext(
  baseUrl: string,
  credential: string,
): Promise<APIRequestContext> {
  return authenticatedInstructorContextWithRequest(request, baseUrl, credential);
}

async function arrange(): Promise<ArrangementOutput> {
  const baseUrl = requiredEnvironment("PLE_UI_WALKTHROUGH_LIVE_BASE_URL");
  const masterSeedText = requiredEnvironment("PLE_UI_WALKTHROUGH_MASTER_SEED");
  if (!/^[0-9]+$/u.test(masterSeedText)) throw new Error("arrangement-input");
  const masterSeed = Number(masterSeedText);
  if (!Number.isSafeInteger(masterSeed) || masterSeed > 0xffffffff)
    throw new Error("arrangement-input");
  const credential = instructorCredential(
    readPrivateRegularFile(requiredEnvironment("PLE_UI_WALKTHROUGH_LIVE_CREDENTIAL_FILE")),
  );
  const corpus = await arrangeRetryCorpus(request, {
    baseUrl,
    instructorCredential: credential,
    masterSeed,
  });
  let context: APIRequestContext | undefined;
  try {
    if (process.env["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY"] === "1") {
      return instructorSetupArrangementOutput(corpus);
    }
    const manifest = launcherManifest(
      readPrivateRegularFile(requiredEnvironment("PLE_UI_WALKTHROUGH_LIVE_MANIFEST_FILE")),
    );
    context = await authenticatedInstructorContext(baseUrl, credential);
    const assignments = await arrangeSeededCourseAssignments(context, manifest, corpus);
    return arrangementOutputFor(manifest.assignmentId, corpus, assignments);
  } finally {
    if (context !== undefined) await context.dispose();
  }
}

async function main(): Promise<void> {
  try {
    const output = await arrange();
    const encoded = JSON.stringify(output);
    if (!/^[\x20-\x7e]+$/u.test(encoded) || encoded.length > 2048)
      throw new Error("arrangement-output");
    process.stdout.write(`${encoded}\n`);
  } catch {
    process.stdout.write('{"stage":"arrangement"}\n');
    process.exitCode = 1;
  }
}

if (process.env["PLE_UI_WALKTHROUGH_ARRANGER_CHILD"] === "1") void main();
