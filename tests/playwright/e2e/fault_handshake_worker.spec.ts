// Playwright-worker-only proof that the private owner handshake crosses the CLI worker boundary.
import { test } from "@playwright/test";

import { faultHandshakeFromEnvironment } from "./fault_handshake";

test("fault handshake worker: exchanges the closed owner phases without a browser", async () => {
  test.skip(
    process.env.PLE_BROWSER_SUITE_FAULT_SOCKET_PATH === undefined,
    "the dedicated owner peer selects this worker-only proof",
  );
  const handshake = await faultHandshakeFromEnvironment(
    process.env,
    "learner_gateway_recovery",
    "bs1-0123456789ab-learner_gateway_recovery",
  );
  try {
    handshake.notify("response_selected");
    await handshake.waitFor("gateway_stopped");
    handshake.notify("network_recovery_visible");
    await handshake.waitFor("gateway_recovered");
    handshake.notify("completed");
  } finally {
    handshake.close();
  }
});
