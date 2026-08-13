// Shared strict same-origin transport fixture for focused HTTP client tests.

import { createMockFetch } from "../src/api/mock/handlers.ts";

export function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

export function createFixtureFetch() {
  const mockFetch = createMockFetch();
  const requests = [];

  async function fixtureFetch(input, init) {
    const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
    requests.push(request.clone());
    const url = new URL(request.url);
    const path = url.pathname.replace(/^\/ple/, "");
    if (path === "/api/validation/response-format") {
      return jsonResponse({ violations: [] });
    }
    if (path === "/api/validation/timer") {
      return jsonResponse("open");
    }
    if (path === "/api/validation/assignment-capabilities") {
      return jsonResponse([]);
    }
    if (/^\/api\/assets\/[0-9a-f-]+\/delivery$/iu.test(path) && request.method === "POST") {
      return jsonResponse({ url: "https://objects.example.test/signed/asset?expires=12345" });
    }
    const body = request.method === "GET" ? undefined : await request.text();
    return mockFetch(`${path}${url.search}`, {
      method: request.method,
      headers: request.headers,
      body,
    });
  }

  return { fixtureFetch, requests };
}
