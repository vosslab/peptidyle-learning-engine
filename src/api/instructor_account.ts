// Browser-safe Sysadmin capability for the deliberate Instructor Account lifecycle.

import type { AccountId } from "../../generated/api/AccountId";

export type InstructorAccountState = "active" | "deactivated" | "closed";

export type InstructorAccountPageSize = 50 | 100 | 250;

/** One server page of the Sysadmin Instructor Account list. */
export interface InstructorAccountBrowse {
  readonly query: string;
  readonly state: InstructorAccountState | null;
  readonly pageSize: InstructorAccountPageSize;
  readonly afterAccountId: AccountId | null;
}

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
  /** Last Account ID on this page when another page exists. */
  readonly nextCursor: AccountId | null;
}

/** Create input is sent once and is never reflected by any browser-safe DTO. */
export interface CreateInstructorAccountInput {
  readonly normalizedEmail: string;
  readonly firstName: string;
  readonly lastName: string;
  readonly affiliation: string;
}

export interface CreatedInstructorAccount {
  readonly account: InstructorAccountSummary;
  readonly setupEmailSent: boolean;
}

export interface DeactivateInstructorAccountInput {
  readonly reason: string;
}

/** Same-origin, Sysadmin-only Instructor Account lifecycle boundary. */
export interface InstructorAccountClient {
  readonly listInstructorAccounts: () => Promise<InstructorAccountList>;
  readonly findInstructorAccounts: (
    browse: InstructorAccountBrowse,
  ) => Promise<InstructorAccountList>;
  readonly createInstructorAccount: (
    input: CreateInstructorAccountInput,
  ) => Promise<CreatedInstructorAccount>;
  readonly sendInstructorSetupEmail: (accountId: AccountId) => Promise<boolean>;
  readonly deactivateInstructorAccount: (
    accountId: AccountId,
    input: DeactivateInstructorAccountInput,
  ) => Promise<InstructorAccountSummary>;
  readonly reactivateInstructorAccount: (accountId: AccountId) => Promise<InstructorAccountSummary>;
}
