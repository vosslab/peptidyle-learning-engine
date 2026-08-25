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
const STATIC_STYLESHEETS = ["style.css", "styles/accessibility.css"];

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
 * `src/main.tsx` is canonical. `src/main.ts` is accepted for a client with no
 * JSX.
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

  copyWasmBridge();

  for (const required of ["index.html", "main.js", "main.css", ...STATIC_STYLESHEETS]) {
    if (!fs.existsSync(path.join(distDir, required))) {
      throw new Error(`build finished but dist/${required} is missing`);
    }
  }

  console.log(`Built ${path.relative(repoRoot, distDir)}/ (bundle ${bundleHash})`);
}

await main();
