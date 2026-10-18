import { createServer } from "node:http";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";
import { chromium } from "playwright";
import {
  parseOrigin,
  parsePositiveInteger,
  RequestError,
  safeRendererAssetPath,
  validatePayload,
} from "./contracts.js";
import { createCaptureLifecycle } from "./capture_lifecycle.js";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const documentOrigin = parseOrigin(process.env.PLE_BROWSER_ORIGIN);
const port = parsePositiveInteger(process.env.PORT ?? "3001", 3001, 65535);
const maxRequestBytes = 1_100_000;
const maxHtmlBytes = 1_048_576;
const maxCspBytes = 8_192;
const captureDeadlineMs = 20_000;
const maxImageBytes = 2_000_000;
const cwebpTimeoutMs = 5_000;
const documentPath = "/__ple_thumbnail_capture__";
const proxyOrigin = "http://api:3000";
const viewport = { width: 960, height: 900 };

if (!documentOrigin) {
  throw new Error("PLE_BROWSER_ORIGIN must be a credential-free HTTP(S) origin");
}

async function readPackagedAsset(assetPath, sourcePath) {
  try {
    return await readFile(resolve(packageRoot, "assets", assetPath));
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
    return readFile(resolve(packageRoot, "../src", sourcePath));
  }
}

const embedCss = await readPackagedAsset("styles/ple_embed.css", "styles/ple_embed.css");
const bridgeJs = await readPackagedAsset("public/ple_bridge.js", "public/ple_bridge.js");
const fontFiles = new Map(
  await Promise.all(
    [
      [
        "/assets/fonts/atkinson_hyperlegible_next/atkinson_hyperlegible_next_variable.woff2",
        "fonts/atkinson_hyperlegible_next/atkinson_hyperlegible_next_variable.woff2",
      ],
      [
        "/assets/fonts/atkinson_hyperlegible_next/atkinson_hyperlegible_next_variable_italic.woff2",
        "fonts/atkinson_hyperlegible_next/atkinson_hyperlegible_next_variable_italic.woff2",
      ],
      [
        "/assets/fonts/atkinson_hyperlegible_mono/atkinson_hyperlegible_mono_variable.woff2",
        "fonts/atkinson_hyperlegible_mono/atkinson_hyperlegible_mono_variable.woff2",
      ],
      [
        "/assets/fonts/atkinson_hyperlegible_mono/atkinson_hyperlegible_mono_variable_italic.woff2",
        "fonts/atkinson_hyperlegible_mono/atkinson_hyperlegible_mono_variable_italic.woff2",
      ],
    ].map(async ([urlPath, filename]) => [
      urlPath,
      await readPackagedAsset(filename, `assets/${filename}`),
    ]),
  ),
);

let browser;
let browserStarting;
let browserRetirement;
let captureActive = false;

async function getBrowser() {
  if (browserRetirement) await browserRetirement;
  if (browser?.isConnected()) return browser;
  if (!browserStarting) {
    browserStarting = chromium
      .launch({ headless: true, timeout: 8_000 })
      .then((started) => {
        browser = started;
        started.on("disconnected", () => {
          if (browser === started) browser = undefined;
        });
        return started;
      })
      .finally(() => {
        browserStarting = undefined;
      });
  }
  return browserStarting;
}

async function retireBrowser(instance) {
  if (browser === instance) browser = undefined;
  const retirement = Promise.resolve().then(() => instance.close());
  browserRetirement = retirement;
  try {
    await retirement;
  } finally {
    if (browserRetirement === retirement) browserRetirement = undefined;
  }
}

async function closeContextInstance(browserInstance, context) {
  const contextClose = Promise.resolve().then(() => context.close());
  contextClose.catch(() => {});
  let timer;
  try {
    await Promise.race([
      contextClose,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error("context_close_timeout")), 500);
      }),
    ]);
  } catch {
    await retireBrowser(browserInstance);
  } finally {
    clearTimeout(timer);
  }
}

function sendJson(response, status, code, extraHeaders = {}) {
  if (response.destroyed) return;
  const body = Buffer.from(JSON.stringify({ error: code }));
  response.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": body.length,
    "cache-control": "no-store",
    "x-content-type-options": "nosniff",
    ...extraHeaders,
  });
  response.end(body);
}

async function readRequestBody(request) {
  const chunks = [];
  let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > maxRequestBytes) throw new RequestError(413, "request_too_large");
    chunks.push(chunk);
  }
  return Buffer.concat(chunks, size);
}

function fulfillAsset(route, path) {
  if (path === "/styles/ple_embed.css") {
    return route.fulfill({ status: 200, contentType: "text/css; charset=utf-8", body: embedCss });
  }
  if (path === "/ple_bridge.js") {
    return route.fulfill({
      status: 200,
      contentType: "text/javascript; charset=utf-8",
      body: bridgeJs,
    });
  }
  const fontBytes = fontFiles.get(path);
  if (fontBytes) {
    return route.fulfill({
      status: 200,
      contentType: "font/woff2",
      headers: {
        "access-control-allow-origin": "*",
        "cross-origin-resource-policy": "cross-origin",
        "cache-control": "no-store",
        "x-content-type-options": "nosniff",
      },
      body: fontBytes,
    });
  }
  return undefined;
}

async function encodeWebp(png, timeoutMs, signal, onChild) {
  const temporaryDirectory = await mkdtemp(join(tmpdir(), "ple-thumbnail-"));
  const pngPath = join(temporaryDirectory, "capture.png");
  try {
    await writeFile(pngPath, png, { mode: 0o600 });
    return await new Promise((resolvePromise, rejectPromise) => {
      // cwebp requires a seekable input; keep its temporary source private and short-lived.
      const child = spawn("cwebp", ["-quiet", "-q", "80", pngPath, "-o", "-"], {
        shell: false,
        stdio: ["ignore", "pipe", "ignore"],
      });
      const chunks = [];
      let outputSize = 0;
      let settled = false;
      let spawnFailed = false;
      const timer = setTimeout(() => child.kill("SIGKILL"), Math.min(timeoutMs, cwebpTimeoutMs));
      onChild(child);
      const abort = () => child.kill("SIGKILL");
      signal.addEventListener("abort", abort, { once: true });
      if (signal.aborted) abort();
      child.on("error", () => {
        spawnFailed = true;
      });
      child.stdout.on("data", (chunk) => {
        outputSize += chunk.length;
        if (outputSize > maxImageBytes) {
          child.kill("SIGKILL");
          return;
        }
        chunks.push(chunk);
      });
      child.on("close", (code) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        signal.removeEventListener("abort", abort);
        onChild(undefined);
        if (spawnFailed || code !== 0 || outputSize === 0 || outputSize > maxImageBytes) {
          rejectPromise(new Error("encoder_failed"));
        } else {
          resolvePromise(Buffer.concat(chunks, outputSize));
        }
      });
    });
  } finally {
    await rm(temporaryDirectory, { recursive: true, force: true });
  }
}

async function performCapture({ html, contentSecurityPolicy }, signal) {
  const lifecycle = createCaptureLifecycle({ signal, timeoutMs: captureDeadlineMs });
  const deadlineAt = Date.now() + captureDeadlineMs;
  let browserInstance;
  let context;
  let disallowedRequest = false;
  let documentServed = false;
  let activeEncoder;
  let contextClosing;
  const closeContext = () => {
    if (!context) return Promise.resolve();
    contextClosing ??= lifecycle.cleanup(() => closeContextInstance(browserInstance, context));
    return contextClosing;
  };
  const abort = () => {
    activeEncoder?.kill("SIGKILL");
    void closeContext().catch(() => {});
  };
  lifecycle.signal.addEventListener("abort", abort, { once: true });

  try {
    lifecycle.requireActive();
    browserInstance = await lifecycle.phase(getBrowser(), (lateBrowser) =>
      retireBrowser(lateBrowser),
    );
    context = await lifecycle.phase(
      browserInstance.newContext({
        viewport,
        deviceScaleFactor: 1,
        serviceWorkers: "block",
      }),
      (lateContext) => closeContextInstance(browserInstance, lateContext),
    );
    const page = await lifecycle.phase(context.newPage());
    page.setDefaultTimeout(Math.max(1, deadlineAt - Date.now()));
    const remaining = () => Math.max(1, deadlineAt - Date.now());
    // ASVS 3.2.1: attach the supplied CSP to the isolated document response.
    await lifecycle.phase(
      page.route("**/*", async (route) => {
        const requestUrl = new URL(route.request().url());
        if (requestUrl.origin !== documentOrigin || requestUrl.search || requestUrl.hash) {
          disallowedRequest = true;
          await route.abort("blockedbyclient");
          return;
        }
        if (requestUrl.pathname === documentPath && route.request().isNavigationRequest()) {
          documentServed = true;
          await route.fulfill({
            status: 200,
            contentType: "text/html; charset=utf-8",
            headers: { "content-security-policy": contentSecurityPolicy },
            body: html,
          });
          return;
        }
        const packaged = fulfillAsset(route, requestUrl.pathname);
        if (packaged) {
          await packaged;
          return;
        }
        if (safeRendererAssetPath(requestUrl.pathname)) {
          try {
            // ASVS 15.3.2: fetch the fixed proxy target and reject redirects.
            const assetResponse = await route.fetch({
              url: `${proxyOrigin}${requestUrl.pathname}`,
              maxRedirects: 0,
              timeout: remaining(),
            });
            if (assetResponse.status() >= 300 || assetResponse.status() < 200) {
              disallowedRequest = true;
              await route.abort("blockedbyclient");
              return;
            }
            await route.fulfill({ response: assetResponse });
          } catch {
            disallowedRequest = true;
            await route.abort("failed").catch(() => {});
          }
          return;
        }
        disallowedRequest = true;
        await route.abort("blockedbyclient");
      }),
    );

    await lifecycle.phase(
      page.goto(`${documentOrigin}${documentPath}`, { waitUntil: "load", timeout: remaining() }),
    );
    await lifecycle.phase(
      page.evaluate(async (timeoutMs) => {
        const { window, document, requestAnimationFrame } = globalThis;
        const readiness = async () => {
          const mathJax = window.MathJax;
          if (mathJax?.startup?.promise) await mathJax.startup.promise;
          await document.fonts.ready;
          await Promise.all(
            Array.from(document.images, (image) => (image.currentSrc ? image.decode() : undefined)),
          );
          await new Promise((resolveFrame) =>
            requestAnimationFrame(() => requestAnimationFrame(resolveFrame)),
          );
        };
        await Promise.race([
          readiness(),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error("readiness_timeout")), timeoutMs),
          ),
        ]);
      }, remaining()),
    );

    if (!documentServed || disallowedRequest) throw new Error("request_blocked");
    const problem = page.locator("#problem_body");
    const bounds = await lifecycle.phase(problem.boundingBox({ timeout: remaining() }));
    if (
      !bounds ||
      bounds.width <= 0 ||
      bounds.height <= 0 ||
      bounds.width > viewport.width ||
      bounds.height > 100_000
    ) {
      throw new Error("problem_region_missing");
    }
    const side = Math.min(bounds.width, bounds.height);
    const session = await lifecycle.phase(context.newCDPSession(page));
    const captured = await lifecycle.phase(
      session.send("Page.captureScreenshot", {
        format: "png",
        fromSurface: true,
        captureBeyondViewport: true,
        clip: {
          x: bounds.x + (bounds.width - side) / 2,
          y: bounds.y,
          width: side,
          height: side,
          scale: 256 / side,
        },
      }),
    );
    const png = Buffer.from(captured.data, "base64");
    if (disallowedRequest) throw new Error("request_blocked");
    return await lifecycle.phase(
      encodeWebp(png, remaining(), lifecycle.signal, (child) => {
        activeEncoder = child;
      }),
    );
  } finally {
    lifecycle.close();
    lifecycle.signal.removeEventListener("abort", abort);
    await closeContext();
  }
}

const server = createServer(async (request, response) => {
  if (request.method === "GET" && request.url === "/health") {
    if (!browser?.isConnected()) void getBrowser().catch(() => {});
    const healthy = Boolean(browser?.isConnected());
    const body = Buffer.from(JSON.stringify({ status: healthy ? "ready" : "starting" }));
    response.writeHead(healthy ? 200 : 503, {
      "content-type": "application/json; charset=utf-8",
      "content-length": body.length,
      "cache-control": "no-store",
      "x-content-type-options": "nosniff",
    });
    response.end(body);
    return;
  }
  if (request.method !== "POST" || request.url !== "/capture") {
    sendJson(response, 404, "not_found");
    return;
  }
  if (captureActive) {
    sendJson(response, 503, "busy", { "retry-after": "1" });
    request.resume();
    return;
  }
  captureActive = true;
  // ASVS 15.4.1: one in-flight request owns browser/context/encoder resources until cleanup ends.
  const abortController = new AbortController();
  // ASVS 15.4.1: an abandoned caller releases its context and active encoder.
  request.once("aborted", () => abortController.abort());
  response.once("close", () => {
    if (!response.writableEnded) abortController.abort();
  });
  let admitted = false;
  let captureStatus = 422;
  try {
    if (
      request.headers["content-type"]?.split(";")[0].trim().toLowerCase() !== "application/json"
    ) {
      request.resume();
      throw new RequestError(415, "unsupported_media_type");
    }
    const body = await readRequestBody(request);
    let parsed;
    try {
      parsed = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(body));
    } catch {
      throw new RequestError(400, "invalid_request");
    }
    const payload = validatePayload(parsed, { maxHtmlBytes, maxCspBytes });
    admitted = true;
    // ASVS 16.2.5: fixed events contain no source, request, or renderer content.
    console.log(JSON.stringify({ event: "thumbnail_capture_admitted" }));
    const image = await performCapture(payload, abortController.signal);
    // ASVS 3.2.1: return only declared WebP bytes with MIME sniffing disabled.
    response.writeHead(200, {
      "content-type": "image/webp",
      "content-length": image.length,
      "cache-control": "no-store",
      "x-content-type-options": "nosniff",
    });
    response.end(image);
    captureStatus = 200;
  } catch (error) {
    const status = error instanceof RequestError ? error.status : 422;
    captureStatus = status;
    const code = error instanceof RequestError ? error.code : "capture_failed";
    // ASVS 16.5.1: keep browser, renderer, and process details out of client errors and logs.
    sendJson(response, status, code);
  } finally {
    if (admitted) {
      console.log(
        JSON.stringify({
          event: "thumbnail_capture_completed",
          outcome: captureStatus === 200 ? "succeeded" : "failed",
          status: captureStatus,
        }),
      );
    }
    captureActive = false;
  }
});

server.requestTimeout = 10_000;
server.headersTimeout = 5_000;
server.listen(port, "0.0.0.0");

await getBrowser();

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.once(signal, async () => {
    server.close();
    await browser?.close().catch(() => {});
    process.exit(0);
  });
}
