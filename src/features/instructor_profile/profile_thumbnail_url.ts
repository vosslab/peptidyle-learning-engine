import { createEffect, createResource, createSignal, onCleanup, type Accessor } from "solid-js";

import type { ApiClient } from "../../api/client";
import type { InstructorProfileThumbnail } from "../../api/instructor_profile";

/** Narrow browser capability needed to turn saved Profile metadata into a local image URL. */
export type ProfileThumbnailDeliveryClient = Pick<ApiClient, "fetchInstructorProfileThumbnail">;

interface ProfileThumbnailUrlOptions {
  readonly onDeliveryErrorChange?: (hasDeliveryError: boolean) => void;
}

/** Signals that the authenticated Instructor replaced the account-owned thumbnail. */
export const PROFILE_THUMBNAIL_REPLACED_EVENT = "ple-profile-thumbnail-replaced" as const;

export type ProfileThumbnailReplacedDetail = InstructorProfileThumbnail;

/** Publishes replacement metadata to persistent Instructor Profile presentations in this document. */
export function dispatchProfileThumbnailReplaced(detail: ProfileThumbnailReplacedDetail): void {
  window.dispatchEvent(
    new CustomEvent<ProfileThumbnailReplacedDetail>(PROFILE_THUMBNAIL_REPLACED_EVENT, { detail }),
  );
}

/** Narrows an unknown DOM event to the account-owned thumbnail metadata event. */
export function isProfileThumbnailReplacedEvent(
  event: Event,
): event is CustomEvent<ProfileThumbnailReplacedDetail> {
  if (!(event instanceof CustomEvent)) return false;
  const detail: unknown = event.detail;
  if (detail === null || typeof detail !== "object" || !("reference" in detail)) return false;
  return detail.reference === null || typeof detail.reference === "string";
}

/**
 * Projects reactive, server-normalized Profile thumbnail metadata into an object URL.
 * The component root that calls this helper owns the resulting URL's lifetime.
 */
export function createProfileThumbnailUrl(
  reference: Accessor<string | null>,
  client: ProfileThumbnailDeliveryClient,
  options: ProfileThumbnailUrlOptions = {},
): Accessor<string | undefined> {
  const [delivery] = createResource(reference, async (thumbnailReference) => {
    if (thumbnailReference === null) return undefined;
    return client.fetchInstructorProfileThumbnail(thumbnailReference);
  });
  const [url, setUrl] = createSignal<string>();
  let activeBlob: Blob | undefined;
  let activeUrl: string | undefined;

  function clearUrl(): void {
    if (activeUrl !== undefined) URL.revokeObjectURL(activeUrl);
    activeBlob = undefined;
    activeUrl = undefined;
    setUrl(undefined);
  }

  createEffect(() => {
    options.onDeliveryErrorChange?.(delivery.error !== undefined);
    if (delivery.loading || delivery.error !== undefined) {
      clearUrl();
      return;
    }
    const blob = delivery();
    if (blob === undefined) {
      clearUrl();
      return;
    }
    if (blob === activeBlob) return;
    clearUrl();
    activeBlob = blob;
    activeUrl = URL.createObjectURL(blob);
    setUrl(activeUrl);
  });

  onCleanup(clearUrl);
  return url;
}
