// Bounded async state for an Instructor's exact submitted Assignment Attempt choice.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseMembershipReference } from "../../generated/api/CourseMembershipReference";
import type { InstructorGradingOperationReference } from "../../generated/api/InstructorGradingOperationReference";
import type {
  SubmittedAssignmentAttemptChoice,
  SubmittedAssignmentAttemptChoicesPage,
  SubmittedAssignmentAttemptChoicesQuery,
} from "../api/decoders/gradebook_selection";

/** Immutable server request scope for one exact Assignment Attempt chooser dialog. */
export interface GradebookAssignmentAttemptChooserScope {
  readonly courseId: CourseId;
  readonly membership: CourseMembershipReference;
  readonly assignment: AssignmentReference;
  readonly operation?: InstructorGradingOperationReference;
}

/** Narrow Assignment Attempt boundary used by the chooser; the API client owns transport. */
export interface GradebookAssignmentAttemptChooserRepository {
  readonly getSubmittedAssignmentAttemptChoices: (
    courseId: CourseId,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
    query?: SubmittedAssignmentAttemptChoicesQuery,
  ) => Promise<SubmittedAssignmentAttemptChoicesPage>;
}

export type GradebookAssignmentAttemptChooserState =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | {
      readonly kind: "ready";
      readonly rows: ReadonlyArray<SubmittedAssignmentAttemptChoice>;
      readonly nextCursor: string | null;
      readonly loadingMore: boolean;
      readonly moreError: boolean;
    };

/**
 * Owns one immutable submitted Assignment Attempt chooser request chain.
 *
 * Replaced initial reads and stale continuations may finish, but only a current
 * generation can update the visible exact Assignment Attempt choices.
 */
export class GradebookAssignmentAttemptChooserSession {
  #generation = 0;
  #state: GradebookAssignmentAttemptChooserState = { kind: "loading" };
  #disposed = false;
  #nextContinuationRequest = 0;
  #activeContinuationRequest: number | undefined;

  public constructor(
    private readonly scope: GradebookAssignmentAttemptChooserScope,
    private readonly repository: GradebookAssignmentAttemptChooserRepository,
    private readonly publish: (state: GradebookAssignmentAttemptChooserState) => void,
  ) {}

  public get state(): GradebookAssignmentAttemptChooserState {
    return this.#state;
  }

  /** Begins the first exact Assignment Attempt read for this immutable chooser scope. */
  public async start(): Promise<void> {
    await this.loadInitial();
  }

  /** Replaces any earlier initial read; only this retry may publish its result. */
  public async retry(): Promise<void> {
    await this.loadInitial();
  }

  /** Appends one cursor page while retaining the visible choices on a current failure. */
  public async loadMore(): Promise<void> {
    const current = this.#state;
    if (
      this.#disposed ||
      current.kind !== "ready" ||
      current.nextCursor === null ||
      this.#activeContinuationRequest !== undefined
    ) {
      return;
    }
    const generation = this.#generation;
    const cursor = current.nextCursor;
    const request = this.#nextContinuationRequest + 1;
    this.#nextContinuationRequest = request;
    this.#activeContinuationRequest = request;
    const loadingState = { ...current, loadingMore: true, moreError: false };
    this.setState(loadingState);
    try {
      const page = await this.getChoices({ cursor });
      if (!this.isCurrentContinuation(generation, loadingState, cursor, request)) return;
      const rows = appendNewAssignmentAttempts(current.rows, page.rows);
      this.setState({
        kind: "ready",
        rows,
        nextCursor: page.nextCursor,
        loadingMore: false,
        moreError: false,
      });
    } catch {
      if (this.isCurrentContinuation(generation, loadingState, cursor, request)) {
        this.setState({ ...current, loadingMore: false, moreError: true });
      }
    } finally {
      if (this.#activeContinuationRequest === request) {
        this.#activeContinuationRequest = undefined;
      }
    }
  }

  /** Invalidates every pending completion and permanently silences this dialog instance. */
  public dispose(): void {
    this.#disposed = true;
    this.#generation += 1;
    this.#activeContinuationRequest = undefined;
  }

  private async loadInitial(): Promise<void> {
    if (this.#disposed) return;
    this.#generation += 1;
    this.#activeContinuationRequest = undefined;
    const generation = this.#generation;
    this.setState({ kind: "loading" });
    try {
      const page = await this.getChoices({});
      if (!this.isCurrentGeneration(generation)) return;
      validateUniqueAssignmentAttempts(page.rows);
      this.setState({
        kind: "ready",
        rows: page.rows,
        nextCursor: page.nextCursor,
        loadingMore: false,
        moreError: false,
      });
    } catch {
      if (this.isCurrentGeneration(generation)) this.setState({ kind: "error" });
    }
  }

  private getChoices(
    query: SubmittedAssignmentAttemptChoicesQuery,
  ): Promise<SubmittedAssignmentAttemptChoicesPage> {
    const request =
      this.scope.operation === undefined ? query : { ...query, operationRef: this.scope.operation };
    return this.repository.getSubmittedAssignmentAttemptChoices(
      this.scope.courseId,
      this.scope.membership,
      this.scope.assignment,
      request,
    );
  }

  private isCurrentGeneration(generation: number): boolean {
    return !this.#disposed && generation === this.#generation;
  }

  private isCurrentContinuation(
    generation: number,
    base: Extract<GradebookAssignmentAttemptChooserState, { readonly kind: "ready" }>,
    cursor: string,
    request: number,
  ): boolean {
    return (
      this.isCurrentGeneration(generation) &&
      this.#state === base &&
      this.#state.nextCursor === cursor &&
      this.#activeContinuationRequest === request
    );
  }

  private setState(state: GradebookAssignmentAttemptChooserState): void {
    if (this.#disposed) return;
    this.#state = state;
    this.publish(state);
  }
}

function appendNewAssignmentAttempts(
  previous: ReadonlyArray<SubmittedAssignmentAttemptChoice>,
  incoming: ReadonlyArray<SubmittedAssignmentAttemptChoice>,
): ReadonlyArray<SubmittedAssignmentAttemptChoice> {
  const rows = [...previous, ...incoming];
  validateUniqueAssignmentAttempts(rows);
  return rows;
}

function validateUniqueAssignmentAttempts(
  rows: ReadonlyArray<SubmittedAssignmentAttemptChoice>,
): void {
  const knownAssignmentAttempts = new Set<string>();
  for (const row of rows) {
    if (knownAssignmentAttempts.has(row.assignmentAttempt)) {
      throw new Error("Submitted Assignment Attempt choices repeated an Assignment Attempt");
    }
    knownAssignmentAttempts.add(row.assignmentAttempt);
  }
}
