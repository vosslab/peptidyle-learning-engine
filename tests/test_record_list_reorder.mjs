import assert from "node:assert/strict";

import { reorderedRecordListRows } from "../src/components/record_list/record_list_reorder.tsx";
import { reordered } from "../src/features/blueprint_forks/blueprint_fork_apply_model.ts";

const rows = ["alpha", "bravo", "charlie"];

assert.deepEqual(reorderedRecordListRows(rows, 1, 0), ["bravo", "alpha", "charlie"]);
assert.deepEqual(reorderedRecordListRows(rows, 0, 2), ["bravo", "charlie", "alpha"]);
assert.deepEqual(reorderedRecordListRows(rows, -1, 1), rows);
assert.deepEqual(reorderedRecordListRows(rows, 1, 3), rows);
assert.deepEqual(reordered(rows, 2, -1), ["alpha", "charlie", "bravo"]);
assert.deepEqual(reordered(rows, 0, -1), rows);

process.stdout.write("RecordList pure reorder helper checks passed.\n");
