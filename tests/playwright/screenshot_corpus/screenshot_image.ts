// screenshot_image.ts - text-optimized lossy WebP capture and artifact dimensions.

import { execFile } from "node:child_process";

import type { Page } from "playwright";

/** Pipe lossless capture pixels into the lossy text encoder; publish only WebP. */
export async function captureScreenshotWebp(page: Page): Promise<Buffer> {
  const png = await page.screenshot({ type: "png", animations: "disabled", caret: "hide" });
  return await new Promise<Buffer>((resolve, reject) => {
    // ASVS 1.2.5: fixed executable and structured arguments; no shell interpolation.
    const encoder = execFile(
      "cwebp",
      ["-quiet", "-preset", "text", "-q", "90", "-m", "6", "-o", "-", "--", "-"],
      { encoding: "buffer", maxBuffer: 16 * 1024 * 1024 },
      (error, stdout): void => {
        if (error !== null) {
          reject(new Error(`WebP screenshot encoding failed: ${error.message}`));
          return;
        }
        resolve(stdout);
      },
    );
    if (encoder.stdin === null) {
      encoder.kill();
      reject(new Error("WebP encoder has no input stream"));
      return;
    }
    encoder.stdin.on("error", reject);
    encoder.stdin.end(png);
  });
}

/** Read the dimension header used by lossy, lossless, and extended WebP files. */
export function webpDimensions(bytes: Uint8Array): {
  readonly width: number;
  readonly height: number;
} {
  const buffer = Buffer.from(bytes);
  if (
    buffer.length < 20 ||
    buffer.toString("ascii", 0, 4) !== "RIFF" ||
    buffer.toString("ascii", 8, 12) !== "WEBP" ||
    buffer.readUInt32LE(4) + 8 !== buffer.length
  ) {
    throw new Error("artifact is not a complete WebP container");
  }
  const kind = buffer.toString("ascii", 12, 16);
  const chunkSize = buffer.readUInt32LE(16);
  // ASVS 2.2.1: validate the binary header bounds before reading dimensions.
  if (chunkSize > buffer.length - 20) throw new Error("WebP header is truncated");
  let width: number;
  let height: number;
  if (kind === "VP8X" && chunkSize >= 10) {
    width = buffer.readUIntLE(24, 3) + 1;
    height = buffer.readUIntLE(27, 3) + 1;
  } else if (kind === "VP8 " && chunkSize >= 10 && buffer.readUIntLE(23, 3) === 0x2a019d) {
    width = buffer.readUInt16LE(26) & 0x3fff;
    height = buffer.readUInt16LE(28) & 0x3fff;
  } else if (kind === "VP8L" && chunkSize >= 5 && buffer[20] === 0x2f) {
    const dimensions = buffer.readUInt32LE(21);
    width = (dimensions & 0x3fff) + 1;
    height = ((dimensions >>> 14) & 0x3fff) + 1;
  } else {
    throw new Error("WebP has no supported dimension header");
  }
  if (width === 0 || height === 0) throw new Error("WebP dimensions must be nonzero");
  return { width, height };
}
