import { publishedProblemFixture } from "../../../../generated/fixtures/published_problem";
import type { AssignmentSummary } from "../../../../generated/api/AssignmentSummary";
import type { AssignmentEditorInput } from "../../contracts";
import {
  decodeAddAssignmentItemInput,
  decodeAssignmentCreateInput,
  decodeAssignmentEditorInput,
  decodeReplaceAssignmentItemQuestionInput,
} from "../../decoders";
import { jsonResponse, methodNotAllowed, pathSegments } from "./shared";

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
function headers(revision: bigint): HeadersInit {
  return { "cache-control": "no-store", etag: `"${revision}"` };
}
function response(state: MockAssignmentState, status = 200): Response {
  return jsonResponse(
    { ...state.assignment, assignmentTiming: state.assignmentTiming },
    status,
    headers(state.revision),
  );
}
function error(status: number, message: string): Response {
  return jsonResponse({ error: message }, status, { "cache-control": "no-store" });
}
function itemId(value: bigint): string {
  return `0198e000-0000-7000-8000-${value.toString().padStart(12, "0")}`;
}
function valid(revision: string | null, state: MockAssignmentState): Response | undefined {
  if (revision === null) return error(428, "If-Match assignment revision is required");
  if (revision !== `"${state.revision}"`) return error(409, "assignment changed; reload it");
  return undefined;
}
async function json(request: Request): Promise<unknown> {
  try {
    return JSON.parse(await request.text());
  } catch {
    return undefined;
  }
}
function known(questionId: string): typeof publishedProblemFixture.catalogProblem | undefined {
  return questionId === publishedProblemFixture.catalogProblem.questionId
    ? publishedProblemFixture.catalogProblem
    : undefined;
}
function makeItems(
  state: MockAssignmentState,
  questionIds: ReadonlyArray<string>,
): AssignmentSummary["items"] {
  return questionIds.map((questionId, position) => {
    const summary = known(questionId);
    return {
      id: itemId(state.nextItemId++),
      questionId,
      title: summary?.metadata.title ?? "Published question",
      backend: summary?.backend ?? "native",
      capabilities: summary?.capabilities ?? [],
      position,
      pointsPossible: "1",
      deliveryState: "active",
      scoringMode: "normal",
    };
  });
}
export async function respondAuthoring(
  request: Request,
  state: MockAssignmentState,
  secondaryCourseId: string,
): Promise<Response | undefined> {
  const s = pathSegments(request);
  const course = publishedProblemFixture.course.id;
  if (s[1] === "courses" && s[2] === secondaryCourseId && s[3] === "assignments" && s.length === 4)
    return request.method === "GET"
      ? jsonResponse({ items: [], nextCursor: null })
      : methodNotAllowed(request);
  if (s[1] === "courses" && s[2] === course && s[3] === "assignments" && s.length === 4) {
    if (request.method === "GET")
      return jsonResponse({ items: [state.assignment], nextCursor: null });
    if (request.method !== "POST") return methodNotAllowed(request);
    const body = await json(request);
    try {
      const input = decodeAssignmentCreateInput(body, "request");
      state.assignment = {
        ...state.assignment,
        id: itemId(state.nextId++),
        title: input.title,
        items: makeItems(state, input.questionIds),
        selectionGroups: [],
        policies: input.policies,
      };
      state.assignmentTiming = input.assignmentTiming;
      state.revision = 1n;
      return response(state, 201);
    } catch {
      return error(422, "assignment request is invalid");
    }
  }
  if (
    s[1] === "courses" &&
    s[2] === course &&
    s[3] === "assignments" &&
    s[4] === state.assignment.id &&
    s.length === 5
  ) {
    if (request.method !== "PUT") return methodNotAllowed(request);
    const conflict = valid(request.headers.get("if-match"), state);
    if (conflict !== undefined) return conflict;
    try {
      const input = decodeAssignmentEditorInput(await json(request), "request");
      state.assignment = {
        ...state.assignment,
        title: input.title,
        items: input.items.map((item) => ({
          ...state.assignment.items.find((old) => old.id === item.id)!,
          ...item,
        })),
        policies: input.policies,
      };
      state.assignmentTiming = input.assignmentTiming;
      state.revision += 1n;
      return response(state);
    } catch {
      return error(422, "assignment request is invalid");
    }
  }
  if (
    s[1] === "courses" &&
    s[2] === course &&
    s[3] === "assignments" &&
    s[4] === state.assignment.id &&
    s[5] === "items"
  ) {
    const conflict = valid(request.headers.get("if-match"), state);
    if (conflict !== undefined) return conflict;
    if (s.length === 6 && request.method === "POST") {
      let body;
      try {
        body = decodeAddAssignmentItemInput(await json(request), "request");
      } catch {
        return error(422, "assignment item request is invalid");
      }
      const next = makeItems(state, [body.questionId])[0];
      if (next === undefined) return error(422, "assignment item request is invalid");
      state.assignment = {
        ...state.assignment,
        items: [...state.assignment.items, { ...next, position: body.position }],
      };
      state.revision += 1n;
      return response(state);
    }
    const item = s[6];
    if (item === undefined) return methodNotAllowed(request);
    if (s.length === 7 && request.method === "DELETE") {
      if ((await request.text()).length !== 0)
        return error(400, "assignment item removal does not accept a request body");
      state.assignment = {
        ...state.assignment,
        items: state.assignment.items.filter((candidate) => candidate.id !== item),
      };
      state.revision += 1n;
      return response(state);
    }
    if (s.length === 8 && s[7] === "question" && request.method === "PUT") {
      let body;
      try {
        body = decodeReplaceAssignmentItemQuestionInput(await json(request), "request");
      } catch {
        return error(422, "assignment item request is invalid");
      }
      const summary = known(body.questionId);
      if (summary === undefined) return error(422, "assignment item request is invalid");
      state.assignment = {
        ...state.assignment,
        items: state.assignment.items.map((candidate) =>
          candidate.id === item
            ? {
                ...candidate,
                questionId: summary.questionId,
                title: summary.metadata.title,
                backend: summary.backend,
                capabilities: summary.capabilities,
              }
            : candidate,
        ),
      };
      state.revision += 1n;
      return response(state);
    }
    return methodNotAllowed(request);
  }
  if (s[1] === "assignments" && s[2] === state.assignment.id && request.method === "GET")
    return response(state);
  return undefined;
}
