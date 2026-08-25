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
export class ProblemCurationConflictError extends ApiRequestError {
  declare public readonly status: 412;

  public constructor(path: string) {
    super(412, path);
    this.name = "ProblemCurationConflictError";
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

/** A bounded server correction tied to exactly one teaching-settings control. */
export class AssignmentTeachingSettingsValidationError extends ApiRequestError {
  public constructor(
    path: string,
    public readonly failure: import("../../../generated/api/AssignmentTeachingSettingsValidationFailure").AssignmentTeachingSettingsValidationFailure,
  ) {
    super(422, path);
    this.name = "AssignmentTeachingSettingsValidationError";
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
