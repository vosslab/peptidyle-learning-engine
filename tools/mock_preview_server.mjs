// mock_preview_server.mjs - serves the generated browser mock with typed fixture assets.

import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const distDir = path.join(repoRoot, "dist");
const portArgument = process.argv[2];
const port = portArgument === undefined ? 4173 : Number(portArgument);

if (!Number.isSafeInteger(port) || port <= 0 || port > 65_535) {
  throw new Error("preview port must be an integer from 1 through 65535");
}

function contentType(requestPath) {
  if (requestPath.startsWith("/api/assets/")) return "image/svg+xml; charset=utf-8";
  if (requestPath === "/" || requestPath.endsWith(".html")) return "text/html; charset=utf-8";
  if (requestPath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (requestPath.endsWith(".css")) return "text/css; charset=utf-8";
  if (requestPath.endsWith(".wasm")) return "application/wasm";
  if (requestPath.endsWith(".map")) return "application/json; charset=utf-8";
  return "application/octet-stream";
}

function resolveStaticPath(requestPath) {
  const relativePath = requestPath === "/" ? "index.html" : requestPath.slice(1);
  const resolved = path.resolve(distDir, relativePath);
  if (resolved !== distDir && !resolved.startsWith(`${distDir}${path.sep}`)) return null;
  return resolved;
}

const server = http.createServer((request, response) => {
  const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
  const filePath = resolveStaticPath(requestUrl.pathname);
  if (request.method !== "GET" || filePath === null || !fs.existsSync(filePath)) {
    response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    response.end("Not found");
    return;
  }
  let bytes = fs.readFileSync(filePath);
  if (requestUrl.pathname === "/" || requestUrl.pathname.endsWith(".html")) {
    bytes = Buffer.from(
      bytes
        .toString("utf8")
        .replace("</head>", "<script>window.__PLE_USE_MOCK_API__=true;</script></head>"),
      "utf8",
    );
  }
  response.writeHead(200, { "content-type": contentType(requestUrl.pathname) });
  response.end(bytes);
});

server.listen(port, "127.0.0.1", () => {
  console.log(`Mock preview serving dist at http://127.0.0.1:${port}/`);
});
