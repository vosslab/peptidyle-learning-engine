// gradebook_page_model.ts - async state owner for the calculated Gradebook page.

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseMembershipReference } from "../../generated/api/CourseMembershipReference";
import type { InstructorGradingOperationReference } from "../../generated/api/InstructorGradingOperationReference";
import type {
  CalculatedAssignmentCell,
  CalculatedGradebookResult,
  CalculatedGradebookRow,
  CalculatedGradebookQuery,
} from "../api/decoders/calculated_gradebook";
import type {
  AssignmentInspectionChoice,
  GradebookSelectionResult,
  StudentSelectionRow,
} from "../api/decoders/gradebook_selection";
import {
  gradebookQueryForFilter,
  type GradebookRouteFilter,
  type GradebookRouteSearchResult,
} from "./gradebook_navigation";

type GradebookPageResult = Extract<CalculatedGradebookResult, { readonly kind: "page" }>;

export type GradebookPageGradebookState =
  | { readonly kind: "loading" }
  | { readonly kind: "error" }
  | { readonly kind: "invalidRoute" }
  | { readonly kind: "reloadRequired"; readonly reason: string }
  | {
      readonly kind: "ready";
      readonly page: GradebookPageResult;
      readonly rows: ReadonlyArray<CalculatedGradebookRow>;
      readonly filter: GradebookRouteFilter | undefined;
      readonly loadingMore: boolean;
      readonly moreError: boolean;
    };

export type GradebookPageSelectionState =
  | { readonly kind: "idle" | "loading" | "error" }
  | {
      readonly kind: "singleStudent";
      readonly membership: CourseMembershipReference;
      readonly assignment: AssignmentReference;
      readonly inspectionChoice: AssignmentInspectionChoice;
    }
  | {
      readonly kind: "studentSelection";
      readonly rows: ReadonlyArray<StudentSelectionRow>;
      readonly nextCursor: string | null;
      readonly loadingMore: boolean;
      readonly moreError: boolean;
    };

/** All data visible to the Gradebook JSX comes through this closed Gradebook Summary Row. */
export interface GradebookPageState {
  readonly route: GradebookRouteSearchResult;
  readonly gradebook: GradebookPageGradebookState;
  readonly operationSelection: GradebookPageSelectionState;
}

/** Narrow transport boundary; generated client decoding remains the transport authority. */
export interface GradebookPageRepository {
  readonly getCalculatedGradebook: (
    courseId: CourseId,
    request: CalculatedGradebookQuery,
  ) => Promise<CalculatedGradebookResult>;
  readonly getGradebookSelection: (
    courseId: CourseId,
    request: {
      readonly filter: {
        readonly kind: "operation";
        readonly operation: InstructorGradingOperationReference;
      };
      readonly cursor?: string;
    },
  ) => Promise<GradebookSelectionResult>;
}

function routeKey(route: GradebookRouteSearchResult): string {
  if (route.kind === "invalid") return `invalid:${route.reason}:${route.key ?? ""}`;
  const filter = route.filter;
  if (filter === undefined) return "all";
  if (filter.kind === "assignment") return `assignment:${filter.assignment}`;
  if (filter.kind === "student") return `student:${filter.membership}`;
  return `operation:${filter.operation}`;
}

function sameColumns(
  expected: ReadonlyArray<CalculatedAssignmentCell>,
  actual: ReadonlyArray<CalculatedAssignmentCell>,
): boolean {
  return (
    expected.length === actual.length &&
    expected.every((cell, index) => cell.assignment === actual[index]?.assignment)
  );
}

function validatePageRows(page: GradebookPageResult): void {
  validateUniqueMemberships(page.rows, (row) => row.membership);
  const expected = page.rows[0]?.assignmentCells;
  if (expected === undefined) return;
  if (page.rows.some((row) => !sameColumns(expected, row.assignmentCells))) {
    throw new Error("Gradebook rows do not share one assignment structure");
  }
  if (
    expected.length !== page.assignmentScoringSnapshots.length ||
    !expected.every(
      (cell, index) => cell.assignment === page.assignmentScoringSnapshots[index]?.assignment,
    )
  ) {
    throw new Error(
      "Gradebook Assignment Scoring Snapshots do not match the visible assignment structure",
    );
  }
}

function validateUniqueMemberships<T>(rows: ReadonlyArray<T>, key: (row: T) => string): void {
  const memberships = new Set<string>();
  for (const row of rows) {
    const membership = key(row);
    if (memberships.has(membership)) {
      throw new Error("Gradebook rows repeated a membership");
    }
    memberships.add(membership);
  }
}

function verifyContinuation(current: GradebookPageResult, next: GradebookPageResult): void {
  if (
    current.schemeRevision !== next.schemeRevision ||
    current.rosterChangeNumber !== next.rosterChangeNumber ||
    current.mode !== next.mode ||
    current.rounding !== next.rounding
  ) {
    throw new Error("Gradebook continuation changed its structure");
  }
  const expected = current.rows[0]?.assignmentCells ?? [];
  if (next.rows.some((row) => !sameColumns(expected, row.assignmentCells))) {
    throw new Error("Gradebook continuation changed its assignment structure");
  }
}

function appendRows<T>(
  existing: ReadonlyArray<T>,
  incoming: ReadonlyArray<T>,
  key: (row: T) => string,
): ReadonlyArray<T> {
  const rows = [...existing, ...incoming];
  validateUniqueMemberships(rows, key);
  return rows;
}

export class GradebookPageSession {
  #generation = 0;
  #requestNumber = 0;
  #gradebookRequest: number | undefined;
  #selectionRequest: number | undefined;
  #disposed = false;
  #state: GradebookPageState = {
    route: { kind: "valid", filter: undefined },
    gradebook: { kind: "loading" },
    operationSelection: { kind: "idle" },
  };

  public constructor(
    private readonly courseId: CourseId,
    private readonly repository: GradebookPageRepository,
    private readonly publish: (state: GradebookPageState) => void,
  ) {}

  public get state(): GradebookPageState {
    return this.#state;
  }

  public reset(route: GradebookRouteSearchResult): void {
    if (this.#disposed) return;
    this.#generation += 1;
    this.#gradebookRequest = undefined;
    this.#selectionRequest = undefined;
    if (route.kind === "invalid") {
      this.setState({
        route,
        gradebook: { kind: "invalidRoute" },
        operationSelection: { kind: "idle" },
      });
      return;
    }
    const selection =
      route.filter?.kind === "operation" ? { kind: "loading" as const } : { kind: "idle" as const };
    this.setState({ route, gradebook: { kind: "loading" }, operationSelection: selection });
    this.startGradebook(route, this.#generation);
    if (route.filter?.kind === "operation")
      this.startSelection(route.filter.operation, this.#generation);
  }

  public reload(): void {
    this.reset(this.#state.route);
  }

  public loadMoreGradebook(): void {
    const current = this.#state.gradebook;
    if (
      this.#disposed ||
      this.#gradebookRequest !== undefined ||
      current.kind !== "ready" ||
      current.page.nextCursor === null
    ) {
      return;
    }
    const generation = this.#generation;
    const identity = routeKey(this.#state.route);
    const request = this.nextRequest();
    this.#gradebookRequest = request;
    this.setState({
      ...this.#state,
      gradebook: { ...current, loadingMore: true, moreError: false },
    });
    void this.continueGradebook(current, generation, identity, request);
  }

  public loadMoreSelection(): void {
    const current = this.#state.operationSelection;
    const filter = this.#state.route.kind === "valid" ? this.#state.route.filter : undefined;
    if (
      this.#disposed ||
      this.#selectionRequest !== undefined ||
      current.kind !== "studentSelection" ||
      current.nextCursor === null ||
      filter?.kind !== "operation"
    ) {
      return;
    }
    const generation = this.#generation;
    const identity = routeKey(this.#state.route);
    const request = this.nextRequest();
    this.#selectionRequest = request;
    this.setState({
      ...this.#state,
      operationSelection: { ...current, loadingMore: true, moreError: false },
    });
    void this.continueSelection(current, filter.operation, generation, identity, request);
  }

  public retrySelection(): void {
    const route = this.#state.route;
    if (this.#disposed || route.kind !== "valid" || route.filter?.kind !== "operation") return;
    this.#selectionRequest = undefined;
    this.setState({ ...this.#state, operationSelection: { kind: "loading" } });
    this.startSelection(route.filter.operation, this.#generation);
  }

  public dispose(): void {
    this.#disposed = true;
    this.#generation += 1;
    this.#gradebookRequest = undefined;
    this.#selectionRequest = undefined;
  }

  private nextRequest(): number {
    this.#requestNumber += 1;
    return this.#requestNumber;
  }

  private setState(state: GradebookPageState): void {
    if (this.#disposed) return;
    this.#state = state;
    this.publish(state);
  }

  private current(generation: number, identity: string): boolean {
    return (
      !this.#disposed && generation === this.#generation && identity === routeKey(this.#state.route)
    );
  }

  private startGradebook(
    route: Extract<GradebookRouteSearchResult, { readonly kind: "valid" }>,
    generation: number,
  ): void {
    const request = this.nextRequest();
    const identity = routeKey(route);
    this.#gradebookRequest = request;
    void this.loadInitialGradebook(route, generation, identity, request);
  }

  private async loadInitialGradebook(
    route: Extract<GradebookRouteSearchResult, { readonly kind: "valid" }>,
    generation: number,
    identity: string,
    request: number,
  ): Promise<void> {
    try {
      const page = await this.repository.getCalculatedGradebook(
        this.courseId,
        gradebookQueryForFilter(route.filter),
      );
      if (!this.current(generation, identity) || this.#gradebookRequest !== request) return;
      if (page.kind === "reloadRequired") {
        this.setState({
          ...this.#state,
          gradebook: { kind: "reloadRequired", reason: page.reason },
        });
        return;
      }
      validatePageRows(page);
      this.setState({
        ...this.#state,
        gradebook: {
          kind: "ready",
          page,
          rows: page.rows,
          filter: route.filter,
          loadingMore: false,
          moreError: false,
        },
      });
    } catch {
      if (this.current(generation, identity) && this.#gradebookRequest === request) {
        this.setState({ ...this.#state, gradebook: { kind: "error" } });
      }
    } finally {
      if (this.#gradebookRequest === request) this.#gradebookRequest = undefined;
    }
  }

  private async continueGradebook(
    current: Extract<GradebookPageGradebookState, { readonly kind: "ready" }>,
    generation: number,
    identity: string,
    request: number,
  ): Promise<void> {
    try {
      const page = await this.repository.getCalculatedGradebook(this.courseId, {
        ...gradebookQueryForFilter(current.filter),
        cursor: current.page.nextCursor ?? undefined,
      });
      if (!this.current(generation, identity) || this.#gradebookRequest !== request) return;
      if (page.kind === "reloadRequired") {
        this.setState({
          ...this.#state,
          gradebook: { kind: "reloadRequired", reason: page.reason },
        });
        return;
      }
      validatePageRows(page);
      verifyContinuation(current.page, page);
      const rows = appendRows(current.rows, page.rows, (row) => row.membership);
      this.setState({
        ...this.#state,
        gradebook: {
          kind: "ready",
          page,
          rows,
          filter: current.filter,
          loadingMore: false,
          moreError: false,
        },
      });
    } catch {
      if (this.current(generation, identity) && this.#gradebookRequest === request) {
        this.setState({
          ...this.#state,
          gradebook: { ...current, loadingMore: false, moreError: true },
        });
      }
    } finally {
      if (this.#gradebookRequest === request) this.#gradebookRequest = undefined;
    }
  }

  private startSelection(operation: InstructorGradingOperationReference, generation: number): void {
    const request = this.nextRequest();
    const identity = `operation:${operation}`;
    this.#selectionRequest = request;
    void this.loadInitialSelection(operation, generation, identity, request);
  }

  private async loadInitialSelection(
    operation: InstructorGradingOperationReference,
    generation: number,
    identity: string,
    request: number,
  ): Promise<void> {
    try {
      const result = await this.repository.getGradebookSelection(this.courseId, {
        filter: { kind: "operation", operation },
      });
      if (!this.current(generation, identity) || this.#selectionRequest !== request) return;
      this.setState({ ...this.#state, operationSelection: selectionState(result) });
    } catch {
      if (this.current(generation, identity) && this.#selectionRequest === request) {
        this.setState({ ...this.#state, operationSelection: { kind: "error" } });
      }
    } finally {
      if (this.#selectionRequest === request) this.#selectionRequest = undefined;
    }
  }

  private async continueSelection(
    current: Extract<GradebookPageSelectionState, { readonly kind: "studentSelection" }>,
    operation: InstructorGradingOperationReference,
    generation: number,
    identity: string,
    request: number,
  ): Promise<void> {
    try {
      const result = await this.repository.getGradebookSelection(this.courseId, {
        filter: { kind: "operation", operation },
        cursor: current.nextCursor ?? undefined,
      });
      if (!this.current(generation, identity) || this.#selectionRequest !== request) return;
      if (result.kind !== "studentSelection")
        throw new Error("Gradebook selection changed its result shape");
      const rows = appendRows(current.rows, result.rows, (row) => row.membership);
      this.setState({
        ...this.#state,
        operationSelection: {
          kind: "studentSelection",
          rows,
          nextCursor: result.nextCursor,
          loadingMore: false,
          moreError: false,
        },
      });
    } catch {
      if (this.current(generation, identity) && this.#selectionRequest === request) {
        this.setState({
          ...this.#state,
          operationSelection: { ...current, loadingMore: false, moreError: true },
        });
      }
    } finally {
      if (this.#selectionRequest === request) this.#selectionRequest = undefined;
    }
  }
}

function selectionState(result: GradebookSelectionResult): GradebookPageSelectionState {
  if (result.kind === "singleStudent") return result;
  validateUniqueMemberships(result.rows, (row) => row.membership);
  return {
    kind: "studentSelection",
    rows: result.rows,
    nextCursor: result.nextCursor,
    loadingMore: false,
    moreError: false,
  };
}
