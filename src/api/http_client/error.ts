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

/** A current Assessment mutation cannot proceed in its present lifecycle or edit state. */
export class AssessmentConflictError extends ApiRequestError {
  public constructor(status: 409 | 412 | 428, path: string) {
    super(status, path);
    this.name = "AssessmentConflictError";
  }
}

/** A Blueprint Course replacement lost its strong revision race. */
export class BlueprintCourseConflictError extends ApiRequestError {
  declare public readonly status: 412;

  public constructor(path: string) {
    super(412, path);
    this.name = "BlueprintCourseConflictError";
  }
}
