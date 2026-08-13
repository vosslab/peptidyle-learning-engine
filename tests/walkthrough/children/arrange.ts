// arrange.ts - fixed, redacted supported-API setup for the UI walkthrough.

import { lstatSync, readFileSync } from "node:fs";

import { type AssignmentArrangement } from "../../playwright/simulator/assignment_arrangement";
import { childInputsFromArguments } from "./child_inputs";

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
    | "api-exam-assignment"
    | "launcher-chapter-one-genetics";
  readonly baselineAssignmentId?: string;
  readonly courseId?: string;
  readonly problemId?: string;
  readonly versionId?: string;
  /** Private runner handoff only; it is stripped before any report is written. */
  readonly catalogSearchTitle?: string;
  readonly masteryAssignmentId?: string;
  readonly examAssignmentId?: string;
  /** Private instructor-only source of the four exact immutable Genetics selections. */
  readonly questions?: readonly ChapterOneQuestionReference[];
}

export interface ArrangementOutput {
  readonly arrangements: readonly ArrangementRecord[];
}

export interface ChapterOneQuestionReference {
  readonly displayId: string;
  readonly problemId: string;
  readonly versionId: string;
}

const DISPLAY_ID = /^P-[1-9][0-9]*-v[1-9][0-9]*$/u;
const GENETICS_CHAPTER_ONE_SLUGS = [
  "genetics-disorders-webwork-mc",
  "genetics-disorders-webwork-matching",
  "genetics-disorders-flat-mc",
  "genetics-disorders-flat-matching",
] as const;

/**
 * Reads the launcher-produced, answer-free Genetics manifest. This is product
 * runtime output, rather than a test-arranged replacement corpus.
 */
export function chapterOneGeneticsQuestions(
  manifestContents: string,
): readonly ChapterOneQuestionReference[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(manifestContents);
  } catch {
    throw new Error("arrangement-input");
  }
  if (!isRecord(parsed) || Object.keys(parsed).length !== 1 || !Array.isArray(parsed["chapters"])) {
    throw new Error("arrangement-input");
  }
  const chapter = parsed["chapters"].find(
    (value): value is Readonly<Record<string, unknown>> =>
      isRecord(value) && value["slug"] === "genetics-chapter-1",
  );
  if (
    chapter === undefined ||
    Object.keys(chapter).length !== 5 ||
    !["slug", "courseId", "assignmentId", "enrollmentId", "questions"].every((key) =>
      owns(chapter, key),
    ) ||
    !Array.isArray(chapter["questions"]) ||
    chapter["questions"].length !== 4
  ) {
    throw new Error("arrangement-input");
  }
  const questions = chapter["questions"].map((value, index): ChapterOneQuestionReference => {
    if (
      !isRecord(value) ||
      Object.keys(value).length !== 4 ||
      !["slug", "displayId", "problemId", "versionId"].every((key) => owns(value, key)) ||
      value["slug"] !== GENETICS_CHAPTER_ONE_SLUGS[index]
    )
      throw new Error("arrangement-input");
    const displayId = value["displayId"];
    const problemId = value["problemId"];
    const versionId = value["versionId"];
    // The seed manifest intentionally does not duplicate titles. The visible
    // catalog's exact human ID is the authoritative selection key.
    if (
      typeof displayId !== "string" ||
      !DISPLAY_ID.test(displayId) ||
      !isUuid(problemId) ||
      !isUuid(versionId)
    ) {
      throw new Error("arrangement-input");
    }
    return { displayId, problemId, versionId };
  });
  if (new Set(questions.map((question) => question.displayId)).size !== 4) {
    throw new Error("arrangement-input");
  }
  return questions;
}

export function instructorSetupArrangementOutput(
  questions: readonly ChapterOneQuestionReference[],
): ArrangementOutput {
  if (questions.length !== 4) throw new Error("arrangement-input");
  return {
    arrangements: [
      {
        label: "launcher-chapter-one-genetics",
        questions,
      },
    ],
  };
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

function owns(value: object, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(value, key);
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

/** Retained for the focused arranger API boundary test without ambient configuration. */
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

function arrange(): ArrangementOutput {
  const inputs = childInputsFromArguments(process.argv.slice(2), "arrangement");
  const manifest = readPrivateRegularFile(inputs.chapterOneManifestFile);
  return instructorSetupArrangementOutput(chapterOneGeneticsQuestions(manifest));
}

function main(): void {
  try {
    const output = arrange();
    const encoded = JSON.stringify(output);
    if (!/^[\x20-\x7e]+$/u.test(encoded) || encoded.length > 2048)
      throw new Error("arrangement-output");
    process.stdout.write(`${encoded}\n`);
  } catch {
    process.stdout.write('{"stage":"arrangement"}\n');
    process.exitCode = 1;
  }
}

main();
