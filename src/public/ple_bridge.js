/* global window, document, HTMLFormElement */

// Same-origin bridge for a backend-owned document. It transports form data only.

(() => {
  "use strict";

  const parentOrigin = window.location.origin;
  const captureRequestPattern = /^ple\.backendOwned\.capture:([a-f0-9]{16})$/;
  const responseKind = "ple.backendOwned.response";

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
