// mock_preview_server.mjs - serves the generated browser mock with typed fixture assets.

import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { routeContractForPathname } from "../src/route_contract.ts";

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

function isRegularFile(filePath) {
  return fs.existsSync(filePath) && fs.statSync(filePath).isFile();
}

function acceptsHtmlDocument(request) {
  const accept = request.headers.accept ?? "";
  return accept
    .split(",")
    .map((value) => value.trim().split(";", 1)[0])
    .includes("text/html");
}

function declaredRouteShell(request, requestPath) {
  if (!acceptsHtmlDocument(request) || path.posix.extname(requestPath) !== "") return null;
  if (routeContractForPathname(requestPath) === undefined) return null;
  return path.join(distDir, "index.html");
}

const server = http.createServer((request, response) => {
  const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
  const staticPath = resolveStaticPath(requestUrl.pathname);
  if (request.method !== "GET" || staticPath === null) {
    response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    response.end("Not found");
    return;
  }
  const servesDeclaredRouteShell = !isRegularFile(staticPath);
  const filePath = servesDeclaredRouteShell
    ? declaredRouteShell(request, requestUrl.pathname)
    : staticPath;
  if (filePath === null || !isRegularFile(filePath)) {
    response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    response.end("Not found");
    return;
  }
  let bytes = fs.readFileSync(filePath);
  const servesHtml = filePath.endsWith(".html");
  if (servesHtml) {
    let html = bytes.toString("utf8");
    if (servesDeclaredRouteShell) {
      html = html.replace("<head>", '<head><base href="/">');
    }
    bytes = Buffer.from(
      html.replace("</head>", "<script>window.__PLE_USE_MOCK_API__=true;</script></head>"),
      "utf8",
    );
  }
  const responseType = servesHtml ? "text/html; charset=utf-8" : contentType(requestUrl.pathname);
  response.writeHead(200, { "content-type": responseType });
  response.end(bytes);
});

server.listen(port, "127.0.0.1", () => {
  console.log(`Mock preview serving dist at http://127.0.0.1:${port}/`);
});
