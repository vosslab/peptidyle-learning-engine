// Pure presentation helpers for the bounded teaching-team API projections.

import type { CourseInvitationStateView } from "../../generated/api/CourseInvitationStateView";

export interface ReferenceRow {
  readonly reference: string;
}

/** Appends one cursor page without duplicating a stable server row. */
export function appendTeachingTeamPage<T extends ReferenceRow>(
  current: ReadonlyArray<T>,
  next: ReadonlyArray<T>,
): ReadonlyArray<T> {
  return appendTeachingTeamRows(current, next, (row) => row.reference);
}

/** Appends a cursor page using the API row's stable, browser-safe Reference. */
export function appendTeachingTeamRows<T>(
  current: ReadonlyArray<T>,
  next: ReadonlyArray<T>,
  key: (row: T) => string,
): ReadonlyArray<T> {
  const existing = new Set(current.map(key));
  const appended = next.filter((row) => !existing.has(key(row)));
  return [...current, ...appended];
}

export function invitationStateLabel(state: CourseInvitationStateView): string {
  switch (state) {
    case "pending":
      return "Pending response";
    case "expired":
      return "Expired";
    case "accepted":
      return "Accepted";
    case "declined":
      return "Declined";
    case "revoked":
      return "Canceled";
  }
}

/** The server timestamp is displayed as an absolute value; no browser clock decides actionability. */
export function serverExpiryCopy(expiresAt: number): string {
  return `Expires at ${new Date(expiresAt).toLocaleString()} (server supplied)`;
}

export function isPendingInvitation(state: CourseInvitationStateView): boolean {
  return state === "pending";
}

export function finalInstructorConflictCopy(): string {
  return "This course must keep one active instructor. Reload the teaching team before trying again.";
}

export function conflictRecoveryCopy(): string {
  return "The teaching team changed. The current list was reloaded; your search and selected invitee remain available.";
}
