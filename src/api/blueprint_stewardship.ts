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
  readonly kind: "revision" | "published" | "archived" | "restored";
  readonly occurredAt: number;
}

export interface BlueprintPromotion {
  readonly promoted: boolean;
  readonly blueprintEditNumber: BlueprintEditNumber;
}

export interface BlueprintStewardshipClient {
  readonly getBlueprintStar: (reference: BlueprintCourseId) => Promise<BlueprintStarProjection>;
  readonly setBlueprintStar: (
    reference: BlueprintCourseId,
    starred: boolean,
  ) => Promise<BlueprintStarProjection>;
  readonly getBlueprintStarredInstructors: (
    reference: BlueprintCourseId,
  ) => Promise<readonly BlueprintStarredInstructor[]>;
  readonly getBlueprintWatch: (reference: BlueprintCourseId) => Promise<BlueprintWatchProjection>;
  readonly setBlueprintWatch: (
    reference: BlueprintCourseId,
    watching: boolean,
  ) => Promise<BlueprintWatchProjection>;
  readonly getBlueprintWatchEvents: (
    reference: BlueprintCourseId,
    limit?: number,
  ) => Promise<readonly BlueprintWatchEvent[]>;
  readonly getBlueprintPromotion: (reference: BlueprintCourseId) => Promise<BlueprintPromotion>;
  readonly setBlueprintPromotion: (
    reference: BlueprintCourseId,
    promoted: boolean,
    blueprintEditNumber: BlueprintEditNumber,
  ) => Promise<BlueprintPromotion>;
}
