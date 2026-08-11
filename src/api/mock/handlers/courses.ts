import { publishedProblemFixture } from "../../../../generated/fixtures/published_problem";
import type { CourseAppearance } from "../../../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../../../generated/api/CourseAppearanceUpdate";
import type { CourseSummary } from "../../../../generated/api/CourseSummary";
import { DecodeError } from "../../decoder";
import { createMockAssignmentState, respondAuthoring } from "./authoring";
import { decodeCourseAppearanceUpdate, decodeCourseCreateInput } from "../../decoders";
import {
  handlesResource,
  hasDuplicateJsonObjectMember,
  jsonResponse,
  methodNotAllowed,
  pathSegments,
  routeNotFound,
} from "./shared";

/** Default course appearance shared by the browser mock and route fixtures. */
export const mockCourseAppearance: CourseAppearance = {
  theme: "grass",
  revision: "1",
  banner: null,
};

/** A second authorized course used to prove cross-course theme replacement. */
export const secondaryMockCourse: CourseSummary = {
  ...publishedProblemFixture.course,
  id: "0198e000-0000-7000-8000-000000000015",
  title: "Genetics pilot",
};

export const secondaryMockCourseAppearance: CourseAppearance = {
  theme: "ocean",
  revision: "1",
  banner: null,
};

function mockCourse(courseId: string | undefined): CourseSummary | undefined {
  if (courseId === publishedProblemFixture.course.id) return publishedProblemFixture.course;
  return courseId === secondaryMockCourse.id ? secondaryMockCourse : undefined;
}

export function canHandleCourse(request: Request): boolean {
  return handlesResource(request, ["courses", "assignments"]);
}

interface MockCourseAppearanceState {
  readonly appearances: Map<string, CourseAppearance>;
  readonly candidates: Map<string, string>;
  nextCandidate: bigint;
  nextBanner: bigint;
}

export function createMockCourseAppearanceState(): MockCourseAppearanceState {
  return {
    appearances: new Map([
      [publishedProblemFixture.course.id, structuredClone(mockCourseAppearance)],
      [secondaryMockCourse.id, structuredClone(secondaryMockCourseAppearance)],
    ]),
    candidates: new Map(),
    nextCandidate: 800n,
    nextBanner: 900n,
  };
}

function noStoreHeaders(revision?: bigint): HeadersInit {
  const headers: Record<string, string> = { "cache-control": "no-store" };
  if (revision !== undefined) headers["etag"] = `"${revision}"`;
  return headers;
}

function mockCourseAppearanceId(value: bigint): string {
  return `0198e000-0000-7000-8000-${value.toString().padStart(12, "0")}`;
}

async function appearanceInput(request: Request): Promise<CourseAppearanceUpdate | undefined> {
  try {
    const text = await request.text();
    if (hasDuplicateJsonObjectMember(text)) return undefined;
    return decodeCourseAppearanceUpdate(JSON.parse(text), "request");
  } catch (error: unknown) {
    if (error instanceof DecodeError || error instanceof SyntaxError) return undefined;
    throw error;
  }
}

async function courseCreateInput(request: Request): Promise<string | undefined> {
  try {
    const text = await request.text();
    if (hasDuplicateJsonObjectMember(text)) return undefined;
    return decodeCourseCreateInput(JSON.parse(text), "request").title;
  } catch (error: unknown) {
    if (error instanceof DecodeError || error instanceof SyntaxError) return undefined;
    throw error;
  }
}

function appearanceError(status: number, error: string): Response {
  return jsonResponse({ error }, status, noStoreHeaders());
}

function appearanceResponse(appearance: CourseAppearance): Response {
  return jsonResponse(appearance, 200, noStoreHeaders(BigInt(appearance.revision)));
}

async function uploadMockBannerCandidate(
  request: Request,
  state: MockCourseAppearanceState,
  courseId: string,
): Promise<Response> {
  const mediaType = request.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase();
  if (!["image/jpeg", "image/png", "image/webp"].includes(mediaType ?? "")) {
    return appearanceError(415, "banner must be JPEG, PNG, or WebP");
  }
  const bytes = await request.arrayBuffer();
  if (bytes.byteLength === 0) return appearanceError(422, "banner image is empty");
  if (bytes.byteLength > 2 * 1_024 * 1_024) {
    return appearanceError(413, "banner upload is too large");
  }
  const candidate = mockCourseAppearanceId(state.nextCandidate);
  state.nextCandidate += 1n;
  state.candidates.set(candidate, courseId);
  return jsonResponse({ candidate }, 201, noStoreHeaders());
}

async function saveMockCourseAppearance(
  request: Request,
  state: MockCourseAppearanceState,
  courseId: string,
  current: CourseAppearance,
): Promise<Response> {
  const revision = request.headers.get("if-match");
  if (revision === null) return appearanceError(428, "If-Match is required");
  if (revision !== `"${current.revision}"`) {
    return appearanceError(412, "course appearance changed; reload current settings");
  }
  const update = await appearanceInput(request);
  if (update === undefined) return appearanceError(422, "appearance update is invalid");
  let banner = current.banner;
  switch (update.banner.kind) {
    case "remove":
      banner = null;
      break;
    case "keep":
      if (banner === null) return appearanceError(422, "there is no current banner to keep");
      banner = { ...banner, alternativeText: update.banner.alternativeText };
      break;
    case "replace":
      if (state.candidates.get(update.banner.candidate) !== courseId) {
        return appearanceError(422, "banner candidate is no longer available");
      }
      state.candidates.delete(update.banner.candidate);
      banner = {
        id: mockCourseAppearanceId(state.nextBanner),
        alternativeText: update.banner.alternativeText,
      };
      state.nextBanner += 1n;
      break;
  }
  const next = {
    theme: update.theme,
    revision: (BigInt(current.revision) + 1n).toString(),
    banner,
  } satisfies CourseAppearance;
  state.appearances.set(courseId, next);
  return appearanceResponse(next);
}

export async function respondCourse(
  request: Request,
  assignmentState: import("./authoring").MockAssignmentState = createMockAssignmentState(),
  appearanceState = createMockCourseAppearanceState(),
): Promise<Response> {
  const segments = pathSegments(request);
  const resource = segments[1];
  if (resource === "courses" && segments.length === 2) {
    if (request.method === "GET") {
      return jsonResponse({ items: [publishedProblemFixture.course], nextCursor: null });
    }
    if (request.method === "POST") {
      const title = await courseCreateInput(request);
      if (title === undefined) return appearanceError(422, "course title is invalid");
      return jsonResponse(
        { ...publishedProblemFixture.course, title, role: "instructor" },
        201,
        noStoreHeaders(),
      );
    }
    return methodNotAllowed(request);
  }
  if (resource === "courses" && segments.length === 3 && mockCourse(segments[2]) !== undefined) {
    if (request.method !== "GET") return methodNotAllowed(request);
    return jsonResponse(mockCourse(segments[2]));
  }
  if (
    resource === "courses" &&
    segments.length === 4 &&
    appearanceState.appearances.has(segments[2] ?? "") &&
    segments[3] === "appearance"
  ) {
    const courseId = segments[2];
    if (courseId === undefined) return routeNotFound(request);
    const appearance = appearanceState.appearances.get(courseId);
    if (appearance === undefined) return routeNotFound(request);
    if (request.method === "GET") return appearanceResponse(appearance);
    if (request.method === "PUT") {
      return await saveMockCourseAppearance(request, appearanceState, courseId, appearance);
    }
    return methodNotAllowed(request);
  }
  if (
    resource === "courses" &&
    segments.length === 5 &&
    appearanceState.appearances.has(segments[2] ?? "") &&
    segments[3] === "appearance" &&
    segments[4] === "banner-candidates"
  ) {
    if (request.method !== "POST") return methodNotAllowed(request);
    const courseId = segments[2];
    if (courseId === undefined) return routeNotFound(request);
    return await uploadMockBannerCandidate(request, appearanceState, courseId);
  }
  if (
    resource === "courses" &&
    segments.length === 4 &&
    segments[2] === publishedProblemFixture.course.id &&
    segments[3] === "gradebook"
  ) {
    if (request.method !== "GET") return methodNotAllowed(request);
    return jsonResponse({ items: publishedProblemFixture.gradebook, nextCursor: null });
  }
  const authoring = await respondAuthoring(request, assignmentState, secondaryMockCourse.id);
  if (authoring !== undefined) return authoring;
  return routeNotFound(request);
}
