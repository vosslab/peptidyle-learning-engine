// build.mjs - production build for the Solid browser client (WP-F3).
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
//   4. copy static assets, fingerprinting script and stylesheet URLs so browsers and
//      GitHub Pages cannot serve yesterday's bundle
//
// Run: node pipeline/build.mjs [--skip-wasm] [--asset-base=root|relative]
//   --skip-wasm  reuse an existing dist_wasm/ (or omit the bridge entirely).
//                Useful while iterating on UI only; never used for a release.
//   --asset-base=root      emit root-relative browser assets for the live gateway (default).
//   --asset-base=relative  retain relative assets for a GitHub Pages project site.

import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

import * as esbuild from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

const repoRoot = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  encoding: "utf8",
}).trim();

const skipWasm = process.argv.includes("--skip-wasm");

function browserAssetBase(argumentsList) {
  const values = argumentsList
    .filter((argument) => argument.startsWith("--asset-base="))
    .map((argument) => argument.slice("--asset-base=".length));
  if (values.length > 1) throw new Error("--asset-base may be specified at most once");
  const value = values[0] ?? "root";
  if (value !== "root" && value !== "relative") {
    throw new Error("--asset-base must be root or relative");
  }
  return value;
}

const assetBase = browserAssetBase(process.argv.slice(2));

const localDevelopmentAuth = process.env.PLE_BROWSER_LOCAL_DEVELOPMENT_AUTH ?? "0";
if (!["0", "1"].includes(localDevelopmentAuth)) {
  throw new Error("PLE_BROWSER_LOCAL_DEVELOPMENT_AUTH must be unset, 0, or exactly 1");
}
const browserTestTransport = process.env.PLE_BROWSER_TEST_TRANSPORT ?? "0";
if (!["0", "1"].includes(browserTestTransport)) {
  throw new Error("PLE_BROWSER_TEST_TRANSPORT must be unset, 0, or exactly 1");
}

// Tests may direct an isolated build and machine-readable dependency graph
// elsewhere. Ordinary builds use dist/ and do not emit the diagnostic graph.
const configuredOutputDirectory = process.env.PLE_BROWSER_OUTPUT_DIRECTORY;
const distDir =
  configuredOutputDirectory === undefined
    ? path.join(repoRoot, "dist")
    : path.resolve(repoRoot, configuredOutputDirectory);
const configuredMetafilePath = process.env.PLE_BROWSER_METAFILE_PATH;
const metafilePath =
  configuredMetafilePath === undefined ? undefined : path.resolve(repoRoot, configuredMetafilePath);
if (
  metafilePath !== undefined &&
  path.relative(distDir, metafilePath).split(path.sep).includes("..")
) {
  throw new Error("PLE_BROWSER_METAFILE_PATH must remain inside the browser output directory");
}
const srcDir = path.join(repoRoot, "src");
const wasmWebDir = path.join(repoRoot, "dist_wasm", "web");
const fixtureProjectionPath = path.join(repoRoot, "generated", "fixtures", "published_problem.ts");
const localDevelopmentBoundary = path.join(
  srcDir,
  "auth",
  localDevelopmentAuth === "0"
    ? "local_development_disabled.tsx"
    : browserTestTransport === "1"
      ? "local_development_browser_test.tsx"
      : "local_development.tsx",
);
const browserClientBoundary = path.join(
  srcDir,
  "api",
  browserTestTransport === "1" ? "browser_client_browser_test.ts" : "browser_client.ts",
);

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
 * The browser reference app uses the same logical asset endpoint as production.  Its static
 * preview therefore emits the generated fixture bytes at that route so asset-bearing questions
 * exercise a real image load instead of silently accepting an HTTP 404.
 *
 * The generated projection intentionally stores this map as a TypeScript object literal. Reading
 * it here avoids adding a second hand-maintained asset corpus solely for the static mock preview.
 *
 * @returns {void}
 */
function copyBrowserTestAssets() {
  const projection = fs.readFileSync(fixtureProjectionPath, "utf8");
  const marker = "export const publishedProblemAssetBodies: Readonly<Record<string, string>> = ";
  const start = projection.indexOf(marker);
  if (start < 0) throw new Error("generated fixture asset map is missing");
  const objectStart = start + marker.length;
  const objectEnd = projection.indexOf("\n};", objectStart) + 2;
  if (objectEnd < objectStart) throw new Error("generated fixture asset map is incomplete");
  const expression = projection.slice(objectStart, objectEnd);
  // Generated TypeScript may use either quote style after Prettier, so JSON.parse is insufficient.
  const evaluateFixtureMap = Function(`"use strict"; return (${expression});`);
  const bodies = evaluateFixtureMap();
  if (typeof bodies !== "object" || bodies === null || Array.isArray(bodies)) {
    throw new Error("generated fixture asset map must be an object");
  }
  const assetDir = path.join(distDir, "api", "assets");
  fs.mkdirSync(assetDir, { recursive: true });
  for (const [assetId, body] of Object.entries(bodies)) {
    if (typeof body !== "string") throw new Error(`fixture asset ${assetId} is not text`);
    fs.writeFileSync(path.join(assetDir, assetId), body);
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
 * @param {string} stylesheetHash short content hash of the authored stylesheet
 * @param {string} componentStylesheetHash short content hash of bundled component styles
 * @returns {void}
 */
function copyIndexHtml(bundleHash, stylesheetHash, componentStylesheetHash) {
  const source = fs.readFileSync(path.join(srcDir, "index.html"), "utf8");
  const assetPrefix = assetBase === "root" ? "/" : "";
  const fingerprinted = source
    .replace(/(src=")(\.?\/?main\.js)(")/, `$1${assetPrefix}main.js?v=${bundleHash}$3`)
    .replace(/(href=")(\.?\/?style\.css)(")/, `$1${assetPrefix}style.css?v=${stylesheetHash}$3`)
    .replace(
      /(href=")(\.?\/?main\.css)(")/,
      `$1${assetPrefix}main.css?v=${componentStylesheetHash}$3`,
    );
  fs.writeFileSync(path.join(distDir, "index.html"), fingerprinted);
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
  const buildResult = await esbuild.build({
    entryPoints: [path.join(repoRoot, entry)],
    outfile: path.join(distDir, "main.js"),
    bundle: true,
    format: "esm",
    target: "es2020",
    platform: "browser",
    minify: true,
    sourcemap: true,
    metafile: metafilePath !== undefined,
    logLevel: "info",
    define: {
      __PLE_BROWSER_ASSET_BASE__: JSON.stringify(assetBase),
    },
    plugins: [
      {
        name: "local-development-browser-boundary",
        setup(build) {
          build.onResolve({ filter: /^\.\/auth\/local_development$/ }, () => ({
            path: localDevelopmentBoundary,
          }));
        },
      },
      {
        name: "browser-client-boundary",
        setup(build) {
          build.onResolve({ filter: /^\.\/api\/browser_client$/ }, () => ({
            path: browserClientBoundary,
          }));
        },
      },
      solidPlugin(),
    ],
  });
  if (metafilePath !== undefined && buildResult.metafile !== undefined) {
    fs.writeFileSync(metafilePath, `${JSON.stringify(buildResult.metafile, null, 2)}\n`);
  }

  const bundleBytes = fs.readFileSync(path.join(distDir, "main.js"));
  const bundleHash = crypto.createHash("sha256").update(bundleBytes).digest("hex").slice(0, 8);
  const componentStylesheetBytes = fs.readFileSync(path.join(distDir, "main.css"));
  const componentStylesheetHash = crypto
    .createHash("sha256")
    .update(componentStylesheetBytes)
    .digest("hex")
    .slice(0, 8);
  const stylesheetBytes = fs.readFileSync(path.join(srcDir, "style.css"));
  const stylesheetHash = crypto
    .createHash("sha256")
    .update(stylesheetBytes)
    .digest("hex")
    .slice(0, 8);

  copyIndexHtml(bundleHash, stylesheetHash, componentStylesheetHash);
  fs.copyFileSync(path.join(srcDir, "style.css"), path.join(distDir, "style.css"));
  // GitHub Pages runs Jekyll unless this file exists, and Jekyll drops any
  // directory whose name starts with an underscore.
  fs.writeFileSync(path.join(distDir, ".nojekyll"), "");

  copyWasmBridge();
  if (browserTestTransport === "1") {
    copyBrowserTestAssets();
  }

  for (const required of ["index.html", "main.js", "main.css", "style.css"]) {
    if (!fs.existsSync(path.join(distDir, required))) {
      throw new Error(`build finished but dist/${required} is missing`);
    }
  }

  console.log(`Built ${path.relative(repoRoot, distDir)}/ (bundle ${bundleHash})`);
}

await main();
