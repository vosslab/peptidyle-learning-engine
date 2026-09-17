/* global window, document, HTMLFormElement, ResizeObserver */

// A same-origin backend bridge and an opaque preview size reporter share this one public file.

(() => {
  "use strict";

  const opaquePreview = window.origin === "null";
  const parentOrigin = window.location.origin;
  const captureRequestPattern = /^ple\.backendOwned\.capture:([a-f0-9]{16})$/;
  const responseKind = "ple.backendOwned.response";
  const previewResizeKind = "ple.webwork.preview.resize";
  const previewMinimumHeight = 160;
  const previewMaximumHeight = 1200;

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
