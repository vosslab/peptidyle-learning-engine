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
  public constructor(status: 409 | 428, path: string) {
    super(status, path);
    this.name = "AssignmentConflictError";
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

/** A local banner cannot satisfy the bounded upload transport contract. */
export class CourseAppearanceFileError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "CourseAppearanceFileError";
  }
}

/** A complete, authoritative assignment capability report from a 422 response. */
export class AssignmentValidationError extends ApiRequestError {
  public readonly violations: ReadonlyArray<import("../contracts").AssignmentCapabilityViolation>;

  public constructor(
    path: string,
    violations: ReadonlyArray<import("../contracts").AssignmentCapabilityViolation>,
  ) {
    super(422, path);
    this.name = "AssignmentValidationError";
    this.violations = violations;
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
