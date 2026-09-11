// Viewer-zone formatting for the Sysadmin Instructor Account list.

/** Formats a server-supplied sign-in instant in the authenticated viewer's zone. */
export function formatSignInLabel(timestamp: number | null, displayTimeZone: string): string {
  if (timestamp === null) return "No successful sign-in recorded";
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return "Sign-in time unavailable";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: displayTimeZone,
  }).format(date);
}
