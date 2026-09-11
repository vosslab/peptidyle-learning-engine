import { Show, createSignal, type Accessor, type JSX } from "solid-js";

import type { ApiClient } from "../../api/client";
import { RibbonIcon } from "../../ribbon/ribbon_icon";
import { createProfileThumbnailUrl } from "./profile_thumbnail_url";
import "./profile_thumbnail.css";

interface ProfileThumbnailProps {
  readonly reference: Accessor<string | null>;
  readonly unavailable: boolean;
  readonly client: Pick<ApiClient, "fetchInstructorProfileThumbnail">;
}

/** Renders the one server-normalized Profile image behind the shared silhouette. */
export function ProfileThumbnail(props: ProfileThumbnailProps): JSX.Element {
  const [deliveryFailed, setDeliveryFailed] = createSignal(false);
  const url = createProfileThumbnailUrl(props.reference, props.client, {
    onDeliveryErrorChange: setDeliveryFailed,
  });

  return (
    <>
      <div class="profile-thumbnail" aria-label="Profile image">
        <Show
          when={url()}
          fallback={
            <span class="profile-thumbnail-placeholder" aria-hidden="true">
              <RibbonIcon glyph="circle-user" />
            </span>
          }
        >
          {(imageUrl) => <img src={imageUrl()} alt="Your profile" />}
        </Show>
      </div>
      <Show when={props.unavailable || deliveryFailed()}>
        <p class="calm-status" role="status">
          Your profile image is unavailable.
        </p>
      </Show>
    </>
  );
}
