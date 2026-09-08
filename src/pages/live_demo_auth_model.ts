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
      return "Explore Elena Rivera's seeded Instructor account.";
    case "maryStudent":
      return "Explore Mary Okafor's seeded Student account.";
    case "jackStudent":
      return "Explore Jack Nguyen's seeded Student account.";
    case "averyStudent":
      return "Explore Avery Thompson's seeded Student account.";
    case "morganSysadmin":
      return "Explore Morgan Delgado's seeded Sysadmin account with administrator tools.";
  }
}

/** Public status names availability without exposing a missing Account's identity or state. */
export function seededDemoAvailabilityStatus(unavailableAccountCount: number): string {
  if (unavailableAccountCount === 1) {
    return "One demo Account is unavailable. The available choices remain usable.";
  }
  return `${unavailableAccountCount} demo Accounts are unavailable. The available choices remain usable.`;
}
