const MOCK_ORIGIN = "https://mock.peptidyle.invalid";

export function requestFrom(input: RequestInfo | URL, init?: RequestInit): Request {
  if (input instanceof Request) {
    return new Request(input, init);
  }
  const url = new URL(input.toString(), MOCK_ORIGIN);
  return new Request(url, init);
}

export function pathSegments(request: Request): ReadonlyArray<string> {
  return new URL(request.url).pathname.split("/").filter(Boolean);
}

function routeResource(request: Request): string | undefined {
  const segments = pathSegments(request);
  if (segments[0] !== "api") {
    return undefined;
  }
  return segments[1];
}

export function handlesResource(request: Request, resources: ReadonlyArray<string>): boolean {
  const resource = routeResource(request);
  return resource !== undefined && resources.includes(resource);
}

export function jsonResponse(value: unknown, status = 200, headers: HeadersInit = {}): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", ...headers },
  });
}

export function routeNotFound(request: Request): Response {
  const url = new URL(request.url);
  return jsonResponse({ error: `No mock route for ${request.method} ${url.pathname}` }, 404);
}

export function methodNotAllowed(request: Request): Response {
  return jsonResponse(
    { error: `Method ${request.method} is not supported by this mock route` },
    405,
  );
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function hasOnlyFields(
  record: Record<string, unknown>,
  fields: ReadonlyArray<string>,
): boolean {
  const keys = Object.keys(record);
  return keys.length === fields.length && keys.every((key) => fields.includes(key));
}

/** Mirrors the server's bounded visible-ASCII idempotency-key grammar. */
export function validIdempotencyKey(value: string | null): boolean {
  return value !== null && value.length >= 1 && value.length <= 200 && !/[^\x21-\x7e]/.test(value);
}

/**
 * JSON.parse deliberately keeps the last duplicate object member, whereas the
 * server's serde boundary rejects duplicates. Scan the syntax first so the
 * browser mock cannot make a hostile payload appear less permissive than the
 * protected route it represents.
 */
export function hasDuplicateJsonObjectMember(text: string): boolean {
  let index = 0;
  let duplicate = false;

  const whitespace = (): void => {
    while (/[ \t\n\r]/.test(text[index] ?? "")) index += 1;
  };
  const string = (): string => {
    const start = index;
    if (text[index] !== '"') throw new Error("expected JSON string");
    index += 1;
    while (index < text.length) {
      const character = text[index] ?? "";
      if (character === '"') {
        index += 1;
        return text.slice(start, index);
      }
      if (character < " ") throw new Error("unescaped control character");
      if (character === "\\") {
        const escaped = text[index + 1];
        if (escaped === "u") {
          if (!/^[0-9a-fA-F]{4}$/.test(text.slice(index + 2, index + 6))) {
            throw new Error("invalid unicode escape");
          }
          index += 6;
          continue;
        }
        if (escaped === undefined || !'"\\/bfnrt'.includes(escaped)) {
          throw new Error("invalid JSON escape");
        }
        index += 2;
        continue;
      }
      index += 1;
    }
    throw new Error("unterminated JSON string");
  };
  const value = (): void => {
    whitespace();
    const character = text[index];
    if (character === "{") {
      index += 1;
      whitespace();
      const keys = new Set<string>();
      if (text[index] === "}") {
        index += 1;
        return;
      }
      while (true) {
        whitespace();
        const rawKey = string();
        const key = JSON.parse(rawKey) as string;
        if (keys.has(key)) duplicate = true;
        keys.add(key);
        whitespace();
        if (text[index] !== ":") throw new Error("expected JSON object colon");
        index += 1;
        value();
        whitespace();
        if (text[index] === "}") {
          index += 1;
          return;
        }
        if (text[index] !== ",") throw new Error("expected JSON object separator");
        index += 1;
      }
    }
    if (character === "[") {
      index += 1;
      whitespace();
      if (text[index] === "]") {
        index += 1;
        return;
      }
      while (true) {
        value();
        whitespace();
        if (text[index] === "]") {
          index += 1;
          return;
        }
        if (text[index] !== ",") throw new Error("expected JSON array separator");
        index += 1;
      }
    }
    if (character === '"') {
      string();
      return;
    }
    if (
      text.startsWith("true", index) ||
      text.startsWith("false", index) ||
      text.startsWith("null", index)
    ) {
      index += text.startsWith("true", index) ? 4 : text.startsWith("false", index) ? 5 : 4;
      return;
    }
    const number = /-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/y;
    number.lastIndex = index;
    if (!number.exec(text)) throw new Error("expected JSON value");
    index = number.lastIndex;
  };

  value();
  whitespace();
  if (index !== text.length) throw new Error("unexpected JSON suffix");
  return duplicate;
}
