/** Shared canonical contract for the server-brokered external-tool route. */

export function externalToolLaunchPath(attemptId: string): string {
  return `/api/attempts/${encodeURIComponent(attemptId)}/external-tool/launch`;
}

/**
 * Checks the exact broker path for the current attempt and its browser origin.
 * Keeping the route equality here prevents transport and UI validators from
 * drifting into different same-origin interpretations.
 */
export function isCanonicalExternalToolLaunchPath(
  launchUrl: string,
  attemptId: string,
  origin: string,
): boolean {
  if (
    launchUrl !== externalToolLaunchPath(attemptId) ||
    !launchUrl.startsWith("/") ||
    launchUrl.startsWith("//") ||
    launchUrl.includes("?") ||
    launchUrl.includes("#") ||
    launchUrl.includes("\\") ||
    launchUrl.includes("%") ||
    launchUrl.includes("@") ||
    Array.from(launchUrl).some((character) => {
      const code = character.codePointAt(0) ?? 0;
      return code <= 0x1f || code === 0x7f;
    })
  ) {
    return false;
  }

  let trustedOrigin: URL;
  let parsed: URL;
  try {
    trustedOrigin = new URL(origin);
    parsed = new URL(launchUrl, trustedOrigin);
  } catch (_error: unknown) {
    return false;
  }
  return (
    (trustedOrigin.protocol === "https:" || trustedOrigin.protocol === "http:") &&
    trustedOrigin.username === "" &&
    trustedOrigin.password === "" &&
    trustedOrigin.pathname === "/" &&
    trustedOrigin.search === "" &&
    trustedOrigin.hash === "" &&
    parsed.origin === trustedOrigin.origin &&
    parsed.username === "" &&
    parsed.password === "" &&
    parsed.search === "" &&
    parsed.hash === "" &&
    parsed.pathname === launchUrl
  );
}
