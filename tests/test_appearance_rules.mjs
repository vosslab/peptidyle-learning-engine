// Pure ownership and display-mode rules from the whole-interface Theme plan.

import assert from "node:assert/strict";
import test from "node:test";

import { resolveDisplayMode, resolveTheme } from "../src/appearance/appearance_rules.ts";

test("Course pages use their Course Theme", () => {
  assert.equal(
    resolveTheme({
      courseTheme: "ocean",
      instructorPersonalTheme: "forest",
      signedInInstructor: true,
    }),
    "ocean",
  );
});

test("global Instructor pages use the Instructor personal Theme", () => {
  assert.equal(
    resolveTheme({ instructorPersonalTheme: "forest", signedInInstructor: true }),
    "forest",
  );
});

test("global Student and signed-out pages use grass", () => {
  assert.equal(
    resolveTheme({ instructorPersonalTheme: "forest", signedInInstructor: false }),
    "grass",
  );
});

test("an unset display preference follows the browser", () => {
  assert.equal(resolveDisplayMode(null, "dark"), "dark");
});

test("a Light display preference wins everywhere", () => {
  assert.equal(resolveDisplayMode("light", "dark"), "light");
});

test("a Dark display preference wins everywhere", () => {
  assert.equal(resolveDisplayMode("dark", "light"), "dark");
});
