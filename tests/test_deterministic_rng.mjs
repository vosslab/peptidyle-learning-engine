import assert from "node:assert/strict";
import test from "node:test";

import {
  allocate_without_replacement,
  choose_value,
  create_named_stream,
  select_index,
  sort_public_identifiers,
  validate_master_seed,
} from "./support/deterministic_rng.ts";

function replay_values(master_seed, label, count) {
  const stream = create_named_stream(master_seed, label);
  const values = [];
  for (let index = 0; index < count; index += 1) {
    values.push(stream.next_uint32());
  }
  return values;
}

test("a master seed and label replay fixed uint32 decisions", () => {
  const values = replay_values(42, "learner.alpha", 5);
  assert.deepEqual(values, [1835804778, 1372660255, 1007088323, 974384073, 437026343]);
});

test("named streams remain isolated when another stream consumes decisions", () => {
  const baseline = replay_values(77, "observer.review", 4);
  const unrelated = create_named_stream(77, "learner.answer");
  replay_values(77, "learner.answer", 3);
  unrelated.next_uint32();
  unrelated.next_uint32();
  const replay = replay_values(77, "observer.review", 4);
  assert.deepEqual(replay, baseline);
});

test("seed, label, and selection bounds fail closed", () => {
  assert.throws(() => validate_master_seed(-1), /unsigned 32-bit/);
  assert.throws(() => validate_master_seed(1.5), /unsigned 32-bit/);
  assert.throws(() => create_named_stream(1, "Learner"), /stable lowercase/);
  assert.throws(() => create_named_stream(1, ""), /stable lowercase/);
  const stream = create_named_stream(1, "learner.answer");
  assert.throws(() => select_index(stream, 0), /selection bound/);
  assert.throws(() => select_index(stream, 1.5), /selection bound/);
  assert.throws(() => select_index(stream, 0x1_0000_0001), /selection bound/);
  assert.throws(() => choose_value(stream, []), /empty collection/);
});

test("selection retries rejected tail values and permits the uint32 range bound", () => {
  const scripted_values = [0xffff_ffff, 5];
  let consumption_count = 0;
  const scripted_stream = {
    next_uint32() {
      const value = scripted_values[consumption_count];
      consumption_count += 1;
      return value;
    },
  };
  assert.equal(select_index(scripted_stream, 3), 2);
  assert.equal(consumption_count, 2);

  const maximum_bound_stream = { next_uint32: () => 0xffff_ffff };
  assert.equal(select_index(maximum_bound_stream, 0x1_0000_0000), 0xffff_ffff);
});

test("allocation and choice replay without mutating candidate order", () => {
  const candidates = ["MC", "MA", "FIB", "MATCH"];
  const allocation_stream = create_named_stream(91, "family.allocation");
  const allocation = allocate_without_replacement(allocation_stream, candidates, 3);
  const replay = allocate_without_replacement(
    create_named_stream(91, "family.allocation"),
    candidates,
    3,
  );
  const choice = choose_value(create_named_stream(91, "family.choice"), candidates);
  assert.deepEqual(allocation, replay);
  assert.deepEqual(allocation, ["FIB", "MC", "MATCH"]);
  assert.equal(new Set(allocation).size, 3);
  assert.equal(choice, "FIB");
  assert.deepEqual(candidates, ["MC", "MA", "FIB", "MATCH"]);
  const invalid_allocation = () => {
    return allocate_without_replacement(allocation_stream, candidates, 5);
  };
  assert.throws(invalid_allocation, /allocation/);
});

test("public identifiers sort predictably without mutating the report input", () => {
  const identifiers = ["learner-10", "learner-2", "learner-10", "learner-1"];
  const ordered = sort_public_identifiers(identifiers);
  assert.deepEqual(ordered, ["learner-1", "learner-10", "learner-10", "learner-2"]);
  assert.deepEqual(identifiers, ["learner-10", "learner-2", "learner-10", "learner-1"]);
});
