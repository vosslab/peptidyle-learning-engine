// Coarse browser-safe delivery wording for one invitation or a bulk import.

import type { CourseInvitationEmailDelivery, RosterImportDelivery } from "../api/enrollment";

export function deliveryStatusLabel(outcome: CourseInvitationEmailDelivery): string {
  switch (outcome) {
    case "queued":
      return "Queued";
    case "sentToProvider":
      return "Accepted by submission server";
    case "needsAttention":
      return "Needs attention";
    case "cancelled":
      return "Cancelled";
  }
}

export function invitationDeliveryAnnouncement(outcome: CourseInvitationEmailDelivery): string {
  switch (outcome) {
    case "queued":
      return "Invitation saved. The copy link is ready to share through a trusted course channel.";
    case "sentToProvider":
      return "Invitation saved. The submission server accepted the email; the copy link is also ready.";
    case "needsAttention":
      return "Invitation saved, but email needs attention. Use a fresh explicit resend only when available; otherwise cancel and create a new invitation.";
    case "cancelled":
      return "This invitation is cancelled. Do not share its link; create a new invitation if needed.";
  }
}

export function bulkDeliveryAnnouncement(delivery: ReadonlyArray<RosterImportDelivery>): string {
  const counts = new Map<CourseInvitationEmailDelivery, number>();
  for (const entry of delivery) counts.set(entry.outcome, (counts.get(entry.outcome) ?? 0) + 1);
  const summary = [...counts.entries()]
    .map(([outcome, count]) => `${count} ${deliveryStatusLabel(outcome).toLowerCase()}`)
    .join(", ");
  return summary.length === 0
    ? "No invitations were created."
    : `Bulk invitation status: ${summary}.`;
}
