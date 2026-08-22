import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

import { canvasQtiFixtureArchive } from "./playwright/e2e/qti_fixture_archive.ts";

const fixtureDirectory = "crates/adapters/qti/tests/fixtures/profiles";

function crc32(contents) {
  let value = 0xffffffff;
  for (const byte of contents) {
    value ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      value = (value >>> 1) ^ (value & 1 ? 0xedb88320 : 0);
    }
  }
  return (value ^ 0xffffffff) >>> 0;
}

function expectedEntries() {
  return [
    ["canvas_qti12_questions/assessment_meta.xml", "canvas_assessment_meta.xml"],
    ["canvas_qti12_questions/canvas-1.xml", "canvas_positive_item.xml"],
    ["imsmanifest.xml", "canvas_positive_manifest.xml"],
  ].map(([name, fixture]) => ({
    name,
    contents: readFileSync(resolve(fixtureDirectory, fixture)),
  }));
}

function parseCentralDirectory(archive) {
  const eocd = archive.length - 22;
  assert.equal(archive.readUInt32LE(eocd), 0x06054b50);
  assert.equal(archive.readUInt16LE(eocd + 4), 0);
  assert.equal(archive.readUInt16LE(eocd + 6), 0);
  const count = archive.readUInt16LE(eocd + 10);
  assert.equal(archive.readUInt16LE(eocd + 8), count);
  const size = archive.readUInt32LE(eocd + 12);
  let offset = archive.readUInt32LE(eocd + 16);
  const directoryOffset = offset;
  assert.equal(archive.readUInt16LE(eocd + 20), 0);
  assert.equal(offset + size, eocd);
  const entries = [];
  for (let index = 0; index < count; index += 1) {
    assert.equal(archive.readUInt32LE(offset), 0x02014b50);
    assert.equal(archive.readUInt16LE(offset + 4), 20);
    assert.equal(archive.readUInt16LE(offset + 6), 20);
    assert.equal(archive.readUInt16LE(offset + 8), 0);
    assert.equal(archive.readUInt16LE(offset + 10), 0);
    assert.equal(archive.readUInt16LE(offset + 12), 0);
    assert.equal(archive.readUInt16LE(offset + 14), 0);
    const checksum = archive.readUInt32LE(offset + 16);
    const compressedSize = archive.readUInt32LE(offset + 20);
    const uncompressedSize = archive.readUInt32LE(offset + 24);
    const nameLength = archive.readUInt16LE(offset + 28);
    const extraLength = archive.readUInt16LE(offset + 30);
    const commentLength = archive.readUInt16LE(offset + 32);
    assert.equal(archive.readUInt16LE(offset + 34), 0);
    assert.equal(archive.readUInt16LE(offset + 36), 0);
    assert.equal(archive.readUInt32LE(offset + 38), 0);
    const localOffset = archive.readUInt32LE(offset + 42);
    const name = archive.subarray(offset + 46, offset + 46 + nameLength).toString("ascii");
    entries.push({ checksum, compressedSize, localOffset, name, uncompressedSize });
    offset += 46 + nameLength + extraLength + commentLength;
  }
  assert.equal(offset, eocd);
  return { directoryOffset, entries };
}

function validateLocalEntries(archive, entries, expected, directoryOffset) {
  const localEntries = [...entries].sort((left, right) => left.localOffset - right.localOffset);
  let nextOffset = 0;
  for (const [index, entry] of localEntries.entries()) {
    const expectedEntry = expected[index];
    assert.notEqual(expectedEntry, undefined);
    assert.equal(entry.localOffset, nextOffset);
    assert.equal(archive.readUInt32LE(entry.localOffset), 0x04034b50);
    assert.equal(archive.readUInt16LE(entry.localOffset + 4), 20);
    assert.equal(archive.readUInt16LE(entry.localOffset + 6), 0);
    assert.equal(archive.readUInt16LE(entry.localOffset + 8), 0);
    assert.equal(archive.readUInt16LE(entry.localOffset + 10), 0);
    assert.equal(archive.readUInt16LE(entry.localOffset + 12), 0);
    assert.equal(archive.readUInt32LE(entry.localOffset + 14), entry.checksum);
    assert.equal(archive.readUInt32LE(entry.localOffset + 18), entry.compressedSize);
    assert.equal(archive.readUInt32LE(entry.localOffset + 22), entry.uncompressedSize);
    const nameLength = archive.readUInt16LE(entry.localOffset + 26);
    assert.equal(archive.readUInt16LE(entry.localOffset + 28), 0);
    const nameStart = entry.localOffset + 30;
    const name = archive.subarray(nameStart, nameStart + nameLength).toString("ascii");
    const body = archive.subarray(
      nameStart + nameLength,
      nameStart + nameLength + entry.uncompressedSize,
    );
    assert.equal(name, entry.name);
    assert.equal(name, expectedEntry.name);
    assert.equal(entry.compressedSize, expectedEntry.contents.length);
    assert.equal(entry.uncompressedSize, expectedEntry.contents.length);
    assert.equal(entry.checksum, crc32(expectedEntry.contents));
    assert.deepEqual(body, expectedEntry.contents);
    nextOffset = nameStart + nameLength + entry.uncompressedSize;
  }
  assert.equal(nextOffset, directoryOffset);
}

test("the Canvas fixture archive is deterministic and preserves its tracked package layout", () => {
  const first = canvasQtiFixtureArchive();
  const second = canvasQtiFixtureArchive();
  assert.deepEqual(first, second);
  const expected = expectedEntries();
  const directory = parseCentralDirectory(first);
  assert.deepEqual(
    directory.entries.map((entry) => entry.name),
    expected.map((entry) => entry.name),
  );
  validateLocalEntries(first, directory.entries, expected, directory.directoryOffset);
});
