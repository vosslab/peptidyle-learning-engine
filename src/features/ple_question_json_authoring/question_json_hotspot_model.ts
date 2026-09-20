import type { PleQuestionJsonImageDescriptor } from "./question_json_client";
import type {
  PleQuestionJsonDocument,
  PleQuestionJsonHotspotResponse,
} from "./question_json_source";

/** A source surface only comes from a verified upload; replacing it preserves authored content. */
export function setPleQuestionJsonHotspotImage(
  source: PleQuestionJsonDocument,
  asset: PleQuestionJsonImageDescriptor,
): PleQuestionJsonDocument {
  const previous = source.response.kind === "hotspot" ? source.response : null;
  const response: PleQuestionJsonHotspotResponse = {
    kind: "hotspot",
    surface: {
      questionImageAssetId: asset.questionImageAssetId,
      checksum: asset.checksum,
      description:
        previous?.surface.description ??
        "Describe the image and the regions students should identify.",
    },
    regions: previous?.regions ?? [
      { id: "region_1", label: "Region 1", x: 0, y: 0, width: 10000, height: 10000 },
    ],
    correctRegions: previous?.correctRegions ?? ["region_1"],
  };
  return { ...source, response };
}

export function nextHotspotRegionId(response: PleQuestionJsonHotspotResponse): string {
  const ids = new Set(response.regions.map((region) => region.id));
  let index = 1;
  while (ids.has(`region_${index}`)) index += 1;
  return `region_${index}`;
}
