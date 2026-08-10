import {
  publishedProblemAssetBodies,
  publishedProblemFixture,
} from "../../../../generated/fixtures/published_problem";

import { handlesResource, jsonResponse, methodNotAllowed, pathSegments } from "./shared";

export function canHandleAsset(request: Request): boolean {
  return handlesResource(request, ["assets"]);
}

export function respondAsset(request: Request): Response {
  if (request.method !== "GET") {
    return methodNotAllowed(request);
  }
  const assetId = pathSegments(request)[2];
  const asset = publishedProblemFixture.assets.find((candidate) => candidate.id === assetId);
  const body = assetId === undefined ? undefined : publishedProblemAssetBodies[assetId];
  if (asset === undefined || body === undefined) {
    return jsonResponse({ error: `Unknown fixture asset ${assetId ?? ""}` }, 404);
  }
  return new Response(body, {
    status: 200,
    headers: {
      "content-type": asset.mediaType,
      etag: `"${asset.sha256}"`,
    },
  });
}
