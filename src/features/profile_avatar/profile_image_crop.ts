// profile_image_crop.ts - local square composition, shared by preview and upload.

import type { ProfileImageCropInput } from "../../api/profile_avatar";
import { decodeProfileImageCropInput } from "../../api/decoders/profile_avatar";

const MIN_SIDE_PIXELS = 128;
const MAX_DECODED_PIXELS = 20_000_000;
const MAX_SOURCE_BYTES = 8 * 1024 * 1024;
export const PROFILE_IMAGE_SIDE_PIXELS = 256;

export interface ProfileImageCrop {
  readonly x: number;
  readonly y: number;
  readonly side: number;
}

/** Computes a square source rectangle; positions are percentages of available travel. */
export function profileImageCrop(input: ProfileImageCropInput): ProfileImageCrop {
  const {
    sourceWidth: width,
    sourceHeight: height,
    horizontal,
    vertical,
    zoomPercent,
  } = decodeProfileImageCropInput(input);
  // Integer pixel edges and floor rounding match the trusted server crop exactly.
  const side = Math.floor((Math.min(width, height) * 100) / zoomPercent);
  return {
    x: Math.floor(((width - side) * horizontal) / 100),
    y: Math.floor(((height - side) * vertical) / 100),
    side,
  };
}

function validateDimensions(width: number, height: number): void {
  if (
    !Number.isInteger(width) ||
    !Number.isInteger(height) ||
    width < MIN_SIDE_PIXELS ||
    height < MIN_SIDE_PIXELS
  ) {
    throw new Error("Choose an image at least 128 pixels wide and tall.");
  }
  if (width * height > MAX_DECODED_PIXELS) {
    throw new Error("Choose an image with at most 20 million pixels.");
  }
}

/** The caller owns and closes the returned, orientation-corrected bitmap. */
export async function decodeProfileImage(file: File): Promise<ImageBitmap> {
  // ASVS 5.2.1: cap local input bytes; the server independently validates uploaded bytes.
  if (file.size === 0 || file.size > MAX_SOURCE_BYTES) {
    throw new Error("Choose an image no larger than 8 MiB.");
  }
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file, { imageOrientation: "from-image" });
  } catch {
    throw new Error("Choose a complete, readable image file.");
  }
  try {
    validateDimensions(bitmap.width, bitmap.height);
    return bitmap;
  } catch (error) {
    bitmap.close();
    throw error;
  }
}

/** Previews the source rectangle the server will crop after validating the original. */
export function drawProfileImageCrop(
  canvas: HTMLCanvasElement,
  bitmap: ImageBitmap,
  crop: ProfileImageCrop,
): void {
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("Your browser could not prepare the image preview.");
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.drawImage(
    bitmap,
    crop.x,
    crop.y,
    crop.side,
    crop.side,
    0,
    0,
    canvas.width,
    canvas.height,
  );
}
