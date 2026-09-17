// Closed projections from the existing Blueprint stewardship and promotion routes.
import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";

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
  /** Strong, quoted metadata validator, retained exactly for If-Match. */
  readonly metadataEtag: string;
}

export interface BlueprintStewardshipClient {
  readonly getBlueprintStar: (
    reference: BlueprintCourseReference,
  ) => Promise<BlueprintStarProjection>;
  readonly setBlueprintStar: (
    reference: BlueprintCourseReference,
    starred: boolean,
  ) => Promise<BlueprintStarProjection>;
  readonly getBlueprintStarredInstructors: (
    reference: BlueprintCourseReference,
  ) => Promise<readonly BlueprintStarredInstructor[]>;
  readonly getBlueprintWatch: (
    reference: BlueprintCourseReference,
  ) => Promise<BlueprintWatchProjection>;
  readonly setBlueprintWatch: (
    reference: BlueprintCourseReference,
    watching: boolean,
  ) => Promise<BlueprintWatchProjection>;
  readonly getBlueprintWatchEvents: (
    reference: BlueprintCourseReference,
    limit?: number,
  ) => Promise<readonly BlueprintWatchEvent[]>;
  readonly getBlueprintPromotion: (
    reference: BlueprintCourseReference,
  ) => Promise<BlueprintPromotion>;
  readonly setBlueprintPromotion: (
    reference: BlueprintCourseReference,
    promoted: boolean,
    metadataEtag: string,
  ) => Promise<BlueprintPromotion>;
}
