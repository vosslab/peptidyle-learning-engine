// Viewer-zone formatting for the Sysadmin Instructor Account list.

import { createDisplayDateTimeFormatter } from "../format_datetime";

/** Formats a server-supplied sign-in instant in the authenticated viewer's zone. */
export function formatSignInLabel(
  timestamp: number | null,
  formatDateTime: ReturnType<typeof createDisplayDateTimeFormatter>,
): string {
  if (timestamp === null) return "No successful sign-in recorded";
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return "Sign-in time unavailable";
  return formatDateTime(date);
}
