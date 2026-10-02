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

import {
  browserFontPathFromUrl,
  browserFontUrlsFromStylesheet,
} from "../src/browser_font_assets.mjs";

// This pipeline is part of the shipped build boundary. Its own stable location
// anchors the repository even when invoked from an arbitrary directory or an
// exported source tree with no version-control metadata.
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const skipWasm = process.argv.includes("--skip-wasm");

const publishedDistDir = path.join(repoRoot, "dist");
let distDir = publishedDistDir;
const srcDir = path.join(repoRoot, "src");
const wasmWebDir = path.join(repoRoot, "dist_wasm", "web");
const STANDALONE_STYLESHEETS = ["styles/ple_embed.css"];
const PUBLIC_BROWSER_FILES = ["ple_bridge.js"];
const RIBBON_ICON_SPRITE = "assets/ribbon-icons.svg";
const AVATAR_CATALOG_MANIFEST = "assets/avatar_catalog/manifest.json";
const AVATAR_CATALOG_SOURCE_DIRECTORY = "assets/avatar_catalog";
const AVATAR_CATALOG_DIST_DIRECTORY = "assets/avatar_catalog";
const SAFE_AVATAR_CATALOG_FILE = /^svg\/[a-z][a-z0-9]*(?:-[a-z0-9]+)*\.svg$/u;
const BROWSER_FONT_DISTRIBUTION_FILES = ["ofl_1_1.txt", "provenance.txt"];

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

/** Resolves the generated web bridge files or reports the supported repair. */
function wasmBridgeFiles() {
  if (!fs.existsSync(wasmWebDir)) {
    throw new Error(`WASM bridge missing at ${wasmWebDir}. Build it with ./pipeline/build_wasm.sh`);
  }
  return fs.readdirSync(wasmWebDir);
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
 * Copies the generated WASM bridge into its content-addressed dist/wasm/ directory.
 *
 * A missing bridge is a hard failure rather than a warning: the client shares
 * generation, validation, and timer logic with the server through this module,
 * so a site built without it would silently behave differently from the server.
 *
 * @returns {void}
 */
function copyWasmBridge(wasmAssetVersion) {
  const targetDir = path.join(distDir, "wasm", wasmAssetVersion);
  fs.mkdirSync(targetDir, { recursive: true });
  for (const entry of wasmBridgeFiles()) {
    fs.copyFileSync(path.join(wasmWebDir, entry), path.join(targetDir, entry));
  }
}

//============================================

/**
 * Copies index.html into dist/, fingerprinting the production script and bundled stylesheet URLs.
 *
 * Cachebusting is not cosmetic here: a stale bundle served from cache is the
 * classic "my change did nothing" bug, and it wastes more time in playtests
 * than it costs to prevent.
 *
 * @param {string} bundleFile content-addressed browser bundle filename
 * @param {string} stylesheetFile content-addressed stylesheet filename
 * @returns {void}
 */
function copyIndexHtml(bundleFile, stylesheetFile) {
  const source = fs.readFileSync(path.join(srcDir, "index.html"), "utf8");
  const fingerprinted = source
    .replace(/(src=")(\.?\/?main\.js)(")/, `$1/${bundleFile}$3`)
    .replace(/(href=")(\.?\/?main\.css)(")/, `$1/${stylesheetFile}$3`);
  fs.writeFileSync(path.join(distDir, "index.html"), fingerprinted);
}

//============================================

/**
 * Copies standalone authored stylesheets used outside the SPA browser entry.
 */
function copyStandaloneStylesheets() {
  for (const stylesheet of STANDALONE_STYLESHEETS) {
    const sourcePath = path.join(srcDir, stylesheet);
    const targetPath = path.join(distDir, stylesheet);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    fs.copyFileSync(sourcePath, targetPath);
  }
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
function copyBrowserFontAssets(fontUrls) {
  for (const fontPath of browserFontDistributionPaths(fontUrls)) {
    const sourcePath = path.join(srcDir, "assets", "fonts", fontPath);
    if (!fs.existsSync(sourcePath)) {
      throw new Error(
        `browser font asset missing at ${sourcePath}; restore its local distribution`,
      );
    }
    const targetPath = path.join(distDir, "assets", "fonts", fontPath);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    fs.copyFileSync(sourcePath, targetPath);
  }
}

/** Returns CSS-declared fonts plus the required records beside each font. */
function browserFontDistributionPaths(fontUrls) {
  const fontPaths = fontUrls.map(browserFontPathFromUrl);
  return [
    ...new Set([
      ...fontPaths,
      ...fontPaths.flatMap((fontPath) => {
        const fontDirectory = path.dirname(fontPath);
        return BROWSER_FONT_DISTRIBUTION_FILES.map((file) => path.join(fontDirectory, file));
      }),
    ]),
  ];
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
 * Verifies that the production browser stylesheet names only the copied local font files.
 *
 * @returns {void}
 */
function checkBrowserFontDelivery(fontUrls) {
  for (const fontUrl of fontUrls) {
    const deliveredPath = path.join(distDir, fontUrl);
    if (!fs.existsSync(deliveredPath)) {
      throw new Error(`production font URL is not delivered: ${fontUrl}`);
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

  const stagingDir = fs.mkdtempSync(path.join(repoRoot, ".dist-staging-"));
  distDir = stagingDir;
  try {
    fs.mkdirSync(distDir, { recursive: true });

    const wasmFiles = wasmBridgeFiles().sort();
    const wasmHasher = crypto.createHash("sha256");
    for (const file of wasmFiles) {
      wasmHasher.update(file).update(fs.readFileSync(path.join(wasmWebDir, file)));
    }
    const wasmAssetVersion = wasmHasher.digest("hex").slice(0, 12);

    console.log("==> bundle");
    await esbuild.build({
      entryPoints: [path.join(repoRoot, entry)],
      outfile: path.join(distDir, "main.js"),
      bundle: true,
      external: ["/assets/fonts/*"],
      format: "esm",
      target: "es2020",
      platform: "browser",
      minify: true,
      sourcemap: true,
      logLevel: "info",
      plugins: [solidPlugin()],
      define: { __PLE_WASM_ASSET_VERSION__: JSON.stringify(wasmAssetVersion) },
    });
    const bundlePath = path.join(distDir, "main.js");
    const bundleMapPath = `${bundlePath}.map`;
    const bundleMapName = `main.${crypto
      .createHash("sha256")
      .update(fs.readFileSync(bundlePath))
      .digest("hex")
      .slice(0, 8)}.js.map`;
    const bundleBytes = fs
      .readFileSync(bundlePath, "utf8")
      .replace("sourceMappingURL=main.js.map", `sourceMappingURL=${bundleMapName}`);
    const bundleHash = crypto.createHash("sha256").update(bundleBytes).digest("hex").slice(0, 8);
    const stylesheetPath = path.join(distDir, "main.css");
    const stylesheetMapPath = `${stylesheetPath}.map`;
    const stylesheetMapName = `main.${crypto
      .createHash("sha256")
      .update(fs.readFileSync(stylesheetPath))
      .digest("hex")
      .slice(0, 8)}.css.map`;
    const stylesheetBytes = fs
      .readFileSync(stylesheetPath, "utf8")
      .replace("sourceMappingURL=main.css.map", `sourceMappingURL=${stylesheetMapName}`);
    const componentStylesheetHash = crypto
      .createHash("sha256")
      .update(stylesheetBytes)
      .digest("hex")
      .slice(0, 8);
    const bundleFile = `main.${bundleHash}.js`;
    const stylesheetFile = `main.${componentStylesheetHash}.css`;
    fs.writeFileSync(bundlePath, bundleBytes);
    fs.writeFileSync(stylesheetPath, stylesheetBytes);
    for (const [mapPath, mapName, outputFile] of [
      [bundleMapPath, bundleMapName, bundleFile],
      [stylesheetMapPath, stylesheetMapName, stylesheetFile],
    ]) {
      const sourceMap = JSON.parse(fs.readFileSync(mapPath, "utf8"));
      sourceMap.file = outputFile;
      fs.writeFileSync(path.join(distDir, mapName), `${JSON.stringify(sourceMap)}\n`);
      fs.rmSync(mapPath);
    }
    fs.renameSync(bundlePath, path.join(distDir, bundleFile));
    fs.renameSync(stylesheetPath, path.join(distDir, stylesheetFile));
    copyStandaloneStylesheets();
    const fontUrls = browserFontUrlsFromStylesheet(
      [stylesheetFile, ...STANDALONE_STYLESHEETS]
        .map((stylesheet) => fs.readFileSync(path.join(distDir, stylesheet), "utf8"))
        .join("\n"),
    );
    copyIndexHtml(bundleFile, stylesheetFile);
    copyPublicBrowserFiles();
    copyRibbonIconSprite();
    copyBrowserFontAssets(fontUrls);
    copyAvatarCatalogAssets();
    checkBrowserFontDelivery(fontUrls);
    checkAvatarCatalogDelivery();

    copyWasmBridge(wasmAssetVersion);

    for (const required of [
      "index.html",
      bundleFile,
      stylesheetFile,
      ...STANDALONE_STYLESHEETS,
      ...PUBLIC_BROWSER_FILES,
      RIBBON_ICON_SPRITE,
      ...avatarCatalogFiles().map((file) => path.join(AVATAR_CATALOG_DIST_DIRECTORY, file)),
      ...browserFontDistributionPaths(fontUrls).map((fontPath) =>
        path.join("assets", "fonts", fontPath),
      ),
    ]) {
      if (!fs.existsSync(path.join(distDir, required))) {
        throw new Error(`build finished but dist/${required} is missing`);
      }
    }

    // All fallible compilation and copy preparation is complete. Publish each
    // prepared asset atomically inside the existing mounted directory; retain
    // prior fingerprinted bundle/WASM files for clients holding older HTML.
    fs.mkdirSync(publishedDistDir, { recursive: true });
    const publishFiles = (sourceDir, relativeDir = "") => {
      for (const entry of fs.readdirSync(path.join(sourceDir, relativeDir), {
        withFileTypes: true,
      })) {
        const relativePath = path.join(relativeDir, entry.name);
        if (relativePath === "index.html") continue;
        if (entry.isDirectory()) {
          publishFiles(sourceDir, relativePath);
        } else {
          const targetPath = path.join(publishedDistDir, relativePath);
          fs.mkdirSync(path.dirname(targetPath), { recursive: true });
          fs.renameSync(path.join(sourceDir, relativePath), targetPath);
        }
      }
    };
    publishFiles(stagingDir);
    const stagedIndex = path.join(stagingDir, "index.html");
    const publishedIndex = path.join(publishedDistDir, "index.html");
    fs.renameSync(stagedIndex, publishedIndex);

    console.log(`Built ${path.relative(repoRoot, publishedDistDir)}/ (bundle ${bundleHash})`);
  } finally {
    fs.rmSync(stagingDir, { recursive: true, force: true });
    distDir = publishedDistDir;
  }
}

await main();
