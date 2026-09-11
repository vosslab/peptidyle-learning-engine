// assignment_attempt_response_state.ts - exact-position response edit and validation ownership.

import type { StudentResponse } from "../../generated/api/StudentResponse";

export interface ActiveAssignmentAttemptResponse {
  readonly position: number;
  readonly response: StudentResponse;
  readonly revision: number;
  readonly valid: boolean;
}

function sameResponse(left: StudentResponse, right: StudentResponse): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

/**
 * Keeps raw response edits tied to their rendered Question position.  A late
 * format check may update only the exact edit that initiated it.
 */
export class AssignmentAttemptResponseState {
  #active: ActiveAssignmentAttemptResponse | undefined;
  #nextRevision = 0;

  clear(): void {
    this.#nextRevision += 1;
    this.#active = undefined;
  }

  restore(position: number, response: StudentResponse): number {
    return this.replace(position, response, true);
  }

  edit(position: number, response: StudentResponse): number {
    return this.replace(position, response, false);
  }

  validate(
    position: number,
    response: StudentResponse,
    revision: number | undefined,
    valid: boolean,
  ): boolean {
    const active = this.#active;
    if (
      active === undefined ||
      revision === undefined ||
      active.position !== position ||
      active.revision !== revision ||
      !sameResponse(active.response, response)
    ) {
      return false;
    }
    this.#active = { ...active, valid };
    return true;
  }

  current(position: number): ActiveAssignmentAttemptResponse | undefined {
    const active = this.#active;
    return active?.position === position ? active : undefined;
  }

  private replace(position: number, response: StudentResponse, valid: boolean): number {
    this.#nextRevision += 1;
    this.#active = { position, response, revision: this.#nextRevision, valid };
    return this.#nextRevision;
  }
}
