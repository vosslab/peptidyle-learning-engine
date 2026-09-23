import type { LocalDateAndTime } from "../generated/api/LocalDateAndTime";

/** Returns the browser's resolved IANA zone for callers that preserve browser-local display. */
export function browserDisplayTimeZone(): string {
  return new Intl.DateTimeFormat().resolvedOptions().timeZone;
}

/** Creates a formatter for instants shown in one explicitly chosen display time zone. */
export function createDisplayDateTimeFormatter(
  displayTimeZone: string,
): (timestamp: number | Date) => string {
  const formatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: displayTimeZone,
  });

  function formatDateTime(timestamp: number | Date): string {
    return formatter.format(timestamp);
  }

  return formatDateTime;
}

const localWallClockDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
  timeZone: "UTC",
});

/**
 * Formats a server-projected local wall-clock value without converting its date or time fields.
 *
 * UTC is a neutral carrier for Intl formatting only. LocalDateAndTime has no time-zone claim.
 */
export function formatLocalWallClockDateTime(value: LocalDateAndTime): string {
  const neutralCarrier = new Date(`${value}Z`);
  return localWallClockDateTimeFormatter.format(neutralCarrier);
}
