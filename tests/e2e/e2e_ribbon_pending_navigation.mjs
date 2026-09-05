// e2e_ribbon_pending_navigation.mjs - browser-condition integration for Ribbon pending feedback.

import assert from "node:assert/strict";
import test from "node:test";

import { createRoot, createSignal } from "solid-js";

import { createRibbonPendingNavigation } from "../../src/ribbon/ribbon_pending_navigation.ts";

function createTestScheduler() {
  const callbacks = [];
  return {
    schedule(callback) {
      callbacks.push(callback);
    },
    flush() {
      while (callbacks.length > 0) {
        const callback = callbacks.shift();
        callback();
      }
    },
  };
}

function createPendingNavigationFixture(initialInFlight = false) {
  const scheduler = createTestScheduler();
  let setInFlight;
  let controller;
  let dispose;
  createRoot((disposeRoot) => {
    const [routingInFlight, setRoutingInFlight] = createSignal(initialInFlight);
    setInFlight = setRoutingInFlight;
    controller = createRibbonPendingNavigation({
      routingInFlight,
      scheduleMicrotask: scheduler.schedule,
    });
    dispose = disposeRoot;
  });
  return { controller, dispose, scheduler, setInFlight };
}

test("only the exact Ribbon link that initiated routing is pending", () => {
  const fixture = createPendingNavigationFixture();
  fixture.controller.activate("/courses/C-1");
  fixture.setInFlight(true);

  assert.equal(fixture.controller.pendingDestination(), "/courses/C-1");
  assert.equal(fixture.controller.isPending("/courses/C-1"), true);
  assert.equal(fixture.controller.isPending("/courses/C-2"), false);
  fixture.dispose();
});

test("routing begun outside the Ribbon does not decorate any destination", () => {
  const fixture = createPendingNavigationFixture();
  fixture.setInFlight(true);

  assert.equal(fixture.controller.pendingDestination(), undefined);
  assert.equal(fixture.controller.isPending("/courses/C-1"), false);
  fixture.dispose();
});

test("a newer Ribbon activation replaces the prior pending destination", () => {
  const fixture = createPendingNavigationFixture(true);
  fixture.controller.activate("/courses/C-1");
  fixture.controller.activate("/courses/C-2");

  assert.equal(fixture.controller.isPending("/courses/C-1"), false);
  assert.equal(fixture.controller.isPending("/courses/C-2"), true);
  fixture.dispose();
});

test("settling clears pending feedback after success or a redirect", () => {
  const fixture = createPendingNavigationFixture();
  fixture.controller.activate("/courses/C-1");
  fixture.setInFlight(true);
  fixture.setInFlight(false);

  assert.equal(fixture.controller.pendingDestination(), undefined);
  assert.equal(fixture.controller.isPending("/courses/C-1"), false);

  fixture.controller.activate("/courses/C-2");
  fixture.setInFlight(true);
  fixture.setInFlight(false);

  assert.equal(fixture.controller.pendingDestination(), undefined);
  assert.equal(fixture.controller.isPending("/courses/C-2"), false);
  fixture.dispose();
});

test("an activation that never starts routing cannot decorate a later navigation", () => {
  const fixture = createPendingNavigationFixture();
  fixture.controller.activate("/courses/C-1");
  fixture.scheduler.flush();

  assert.equal(fixture.controller.pendingDestination(), undefined);
  fixture.setInFlight(true);
  assert.equal(fixture.controller.isPending("/courses/C-1"), false);
  fixture.dispose();
});

test("same-turn routing start retains the recorded destination through its microtask check", () => {
  const fixture = createPendingNavigationFixture();
  fixture.controller.activate("/courses/C-1");
  fixture.setInFlight(true);
  fixture.scheduler.flush();

  assert.equal(fixture.controller.isPending("/courses/C-1"), true);
  fixture.dispose();
});

test("disposal clears state and makes a queued navigation check inert", () => {
  const fixture = createPendingNavigationFixture();
  fixture.controller.activate("/courses/C-1");
  fixture.dispose();
  fixture.scheduler.flush();

  assert.equal(fixture.controller.pendingDestination(), undefined);
  fixture.controller.activate("/courses/C-2");
  assert.equal(fixture.controller.pendingDestination(), undefined);
});

test("activation admits canonical root-relative Ribbon paths", () => {
  const fixture = createPendingNavigationFixture();
  for (const href of ["/", "/courses/C-1", "/assignment-attempts/R-1/summary"]) {
    fixture.controller.activate(href);
    fixture.setInFlight(true);
    assert.equal(fixture.controller.isPending(href), true);
    fixture.setInFlight(false);
  }
  fixture.dispose();
});

test("activation rejects noncanonical or non-path destination hrefs", () => {
  const fixture = createPendingNavigationFixture();
  const invalidHrefs = [
    "",
    "   ",
    "/courses/C-1 ",
    "javascript:alert(1)",
    "data:text/html,pending",
    "mailto:student@example.test",
    "//foreign.example/courses/C-1",
    "https://foreign.example/courses/C-1",
    "https://ribbon-navigation.invalid/courses/C-1",
    "/courses/C-1?next=/",
    "/courses/C-1#summary",
    "/courses\\C-1",
    "/courses/../admin",
    "/courses/%2e%2e/admin",
    "/courses//C-1",
    `/${"x".repeat(2048)}`,
  ];

  for (const href of invalidHrefs) {
    assert.throws(
      () => fixture.controller.activate(href),
      /destination href|canonical root-relative/,
    );
    assert.equal(fixture.controller.pendingDestination(), undefined);
  }
  fixture.dispose();
});
