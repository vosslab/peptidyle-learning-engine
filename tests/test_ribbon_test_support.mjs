// test_ribbon_test_support.mjs - direct consumers for the durable Ribbon test harness.

import assert from "node:assert/strict";
import test from "node:test";

import { createRoot } from "solid-js";

import {
  createDeferredResolution,
  createRoutingInFlightSignal,
  createScrollIntoViewStub,
  mountForEachProductRole,
  useProductRoleFixture,
  walkPathnamesThroughMountedApp,
} from "./support/ribbon_test_support.ts";
import {
  RIBBON_PLAYWRIGHT_CONTEXT_OPTIONS,
  RIBBON_RESPONSIVE_PROFILES,
} from "./playwright/ui_corpus_manifest.ts";

test("Product Role fixtures render one trivial Solid consumer for each immutable role", () => {
  function ProductRoleConsumer() {
    return useProductRoleFixture().productRole;
  }

  const renderedRoles = mountForEachProductRole(ProductRoleConsumer);
  assert.deepEqual(renderedRoles, ["student", "instructor", "sysadmin"]);
});

test("transition driver retains one mounted app while it walks pathnames", async () => {
  const transitions = [];
  let mounts = 0;
  await walkPathnamesThroughMountedApp({
    pathnames: ["/courses", "/courses/course-a", "/courses/course-a/assignments"],
    mount: () => {
      mounts += 1;
      return {
        navigate: (pathname) => transitions.push(pathname),
        dispose: () => transitions.push("disposed"),
      };
    },
  });
  assert.equal(mounts, 1);
  assert.deepEqual(transitions, [
    "/courses",
    "/courses/course-a",
    "/courses/course-a/assignments",
    "disposed",
  ]);
});

test("deferred resolution stays pending until the consumer releases it", async () => {
  const deferred = createDeferredResolution();
  let settled = false;
  void deferred.promise.then(function observeSettlement() {
    settled = true;
  });

  await Promise.resolve();
  assert.equal(settled, false);
  deferred.resolve("ready");
  await Promise.resolve();
  assert.equal(settled, true);
  assert.equal(await deferred.promise, "ready");
});

test("scroll stub records the exact request without a browser layout engine", () => {
  const scroll = createScrollIntoViewStub();
  scroll.element.scrollIntoView({ inline: "nearest" });
  assert.deepEqual(scroll.calls(), [{ inline: "nearest" }]);
});

test("routing signal is an injectable Solid reactive dependency", () => {
  const signal = createRoot(() => createRoutingInFlightSignal());
  signal.setInFlight(true);
  assert.equal(signal.inFlight(), true);
});

test("Ribbon Playwright context honors forced colors and reduced motion", () => {
  assert.deepEqual(RIBBON_PLAYWRIGHT_CONTEXT_OPTIONS, {
    forcedColors: "active",
    reducedMotion: "reduce",
  });
});

test("Ribbon corpus retains desktop, portrait-tablet, and narrow-phone profiles", () => {
  assert.deepEqual(
    RIBBON_RESPONSIVE_PROFILES.map((profile) => profile.id),
    ["instructor_desktop", "portrait_tablet", "narrow_phone"],
  );
  assert.deepEqual(RIBBON_RESPONSIVE_PROFILES[0]?.contextOptions.viewport, {
    width: 1280,
    height: 800,
  });
  assert.deepEqual(RIBBON_RESPONSIVE_PROFILES.at(-1)?.contextOptions.viewport, {
    width: 320,
    height: 640,
  });
  for (const profile of RIBBON_RESPONSIVE_PROFILES) {
    assert.equal(profile.contextOptions.forcedColors, "active");
    assert.equal(profile.contextOptions.reducedMotion, "reduce");
  }
});
