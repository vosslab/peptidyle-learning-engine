// Current Question and Pool titles used to recognize public IDs.

/** Matches `ple_api.load_recognition_titles`: at most 1000 IDs on each side of one request. */
export const RECOGNITION_TITLE_BATCH_LIMIT = 1000;

export interface RecognitionTitleMaps {
  readonly questions: ReadonlyMap<string, string>;
  readonly pools: ReadonlyMap<string, string>;
}

export function emptyRecognitionTitles(): RecognitionTitleMaps {
  return { questions: new Map(), pools: new Map() };
}

export function mergeRecognitionTitles(
  current: RecognitionTitleMaps,
  incoming: RecognitionTitleMaps,
): RecognitionTitleMaps {
  return {
    questions: new Map([...current.questions, ...incoming.questions]),
    pools: new Map([...current.pools, ...incoming.pools]),
  };
}

/** Splits already distinct canonical IDs into requests the recognition route accepts. */
export function recognitionTitleBatches(
  questionIds: readonly string[],
  poolIds: readonly string[],
): ReadonlyArray<{
  readonly questionIds: readonly string[];
  readonly poolIds: readonly string[];
}> {
  if (questionIds.length === 0 && poolIds.length === 0) return [];
  const count = Math.max(
    Math.ceil(questionIds.length / RECOGNITION_TITLE_BATCH_LIMIT),
    Math.ceil(poolIds.length / RECOGNITION_TITLE_BATCH_LIMIT),
  );
  return Array.from({ length: count }, (_, index) => {
    const start = index * RECOGNITION_TITLE_BATCH_LIMIT;
    const end = start + RECOGNITION_TITLE_BATCH_LIMIT;
    return {
      questionIds: questionIds.slice(start, end),
      poolIds: poolIds.slice(start, end),
    };
  });
}
