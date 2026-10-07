// Compact display for independently optional Bloom metadata.

import type { BloomClassificationView } from "../../generated/api/BloomClassificationView";
import type { JSX } from "solid-js";

export function BloomClassificationText(props: {
  readonly bloom: BloomClassificationView | null;
}): JSX.Element {
  return (
    <span>
      Bloom Cognitive Process: {props.bloom?.cognitiveProcess ?? "Not assigned"}; Bloom Knowledge
      Dimension: {props.bloom?.knowledgeDimension ?? "Not assigned"}
    </span>
  );
}
