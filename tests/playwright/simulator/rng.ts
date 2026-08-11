export type MasterSeed = number & { readonly __master_seed: "MasterSeed" };

export interface DeterministicStream {
  next_uint32(): number;
}

const UINT32_RANGE = 0x1_0000_0000;
const MAX_LABEL_LENGTH = 100;
const STABLE_LABEL = /^[a-z][a-z0-9_.:-]*$/;

class Sfc32Stream implements DeterministicStream {
  private a: number;
  private b: number;
  private c: number;
  private d: number;

  public constructor(a: number, b: number, c: number, d: number) {
    this.a = a;
    this.b = b;
    this.c = c;
    this.d = d;
  }

  public next_uint32(): number {
    const output = (this.a + this.b + this.d) >>> 0;
    this.d = (this.d + 1) >>> 0;
    this.a = (this.b ^ (this.b >>> 9)) >>> 0;
    this.b = (this.c + (this.c << 3)) >>> 0;
    this.c = ((this.c << 21) | (this.c >>> 11)) >>> 0;
    this.c = (this.c + output) >>> 0;
    return output;
  }
}

function splitmix32(state: number): number {
  let mixed = (state + 0x9e37_79b9) >>> 0;
  mixed = Math.imul(mixed ^ (mixed >>> 16), 0x85eb_ca6b) >>> 0;
  mixed = Math.imul(mixed ^ (mixed >>> 13), 0xc2b2_ae35) >>> 0;
  const output = (mixed ^ (mixed >>> 16)) >>> 0;
  return output;
}

function fnv1a_label_mix(master_seed: MasterSeed, label: string): number {
  let hash = (0x811c_9dc5 ^ master_seed) >>> 0;
  for (let index = 0; index < label.length; index += 1) {
    const character = label.charCodeAt(index);
    hash = Math.imul(hash ^ character, 0x0100_0193) >>> 0;
  }
  return hash;
}

function validate_label(label: string): void {
  if (label.length === 0 || label.length > MAX_LABEL_LENGTH || !STABLE_LABEL.test(label)) {
    throw new Error("substream label must be a stable lowercase ASCII identifier");
  }
}

function validate_count(count: number, candidate_count: number): void {
  if (!Number.isSafeInteger(count) || count < 0 || count > candidate_count) {
    throw new Error("allocation count must be an integer from zero through candidate count");
  }
}

export function validate_master_seed(value: number): MasterSeed {
  if (!Number.isSafeInteger(value) || value < 0 || value >= UINT32_RANGE) {
    throw new Error("master seed must be an unsigned 32-bit integer");
  }
  return value as MasterSeed;
}

export function create_named_stream(master_seed: number, label: string): DeterministicStream {
  const validated_seed = validate_master_seed(master_seed);
  validate_label(label);
  const mixed_label = fnv1a_label_mix(validated_seed, label);
  const a = splitmix32(mixed_label);
  const b = splitmix32(a);
  const c = splitmix32(b);
  const d = splitmix32(c);
  const stream = new Sfc32Stream(a, b, c, d);
  return stream;
}

export function select_index(stream: DeterministicStream, bound: number): number {
  if (!Number.isSafeInteger(bound) || bound < 1 || bound > UINT32_RANGE) {
    throw new Error("selection bound must be an integer from one through 4294967296");
  }
  const accepted_range = UINT32_RANGE - (UINT32_RANGE % bound);
  let value = stream.next_uint32();
  while (value >= accepted_range) {
    value = stream.next_uint32();
  }
  const index = value % bound;
  return index;
}

export function choose_value<T>(stream: DeterministicStream, values: readonly T[]): T {
  if (values.length === 0) {
    throw new Error("cannot choose from an empty collection");
  }
  const index = select_index(stream, values.length);
  if (index < 0 || index >= values.length) {
    throw new Error("selected index was outside the collection");
  }
  return values[index]!;
}

export function allocate_without_replacement<T>(
  stream: DeterministicStream,
  candidates: readonly T[],
  count: number,
): T[] {
  validate_count(count, candidates.length);
  const remaining = [...candidates];
  const allocation: T[] = [];
  for (let position = 0; position < count; position += 1) {
    const index = select_index(stream, remaining.length);
    if (index < 0 || index >= remaining.length) {
      throw new Error("selected index was outside remaining candidates");
    }
    allocation.push(remaining[index]!);
    remaining.splice(index, 1);
  }
  return allocation;
}

export function sort_public_identifiers(identifiers: readonly string[]): string[] {
  const indexed = identifiers.map((identifier, index) => ({ identifier, index }));
  indexed.sort(compare_public_identifiers);
  const ordered = indexed.map(({ identifier }) => identifier);
  return ordered;
}

function compare_public_identifiers(
  left: { readonly identifier: string; readonly index: number },
  right: { readonly identifier: string; readonly index: number },
): number {
  if (left.identifier < right.identifier) {
    return -1;
  }
  if (left.identifier > right.identifier) {
    return 1;
  }
  const difference = left.index - right.index;
  return difference;
}
