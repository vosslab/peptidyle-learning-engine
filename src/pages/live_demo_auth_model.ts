// Pure UI-state helpers for deployment-gated live-demo authentication surfaces.

import { ApiRequestError } from "../api/http_client/error";
import type { SeededDemoPersona } from "../api/live_demo";

export type SysadminOwnershipAvailability = "ready" | "complete";

/** A 404 is intentional deployment absence, while every other error remains actionable. */
export function isLiveDemoUnavailable(error: unknown): boolean {
  return error instanceof ApiRequestError && error.status === 404;
}

/** Keeps the public ownership-status response to its two browser-visible states. */
export function sysadminOwnershipAvailability(available: boolean): SysadminOwnershipAvailability {
  return available ? "ready" : "complete";
}

/** Public copy is derived only from the server's closed persona key, never a role request. */
export function seededDemoDescription(persona: SeededDemoPersona): string {
  switch (persona) {
    case "elenaInstructor":
      return "Explore a seeded account with instructor course work.";
    case "maryStudent":
      return "Explore a seeded account with student course activity.";
    case "jackStudent":
      return "Explore a second seeded account with student course activity.";
    case "averyStudent":
      return "Explore a seeded account that can later receive a course invitation.";
  }
}
