// Browser-owned lifecycle for one authorized same-origin course banner Blob.

import { createEffect, createResource, createSignal, onCleanup, type Accessor } from "solid-js";

import type { CourseBannerId } from "../../../generated/api/CourseBannerId";
import type { ApiClient } from "../../api/client";

/** Creates one ephemeral object URL and revokes it on replacement, failure, or unmount. */
export function createCourseBannerUrl(
  bannerId: Accessor<CourseBannerId | null>,
  client: Pick<ApiClient, "fetchCourseBanner">,
): Accessor<string | undefined> {
  const [delivery] = createResource(bannerId, (id) => client.fetchCourseBanner(id));
  const [url, setUrl] = createSignal<string>();

  createEffect(() => {
    const selected = bannerId();
    const blob = delivery();
    if (
      selected === null ||
      delivery.loading ||
      delivery.error !== undefined ||
      blob === undefined
    ) {
      setUrl(undefined);
      return;
    }
    const next = URL.createObjectURL(blob);
    setUrl(next);
    onCleanup(() => URL.revokeObjectURL(next));
  });
  return url;
}
