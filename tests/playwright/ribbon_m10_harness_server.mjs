// Compiled-harness startup helpers shared by M10 shell evidence.

import { once } from "node:events";
import { createServer } from "node:http";

function formatBootstrapDiagnostics(pageErrors, consoleErrors) {
  const details = [...pageErrors, ...consoleErrors];
  const summary = details.length === 0 ? "no page or console errors" : details.join(" | ");
  return `Application-shell harness bootstrap failed: ${summary}`;
}

export async function mountRibbonM10Harness(page, bundle, pageErrors, consoleErrors) {
  try {
    await page.addScriptTag({ content: Buffer.from(bundle.javascript).toString("utf8") });
    await page.waitForFunction(
      () =>
        "PleRibbonM10Harness" in window &&
        typeof window.PleRibbonM10Harness.mountRibbonM10ShellHarness === "function",
      undefined,
      { timeout: 5_000 },
    );
    await page.evaluate(() => {
      const target = document.querySelector("#root");
      if (!(target instanceof HTMLElement))
        throw new Error("Application-shell harness root is missing.");
      window.ribbonM10 = window.PleRibbonM10Harness.mountRibbonM10ShellHarness(target);
    });
    await page.waitForFunction(() => "ribbonM10" in window, undefined, { timeout: 5_000 });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${formatBootstrapDiagnostics(pageErrors, consoleErrors)}; ${message}`, {
      cause: error,
    });
  }
}

export async function startHarnessServer(markup) {
  const server = createServer((_request, response) => {
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end(markup);
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address();
  if (address === null || typeof address === "string") {
    throw new Error("Application-shell evidence server did not receive a TCP address.");
  }
  return {
    evidenceUrl: `http://127.0.0.1:${String(address.port)}/`,
    async close() {
      server.close();
      await once(server, "close");
    },
  };
}
