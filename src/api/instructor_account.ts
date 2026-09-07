// Browser-safe Sysadmin capability for the deliberate Instructor Account lifecycle.

import type { AccountReference } from "../../generated/api/AccountReference";

export type InstructorAccountState = "active" | "deactivated" | "closed";

/** The only Instructor Account fields available to the browser. */
export interface InstructorAccountSummary {
  readonly reference: AccountReference;
  readonly state: InstructorAccountState;
  readonly lastSuccessfulSignIn: number | null;
}

/** Create input is sent once and is never reflected by any browser-safe DTO. */
export interface CreateInstructorAccountInput {
  readonly normalizedEmail: string;
}

export interface DeactivateInstructorAccountInput {
  readonly reason: string;
}

/** Same-origin, Sysadmin-only Instructor Account lifecycle boundary. */
export interface InstructorAccountClient {
  readonly listInstructorAccounts: () => Promise<ReadonlyArray<InstructorAccountSummary>>;
  readonly createInstructorAccount: (
    input: CreateInstructorAccountInput,
  ) => Promise<InstructorAccountSummary>;
  readonly deactivateInstructorAccount: (
    reference: AccountReference,
    input: DeactivateInstructorAccountInput,
  ) => Promise<InstructorAccountSummary>;
  readonly reactivateInstructorAccount: (
    reference: AccountReference,
  ) => Promise<InstructorAccountSummary>;
}
