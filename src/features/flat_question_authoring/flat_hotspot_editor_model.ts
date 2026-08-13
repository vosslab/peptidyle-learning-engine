// flat_hotspot_editor_model.ts - immutable edits for keyboard-first HOTSPOT authoring.

import type { FlatQuestionAssetDescriptor } from "./flat_question_asset_client";
import type {
  FlatQuestionHotspotRegion,
  FlatQuestionHotspotResponse,
  FlatQuestionSourceV2,
} from "./flat_question_source";

const NORMALIZED_MAXIMUM = 10_000;
const MAXIMUM_REGIONS = 100;

export type FlatHotspotEditResult = {
  readonly response: FlatQuestionHotspotResponse;
  readonly changed: boolean;
  readonly error: string | null;
};

function changed(response: FlatQuestionHotspotResponse): FlatHotspotEditResult {
  return { response, changed: true, error: null };
}

function refused(response: FlatQuestionHotspotResponse, error: string): FlatHotspotEditResult {
  return { response, changed: false, error };
}

/** Creates source only after the picker supplies a server-verified immutable descriptor. */
export function hotspotResponseFromAsset(
  asset: FlatQuestionAssetDescriptor,
  description: string,
): FlatQuestionHotspotResponse {
  return {
    kind: "hotspot",
    surface: {
      asset: asset.assetId,
      checksum: asset.contentChecksum,
      description,
    },
    regions: [
      { id: "region_1", label: "First labeled region", x: 0, y: 0, width: 2_500, height: 2_500 },
    ],
    correctRegions: ["region_1"],
  };
}

export function hotspotSourceFromAsset(
  source: FlatQuestionSourceV2,
  asset: FlatQuestionAssetDescriptor,
  description: string,
): FlatQuestionSourceV2 {
  const response = hotspotResponseFromAsset(asset, description);
  return { ...source, response };
}

function hasWholeCoordinate(value: number): boolean {
  return Number.isInteger(value) && value >= 0 && value <= NORMALIZED_MAXIMUM;
}

function isInsideSurface(region: FlatQuestionHotspotRegion): boolean {
  return (
    hasWholeCoordinate(region.x) &&
    hasWholeCoordinate(region.y) &&
    hasWholeCoordinate(region.width) &&
    hasWholeCoordinate(region.height) &&
    region.width > 0 &&
    region.height > 0 &&
    region.x + region.width <= NORMALIZED_MAXIMUM &&
    region.y + region.height <= NORMALIZED_MAXIMUM
  );
}

/** Edges may meet; only positive-area intersections make region selection ambiguous. */
export function hotspotRegionsOverlap(
  left: FlatQuestionHotspotRegion,
  right: FlatQuestionHotspotRegion,
): boolean {
  return (
    left.x < right.x + right.width &&
    right.x < left.x + left.width &&
    left.y < right.y + right.height &&
    right.y < left.y + left.height
  );
}

function nextRegionId(regions: ReadonlyArray<FlatQuestionHotspotRegion>): string {
  let suffix = 1;
  while (regions.some((region) => region.id === `region_${suffix}`)) suffix += 1;
  return `region_${suffix}`;
}

function updatedRegion(
  response: FlatQuestionHotspotResponse,
  regionId: string,
  replacement: FlatQuestionHotspotRegion,
): FlatHotspotEditResult {
  if (!response.regions.some((region) => region.id === regionId)) {
    return refused(response, "That labeled region no longer exists.");
  }
  if (replacement.label.trim() === "") {
    return refused(response, "Give every hotspot region a learner-facing label.");
  }
  if (!isInsideSurface(replacement)) {
    return refused(
      response,
      "Use whole coordinates inside 0 through 10,000 with positive width and height.",
    );
  }
  if (
    response.regions.some(
      (region) => region.id !== regionId && hotspotRegionsOverlap(region, replacement),
    )
  ) {
    return refused(response, "Move or resize this region so labeled regions do not overlap.");
  }
  const regions = response.regions.map((region) => (region.id === regionId ? replacement : region));
  return changed({ ...response, regions });
}

export function setHotspotDescription(
  response: FlatQuestionHotspotResponse,
  description: string,
): FlatHotspotEditResult {
  if (description.trim() === "") {
    return refused(response, "Describe the image so learners can use the labeled region list.");
  }
  return changed({ ...response, surface: { ...response.surface, description } });
}

export function setHotspotRegionLabel(
  response: FlatQuestionHotspotResponse,
  regionId: string,
  label: string,
): FlatHotspotEditResult {
  const current = response.regions.find((region) => region.id === regionId);
  if (current === undefined) return refused(response, "That labeled region no longer exists.");
  return updatedRegion(response, regionId, { ...current, label });
}

export function setHotspotRegionCoordinate(
  response: FlatQuestionHotspotResponse,
  regionId: string,
  coordinate: "x" | "y" | "width" | "height",
  value: number,
): FlatHotspotEditResult {
  const current = response.regions.find((region) => region.id === regionId);
  if (current === undefined) return refused(response, "That labeled region no longer exists.");
  return updatedRegion(response, regionId, { ...current, [coordinate]: value });
}

export function setHotspotCorrectRegion(
  response: FlatQuestionHotspotResponse,
  regionId: string,
  correct: boolean,
): FlatHotspotEditResult {
  if (!response.regions.some((region) => region.id === regionId)) {
    return refused(response, "Choose a region from the labeled region list.");
  }
  const correctRegions = correct
    ? response.correctRegions.includes(regionId)
      ? response.correctRegions
      : [...response.correctRegions, regionId]
    : response.correctRegions.filter((id) => id !== regionId);
  if (correctRegions.length === 0) {
    return refused(response, "Mark at least one labeled region as correct.");
  }
  return changed({ ...response, correctRegions });
}

export function addHotspotRegion(response: FlatQuestionHotspotResponse): FlatHotspotEditResult {
  if (response.regions.length >= MAXIMUM_REGIONS) {
    return refused(response, "A hotspot question can have at most 100 labeled regions.");
  }
  const id = nextRegionId(response.regions);
  for (let y = 0; y < NORMALIZED_MAXIMUM; y += 1_000) {
    for (let x = 0; x < NORMALIZED_MAXIMUM; x += 1_000) {
      const region: FlatQuestionHotspotRegion = {
        id,
        label: `Region ${response.regions.length + 1}`,
        x,
        y,
        width: 1_000,
        height: 1_000,
      };
      if (!response.regions.some((current) => hotspotRegionsOverlap(current, region))) {
        return changed({ ...response, regions: [...response.regions, region] });
      }
    }
  }
  return refused(
    response,
    "Adjust an existing region before adding another in the available space.",
  );
}

export function removeHotspotRegion(
  response: FlatQuestionHotspotResponse,
  regionId: string,
): FlatHotspotEditResult {
  if (response.regions.length <= 1) {
    return refused(response, "A hotspot question needs at least one labeled region.");
  }
  if (!response.regions.some((region) => region.id === regionId)) {
    return refused(response, "That labeled region no longer exists.");
  }
  const regions = response.regions.filter((region) => region.id !== regionId);
  const correctRegions = response.correctRegions.filter((id) => id !== regionId);
  if (correctRegions.length === 0) {
    return refused(response, "Mark another labeled region correct before removing this one.");
  }
  return changed({ ...response, regions, correctRegions });
}

export function moveHotspotRegion(
  response: FlatQuestionHotspotResponse,
  regionId: string,
  direction: "earlier" | "later",
): FlatHotspotEditResult {
  const index = response.regions.findIndex((region) => region.id === regionId);
  const other = direction === "earlier" ? index - 1 : index + 1;
  if (index < 0 || other < 0 || other >= response.regions.length) {
    return refused(response, "That labeled region cannot move further in this list.");
  }
  const regions = [...response.regions];
  const displaced = regions[other];
  const current = regions[index];
  if (displaced === undefined || current === undefined) {
    return refused(response, "That labeled region cannot move further in this list.");
  }
  regions[index] = displaced;
  regions[other] = current;
  return changed({ ...response, regions });
}
