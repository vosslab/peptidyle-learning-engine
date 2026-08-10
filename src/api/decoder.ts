// decoder.ts - small runtime primitives for untrusted API JSON.

/** Runtime boundary failure with a precise JSON path. */
export class DecodeError extends Error {
  public constructor(path: string, expected: string) {
    super(`${path} must be ${expected}`);
    this.name = "DecodeError";
  }
}

/** One runtime decoder that also supplies its current JSON path. */
export type Decoder<T> = (value: unknown, path: string) => T;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function decodeRecord(value: unknown, path: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new DecodeError(path, "an object");
  }
  return value;
}

export function decodeField(record: Record<string, unknown>, key: string, path: string): unknown {
  if (!(key in record)) {
    throw new DecodeError(`${path}.${key}`, "present");
  }
  return record[key];
}

export function decodeString(value: unknown, path: string): string {
  if (typeof value !== "string") {
    throw new DecodeError(path, "a string");
  }
  return value;
}

export function decodeNonemptyString(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (decoded.length === 0) {
    throw new DecodeError(path, "a nonempty string");
  }
  return decoded;
}

export function decodeUuid(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  // Rust's UUID-backed domain IDs and PostgreSQL's uuid type accept every
  // canonical 16-byte UUID value, including deterministic local/test IDs
  // without an RFC version or variant nibble. The browser validates the wire
  // shape; it must not invent a stricter identity policy than the server.
  const uuid = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i;
  if (!uuid.test(decoded)) {
    throw new DecodeError(path, "a UUID");
  }
  return decoded;
}

export function decodeBoolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") {
    throw new DecodeError(path, "a boolean");
  }
  return value;
}

export function decodeTrue(value: unknown, path: string): true {
  if (value !== true) {
    throw new DecodeError(path, "true");
  }
  return true;
}

export function decodeFiniteNumber(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new DecodeError(path, "a finite number");
  }
  return value;
}

export function decodeSafeInteger(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    throw new DecodeError(path, "a safe integer");
  }
  return value;
}

export function decodeNonnegativeInteger(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 0) {
    throw new DecodeError(path, "a nonnegative safe integer");
  }
  return decoded;
}

export function decodePositiveInteger(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded <= 0) {
    throw new DecodeError(path, "a positive safe integer");
  }
  return decoded;
}

export function decodeArray<T>(value: unknown, path: string, decodeElement: Decoder<T>): Array<T> {
  if (!Array.isArray(value)) {
    throw new DecodeError(path, "an array");
  }
  return value.map((element, index) => decodeElement(element, `${path}[${index}]`));
}

export function decodeNullable<T>(value: unknown, path: string, decodeValue: Decoder<T>): T | null {
  return value === null ? null : decodeValue(value, path);
}

export function decodeStringEnum<T extends string>(
  value: unknown,
  path: string,
  allowed: ReadonlyArray<T>,
): T {
  for (const candidate of allowed) {
    if (value === candidate) {
      return candidate;
    }
  }
  throw new DecodeError(path, `one of ${allowed.join(", ")}`);
}

export function decodeDictionary<T>(
  value: unknown,
  path: string,
  decodeValue: Decoder<T>,
): Record<string, T> {
  const record = decodeRecord(value, path);
  const decoded: Record<string, T> = {};
  for (const [key, entry] of Object.entries(record)) {
    decoded[key] = decodeValue(entry, `${path}.${key}`);
  }
  return decoded;
}
