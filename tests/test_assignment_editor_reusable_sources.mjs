import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";

async function assignmentEditorRepositoryFactory() {
  const result = await build({
    entryPoints: [
      new URL("../src/pages/assignment_editor_repository.ts", import.meta.url).pathname,
    ],
    bundle: true,
    format: "esm",
    loader: { ".css": "text" },
    logLevel: "error",
    platform: "node",
    target: "node22",
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined)
    throw new Error("Assignment editor repository test bundle is unavailable");
  return await import(
    `data:text/javascript;base64,${Buffer.from(output.contents).toString("base64")}`
  );
}

function cursorPage(items, nextCursor) {
  return { items, nextCursor };
}

test("assignment picker composes cursor-complete personal Blueprint and Alpha definition sources", async () => {
  const { createAssignmentEditorRepository } = await assignmentEditorRepositoryFactory();
  const blueprintCursors = [];
  const alphaCursors = [];
  const assignmentCursors = [];
  const client = {
    listProblemCollections: async () => cursorPage([], null),
    listAssignments: async (_course, cursor) => {
      assignmentCursors.push(cursor);
      return cursor === undefined
        ? cursorPage([{ id: "AS-1", title: "Earlier assignment" }], "assignment-next")
        : cursorPage([{ id: "AS-2", title: "Later assignment" }], null);
    },
    getAssignmentWorkspace: async (_course, id) => ({
      id,
      title: `Assignment ${id}`,
      items: [],
      selectionGroups: [],
    }),
    listBlueprints: async (cursor) => {
      blueprintCursors.push(cursor);
      return cursor === undefined
        ? cursorPage([{ reference: "BP-1", title: "Peptide blueprint" }], "blueprint-next")
        : cursorPage([{ reference: "BP-2", title: "Bond blueprint" }], null);
    },
    listAlphaCourses: async (cursor) => {
      alphaCursors.push(cursor);
      return cursor === undefined
        ? cursorPage([{ reference: "AC-1", title: "Alpha sequence" }], "alpha-next")
        : cursorPage([{ reference: "AC-2", title: "Alpha continuation" }], null);
    },
    getAlphaCourse: async (reference) => ({
      alpha: {
        reference,
        title: reference === "AC-1" ? "Alpha sequence" : "Alpha continuation",
        modules: [
          {
            label: "Week one",
            definitions: [{ title: "Structure basics" }, { title: "Bond geometry" }],
          },
        ],
      },
    }),
  };

  const repository = createAssignmentEditorRepository(client);
  const sources = await repository.listProblemPickerSources("CS-1");

  assert.deepEqual(assignmentCursors, [undefined, "assignment-next"]);
  assert.deepEqual(blueprintCursors, [undefined, "blueprint-next"]);
  assert.deepEqual(alphaCursors, [undefined, "alpha-next"]);
  assert.equal(
    sources.some(
      (source) =>
        source.kind === "personalBlueprint" &&
        source.blueprint === "BP-2" &&
        source.label.includes("Bond blueprint"),
    ),
    true,
  );
  assert.equal(
    sources.some(
      (source) =>
        source.kind === "alphaCurriculum" &&
        source.alpha === "AC-2" &&
        source.modulePosition === 1 &&
        source.assignmentPosition === 2 &&
        source.label.includes("Alpha continuation") &&
        source.label.includes("Bond geometry"),
    ),
    true,
  );
});
