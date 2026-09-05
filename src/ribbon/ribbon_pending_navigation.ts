// ribbon_pending_navigation.ts - Ribbon-owned identity for an in-flight navigation.

import { createRenderEffect, createSignal, onCleanup, type Accessor } from "solid-js";

const MAX_RIBBON_DESTINATION_HREF_LENGTH = 2048;
const RIBBON_DESTINATION_BASE_URL = "https://ribbon-navigation.invalid/";
const RIBBON_DESTINATION_ORIGIN = new URL(RIBBON_DESTINATION_BASE_URL).origin;

/** Schedules the same-turn routing check without coupling the Ribbon to a router. */
export type RibbonMicrotaskScheduler = (callback: () => void) => void;

export interface RibbonPendingNavigationOptions {
  /** Router-owned progress; the Ribbon only observes it. */
  readonly routingInFlight: Accessor<boolean>;
  /** Injectable for deterministic tests; defaults to the platform microtask queue. */
  readonly scheduleMicrotask?: RibbonMicrotaskScheduler;
}

/** Narrow presentation state consumed by Ribbon links and their busy treatment. */
export interface RibbonPendingNavigation {
  /** Records the one Ribbon destination which initiated a navigation attempt. */
  readonly activate: (href: string) => void;
  /** The recorded destination while it remains eligible for a pending treatment. */
  readonly pendingDestination: Accessor<string | undefined>;
  /** True only for the recorded Ribbon destination while routing remains in flight. */
  readonly isPending: (href: string) => boolean;
}

function validateDestinationHref(href: string): void {
  if (href.length === 0 || /[\s\\]/u.test(href))
    throw new Error("Ribbon pending navigation needs a nonempty, whitespace-free destination href");
  if (href.length > MAX_RIBBON_DESTINATION_HREF_LENGTH)
    throw new Error("Ribbon pending navigation destination href is too long");

  // This is presentation admission, not authorization. Ribbon controls carry
  // canonical, root-relative route identities, so preserving their exact
  // pathname prevents this small controller from retaining a script, origin,
  // query, fragment, or URL-normalized spelling as pending state.
  if (!href.startsWith("/") || href.includes("//") || href.includes("\\"))
    throw new Error("Ribbon pending navigation needs a canonical root-relative destination href");

  const destination = new URL(href, RIBBON_DESTINATION_BASE_URL);
  if (
    destination.origin !== RIBBON_DESTINATION_ORIGIN ||
    destination.search.length !== 0 ||
    destination.hash.length !== 0 ||
    destination.pathname !== href
  ) {
    throw new Error("Ribbon pending navigation needs a canonical root-relative destination href");
  }
}

function schedulePlatformMicrotask(callback: () => void): void {
  queueMicrotask(callback);
}

/**
 * Keeps navigation feedback with the Ribbon control that caused it. This is
 * presentation state only: it neither reads nor interprets routing, access,
 * session, or API data.
 */
export function createRibbonPendingNavigation(
  options: RibbonPendingNavigationOptions,
): RibbonPendingNavigation {
  const scheduleMicrotask = options.scheduleMicrotask ?? schedulePlatformMicrotask;
  const [pendingDestination, setPendingDestination] = createSignal<string>();
  let disposed = false;

  function clearPendingDestination(): void {
    setPendingDestination(undefined);
  }

  // A settled router clears every Ribbon record, including a redirect to a
  // different destination. The effect deliberately subscribes only to router
  // progress, so activation has one full same-turn chance to start routing.
  createRenderEffect(() => {
    if (!options.routingInFlight()) clearPendingDestination();
  });

  onCleanup(() => {
    disposed = true;
    clearPendingDestination();
  });

  function activate(href: string): void {
    validateDestinationHref(href);
    if (disposed) return;
    setPendingDestination(href);
    scheduleMicrotask(() => {
      if (disposed) return;
      if (pendingDestination() === href && !options.routingInFlight()) clearPendingDestination();
    });
  }

  function isPending(href: string): boolean {
    const pending = pendingDestination();
    return pending === href && options.routingInFlight();
  }

  return { activate, pendingDestination, isPending };
}
