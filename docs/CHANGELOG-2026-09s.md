## 2026-09-28

### Screenshot WebP publication

- Converted all 246 canonical screenshots from PNG to WebP at quality 95. That initial
  conversion reduced corpus size from 22,993,578 to 14,997,350 bytes (34.8% smaller).
- Future capture now pipes Playwright's lossless PNG pixels in memory to
  `cwebp -preset text -q 90 -m 6`, publishing only lossy WebP. The text preset and maximum
  encoding method replace the browser's limited native encoder controls. The driver checks
  that `cwebp` is on PATH before starting the stack; capture requires no temporary PNG file.
  Existing published images retain their prior encoding until the next corpus capture.
- Updated capture paths, WebP dimension validation, receipts, generated galleries, linked
  documentation, and README images. Repaired four obsolete README screenshot filenames.
- Validation: the final PNG-to-WebP text-preset helper passed real Chromium capture at all four
  canonical viewports, including capture-style cleanup. All 246 published images decoded
  independently with Pillow at their declared dimensions; receipt, manifest, atlas, and gallery
  verification passed. All 18 focused screenshot tests, TypeScript compilation, changed-file
  ESLint, and whitespace checks passed. A Forest Dark visual spot-check retained readable text.
  The broader fast gate stopped on existing Rust formatting differences in untouched server
  files. This work did not rerun the full Live Demo corpus or change theme text colors.
