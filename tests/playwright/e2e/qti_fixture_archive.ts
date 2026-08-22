// Deterministic, stored ZIP archive assembled from the tracked Canvas QTI fixture corpus.
import { Buffer } from "node:buffer";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

interface ArchiveEntry {
  readonly name: string;
  readonly contents: Buffer;
  readonly checksum: number;
  readonly offset: number;
}

const archiveRoot = new URL(
  "../../../crates/adapters/qti/tests/fixtures/profiles/",
  import.meta.url,
);

function fixture(name: string): Buffer {
  return readFileSync(fileURLToPath(new URL(name, archiveRoot)));
}

function crc32(contents: Buffer): number {
  let value = 0xffffffff;
  for (const byte of contents) {
    value ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      value = (value >>> 1) ^ (value & 1 ? 0xedb88320 : 0);
    }
  }
  return (value ^ 0xffffffff) >>> 0;
}

function header(size: number): Buffer {
  return Buffer.alloc(size);
}

function localHeader(entry: ArchiveEntry): Buffer {
  const name = Buffer.from(entry.name, "ascii");
  const result = header(30);
  result.writeUInt32LE(0x04034b50, 0);
  result.writeUInt16LE(20, 4);
  result.writeUInt32LE(entry.checksum, 14);
  result.writeUInt32LE(entry.contents.length, 18);
  result.writeUInt32LE(entry.contents.length, 22);
  result.writeUInt16LE(name.length, 26);
  return Buffer.concat([result, name, entry.contents]);
}

function centralDirectoryEntry(entry: ArchiveEntry): Buffer {
  const name = Buffer.from(entry.name, "ascii");
  const result = header(46);
  result.writeUInt32LE(0x02014b50, 0);
  result.writeUInt16LE(20, 4);
  result.writeUInt16LE(20, 6);
  result.writeUInt32LE(entry.checksum, 16);
  result.writeUInt32LE(entry.contents.length, 20);
  result.writeUInt32LE(entry.contents.length, 24);
  result.writeUInt16LE(name.length, 28);
  result.writeUInt32LE(entry.offset, 42);
  return Buffer.concat([result, name]);
}

function endOfCentralDirectory(entryCount: number, size: number, offset: number): Buffer {
  const result = header(22);
  result.writeUInt32LE(0x06054b50, 0);
  result.writeUInt16LE(entryCount, 8);
  result.writeUInt16LE(entryCount, 10);
  result.writeUInt32LE(size, 12);
  result.writeUInt32LE(offset, 16);
  return result;
}

export function canvasQtiFixtureArchive(): Buffer {
  const sources = [
    ["canvas_qti12_questions/assessment_meta.xml", fixture("canvas_assessment_meta.xml")],
    ["canvas_qti12_questions/canvas-1.xml", fixture("canvas_positive_item.xml")],
    ["imsmanifest.xml", fixture("canvas_positive_manifest.xml")],
  ] as const;
  let offset = 0;
  const entries = sources.map(([name, contents]) => {
    const entry = { name, contents, checksum: crc32(contents), offset };
    offset += localHeader(entry).length;
    return entry;
  });
  const locals = entries.map(localHeader);
  const directory = Buffer.concat(entries.map(centralDirectoryEntry));
  return Buffer.concat([
    ...locals,
    directory,
    endOfCentralDirectory(entries.length, directory.length, offset),
  ]);
}
