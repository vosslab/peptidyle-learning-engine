/* global window, document, HTMLFormElement, ResizeObserver */

// A same-origin backend bridge and an opaque preview size reporter share this one public file.

(() => {
  "use strict";

  const opaquePreview = window.origin === "null";
  const parentOrigin = window.location.origin;
  const captureRequestPattern = /^ple\.backendOwned\.capture:([a-f0-9]{16})$/;
  const responseKind = "ple.backendOwned.response";
  const previewResizeKind = "ple.webwork.preview.resize";
  const appearanceKind = "ple.embed.appearance";
  const appearanceVersion = 1;
  const appearancePrefix = `${appearanceKind}:`;
  const previewMinimumHeight = 160;
  const previewMaximumHeight = 1200;
  const hexColor = /^#[0-9a-f]{6}$/i;
  const appearanceColorKeys = [
    "background",
    "foreground",
    "surface",
    "secondary",
    "accent",
    "highlight",
    "muted",
    "border",
    "onAccent",
    "link",
  ];
  const appearanceProperties = [
    ["--ple-document-background", "background"],
    ["--ple-document-foreground", "foreground"],
    ["--ple-document-surface", "surface"],
    ["--ple-document-secondary", "secondary"],
    ["--ple-document-accent", "accent"],
    ["--ple-document-highlight", "highlight"],
    ["--ple-document-muted", "muted"],
    ["--ple-document-border", "border"],
    ["--ple-document-on-accent", "onAccent"],
    ["--ple-document-link", "link"],
    ["--bs-body-bg", "background"],
    ["--bs-body-color", "foreground"],
    ["--bs-secondary-bg", "secondary"],
    ["--bs-tertiary-bg", "secondary"],
    ["--bs-border-color", "border"],
    ["--bs-primary", "accent"],
    ["--bs-link-color", "link"],
    ["--bs-link-hover-color", "link"],
  ];

  function isRecord(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
  }

  function hasExactKeys(value, keys) {
    const received = Object.keys(value);
    return received.length === keys.length && keys.every((key) => received.includes(key));
  }

  function isAppearanceMessage(value) {
    if (!isRecord(value) || !hasExactKeys(value, ["kind", "version", "mode", "colors"]))
      return false;
    if (value.kind !== appearanceKind || value.version !== appearanceVersion) return false;
    if (value.mode !== "light" && value.mode !== "dark") return false;
    if (!isRecord(value.colors) || !hasExactKeys(value.colors, appearanceColorKeys)) return false;
    return appearanceColorKeys.every(
      (key) => typeof value.colors[key] === "string" && hexColor.test(value.colors[key]),
    );
  }

  function applyAppearance(message) {
    // ASVS 1.2.3/2.2.1: validated colors enter only this fixed property list;
    // no received property name or CSS text is ever interpreted.
    const root = document.documentElement;
    if (root === undefined) return;
    root.style.colorScheme = message.mode;
    for (const [property, color] of appearanceProperties)
      root.style.setProperty(property, message.colors[color]);
  }

  function decodedAppearance(value) {
    if (typeof value !== "string" || !value.startsWith(appearancePrefix)) return undefined;
    try {
      const message = JSON.parse(value.slice(appearancePrefix.length));
      return isAppearanceMessage(message) ? message : undefined;
    } catch {
      return undefined;
    }
  }

  function receiveAppearance(event) {
    // ASVS 3.5.5: require the canonical parent origin, exact parent window,
    // and a closed cosmetic record before altering the isolated document.
    const appearance = decodedAppearance(event.data);
    if (event.origin !== parentOrigin || event.source !== window.parent || appearance === undefined)
      return;
    applyAppearance(appearance);
  }

  window.addEventListener("message", receiveAppearance);

  function visibleBottom(element) {
    if (element.getClientRects().length === 0) return 0;
    return element.getBoundingClientRect().bottom;
  }

  function cssPixels(value) {
    const pixels = Number.parseFloat(value);
    return Number.isFinite(pixels) && pixels >= 0 ? pixels : 0;
  }

  function bodyBottomInset() {
    if (document.body === undefined) return 0;
    const style = window.getComputedStyle(document.body);
    return cssPixels(style.paddingBottom) + cssPixels(style.borderBottomWidth);
  }

  function previewHeight() {
    const form = document.querySelector("form");
    const topLevelContent = Array.from(document.body?.children ?? []);
    const contentBottom = Math.max(
      form instanceof HTMLFormElement ? visibleBottom(form) : 0,
      ...topLevelContent.map(visibleBottom),
    );
    const roundedHeight = Math.ceil(contentBottom + bodyBottomInset());
    const boundedHeight = Math.min(
      previewMaximumHeight,
      Math.max(previewMinimumHeight, roundedHeight),
    );
    return Number.isSafeInteger(boundedHeight) ? boundedHeight : previewMinimumHeight;
  }

  function reportOpaquePreviewHeight() {
    let lastHeight;
    let observer;

    function report() {
      const height = previewHeight();
      if (height === lastHeight) return;
      lastHeight = height;
      // ASVS 3.5.5: an opaque frame publishes only this closed size record to
      // the canonical parent origin; the parent also verifies origin and source.
      window.parent.postMessage({ kind: previewResizeKind, version: 1, height }, parentOrigin);
    }

    function observe() {
      if (observer !== undefined || typeof ResizeObserver !== "function") return;
      observer = new ResizeObserver(report);
      if (document.body !== undefined) observer.observe(document.body);
      const form = document.querySelector("form");
      if (form instanceof HTMLFormElement) observer.observe(form);
    }

    report();
    observe();
    window.addEventListener("load", () => {
      report();
      observe();
    });
  }

  if (opaquePreview) {
    reportOpaquePreviewHeight();
    return;
  }

  function pairs(form) {
    return Array.from(new FormData(form), ([name, value]) => [name, String(value)]);
  }

  function isCaptureId(value) {
    return typeof value === "string" && /^[a-f0-9]{16}$/.test(value);
  }

  function send(form, captureId) {
    const message = { kind: responseKind, pairs: pairs(form) };
    if (captureId !== undefined) message.captureId = captureId;
    window.parent.postMessage(message, parentOrigin);
  }

  function firstForm() {
    return document.querySelector("form");
  }

  document.addEventListener("submit", (event) => {
    const form = event.target;
    if (!(form instanceof HTMLFormElement)) return;
    event.preventDefault();
    send(form);
  });

  window.addEventListener("message", (event) => {
    if (event.origin !== parentOrigin || event.source !== window.parent) return;
    if (typeof event.data !== "string") return;
    const match = captureRequestPattern.exec(event.data);
    if (match === null || !isCaptureId(match[1])) return;
    const form = firstForm();
    if (form !== null) send(form, match[1]);
  });

  window.parent.postMessage({ kind: "ple.backendOwned.ready" }, parentOrigin);
})();
