// Loads current Question and Pool titles without dropping titles already shown.

import { createEffect, createSignal, onCleanup, type Accessor } from "solid-js";

import type { BlueprintCourseClient } from "../api/blueprint_course";
import {
  emptyRecognitionTitles,
  mergeRecognitionTitles,
  type RecognitionTitleMaps,
} from "../api/recognition_titles";

const TITLE_LOAD_FAILURE =
  "Question and Pool titles could not be loaded. Each public ID below still identifies its record.";

/** Keeps known titles when a later read fails, and merges a successful read. */
export function recognitionTitlesResource(
  client: () => Pick<BlueprintCourseClient, "loadRecognitionTitles"> | undefined,
  ids: () => { readonly questionIds: readonly string[]; readonly poolIds: readonly string[] },
): {
  readonly titles: Accessor<RecognitionTitleMaps>;
  readonly status: Accessor<string>;
  readonly remember: (incoming: RecognitionTitleMaps) => void;
} {
  const [titles, setTitles] = createSignal<RecognitionTitleMaps>(emptyRecognitionTitles());
  const [status, setStatus] = createSignal("");
  createEffect(() => {
    const current = client();
    const request = ids();
    if (
      current === undefined ||
      (request.questionIds.length === 0 && request.poolIds.length === 0)
    ) {
      return;
    }
    let cancelled = false;
    void current.loadRecognitionTitles(request.questionIds, request.poolIds).then(
      (loaded) => {
        if (cancelled) return;
        setTitles((existing) => mergeRecognitionTitles(existing, loaded));
        setStatus("");
      },
      () => {
        if (!cancelled) setStatus(TITLE_LOAD_FAILURE);
      },
    );
    onCleanup(() => {
      cancelled = true;
    });
  });
  return {
    titles,
    status,
    remember: (incoming) => setTitles((existing) => mergeRecognitionTitles(existing, incoming)),
  };
}
