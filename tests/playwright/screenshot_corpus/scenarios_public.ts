// scenarios_public.ts - signed-out and session-recovery screenshot states.
// Selector contract: sign-in and renewal headings are owned by src/pages/sign_in_page.tsx:92 and
// src/app.tsx:101; seeded identity actions are shared through visible_workflows.ts:16.

import type { ScenarioDefinition } from "./scenario_types";
import { enterInstructor, scrollTop } from "./visible_workflows";

export const PUBLIC_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "public_entry",
    role: "public",
    captures: [
      {
        checkpoint: "sign_in_laptop",
        area: "account",
        workflow: "authentication",
        state: "sign in",
        viewport: "laptop",
        privacyProfile: "public",
        caption: "Seeded Live Demo sign-in",
        featured: true,
      },
      {
        checkpoint: "sign_in_phone",
        area: "account",
        workflow: "authentication",
        state: "sign in",
        viewport: "phone",
        privacyProfile: "public",
        caption: "Seeded Live Demo sign-in on a phone",
      },
      {
        checkpoint: "session_renewal_laptop",
        area: "account",
        workflow: "session recovery",
        state: "expired",
        viewport: "laptop",
        privacyProfile: "public",
        caption: "Expired-session renewal",
      },
    ],
    async run(runtime): Promise<void> {
      const laptop = await runtime.open("sign_in_laptop");
      try {
        await runtime.captureCheckpoint(laptop, "sign_in_laptop");
        await enterInstructor(laptop.page);
        await laptop.privacy.settleResponses();
        await laptop.context.clearCookies();
        await laptop.page.reload({ waitUntil: "commit" });
        await laptop.page
          .getByRole("heading", { name: "Your session needs to be renewed", exact: true })
          .waitFor();
        await scrollTop(laptop.page);
        await runtime.captureCheckpoint(laptop, "session_renewal_laptop");
      } finally {
        await runtime.close(laptop);
      }

      const phone = await runtime.open("sign_in_phone");
      try {
        await runtime.captureCheckpoint(phone, "sign_in_phone");
      } finally {
        await runtime.close(phone);
      }
    },
  },
];
