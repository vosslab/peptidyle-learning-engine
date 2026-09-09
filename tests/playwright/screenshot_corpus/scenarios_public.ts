// scenarios_public.ts - signed-out and session-recovery screenshot states.
// Selector contract: sign-in and renewal headings are owned by src/pages/sign_in_page.tsx:92 and
// src/app.tsx:101; seeded identity actions are shared through visible_workflows.ts:16.

import type { ScenarioDefinition } from "./scenario_types";
import { enterInstructor, scrollTop } from "./visible_workflows";

export const PUBLIC_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "public_entry",
    checkpoints: ["sign_in_laptop", "sign_in_phone", "session_renewal_laptop"],
    async run(runtime): Promise<void> {
      const laptopRecord = runtime.record("public_entry", "sign_in_laptop");
      const laptop = await runtime.open(laptopRecord);
      try {
        await runtime.capture(laptop, laptopRecord);
        const libraryLoaded = laptop.page.waitForEvent("requestfinished", {
          predicate: (request) => new URL(request.url()).pathname === "/api/questions/search",
        });
        await enterInstructor(laptop.page);
        await libraryLoaded;
        await laptop.privacy.settleResponses();
        await laptop.context.clearCookies();
        await laptop.page.reload({ waitUntil: "commit" });
        await laptop.page
          .getByRole("heading", { name: "Your session needs to be renewed", exact: true })
          .waitFor();
        await scrollTop(laptop.page);
        await runtime.capture(laptop, runtime.record("public_entry", "session_renewal_laptop"));
      } finally {
        await runtime.close(laptop);
      }

      const phoneRecord = runtime.record("public_entry", "sign_in_phone");
      const phone = await runtime.open(phoneRecord);
      try {
        await runtime.capture(phone, phoneRecord);
      } finally {
        await runtime.close(phone);
      }
    },
  },
];
