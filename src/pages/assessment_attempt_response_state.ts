// assessment_attempt_response_state.ts - exact-position response edit and validation ownership.

import type { StudentResponse } from "../../generated/api/StudentResponse";

export interface ActiveAssessmentAttemptResponse {
  readonly position: number;
  readonly response: StudentResponse;
  readonly editGeneration: number;
  readonly valid: boolean;
}

function sameResponse(left: StudentResponse, right: StudentResponse): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

/**
 * Keeps raw response edits tied to their rendered Question position.  A late
 * format check may update only the exact edit that initiated it.
 */
export class AssessmentAttemptResponseState {
  #active: ActiveAssessmentAttemptResponse | undefined;
  #nextEditGeneration = 0;

  clear(): void {
    this.#nextEditGeneration += 1;
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
    editGeneration: number | undefined,
    valid: boolean,
  ): boolean {
    const active = this.#active;
    if (
      active === undefined ||
      editGeneration === undefined ||
      active.position !== position ||
      active.editGeneration !== editGeneration ||
      !sameResponse(active.response, response)
    ) {
      return false;
    }
    this.#active = { ...active, valid };
    return true;
  }

  current(position: number): ActiveAssessmentAttemptResponse | undefined {
    const active = this.#active;
    return active?.position === position ? active : undefined;
  }

  private replace(position: number, response: StudentResponse, valid: boolean): number {
    this.#nextEditGeneration += 1;
    this.#active = { position, response, editGeneration: this.#nextEditGeneration, valid };
    return this.#nextEditGeneration;
  }
}
