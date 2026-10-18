// Closed projections from the existing Blueprint stewardship and promotion routes.
import type { BlueprintCourseId } from "../../generated/api/BlueprintCourseId";
import type { BlueprintEditNumber } from "../../generated/api/BlueprintEditNumber";

export interface BlueprintStarProjection {
  readonly starCount: number;
  readonly viewerHasStarred: boolean;
}

export interface BlueprintStarredInstructor {
  readonly displayName: string;
}

export interface BlueprintWatchProjection {
  readonly watching: boolean;
}

export interface BlueprintWatchEvent {
  readonly eventId: string;
  readonly kind: "revision" | "archived" | "restored";
  readonly occurredAt: number;
}

export type BlueprintWatchEventKind = BlueprintWatchEvent["kind"];

export type BlueprintWatchQuery = {
  readonly q: string;
  readonly eventKinds: ReadonlyArray<BlueprintWatchEventKind>;
  readonly direction: "ascending" | "descending";
};

export type BlueprintWatchPage = {
  readonly items: ReadonlyArray<BlueprintWatchEvent>;
  readonly pageNumber: number;
  readonly pageSize: number;
  readonly totalCount: number;
  readonly eventKindFacets: ReadonlyArray<{
    readonly value: BlueprintWatchEventKind;
    readonly count: number;
  }>;
};

export interface BlueprintPromotion {
  readonly promoted: boolean;
  readonly blueprintEditNumber: BlueprintEditNumber;
}

export interface BlueprintStewardshipClient {
  readonly getBlueprintStar: (
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<BlueprintStarProjection>;
  readonly setBlueprintStar: (
    blueprintCourseId: BlueprintCourseId,
    starred: boolean,
  ) => Promise<BlueprintStarProjection>;
  readonly getBlueprintStarredInstructors: (
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<readonly BlueprintStarredInstructor[]>;
  readonly getBlueprintWatch: (
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<BlueprintWatchProjection>;
  readonly setBlueprintWatch: (
    blueprintCourseId: BlueprintCourseId,
    watching: boolean,
  ) => Promise<BlueprintWatchProjection>;
  readonly getBlueprintWatchEvents: (
    blueprintCourseId: BlueprintCourseId,
    query: BlueprintWatchQuery,
    pageNumber: number,
    pageSize: number,
  ) => Promise<BlueprintWatchPage>;
  readonly getBlueprintPromotion: (
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<BlueprintPromotion>;
  readonly setBlueprintPromotion: (
    blueprintCourseId: BlueprintCourseId,
    promoted: boolean,
    blueprintEditNumber: BlueprintEditNumber,
  ) => Promise<BlueprintPromotion>;
}
