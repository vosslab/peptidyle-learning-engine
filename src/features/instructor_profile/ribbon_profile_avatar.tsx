import { Show, createEffect, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import type { ApiClient } from "../../api/client";
import { RibbonIcon } from "../../ribbon/ribbon_icon";
import {
  PROFILE_THUMBNAIL_REPLACED_EVENT,
  createProfileThumbnailUrl,
  isProfileThumbnailReplacedEvent,
} from "./profile_thumbnail_url";

type RibbonProfileAvatarClient = Pick<
  ApiClient,
  "fetchInstructorProfileThumbnail" | "getInstructorProfileThumbnail"
>;

interface RibbonProfileAvatarProps {
  readonly client: RibbonProfileAvatarClient;
}

/** API-aware image presentation injected into the generic Instructor Profile Ribbon control. */
export function RibbonProfileAvatar(props: RibbonProfileAvatarProps): JSX.Element {
  const [thumbnail] = createResource(() => props.client.getInstructorProfileThumbnail());
  const [reference, setReference] = createSignal<string | null>(null);
  const url = createProfileThumbnailUrl(reference, props.client);
  // An upload event is newer than the initial read; keep a late read from restoring stale metadata.
  let receivedReplacement = false;

  createEffect(() => {
    if (receivedReplacement) return;
    if (thumbnail.loading || thumbnail.error !== undefined) {
      setReference(null);
      return;
    }
    setReference(thumbnail()?.reference ?? null);
  });

  function handleReplacement(event: Event): void {
    if (!isProfileThumbnailReplacedEvent(event)) return;
    receivedReplacement = true;
    setReference(event.detail.reference);
  }

  window.addEventListener(PROFILE_THUMBNAIL_REPLACED_EVENT, handleReplacement);
  onCleanup(() => window.removeEventListener(PROFILE_THUMBNAIL_REPLACED_EVENT, handleReplacement));

  return (
    <Show when={url()} fallback={<RibbonIcon glyph="circle-user" />}>
      {(imageUrl) => <img src={imageUrl()} alt="" />}
    </Show>
  );
}
