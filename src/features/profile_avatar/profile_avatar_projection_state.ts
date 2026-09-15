// profile_avatar_projection_state.ts - small browser-lifecycle primitives for self-avatar projections.

import type { ProfileAvatar } from "../../api/profile_avatar";

/** Announces a server-confirmed self-avatar mutation to persistent projections. */
export const PROFILE_AVATAR_CHANGED_EVENT = "ple-profile-avatar-changed" as const;

export interface ObjectUrlApi {
  readonly createObjectURL: (value: Blob) => string;
  readonly revokeObjectURL: (url: string) => void;
}

export interface ProfileAvatarObjectUrlOwner {
  readonly replace: (blob: Blob | undefined) => string | undefined;
  readonly dispose: () => void;
}

/** The visible fallback state stays distinct from a selected provided avatar awaiting its catalog. */
export function profileAvatarProjectionState(
  avatar: ProfileAvatar | null | undefined,
  hasRenderedProvidedAvatar: boolean,
): "generic" | "profileImage" | "provided" | "providedPending" {
  if (avatar === null || avatar === undefined) return "generic";
  if (avatar.kind === "profileImage") return "profileImage";
  return hasRenderedProvidedAvatar ? "provided" : "providedPending";
}

/** Owns exactly one self-image object URL and revokes it on replacement or disposal. */
export function createProfileAvatarObjectUrlOwner(
  objectUrlApi: ObjectUrlApi = URL,
): ProfileAvatarObjectUrlOwner {
  let activeBlob: Blob | undefined;
  let activeUrl: string | undefined;

  function clear(): void {
    if (activeUrl !== undefined) objectUrlApi.revokeObjectURL(activeUrl);
    activeBlob = undefined;
    activeUrl = undefined;
  }

  return {
    replace: (blob): string | undefined => {
      if (blob === activeBlob) return activeUrl;
      clear();
      if (blob === undefined) return undefined;
      activeBlob = blob;
      activeUrl = objectUrlApi.createObjectURL(blob);
      return activeUrl;
    },
    dispose: clear,
  };
}

/** Dispatches an invalidation without carrying Account or avatar data between components. */
export function dispatchProfileAvatarChanged(target: EventTarget = window): void {
  target.dispatchEvent(new Event(PROFILE_AVATAR_CHANGED_EVENT));
}

/** Registers one self-avatar invalidation listener and returns its exact cleanup action. */
export function observeProfileAvatarChanges(target: EventTarget, refresh: () => void): () => void {
  function listener(event: Event): void {
    if (event.type === PROFILE_AVATAR_CHANGED_EVENT) refresh();
  }
  target.addEventListener(PROFILE_AVATAR_CHANGED_EVENT, listener);
  return () => target.removeEventListener(PROFILE_AVATAR_CHANGED_EVENT, listener);
}
