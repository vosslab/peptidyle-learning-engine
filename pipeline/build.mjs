// build.mjs - production build for the Solid browser client.
//
// Why the esbuild JS-API instead of the CLI: Solid compiles JSX to real DOM
// operations through a Babel preset, and that preset is delivered as an esbuild
// plugin. The esbuild CLI cannot load plugins, so this repo takes the JS-API
// path that docs/TYPESCRIPT_STYLE.md sanctions for exactly this case.
//
// Measured against this source rather than assumed: the CLI with
// --jsx=automatic fails with three errors of the form
//   No matching export in "node_modules/solid-js/dist/solid.js" for import "jsx"
// so the wrong path fails loudly at build time rather than shipping a broken
// bundle. Worth knowing if you are ever tempted to "simplify" back to the CLI.
//
// Pipeline order, and why:
//   1. build the WASM bridge first, because dist/ is assembled from its output
//   2. type-check, so a broken build fails before it writes anything
//   3. bundle with esbuild + solid plugin
//   4. copy static assets and fingerprint their root-gateway URLs so browsers
//      cannot serve yesterday's bundle
//
// Run: node pipeline/build.mjs [--skip-wasm]
//   --skip-wasm  reuse an existing dist_wasm/ (or omit the bridge entirely).
//                Useful while iterating on UI only; never used for a release.

import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import * as esbuild from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

// This pipeline is part of the shipped build boundary. Its own stable location
// anchors the repository even when invoked from an arbitrary directory or an
// exported source tree with no version-control metadata.
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const skipWasm = process.argv.includes("--skip-wasm");

const distDir = path.join(repoRoot, "dist");
const srcDir = path.join(repoRoot, "src");
const wasmWebDir = path.join(repoRoot, "dist_wasm", "web");
const STATIC_STYLESHEETS = [
  "styles/browser_fonts.css",
  "style.css",
  "styles/product_role.css",
  "styles/accessibility.css",
  "styles/ple_embed.css",
];
const PUBLIC_BROWSER_FILES = ["ple_bridge.js"];
const BROWSER_FONT_STYLESHEET = "styles/browser_fonts.css";
const RIBBON_ICON_SPRITE = "assets/ribbon-icons.svg";
const AVATAR_CATALOG_MANIFEST = "assets/avatar_catalog/manifest.json";
const AVATAR_CATALOG_SOURCE_DIRECTORY = "assets/avatar_catalog";
const AVATAR_CATALOG_DIST_DIRECTORY = "assets/avatar_catalog";
const SAFE_AVATAR_CATALOG_FILE = /^svg\/[a-z][a-z0-9]*(?:-[a-z0-9]+)*\.svg$/u;
const BROWSER_FONT_BUNDLES = [
  {
    family: "Atkinson Hyperlegible Next",
    assetDir: "assets/fonts/atkinson_hyperlegible_next",
    assets: [
      "atkinson_hyperlegible_next_variable.woff2",
      "atkinson_hyperlegible_next_variable_italic.woff2",
      "ofl_1_1.txt",
      "provenance.txt",
    ],
  },
  {
    family: "Atkinson Hyperlegible Mono",
    assetDir: "assets/fonts/atkinson_hyperlegible_mono",
    assets: [
      "atkinson_hyperlegible_mono_variable.woff2",
      "atkinson_hyperlegible_mono_variable_italic.woff2",
      "ofl_1_1.txt",
      "provenance.txt",
    ],
  },
];

//============================================

/**
 * Runs a command from the repo root, streaming its output to this terminal.
 *
 * @param {string} command
 * @param {string[]} args
 * @returns {void}
 */
function run(command, args) {
  execFileSync(command, args, { cwd: repoRoot, stdio: "inherit" });
}

//============================================

/**
 * Resolves the browser entry point.
 *
 * `src/main.tsx` is the default TypeScript/JSX entry module. `src/main.ts` is
 * accepted for a client with no JSX.
 *
 * @returns {string} repo-relative path to the entry module
 */
function resolveEntry() {
  const candidates = ["src/main.tsx", "src/main.ts"];
  for (const candidate of candidates) {
    if (fs.existsSync(path.join(repoRoot, candidate))) {
      return candidate;
    }
  }
  throw new Error("no entry point found (looked for src/main.tsx, src/main.ts)");
}

//============================================

/**
 * Copies the generated WASM bridge into dist/wasm/.
 *
 * A missing bridge is a hard failure rather than a warning: the client shares
 * generation, validation, and timer logic with the server through this module,
 * so a site built without it would silently behave differently from the server.
 *
 * @returns {void}
 */
function copyWasmBridge() {
  if (!fs.existsSync(wasmWebDir)) {
    throw new Error(`WASM bridge missing at ${wasmWebDir}. Build it with ./pipeline/build_wasm.sh`);
  }
  const targetDir = path.join(distDir, "wasm");
  fs.mkdirSync(targetDir, { recursive: true });
  for (const entry of fs.readdirSync(wasmWebDir)) {
    fs.copyFileSync(path.join(wasmWebDir, entry), path.join(targetDir, entry));
  }
}

//============================================

/**
 * Copies index.html into dist/, fingerprinting the script and stylesheet URLs.
 *
 * Cachebusting is not cosmetic here: a stale bundle served from cache is the
 * classic "my change did nothing" bug, and it wastes more time in playtests
 * than it costs to prevent.
 *
 * @param {string} bundleHash short content hash of the built bundle
 * @param {Record<string, string>} stylesheetHashes short content hashes keyed by source path
 * @param {string} componentStylesheetHash short content hash of bundled component styles
 * @returns {void}
 */
function copyIndexHtml(bundleHash, stylesheetHashes, componentStylesheetHash) {
  const source = fs.readFileSync(path.join(srcDir, "index.html"), "utf8");
  let fingerprinted = source
    .replace(/(src=")(\.?\/?main\.js)(")/, `$1/main.js?v=${bundleHash}$3`)
    .replace(/(href=")(\.?\/?main\.css)(")/, `$1/main.css?v=${componentStylesheetHash}$3`);
  for (const stylesheet of STATIC_STYLESHEETS) {
    const escapedPath = stylesheet.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const stylesheetPattern = new RegExp(`(href=")(\\.?\\/?${escapedPath})(")`);
    fingerprinted = fingerprinted.replace(
      stylesheetPattern,
      `$1/${stylesheet}?v=${stylesheetHashes[stylesheet]}$3`,
    );
  }
  fs.writeFileSync(path.join(distDir, "index.html"), fingerprinted);
}

//============================================

/**
 * Copies authored stylesheets into dist/, preserving nested asset paths.
 *
 * @returns {Record<string, string>} short content hashes keyed by source path
 */
function copyStaticStylesheets() {
  const hashes = {};
  for (const stylesheet of STATIC_STYLESHEETS) {
    const sourcePath = path.join(srcDir, stylesheet);
    const targetPath = path.join(distDir, stylesheet);
    const bytes = fs.readFileSync(sourcePath);
    hashes[stylesheet] = crypto.createHash("sha256").update(bytes).digest("hex").slice(0, 8);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    fs.copyFileSync(sourcePath, targetPath);
  }
  return hashes;
}

/** Copies same-origin browser helpers loaded directly by backend-owned documents. */
function copyPublicBrowserFiles() {
  for (const source of PUBLIC_BROWSER_FILES) {
    fs.copyFileSync(path.join(srcDir, "public", source), path.join(distDir, source));
  }
}

//============================================

/** Checks the committed Ribbon icon subset before it becomes a browser asset. */
function checkRibbonIconSprite() {
  run("node", ["--import", "tsx", "devel/build_ribbon_icon_sprite.mjs", "--check"]);
}

//============================================

/** Copies the checked same-origin Ribbon icon sprite into the production asset root. */
function copyRibbonIconSprite() {
  const sourcePath = path.join(srcDir, "ribbon", RIBBON_ICON_SPRITE);
  const targetPath = path.join(distDir, RIBBON_ICON_SPRITE);
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.copyFileSync(sourcePath, targetPath);
}

//============================================

/**
 * Copies the locally bundled browser typeface and its distribution record.
 *
 * The font is a product accessibility choice, so silently omitting it would
 * make the browser fall back to an unreviewed system typeface. Keeping its
 * license and source record beside the delivered files makes the asset
 * reproducible without a runtime request to a third-party font host.
 *
 * @returns {void}
 */
function copyBrowserFontAssets() {
  for (const bundle of BROWSER_FONT_BUNDLES) {
    const sourceDir = path.join(srcDir, bundle.assetDir);
    const targetDir = path.join(distDir, bundle.assetDir);
    for (const asset of bundle.assets) {
      const sourcePath = path.join(sourceDir, asset);
      if (!fs.existsSync(sourcePath)) {
        throw new Error(
          `browser font asset missing at ${sourcePath}; restore the locally bundled ${bundle.family} distribution`,
        );
      }
      fs.mkdirSync(targetDir, { recursive: true });
      fs.copyFileSync(sourcePath, path.join(targetDir, asset));
    }
  }
}

//============================================

/** Reads only the manifest-declared, static provided-avatar SVG paths. */
function avatarCatalogFiles() {
  const manifestPath = path.join(repoRoot, AVATAR_CATALOG_MANIFEST);
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  if (!Array.isArray(manifest.avatars) || manifest.avatars.length === 0) {
    throw new Error("provided-avatar manifest must declare at least one avatar");
  }
  const files = manifest.avatars.map((avatar) => avatar.file);
  if (
    files.some((file) => typeof file !== "string" || !SAFE_AVATAR_CATALOG_FILE.test(file)) ||
    new Set(files).size !== files.length
  ) {
    throw new Error("provided-avatar manifest must declare unique safe SVG file paths");
  }
  return files;
}

//============================================

/** Copies each closed-catalog SVG to the same absolute path emitted by its registry. */
function copyAvatarCatalogAssets() {
  for (const file of avatarCatalogFiles()) {
    const sourcePath = path.join(repoRoot, AVATAR_CATALOG_SOURCE_DIRECTORY, file);
    const targetPath = path.join(distDir, AVATAR_CATALOG_DIST_DIRECTORY, file);
    if (!fs.existsSync(sourcePath)) {
      throw new Error(`provided-avatar source asset is missing at ${sourcePath}`);
    }
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    fs.copyFileSync(sourcePath, targetPath);
  }
}

//============================================

/** Fails if a generated avatar URL has no exact production asset. */
function checkAvatarCatalogDelivery() {
  for (const file of avatarCatalogFiles()) {
    const deliveredPath = path.join(distDir, AVATAR_CATALOG_DIST_DIRECTORY, file);
    if (!fs.existsSync(deliveredPath)) {
      throw new Error(`build finished but provided-avatar asset ${deliveredPath} is missing`);
    }
  }
}

//============================================

/**
 * Verifies that the browser stylesheet names only the copied local font files.
 *
 * @returns {void}
 */
function checkBrowserFontDelivery() {
  const stylesheetPath = path.join(distDir, BROWSER_FONT_STYLESHEET);
  if (!fs.existsSync(stylesheetPath)) {
    throw new Error(`build finished but dist/${BROWSER_FONT_STYLESHEET} is missing`);
  }
  const stylesheet = fs.readFileSync(stylesheetPath, "utf8");
  const fontFaceBlocks = stylesheet.match(/@font-face\s*\{[^}]*\}/g) ?? [];
  if (fontFaceBlocks.some((block) => /https?:\/\//i.test(block))) {
    throw new Error("browser font stylesheet must not refer to remote font assets");
  }
  for (const bundle of BROWSER_FONT_BUNDLES) {
    for (const [style, asset] of [
      ["normal", bundle.assets[0]],
      ["italic", bundle.assets[1]],
    ]) {
      const fontFace = fontFaceBlocks.find(
        (block) =>
          block.includes(`font-family: "${bundle.family}"`) &&
          new RegExp(`font-style\\s*:\\s*${style}\\s*;`).test(block),
      );
      if (!fontFace) {
        throw new Error(
          `browser font stylesheet must define a local ${bundle.family} ${style} @font-face rule`,
        );
      }
      if (!fontFace.includes(`/${bundle.assetDir}/${asset}`)) {
        throw new Error(
          `browser ${bundle.family} ${style} @font-face rule does not refer to local font asset ${asset}`,
        );
      }
      if (!fs.existsSync(path.join(distDir, bundle.assetDir, asset))) {
        throw new Error(`build finished but dist/${bundle.assetDir}/${asset} is missing`);
      }
    }
  }
}

//============================================

/**
 * Builds the site into dist/.
 *
 * @returns {Promise<void>}
 */
async function main() {
  const entry = resolveEntry();

  if (skipWasm) {
    console.warn("WARNING: --skip-wasm, reusing any existing dist_wasm/");
  } else {
    console.log("==> wasm bridge");
    run("./pipeline/build_wasm.sh", []);
  }

  console.log("==> typecheck");
  run("npx", ["tsc", "--noEmit", "-p", "tsconfig.json"]);

  console.log("==> Ribbon icon sprite");
  checkRibbonIconSprite();

  fs.rmSync(distDir, { recursive: true, force: true });
  fs.mkdirSync(distDir, { recursive: true });

  console.log("==> bundle");
  await esbuild.build({
    entryPoints: [path.join(repoRoot, entry)],
    outfile: path.join(distDir, "main.js"),
    bundle: true,
    format: "esm",
    target: "es2020",
    platform: "browser",
    minify: true,
    sourcemap: true,
    logLevel: "info",
    plugins: [solidPlugin()],
  });
  const bundleBytes = fs.readFileSync(path.join(distDir, "main.js"));
  const bundleHash = crypto.createHash("sha256").update(bundleBytes).digest("hex").slice(0, 8);
  const componentStylesheetBytes = fs.readFileSync(path.join(distDir, "main.css"));
  const componentStylesheetHash = crypto
    .createHash("sha256")
    .update(componentStylesheetBytes)
    .digest("hex")
    .slice(0, 8);
  const stylesheetHashes = copyStaticStylesheets();
  copyIndexHtml(bundleHash, stylesheetHashes, componentStylesheetHash);
  copyPublicBrowserFiles();
  copyRibbonIconSprite();
  copyBrowserFontAssets();
  copyAvatarCatalogAssets();
  checkBrowserFontDelivery();
  checkAvatarCatalogDelivery();

  copyWasmBridge();

  for (const required of [
    "index.html",
    "main.js",
    "main.css",
    ...STATIC_STYLESHEETS,
    ...PUBLIC_BROWSER_FILES,
    RIBBON_ICON_SPRITE,
    ...avatarCatalogFiles().map((file) => path.join(AVATAR_CATALOG_DIST_DIRECTORY, file)),
    ...BROWSER_FONT_BUNDLES.flatMap((bundle) =>
      bundle.assets.map((asset) => path.join(bundle.assetDir, asset)),
    ),
  ]) {
    if (!fs.existsSync(path.join(distDir, required))) {
      throw new Error(`build finished but dist/${required} is missing`);
    }
  }

  console.log(`Built ${path.relative(repoRoot, distDir)}/ (bundle ${bundleHash})`);
}

await main();
