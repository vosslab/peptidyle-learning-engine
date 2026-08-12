// e2e_wasm_determinism.mjs - run wasm-bindgen-test in headless Chromium.

import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

import { chromium } from "playwright";

const repoRoot = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  encoding: "utf8",
}).trim();
const runner = path.join(
  repoRoot,
  "target",
  "tooling",
  "wasm-bindgen-cli",
  "bin",
  "wasm-bindgen-test-runner",
);

if (!fs.existsSync(runner)) {
  console.error(`FAIL: ${runner} is missing.`);
  console.error("Install it with: ./devel/setup_wasm_tests.sh");
  process.exit(1);
}

const runnerVersion = execFileSync(runner, ["--version"], { encoding: "utf8" }).trim();
const wasmBindgenPackage = execFileSync("cargo", ["pkgid", "wasm-bindgen"], {
  cwd: repoRoot,
  encoding: "utf8",
}).trim();
const expectedRunnerVersion = wasmBindgenPackage.match(/@([^@]+)$/u)?.[1];
assert.ok(
  expectedRunnerVersion,
  `could not determine wasm-bindgen version from ${wasmBindgenPackage}`,
);
assert.equal(
  runnerVersion,
  `wasm-bindgen-test-runner ${expectedRunnerVersion}`,
  "wasm-bindgen test runner must match the lockfile-resolved binding crate",
);

const cargo = spawn(
  "cargo",
  [
    "test",
    "--target",
    "wasm32-unknown-unknown",
    "-p",
    "wasm_bridge",
    "--test",
    "test_determinism_wasm",
    "--",
    "--nocapture",
  ],
  {
    cwd: repoRoot,
    detached: true,
    env: {
      ...process.env,
      CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER: runner,
      NO_HEADLESS: "1",
      RUST_BACKTRACE: "1",
      WASM_BINDGEN_TEST_ADDRESS: "127.0.0.1:0",
      WASM_BINDGEN_TEST_ONLY_WEB: "1",
    },
    stdio: ["ignore", "pipe", "pipe"],
  },
);

let serverOutput = "";
let resolveUrl;
let rejectUrl;
const serverUrl = new Promise((resolve, reject) => {
  resolveUrl = resolve;
  rejectUrl = reject;
});

function inspectOutput(chunk) {
  const text = chunk.toString();
  serverOutput += text;
  process.stdout.write(text);
  const match = serverOutput.match(/browsers tests are now available at (http:\/\/[^\s]+)/i);
  if (match?.[1]) {
    resolveUrl(match[1]);
  }
}

cargo.stdout.on("data", inspectOutput);
cargo.stderr.on("data", inspectOutput);
cargo.on("error", (error) => rejectUrl(error));
cargo.on("exit", (code, signal) => {
  rejectUrl(
    new Error(
      `cargo exited before browser startup (code ${String(code)}, signal ${String(signal)})`,
    ),
  );
});

const startupTimeout = setTimeout(() => {
  rejectUrl(new Error("timed out waiting for the wasm-bindgen browser test server"));
}, 60_000);

let browser;
try {
  const url = await serverUrl;
  clearTimeout(startupTimeout);
  browser = await chromium.launch();
  const page = await browser.newPage();
  await page.goto(url);
  await page.waitForFunction(
    () => document.querySelector("#output")?.textContent?.includes("test result:") ?? false,
    undefined,
    { timeout: 60_000 },
  );
  const output = await page.locator("#output").textContent();
  assert.ok(output, "wasm-bindgen-test page returned no output");
  process.stdout.write(`\n${output}`);
  assert.match(output, /test result: ok\./, "wasm-bindgen browser parity test failed");
  assert.match(
    output,
    /committed_seed_vectors_match_browser_generation/u,
    "browser ran the committed deterministic generator vectors",
  );
  assert.match(
    output,
    /flat_v2_public_response_corpus_matches_browser_wasm/u,
    "browser ran the shared answer-free flat-v2 response corpus",
  );
  console.log("PASS: deterministic vectors and flat-v2 response format match in headless Chromium");
} finally {
  clearTimeout(startupTimeout);
  if (browser) {
    await browser.close();
  }
  if (cargo.pid) {
    try {
      process.kill(-cargo.pid, "SIGTERM");
    } catch (error) {
      if (error.code !== "ESRCH") {
        console.error("WARN: could not stop the wasm-bindgen test process group", error);
      }
    }
  }
}
