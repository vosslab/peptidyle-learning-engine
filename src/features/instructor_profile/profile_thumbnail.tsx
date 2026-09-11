import { Show, createEffect, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import type { ApiClient } from "../../api/client";
import "./profile_thumbnail.css";

interface ProfileThumbnailProps {
  readonly reference: string | null;
  readonly unavailable: boolean;
  readonly client: Pick<ApiClient, "fetchInstructorProfileThumbnail">;
}

/** Renders the one server-normalized Profile image behind the shared silhouette. */
export function ProfileThumbnail(props: ProfileThumbnailProps): JSX.Element {
  const [delivery] = createResource(
    () => props.reference,
    async (reference) => {
      if (reference === null) return undefined;
      return props.client.fetchInstructorProfileThumbnail(reference);
    },
  );
  const [url, setUrl] = createSignal<string>();

  createEffect(() => {
    if (props.unavailable || delivery.loading || delivery.error !== undefined) {
      setUrl(undefined);
      return;
    }
    const blob = delivery();
    if (blob === undefined) {
      setUrl(undefined);
      return;
    }
    const next = URL.createObjectURL(blob);
    setUrl(next);
    onCleanup(() => URL.revokeObjectURL(next));
  });

  return (
    <>
      <div class="profile-thumbnail" aria-label="Profile image">
        <Show
          when={url()}
          fallback={<span class="profile-thumbnail-placeholder">Profile image</span>}
        >
          {(imageUrl) => <img src={imageUrl()} alt="Your profile" />}
        </Show>
      </div>
      <Show when={props.unavailable || delivery.error !== undefined}>
        <p class="calm-status" role="status">
          Your profile image is unavailable.
        </p>
      </Show>
    </>
  );
}
