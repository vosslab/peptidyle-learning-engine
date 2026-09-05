import assert from "node:assert/strict";
import test from "node:test";

import {
  RibbonSelectedTabVisibilityController,
  isFullyVisibleInRibbonRow,
} from "../src/ribbon/ribbon_selected_tab_visibility.ts";

function bounds(left, right) {
  return { left, right };
}

function scrollport(left = 0, right = 100) {
  return { getBoundingClientRect: () => bounds(left, right) };
}

function tab(left, right) {
  const calls = [];
  return {
    calls,
    element: {
      getBoundingClientRect: () => bounds(left, right),
      scrollIntoView: (options) => calls.push(options),
    },
  };
}

test("selected Tab visibility uses inclusive horizontal row boundaries", () => {
  const row = bounds(0, 100);
  assert.equal(isFullyVisibleInRibbonRow(bounds(0, 100), row), true);
  assert.equal(isFullyVisibleInRibbonRow(bounds(-1, 100), row), false);
  assert.equal(isFullyVisibleInRibbonRow(bounds(0, 101), row), false);
});

test("a newly selected out-of-view Tab scrolls once with ordinary-motion options", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  const selected = tab(101, 160);

  assert.equal(controller.observe("assignments", selected.element, scrollport(), false), true);
  assert.deepEqual(selected.calls, [{ behavior: "smooth", block: "nearest", inline: "nearest" }]);
  assert.equal(controller.observe("assignments", selected.element, scrollport(), false), false);
  assert.equal(selected.calls.length, 1);
});

test("a same selected key is revealed again only after its geometry changes", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  let left = 101;
  let right = 160;
  const selected = {
    calls: [],
    element: {
      getBoundingClientRect: () => bounds(left, right),
      scrollIntoView: (options) => selected.calls.push(options),
    },
  };

  assert.equal(controller.observe("overview", selected.element, scrollport(), false), true);
  assert.equal(controller.observe("overview", selected.element, scrollport(), false), false);
  left = 121;
  right = 180;
  assert.equal(controller.observe("overview", selected.element, scrollport(), false), true);
  assert.equal(selected.calls.length, 2);
});

test("an over-wide selected control has one deterministic attempt per geometry", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  const selected = tab(10, 180);

  assert.equal(controller.observe("overview", selected.element, scrollport(), false), true);
  assert.equal(controller.observe("overview", selected.element, scrollport(), false), false);
  assert.equal(selected.calls.length, 1);
});

test("a selected key change reveals the newly selected clipped Tab", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  const first = tab(-80, -1);
  const second = tab(101, 160);

  controller.observe("overview", first.element, scrollport(), false);
  assert.equal(controller.observe("assignments", second.element, scrollport(), false), true);
  assert.equal(first.calls.length, 1);
  assert.equal(second.calls.length, 1);
});

test("reselecting a previously clipped Tab starts a fresh selection epoch", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  const first = tab(101, 160);
  const second = tab(10, 90);
  const row = scrollport();

  assert.equal(controller.observe("overview", first.element, row, false), true);
  assert.equal(controller.observe("assignments", second.element, row, false), false);
  assert.equal(controller.observe("overview", first.element, row, false), true);
  assert.equal(first.calls.length, 2);
  assert.equal(second.calls.length, 0);
});

test("clearing and restoring a clipped selection starts a fresh selection epoch", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  const selected = tab(101, 160);
  const row = scrollport();

  assert.equal(controller.observe("overview", selected.element, row, false), true);
  assert.equal(controller.observe(undefined, undefined, row, false), false);
  assert.equal(controller.observe("overview", selected.element, row, false), true);
  assert.equal(selected.calls.length, 2);
});

test("a fully visible selected Tab is not scrolled", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  const selected = tab(10, 90);

  assert.equal(controller.observe("overview", selected.element, scrollport(), false), false);
  assert.equal(selected.calls.length, 0);
});

test("partially clipped Tabs on either side are revealed", () => {
  const leftController = new RibbonSelectedTabVisibilityController();
  const rightController = new RibbonSelectedTabVisibilityController();
  const left = tab(-1, 40);
  const right = tab(60, 101);

  assert.equal(leftController.observe("left", left.element, scrollport(), false), true);
  assert.equal(rightController.observe("right", right.element, scrollport(), false), true);
  assert.equal(left.calls.length, 1);
  assert.equal(right.calls.length, 1);
});

test("a cue-safe scrollport receives the precise reveal delta", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  const selected = tab(80, 130);
  const calls = [];
  const row = {
    getBoundingClientRect: () => bounds(0, 100),
    scrollBy: (options) => calls.push(options),
  };

  assert.equal(controller.observe("overview", selected.element, row, true), true);
  assert.deepEqual(calls, [{ behavior: "auto", left: 30 }]);
  assert.equal(selected.calls.length, 0);
});

test("reduced-motion preference switches the reveal behavior to auto", () => {
  const controller = new RibbonSelectedTabVisibilityController();
  const selected = tab(101, 160);

  assert.equal(
    controller.observe("assignments", selected.element, scrollport(), () => true),
    true,
  );
  assert.deepEqual(selected.calls, [{ behavior: "auto", block: "nearest", inline: "nearest" }]);
});

test("absent or unavailable selected Tabs safely perform no scroll", () => {
  const absentController = new RibbonSelectedTabVisibilityController();
  const unavailableController = new RibbonSelectedTabVisibilityController();
  const row = scrollport();
  const unavailable = { getBoundingClientRect: () => bounds(101, 160) };

  assert.equal(absentController.observe("assignments", undefined, row, false), false);
  assert.equal(unavailableController.observe("assignments", unavailable, row, false), false);
});
