// Dedicated worker-boundary config: it creates neither a browser nor a web server.
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: "fault_handshake_worker.spec.ts",
  timeout: 30_000,
  workers: 1,
  reporter: "list",
  outputDir: "../../../test-results/fault-handshake-worker",
});
