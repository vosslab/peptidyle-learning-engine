// Bounded async state for an Instructor's exact submitted-run choice.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseMembershipReference } from "../../generated/api/CourseMembershipReference";
import type { GradingOperationReference } from "../../generated/api/GradingOperationReference";
import type {
  SubmittedRunChoice,
  SubmittedRunChoicesPage,
  SubmittedRunChoicesQuery,
} from "../api/decoders/gradebook_selection";

/** Immutable server request scope for one exact-run chooser dialog. */
export interface GradebookRunChooserScope {
  readonly courseId: CourseId;
  readonly membership: CourseMembershipReference;
  readonly assignment: AssignmentReference;
  readonly operation?: GradingOperationReference;
}

/** Narrow exact-run boundary used by the chooser; the production adapter is the API client. */
export interface GradebookRunChooserRepository {
  readonly getSubmittedRunChoices: (
    courseId: CourseId,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
    query?: SubmittedRunChoicesQuery,
  ) => Promise<SubmittedRunChoicesPage>;
}

export type GradebookRunChooserState =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | {
      readonly kind: "ready";
      readonly rows: ReadonlyArray<SubmittedRunChoice>;
      readonly nextCursor: string | null;
      readonly loadingMore: boolean;
      readonly moreError: boolean;
    };

/**
 * Owns one immutable submitted-run chooser request chain.
 *
 * Replaced initial reads and stale continuations may finish, but only a current
 * generation can update the visible exact-run choices.
 */
export class GradebookRunChooserSession {
  #generation = 0;
  #state: GradebookRunChooserState = { kind: "loading" };
  #disposed = false;
  #nextContinuationRequest = 0;
  #activeContinuationRequest: number | undefined;

  public constructor(
    private readonly scope: GradebookRunChooserScope,
    private readonly repository: GradebookRunChooserRepository,
    private readonly publish: (state: GradebookRunChooserState) => void,
  ) {}

  public get state(): GradebookRunChooserState {
    return this.#state;
  }

  /** Begins the first exact-run read for this immutable chooser scope. */
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
      const rows = appendNewRuns(current.rows, page.rows);
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
      validateUniqueRuns(page.rows);
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

  private getChoices(query: SubmittedRunChoicesQuery): Promise<SubmittedRunChoicesPage> {
    const request =
      this.scope.operation === undefined ? query : { ...query, operationRef: this.scope.operation };
    return this.repository.getSubmittedRunChoices(
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
    base: Extract<GradebookRunChooserState, { readonly kind: "ready" }>,
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

  private setState(state: GradebookRunChooserState): void {
    if (this.#disposed) return;
    this.#state = state;
    this.publish(state);
  }
}

function appendNewRuns(
  previous: ReadonlyArray<SubmittedRunChoice>,
  incoming: ReadonlyArray<SubmittedRunChoice>,
): ReadonlyArray<SubmittedRunChoice> {
  const rows = [...previous, ...incoming];
  validateUniqueRuns(rows);
  return rows;
}

function validateUniqueRuns(rows: ReadonlyArray<SubmittedRunChoice>): void {
  const knownRuns = new Set<string>();
  for (const row of rows) {
    if (knownRuns.has(row.run)) {
      throw new Error("Submitted-run choices repeated a run");
    }
    knownRuns.add(row.run);
  }
}
