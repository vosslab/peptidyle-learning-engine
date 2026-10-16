// Closed browser contract for one Published Question Star projection.

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { LibraryObjectSearchRequest } from "../../generated/api/LibraryObjectSearchRequest";
import type { QuestionSearchResult } from "../../generated/api/QuestionSearchResult";
import type { LibrarySearchClassification } from "../../generated/api/LibrarySearchClassification";
import type { QuestionSearchFacets } from "../../generated/api/QuestionSearchFacets";
import type { AccountId } from "../../generated/api/AccountId";
import type { LibraryFacetField } from "../../generated/api/LibraryFacetField";
import type { LibraryFacetChoicePage } from "../../generated/api/LibraryFacetChoicePage";

/** One active Instructor shown in an Instructor-only Star list. */
export interface QuestionStarredInstructor {
  readonly displayName: string;
  readonly accountId: string;
}

/**
 * Active-Instructor Star facts for one Published Question.
 *
 * The server returns the canonical Profile identity with a display name. It
 * never returns email, Course, or Watch state.
 */
export interface QuestionStarProjection {
  readonly starCount: number;
  readonly viewerHasStarred: boolean;
  readonly starredInstructors: ReadonlyArray<QuestionStarredInstructor>;
}

/** One Published Question in the signed-in Instructor's personal Star collection. */
export interface StarredQuestionSummary {
  readonly question: QuestionSearchResult;
  readonly ownerAccountId: AccountId;
  readonly classification: LibrarySearchClassification;
  readonly starredAtMicros: number;
}

export type StarredQuestionSort =
  | "starredDateDescending"
  | "starredDateAscending"
  | "titleAscending"
  | "titleDescending"
  | "disciplineAscending"
  | "disciplineDescending"
  | "questionTypeAscending"
  | "questionTypeDescending"
  | "publishedNewest";

/** The signed-in Instructor's Starred Questions, newest first. */
export interface StarredQuestionPage {
  readonly items: ReadonlyArray<StarredQuestionSummary>;
  readonly nextCursor: string | null;
  readonly facets: QuestionSearchFacets;
}

/** Active-Instructor self-service Star boundary for one Published Question. */
export interface QuestionStarClient {
  readonly getQuestionStar: (questionId: PublishedQuestionId) => Promise<QuestionStarProjection>;
  readonly setQuestionStar: (
    questionId: PublishedQuestionId,
    starred: boolean,
  ) => Promise<QuestionStarProjection>;
  readonly listStarredQuestions: (
    query: LibraryObjectSearchRequest,
    sort: StarredQuestionSort,
  ) => Promise<StarredQuestionPage>;
  readonly listStarredFacetChoices: (
    field: LibraryFacetField,
    query: LibraryObjectSearchRequest,
    sort: StarredQuestionSort,
    search: string,
  ) => Promise<LibraryFacetChoicePage>;
}
