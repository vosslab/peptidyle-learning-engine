// Account-selected display instants. The zone name stays on Profile.

import { createMemo, type Accessor } from "solid-js";

import { useAppearance } from "./appearance/appearance_context";
import { createDisplayDateTimeFormatter } from "./format_datetime";

/** Formats instants in the signed-in Account's selected IANA zone and omits the zone name. */
export function useSelectedDisplayDateTimeFormatter(): Accessor<
  (timestamp: number | Date) => string
> {
  const appearance = useAppearance();
  return createMemo(() => {
    const timeZone = appearance.settings()?.timeZone;
    if (timeZone === undefined) {
      if (appearance.settingsState() === "error") {
        // ASVS 16.5.2: settings-load failure keeps timestamp facts visible with an explicit state.
        return () => "Time unavailable";
      }
      return () => "";
    }
    return createDisplayDateTimeFormatter(timeZone);
  });
}
