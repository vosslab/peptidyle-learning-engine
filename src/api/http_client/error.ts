/** Non-successful HTTP result without echoing a potentially sensitive body. */
export class ApiRequestError extends Error {
  public readonly status: number;
  public readonly path: string;

  public constructor(status: number, path: string) {
    super(`API request ${path} failed with status ${status}`);
    this.name = "ApiRequestError";
    this.status = status;
    this.path = path;
  }
}

/** A bounded course-term refusal with one safe field-specific correction. */
export class CourseTermValidationError extends ApiRequestError {
  public readonly failure: import("../../../generated/api/CourseTermValidationFailure").CourseTermValidationFailure;

  public constructor(
    path: string,
    failure: import("../../../generated/api/CourseTermValidationFailure").CourseTermValidationFailure,
  ) {
    super(422, path);
    this.name = "CourseTermValidationError";
    this.failure = failure;
  }
}

/** Successful HTTP response that violated the browser-safe API contract. */
export class ApiProtocolError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "ApiProtocolError";
  }
}

/** An optimistic workspace save lost its strong revision race; retain local edits. */
export class WorkspaceConflictError extends ApiRequestError {
  public constructor(status: 409 | 428, path: string) {
    super(status, path);
    this.name = "WorkspaceConflictError";
  }
}

/** A revisioned assignment save lost its server-side compare-and-swap race. */
export class AssignmentConflictError extends ApiRequestError {
  public constructor(status: 409 | 412 | 428, path: string) {
    super(status, path);
    this.name = "AssignmentConflictError";
  }
}

/** A 409 content-save refusal because immutable Student work has been issued. */
export class AssignmentIssuedWorkError extends ApiRequestError {
  declare public readonly status: 409;

  public constructor(path: string) {
    super(409, path);
    this.name = "AssignmentIssuedWorkError";
  }
}

export type AssignmentContentSaveFailure =
  | { readonly kind: "staleRevision"; readonly message: string }
  | { readonly kind: "issuedStudentWork"; readonly message: string }
  | { readonly kind: "retryable"; readonly message: string };

/**
 * Maps the closed content-save transport boundary to semantic instructor copy.
 *
 * The returned text intentionally contains no endpoint, status, or internal
 * identity diagnostics.  Callers retain their local draft for every outcome.
 */
export function resolveAssignmentContentSaveFailure(error: unknown): AssignmentContentSaveFailure {
  if (error instanceof AssignmentIssuedWorkError) {
    return {
      kind: "issuedStudentWork",
      message:
        "Student work has already been issued, so this assignment's question structure remains unchanged.",
    };
  }
  if (
    (error instanceof AssignmentConflictError && error.status === 412) ||
    (error instanceof ApiRequestError && error.status === 412)
  ) {
    return {
      kind: "staleRevision",
      message:
        "This assignment changed before your questions could be saved. Your entered title and question changes are still here.",
    };
  }
  return {
    kind: "retryable",
    message:
      "Questions could not be saved. Your entered title and question changes are still here.",
  };
}

export type AssignmentFixedItemReplacementFailure =
  | { readonly kind: "staleRevision"; readonly message: string }
  | { readonly kind: "retryable"; readonly message: string };

/** Maps the focused future-run replacement boundary to user-safe recovery copy. */
export function resolveAssignmentFixedItemReplacementFailure(
  error: unknown,
): AssignmentFixedItemReplacementFailure {
  if (
    (error instanceof AssignmentConflictError && error.status === 412) ||
    (error instanceof ApiRequestError && error.status === 412)
  ) {
    return {
      kind: "staleRevision",
      message:
        "This assignment changed before the selected question could be replaced. Reload the latest assignment before retrying.",
    };
  }
  return {
    kind: "retryable",
    message: "The selected question could not be replaced. It remains selected so you can retry.",
  };
}

/** A preview revision became stale; callers retain the hypothetical draft and reload it. */
export class PreviewPlaneConflictError extends ApiRequestError {
  declare public readonly status: 412;

  public constructor(path: string) {
    super(412, path);
    this.name = "PreviewPlaneConflictError";
  }
}

/** A course-appearance save lost its exact strong-revision race. */
export class CourseAppearanceConflictError extends ApiRequestError {
  declare public readonly status: 412;

  public constructor(path: string) {
    super(412, path);
    this.name = "CourseAppearanceConflictError";
  }
}

/** A course-grade save lost its strong ETag race; the caller must retain its draft. */
export class CourseGradeSchemeConflictError extends ApiRequestError {
  declare public readonly status: 412;
  public constructor(path: string) {
    super(412, path);
    this.name = "CourseGradeSchemeConflictError";
  }
}

/** A curation mutation lost its strong revision race; reload its current state before retrying. */
export class QuestionCurationConflictError extends ApiRequestError {
  declare public readonly status: 412;

  public constructor(path: string) {
    super(412, path);
    this.name = "QuestionCurationConflictError";
  }
}

/** A reusable-curriculum replacement lost its strong revision race. */
export class ReusableCurriculumConflictError extends ApiRequestError {
  declare public readonly status: 412;

  public constructor(path: string) {
    super(412, path);
    this.name = "ReusableCurriculumConflictError";
  }
}

/** A local banner cannot satisfy the bounded upload transport contract. */
export class CourseAppearanceFileError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "CourseAppearanceFileError";
  }
}

/** A complete browser-safe Policies correction list from the aggregate save boundary. */
export class AssignmentPoliciesValidationError extends ApiRequestError {
  public constructor(
    path: string,
    public readonly issues: ReadonlyArray<
      import("../../../generated/api/AssignmentPoliciesValidationIssue").AssignmentPoliciesValidationIssue
    >,
  ) {
    super(422, path);
    this.name = "AssignmentPoliciesValidationError";
  }
}

/** A publish 422 that retains every server-reported capability violation. */
export class PublicationValidationError extends ApiRequestError {
  public readonly messageForAuthor: string;
  public readonly violations: ReadonlyArray<import("../contracts").PublicationViolation>;

  public constructor(
    path: string,
    messageForAuthor: string,
    violations: ReadonlyArray<import("../contracts").PublicationViolation>,
  ) {
    super(422, path);
    this.name = "PublicationValidationError";
    this.messageForAuthor = messageForAuthor;
    this.violations = violations;
  }
}
