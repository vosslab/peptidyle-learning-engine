// helper_browser_test_server.mjs - static front door for the isolated browser-test artifact.

import http from "node:http";
import fs from "node:fs/promises";
import path from "node:path";

import { routeContractForPathname } from "../../src/route_contract.ts";

const port = Number.parseInt(process.argv[2] ?? "4173", 10);
if (!Number.isSafeInteger(port) || port < 1 || port > 65535) {
  throw new Error("browser-test server requires a valid TCP port");
}

const artifactDirectory = path.resolve("dist_browser_test");
const mimeTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".map", "application/json; charset=utf-8"],
  [".svg", "image/svg+xml; charset=utf-8"],
  [".wasm", "application/wasm"],
]);

function requestPathname(requestUrl) {
  try {
    const pathname = new URL(requestUrl ?? "/", "http://127.0.0.1").pathname;
    return decodeURIComponent(pathname);
  } catch (error) {
    if (error instanceof URIError) return undefined;
    throw error;
  }
}

function artifactPath(pathname) {
  if (pathname === undefined || !pathname.startsWith("/")) return undefined;
  const relativePath = pathname.slice(1);
  const resolvedPath = path.resolve(artifactDirectory, relativePath || "index.html");
  if (
    resolvedPath !== artifactDirectory &&
    !resolvedPath.startsWith(`${artifactDirectory}${path.sep}`)
  ) {
    return undefined;
  }
  return resolvedPath;
}

function acceptsHtmlDocument(request) {
  return (request.headers.accept ?? "")
    .split(",")
    .map((value) => value.trim().split(";", 1)[0])
    .includes("text/html");
}

async function isRegularFile(filePath) {
  try {
    const stats = await fs.stat(filePath);
    return stats.isFile();
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") return false;
    throw error;
  }
}

async function fileForRequest(request) {
  const pathname = requestPathname(request.url);
  const requestedPath = artifactPath(pathname);
  if (requestedPath === undefined) return undefined;
  if (await isRegularFile(requestedPath)) {
    return { filePath: requestedPath, pathname };
  }
  if (
    pathname === undefined ||
    path.posix.extname(pathname) !== "" ||
    !acceptsHtmlDocument(request) ||
    routeContractForPathname(pathname) === undefined
  ) {
    return undefined;
  }
  const shellPath = artifactPath("/");
  if (shellPath === undefined || !(await isRegularFile(shellPath))) return undefined;
  return { filePath: shellPath, pathname };
}

function contentType(filePath, pathname) {
  if (pathname?.startsWith("/api/assets/")) return "image/svg+xml; charset=utf-8";
  const extension = path.extname(filePath);
  return mimeTypes.get(extension) ?? "application/octet-stream";
}

async function handleRequest(request, response) {
  if (request.method !== "GET") {
    response.writeHead(404);
    response.end();
    return;
  }
  const file = await fileForRequest(request);
  if (file === undefined) {
    response.writeHead(404);
    response.end();
    return;
  }
  const body = await fs.readFile(file.filePath);
  response.writeHead(200, { "Content-Type": contentType(file.filePath, file.pathname) });
  response.end(body);
}

const server = http.createServer((request, response) => {
  void handleRequest(request, response).catch((error) => {
    console.error(error);
    response.writeHead(500);
    response.end();
  });
});

server.listen(port, "127.0.0.1", () => {
  console.log(`browser-test artifact server listening on ${port}`);
});
