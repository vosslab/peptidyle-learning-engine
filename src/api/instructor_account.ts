// Browser-safe Sysadmin capability for the deliberate Instructor Account lifecycle.

import type { AccountId } from "../../generated/api/AccountId";

export type InstructorAccountState = "active" | "deactivated" | "closed";

/** The only Instructor Account fields available to the browser. */
export interface InstructorAccountSummary {
  readonly id: AccountId;
  readonly state: InstructorAccountState;
  readonly lastSuccessfulSignIn: number | null;
  /** Static cross-account projection only; null also conceals private Profile images. */
  readonly providedAvatarId: string | null;
}

/** Sysadmin-owned display context for the closed Instructor Account list. */
export interface InstructorAccountList {
  readonly accounts: ReadonlyArray<InstructorAccountSummary>;
  readonly displayTimeZone: string;
}

/** Sysadmin-only fact recorded before a separate Instructor Account creation. */
export interface CompleteInstructorIdentityVettingInput {
  readonly normalizedEmail: string;
  readonly verifiedInstructorDisplayName: string;
}

/** Opaque receipt; it is never an Account identity or browser projection. */
export interface InstructorIdentityVettingReceipt {
  readonly vettingDecisionId: string;
}

/** Create input is sent once and is never reflected by any browser-safe DTO. */
export interface CreateInstructorAccountInput {
  readonly normalizedEmail: string;
  readonly vettingDecisionId: string;
}

export interface DeactivateInstructorAccountInput {
  readonly reason: string;
}

/** Same-origin, Sysadmin-only Instructor Account lifecycle boundary. */
export interface InstructorAccountClient {
  readonly listInstructorAccounts: () => Promise<InstructorAccountList>;
  readonly completeInstructorIdentityVetting: (
    input: CompleteInstructorIdentityVettingInput,
  ) => Promise<InstructorIdentityVettingReceipt>;
  readonly createInstructorAccount: (
    input: CreateInstructorAccountInput,
  ) => Promise<InstructorAccountSummary>;
  readonly deactivateInstructorAccount: (
    accountId: AccountId,
    input: DeactivateInstructorAccountInput,
  ) => Promise<InstructorAccountSummary>;
  readonly reactivateInstructorAccount: (accountId: AccountId) => Promise<InstructorAccountSummary>;
}
