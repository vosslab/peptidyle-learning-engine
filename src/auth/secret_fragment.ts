// One-time secret extraction without query strings, logs, or browser storage.

const SECRET_PATTERN = /^[A-Za-z0-9_-]{43}$/u;

/** Reads one token fragment into memory and immediately removes it from the visible URL. */
export function consumeTokenFragment(location: Location, history: History): string | null {
  const fragment = location.hash.startsWith("#") ? location.hash.slice(1) : location.hash;
  const parameters = new URLSearchParams(fragment);
  const values = parameters.getAll("token");
  const token = values.length === 1 ? (values[0] ?? null) : null;
  history.replaceState(history.state, "", `${location.pathname}${location.search}`);
  return token !== null && SECRET_PATTERN.test(token) ? token : null;
}
