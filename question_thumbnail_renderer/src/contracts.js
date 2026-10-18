export class RequestError extends Error {
  constructor(status, code) {
    super(code);
    this.status = status;
    this.code = code;
  }
}

export function parseOrigin(value) {
  if (!value) return undefined;
  try {
    const parsed = new URL(value);
    if (
      !["http:", "https:"].includes(parsed.protocol) ||
      !parsed.hostname ||
      parsed.username ||
      parsed.password ||
      parsed.pathname !== "/" ||
      parsed.search ||
      parsed.hash
    )
      return undefined;
    return parsed.origin;
  } catch {
    return undefined;
  }
}

export function parsePositiveInteger(value, minimum, maximum) {
  if (!/^\d+$/.test(value)) return minimum;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= minimum && parsed <= maximum ? parsed : minimum;
}

export function validatePayload(value, limits) {
  if (
    value === null ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    Object.keys(value).length !== 2 ||
    typeof value.html !== "string" ||
    typeof value.contentSecurityPolicy !== "string"
  )
    throw new RequestError(400, "invalid_request");

  const htmlBytes = Buffer.byteLength(value.html, "utf8");
  const cspBytes = Buffer.byteLength(value.contentSecurityPolicy, "utf8");
  // ASVS 2.2.1: bound document and policy inputs before browser allocation.
  if (
    htmlBytes === 0 ||
    htmlBytes > limits.maxHtmlBytes ||
    cspBytes === 0 ||
    cspBytes > limits.maxCspBytes
  )
    throw new RequestError(413, "request_too_large");
  // ASVS 1.2.1: reject non-printable bytes before using the trusted API policy as a header.
  if (/[^\x20-\x7e]/.test(value.contentSecurityPolicy)) {
    throw new RequestError(400, "invalid_request");
  }
  return { html: value.html, contentSecurityPolicy: value.contentSecurityPolicy };
}

export function safeRendererAssetPath(pathname) {
  // ASVS 1.3.6 and 13.2.4: only approved proxy namespaces reach the fixed API origin.
  if (/[?#\\\0]/.test(pathname) || /%(?![0-9a-f]{2})/i.test(pathname)) return false;
  if (/%(?:00|2e|2f|5c)/i.test(pathname)) return false;
  const match =
    /^\/api\/webwork-assets\/(webwork2_files|pg_files)\/([A-Za-z0-9._~!$&'()*+,;=:@%/-]+)$/.exec(
      pathname,
    );
  if (!match) return false;
  return match[2]
    .split("/")
    .every((segment) => segment !== "" && segment !== "." && segment !== "..");
}
