// Pure presentation helpers for account-owned pending Course Invitations.

import type { AccountTimeZone } from "../../generated/api/AccountTimeZone";
import type { CourseInvitationStateView } from "../../generated/api/CourseInvitationStateView";
import { createDisplayDateTimeFormatter } from "../format_datetime";

interface InvitationRow {
  readonly id: string;
}

/** Appends one cursor page without duplicating a stable server row. */
export function appendPendingInvitationPage<T extends InvitationRow>(
  current: ReadonlyArray<T>,
  next: ReadonlyArray<T>,
): ReadonlyArray<T> {
  const existing = new Set(current.map((row) => row.id));
  return [...current, ...next.filter((row) => !existing.has(row.id))];
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

/** The server instant is rendered in the authorized viewer zone; it never decides actionability. */
export function serverExpiryCopy(
  expiresAt: number,
  displayTimeZone: AccountTimeZone,
  formatDateTime: ReturnType<typeof createDisplayDateTimeFormatter>,
): string {
  const rendered = formatDateTime(expiresAt);
  return `Expires at ${rendered} (${displayTimeZone})`;
}

export function isPendingInvitation(state: CourseInvitationStateView): boolean {
  return state === "pending";
}

export function conflictRecoveryCopy(): string {
  return "The invitation changed. The current list was reloaded.";
}
