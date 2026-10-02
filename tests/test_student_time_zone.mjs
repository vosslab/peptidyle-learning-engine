import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import {
  decodeProfileSettings,
  decodeUpdateAccountSettingsInput,
} from "../src/api/decoders/profile_settings.ts";

test("Account Settings uses one self-only no-store GET and PUT contract", async () => {
  const requests = [];
  const client = createHttpApiClient({
    basePath: "/live",
    fetch: async (input, init) => {
      requests.push({ input, init });
      const timeZone = init?.method === "PUT" ? "America/Los_Angeles" : "America/New_York";
      return new Response(
        JSON.stringify({
          timeZone,
          displayModePreference: null,
          personalTheme: "grass",
        }),
        {
          headers: { "content-type": "application/json", "cache-control": "no-store" },
        },
      );
    },
  });

  assert.deepEqual(await client.getAccountSettings(), {
    timeZone: "America/New_York",
    displayModePreference: null,
    personalTheme: "grass",
  });
  assert.deepEqual(await client.updateAccountSettings({ timeZone: "America/Los_Angeles" }), {
    timeZone: "America/Los_Angeles",
    displayModePreference: null,
    personalTheme: "grass",
  });
  assert.equal(requests[0].input, "/live/api/account/settings");
  assert.equal(requests[0].init?.method, undefined);
  assert.equal(requests[1].input, "/live/api/account/settings");
  assert.equal(requests[1].init.method, "PUT");
  assert.equal(requests[1].init.body, '{"timeZone":"America/Los_Angeles"}');
  assert.equal(requests[1].init.credentials, "same-origin");
  assert.equal(requests[1].init.cache, "no-store");
});

test("Account Settings rejects unsupported zones and client-supplied identity", async () => {
  assert.throws(
    () =>
      decodeProfileSettings({
        timeZone: "America/Chicago",
        displayModePreference: null,
        personalTheme: null,
        accountId: "private",
      }),
    DecodeError,
  );
  const client = createHttpApiClient({
    fetch: async () =>
      new Response(
        JSON.stringify({
          timeZone: "not/a-zone",
          displayModePreference: null,
          personalTheme: null,
        }),
        {
          headers: { "content-type": "application/json", "cache-control": "no-store" },
        },
      ),
  });
  await assert.rejects(client.getAccountSettings(), DecodeError);
  assert.throws(
    () => decodeUpdateAccountSettingsInput({ timeZone: "America/Chicago", accountId: "private" }),
    DecodeError,
  );
  await assert.rejects(client.updateAccountSettings({ timeZone: " America/Chicago" }), DecodeError);
});

test("selected display zone formats the Account zone and omits its name", async () => {
  const { createComponent, createRoot } = await import("solid-js");
  const { AppearanceContext } = await import("../src/appearance/appearance_context.ts");
  const { useSelectedDisplayDateTimeFormatter } = await import("../src/selected_display_zone.ts");
  const instant = Date.parse("2026-01-15T18:30:00Z");

  function readZone(timeZone, settingsState = timeZone === undefined ? "loading" : "ready") {
    let formatted = "unset";
    createRoot((dispose) => {
      function Probe() {
        const formatDateTime = useSelectedDisplayDateTimeFormatter();
        formatted = formatDateTime()(instant);
        return null;
      }
      function appearance(zone) {
        return {
          appearance: () => ({ theme: "grass", mode: "light", tokens: {} }),
          initialAppearanceReady: () => true,
          settings: () =>
            zone === undefined
              ? undefined
              : { timeZone: zone, displayModePreference: null, personalTheme: null },
          settingsState: () => settingsState,
          busy: () => false,
          error: () => undefined,
          updateTimeZone: async () => {},
          updateDisplayModePreference: async () => {},
          updatePersonalTheme: async () => {},
          presentCourseTheme: () => {},
        };
      }
      createComponent(AppearanceContext.Provider, {
        value: appearance(timeZone),
        get children() {
          return createComponent(Probe, {});
        },
      });
      dispose();
    });
    return formatted;
  }

  const eastern = readZone("America/New_York");
  const pacific = readZone("America/Los_Angeles");
  assert.equal(eastern, "Jan 15, 2026, 1:30 PM");
  assert.equal(pacific, "Jan 15, 2026, 10:30 AM");
  assert.equal(eastern.includes("America/New_York"), false);
  assert.equal(pacific.includes("America/Los_Angeles"), false);
  assert.equal(readZone(undefined), "");
  const unavailable = readZone(undefined, "error");
  assert.equal(unavailable, "Time unavailable");
  assert.equal(unavailable.includes("1970"), false);
  assert.equal(unavailable.includes("America/"), false);
});
