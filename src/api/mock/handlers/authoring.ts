import { publishedProblemFixture } from "../../../../generated/fixtures/published_problem";
import type { AssignmentSummary } from "../../../../generated/api/AssignmentSummary";
import type { AssignmentEditorInput } from "../../contracts";
import { DecodeError } from "../../decoder";
import { decodeAssignmentEditorInput } from "../../decoders";
import {
  hasDuplicateJsonObjectMember,
  jsonResponse,
  methodNotAllowed,
  pathSegments,
} from "./shared";

export interface MockAssignmentState {
  assignment: AssignmentSummary;
  assignmentTiming: AssignmentEditorInput["assignmentTiming"];
  revision: bigint;
  nextId: bigint;
  nextItemId: bigint;
}

export function createMockAssignmentState(): MockAssignmentState {
  return {
    assignment: structuredClone(publishedProblemFixture.assignment),
    assignmentTiming: { timeLimitSeconds: null },
    revision: 1n,
    nextId: 60n,
    nextItemId: 160n,
  };
}

function noStoreHeaders(revision?: bigint): HeadersInit {
  const headers: Record<string, string> = { "cache-control": "no-store" };
  if (revision !== undefined) headers["etag"] = `"${revision}"`;
  return headers;
}

function assignmentEditorResponse(
  assignment: AssignmentSummary,
  assignmentTiming: AssignmentEditorInput["assignmentTiming"],
  revision: bigint,
  status = 200,
): Response {
  return jsonResponse({ ...assignment, assignmentTiming }, status, noStoreHeaders(revision));
}

function assignmentError(status: number, error: string): Response {
  return jsonResponse({ error }, status, noStoreHeaders());
}

function mockAssignmentId(value: bigint): string {
  return `0198e000-0000-7000-8000-${value.toString().padStart(12, "0")}`;
}

const MOCK_UNSUPPORTED_ASSIGNMENT_VERSION = "0198e000-0000-7000-8000-000000000005";

function mockAssignmentItems(
  state: MockAssignmentState,
  input: AssignmentEditorInput,
  preserveExisting = true,
): AssignmentSummary["items"] {
  const claimed = new Set<string>();
  return input.problems.map((reference, position) => {
    const prior = preserveExisting
      ? state.assignment.items.find(
          (item) =>
            item.deliveryState === "active" &&
            item.reference.problem === reference.problem &&
            item.reference.version === reference.version &&
            !claimed.has(item.id),
        )
      : undefined;
    const id = prior?.id ?? mockAssignmentId(state.nextItemId++);
    claimed.add(id);
    return {
      id,
      reference,
      position,
      pointsPossible: prior?.pointsPossible ?? "1",
      deliveryState: "active",
      scoringMode: prior?.scoringMode ?? "normal",
    };
  });
}

async function assignmentInput(request: Request): Promise<AssignmentEditorInput | undefined> {
  try {
    const text = await request.text();
    if (hasDuplicateJsonObjectMember(text)) return undefined;
    return decodeAssignmentEditorInput(JSON.parse(text), "request");
  } catch (error: unknown) {
    if (error instanceof DecodeError || error instanceof SyntaxError) return undefined;
    throw error;
  }
}

function assignmentValidationFailure(input: AssignmentEditorInput): Response | undefined {
  const violations = input.problems
    .filter((reference) => reference.version === MOCK_UNSUPPORTED_ASSIGNMENT_VERSION)
    .flatMap((reference) => [
      {
        title: "Mock capability-limited published question",
        reference,
        capability: "serverGrading",
      },
      {
        title: "Mock capability-limited published question",
        reference,
        capability: "perQuestionTiming",
      },
    ]);
  if (violations.length === 0) return undefined;
  return jsonResponse(
    { error: "assignment configuration is not supported", violations },
    422,
    noStoreHeaders(),
  );
}

function assignmentRequestFailure(input: AssignmentEditorInput): Response | undefined {
  if (input.problems.length === 0) {
    return assignmentError(422, "assignment must reference at least one published problem version");
  }
  const references = new Set(
    input.problems.map((reference) => `${reference.problem}/${reference.version}`),
  );
  if (references.size !== input.problems.length) {
    return assignmentError(422, "assignment problem references must be unique");
  }
  return undefined;
}

function validAssignmentRevision(value: string | null): boolean {
  if (value === null || !/^"[1-9][0-9]*"$/u.test(value)) return false;
  return BigInt(value.slice(1, -1)) <= 9_223_372_036_854_775_807n;
}

export async function respondAuthoring(
  request: Request,
  state: MockAssignmentState,
  secondaryCourseId: string,
): Promise<Response | undefined> {
  const segments = pathSegments(request);
  const resource = segments[1];
  if (
    resource === "courses" &&
    segments[2] === secondaryCourseId &&
    segments[3] === "assignments" &&
    segments.length === 4
  ) {
    if (request.method !== "GET") return methodNotAllowed(request);
    return jsonResponse({ items: [], nextCursor: null });
  }
  if (
    resource === "courses" &&
    segments[2] === publishedProblemFixture.course.id &&
    segments[3] === "assignments" &&
    segments.length === 4
  ) {
    if (request.method === "GET") {
      return jsonResponse({ items: [state.assignment], nextCursor: null });
    }
    if (request.method !== "POST") return methodNotAllowed(request);
    const input = await assignmentInput(request);
    if (input === undefined) return assignmentError(422, "assignment request is invalid");
    const requestFailure = assignmentRequestFailure(input);
    if (requestFailure !== undefined) return requestFailure;
    const validationFailure = assignmentValidationFailure(input);
    if (validationFailure !== undefined) return validationFailure;
    const id = mockAssignmentId(state.nextId);
    state.nextId += 1n;
    state.assignment = {
      id,
      tenant: publishedProblemFixture.assignment.tenant,
      courseId: publishedProblemFixture.course.id,
      title: input.title,
      items: mockAssignmentItems(state, input, false),
      selectionGroups: [],
      policies: input.policies,
    };
    state.revision = 1n;
    state.assignmentTiming = input.assignmentTiming;
    return assignmentEditorResponse(state.assignment, state.assignmentTiming, state.revision, 201);
  }
  if (
    resource === "courses" &&
    segments[2] === publishedProblemFixture.course.id &&
    segments[3] === "assignments" &&
    segments[4] === state.assignment.id &&
    segments.length === 5
  ) {
    if (request.method !== "PUT") return methodNotAllowed(request);
    const revision = request.headers.get("if-match");
    if (revision === null) return assignmentError(428, "If-Match assignment revision is required");
    if (!validAssignmentRevision(revision)) {
      return assignmentError(422, "If-Match assignment revision is invalid");
    }
    if (revision !== `"${state.revision}"`) {
      return assignmentError(409, "assignment changed; reload it");
    }
    const input = await assignmentInput(request);
    if (input === undefined) return assignmentError(422, "assignment request is invalid");
    const requestFailure = assignmentRequestFailure(input);
    if (requestFailure !== undefined) return requestFailure;
    const validationFailure = assignmentValidationFailure(input);
    if (validationFailure !== undefined) return validationFailure;
    state.assignment = {
      ...state.assignment,
      title: input.title,
      items: mockAssignmentItems(state, input),
      policies: input.policies,
    };
    state.assignmentTiming = input.assignmentTiming;
    state.revision += 1n;
    return assignmentEditorResponse(state.assignment, state.assignmentTiming, state.revision);
  }
  if (resource === "assignments" && segments[2] === state.assignment.id) {
    if (request.method === "GET") {
      return assignmentEditorResponse(state.assignment, state.assignmentTiming, state.revision);
    }
    return methodNotAllowed(request);
  }
  return undefined;
}
