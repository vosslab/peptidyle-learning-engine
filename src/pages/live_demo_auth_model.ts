// Pure UI-state helpers for deployment-gated live-demo authentication surfaces.

import { ApiRequestError } from "../api/http_client/error";
import { DecodeError } from "../api/decoder";
import type { AccountCourse } from "../api/enrollment";
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

export type SysadminSetupState =
  | { readonly kind: "loading" }
  | { readonly kind: SysadminOwnershipAvailability }
  | { readonly kind: "ownership-busy" }
  | { readonly kind: "courses-busy"; readonly message: string }
  | { readonly kind: "courses"; readonly courses: ReadonlyArray<AccountCourse> }
  | { readonly kind: "empty" }
  | { readonly kind: "unavailable" }
  | {
      readonly kind: "availability-error";
      readonly message: string;
    }
  | {
      readonly kind: "ownership-error";
      readonly message: string;
      readonly focus: "proof" | "label";
    }
  | {
      readonly kind: "course-error";
      readonly message: string;
    };

/** The ownership form is a one-claim surface and never returns after a successful claim. */
export function sysadminSetupFormVisible(state: SysadminSetupState): boolean {
  return (
    state.kind === "ready" || state.kind === "ownership-busy" || state.kind === "ownership-error"
  );
}

export function sysadminSetupBusyMessage(state: SysadminSetupState): string {
  switch (state.kind) {
    case "ownership-busy":
      return "Setting up your administrator passkey...";
    case "courses-busy":
      return state.message;
    default:
      return "";
  }
}

export function sysadminSetupErrorMessage(state: SysadminSetupState): string {
  switch (state.kind) {
    case "availability-error":
    case "ownership-error":
    case "course-error":
      return state.message;
    default:
      return "";
  }
}

export function sysadminSetupRetry(
  state: SysadminSetupState,
): "availability" | "courses" | undefined {
  switch (state.kind) {
    case "availability-error":
      return "availability";
    case "course-error":
      return "courses";
    default:
      return undefined;
  }
}

export function sysadminSetupCourseRows(state: SysadminSetupState): ReadonlyArray<AccountCourse> {
  return state.kind === "courses" ? state.courses : [];
}

export function sysadminOwnershipFailure(error: unknown): SysadminSetupState {
  if (error instanceof ApiRequestError && error.status === 409) return { kind: "complete" };
  if (
    error instanceof DecodeError ||
    (error instanceof ApiRequestError && [400, 401, 403].includes(error.status))
  ) {
    return {
      kind: "ownership-error",
      message:
        "The setup code could not be verified. Check the code supplied for this demo and try again.",
      focus: "proof",
    };
  }
  return {
    kind: "ownership-error",
    message:
      "The passkey was not set up. You can try again with this device or another passkey-capable device.",
    focus: "label",
  };
}

/** A post-claim failure retries ordinary course loading, never ownership or passkey enrollment. */
export function sysadminCourseFailure(action: "list" | "select"): SysadminSetupState {
  return {
    kind: "course-error",
    message:
      action === "list"
        ? "Your administrator passkey is ready, but your course list could not load."
        : "That course could not be opened. Reload your course list or return to sign in.",
  };
}

/** The operator proof is intentionally ephemeral, including after both success and failure. */
export function clearSysadminOwnershipProof(): "" {
  return "";
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
