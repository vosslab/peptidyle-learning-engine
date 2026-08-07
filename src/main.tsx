// main.tsx - browser entry point (MOD-UI-SHELL, WP-F3).
//
// Solid's `render` takes a component *function*, not an element. Passing JSX
// directly evaluates it once, outside a reactive root, which is how a Solid app
// ends up rendering a static snapshot that never updates.

import { render } from "solid-js/web";

import { App } from "./app";
import { log } from "./log";

const mountPoint = document.getElementById("root");

if (mountPoint === null) {
  // Failing loudly beats rendering into a detached node and showing a blank
  // page with no console output.
  throw new Error("mount point #root missing from index.html");
}

log.info("peptidyle client booting");

render(() => <App />, mountPoint);
