// Pure UI-state helpers for deployment-gated live-demo account entry.

import { ApiRequestError } from "../api/http_client/error";
import type { SeededDemoPersona } from "../api/live_demo";

/** A 404 is intentional deployment absence, while every other error remains actionable. */
export function isLiveDemoUnavailable(error: unknown): boolean {
  return error instanceof ApiRequestError && error.status === 404;
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
    case "morganSysadmin":
      return "Explore a seeded Sysadmin account with administrator tools.";
  }
}
