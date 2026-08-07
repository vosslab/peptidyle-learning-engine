// WP-C7 behavior tests for the server-free mock API.

import assert from "node:assert/strict";
import test from "node:test";

import { createMockFetch, mockApiHandlers, MOCK_ROUTE_GROUPS } from "../src/api/mock/handlers.ts";

test("mock handlers cover every planned API route group exactly once", () => {
  const actual = mockApiHandlers.map((handler) => handler.group).toSorted();
  const expected = [...MOCK_ROUTE_GROUPS].toSorted();
  assert.deepEqual(actual, expected);
});

test("all route groups answer without a server", async () => {
  const mockFetch = createMockFetch();
  const probes = [
    { group: "auth", path: "/api/auth/session", method: "GET" },
    { group: "catalog", path: "/api/problems", method: "GET" },
    { group: "course", path: "/api/courses", method: "GET" },
    { group: "run", path: "/api/runs", method: "POST" },
    {
      group: "asset",
      path: "/api/assets/0198e000-0000-7000-8000-000000000010",
      method: "GET",
    },
  ];

  for (const probe of probes) {
    const response = await mockFetch(probe.path, { method: probe.method });
    assert.equal(response.ok, true, `${probe.group} returned ${response.status}`);
  }
});

test("mock history preserves fresh seeds and full attempt provenance", async () => {
  const mockFetch = createMockFetch();
  const response = await mockFetch("/api/runs/0198e000-0000-7000-8000-000000000023/attempts");
  assert.equal(response.status, 200);

  const payload = await response.json();
  assert.equal(typeof payload, "object");
  assert.notEqual(payload, null);
  assert.ok("items" in payload);
  assert.ok(Array.isArray(payload.items));
  assert.equal(payload.items.length, 1);

  const attempt = payload.items[0];
  assert.equal(typeof attempt, "object");
  assert.notEqual(attempt, null);
  assert.ok("seed" in attempt);
  assert.ok("parameterHash" in attempt);
  assert.ok("provenance" in attempt);
  assert.equal(typeof attempt.seed, "number");
  assert.equal(typeof attempt.parameterHash, "string");
  assert.equal(attempt.parameterHash.length, 64);
});

test("unknown paths do not fall through to the real network", async () => {
  const response = await createMockFetch()("/api/not-a-route");
  assert.equal(response.status, 404);
});
