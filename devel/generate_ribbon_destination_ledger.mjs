// generate_ribbon_destination_ledger.mjs - render the machine-owned Ribbon capability evidence.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { CAPABILITY_REGISTRY, ribbonAvailability } from "../src/ribbon/capability_registry.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";
import { getRepoRoot } from "./repo_root.mjs";

export const generatedStart = "<!-- BEGIN GENERATED RIBBON DESTINATION LEDGER -->";
export const generatedEnd = "<!-- END GENERATED RIBBON DESTINATION LEDGER -->";
const productRoles = ["instructor", "student", "sysadmin"];
const resolvedAllowedRelationship = { kind: "resolved", allowed: true };
const catalog = [...TAB_CATALOG, ...RIBBON_TASK_CATALOG];

function ledgerPathFor(repoRoot) {
  return path.join(repoRoot, "docs/ux/RIBBON_DESTINATION_LEDGER.md");
}

// Keep one Markdown table cell per catalog value; text cannot add delimiters or line breaks.
export function escapeMarkdownTable(value) {
  return String(value)
    .replaceAll("\\", "\\\\")
    .replaceAll("|", "\\|")
    .replaceAll("\r", " ")
    .replaceAll("\n", " ");
}

function fixedRelativePath(sourcePath) {
  if (typeof sourcePath !== "string" || !/^[a-z0-9][a-z0-9_./-]*$/u.test(sourcePath)) {
    throw new Error(`Ribbon ledger evidence path is unsafe: ${String(sourcePath)}.`);
  }
  const repoRoot = getRepoRoot();
  const resolvedPath = path.resolve(repoRoot, sourcePath);
  if (!resolvedPath.startsWith(`${repoRoot}${path.sep}`) || !fs.statSync(resolvedPath).isFile()) {
    throw new Error(`Ribbon ledger evidence file is unavailable: ${sourcePath}.`);
  }
  return `../../${sourcePath}`;
}

export function evidenceLink(evidence) {
  const [sourcePath, symbol] = evidence.split("::", 2);
  if (sourcePath === undefined || sourcePath.length === 0) {
    throw new Error(`Ribbon ledger evidence is malformed: ${String(evidence)}.`);
  }
  const sourceLink = `[${escapeMarkdownTable(sourcePath)}](${fixedRelativePath(sourcePath)})`;
  if (symbol === undefined) return sourceLink;
  return `${sourceLink}::${escapeMarkdownTable(symbol)}`;
}

function clientMethod(entry) {
  if (entry.capability.kind === "backed") {
    return escapeMarkdownTable(entry.capability.clientMethod);
  }
  const clientEvidence = entry.capability.evidence.find((item) =>
    /^src\/api\/application_api\.tsx::ApiClient\.[A-Za-z0-9_]+$/u.test(item),
  );
  if (clientEvidence === undefined) return "No declared client method.";
  const method = clientEvidence.split("::", 2)[1];
  return `${escapeMarkdownTable(method)} (${evidenceLink(clientEvidence)})`;
}

function routeIdentity(control, entry) {
  if (control.destination.kind === "future") {
    const futureId = escapeMarkdownTable(control.destination.futureId);
    const catalogLink = evidenceLink("src/ribbon/ribbon_catalog.ts");
    return `Future identity: ${futureId} (no route) (${catalogLink})`;
  }
  if (entry.routeId === undefined) {
    throw new Error(`Ribbon ledger route destination ${entry.id} has no route ID.`);
  }
  return `${escapeMarkdownTable(entry.routeId)} (${evidenceLink("src/route_contract.ts")})`;
}

function handlerEvidence(entry) {
  if (entry.capability.kind === "backed") {
    const serverEvidence = entry.capability.serverEvidence;
    const detail =
      serverEvidence.kind === "registeredHandler"
        ? `Registered handler: ${serverEvidence.handler}`
        : `No server call: ${serverEvidence.justification}`;
    const evidenceLinks = entry.capability.evidence.map(evidenceLink).join("<br>");
    return `${escapeMarkdownTable(detail)}<br>${evidenceLinks}`;
  }
  const reason = escapeMarkdownTable(entry.capability.reason);
  const evidenceLinks = entry.capability.evidence.map(evidenceLink).join("<br>");
  return `No complete handler: ${reason}<br>${evidenceLinks}`;
}

function availabilityByRole(entry) {
  return productRoles
    .map((role) => `${role}: ${ribbonAvailability(entry, role, resolvedAllowedRelationship)}`)
    .join("<br>");
}

function renderMarkdownTable(header, rows) {
  const widths = header.map((heading, columnIndex) =>
    Math.max(heading.length, ...rows.map((row) => row[columnIndex].length)),
  );
  const renderRow = (cells) =>
    `| ${cells.map((cell, columnIndex) => cell.padEnd(widths[columnIndex])).join(" | ")} |`;
  return [
    renderRow(header),
    `| ${widths.map((width) => "-".repeat(width)).join(" | ")} |`,
    ...rows.map(renderRow),
  ];
}

export function renderGeneratedLedger() {
  const catalogIds = catalog.map((control) => control.id);
  if (new Set(catalogIds).size !== catalogIds.length) {
    throw new Error("Ribbon ledger catalog contains duplicate destination IDs.");
  }
  const rows = catalog.map((control) => {
    const entry = CAPABILITY_REGISTRY[control.id];
    if (entry === undefined)
      throw new Error(`Ribbon ledger has no registry entry for ${control.id}.`);
    return [
      escapeMarkdownTable(control.label),
      routeIdentity(control, entry),
      clientMethod(entry),
      handlerEvidence(entry),
      availabilityByRole(entry),
    ];
  });
  const table = renderMarkdownTable(
    [
      "Canonical label",
      "Route id or future identity",
      "Client method",
      "Backing handler evidence",
      "Derived Ribbon Availability",
    ],
    rows,
  );
  return [
    generatedStart,
    "",
    "## Generated capability evidence",
    "",
    [
      "This section is machine-owned. Run ",
      "`node --import tsx devel/generate_ribbon_destination_ledger.mjs`",
      "\nafter changing the catalog or capability registry; do not edit the table by hand.",
    ].join(""),
    "",
    [
      "Ribbon Availability is projected with every Product Role and a resolved, allowed ",
      "relationship.",
    ].join(""),
    "This documents the role ceiling before relationship denial: `ribbonAvailability(entry, role,",
    '{ kind: "resolved", allowed: true })`. Runtime authorization remains at the route and server',
    "boundaries.",
    "",
    ...table,
    "",
    generatedEnd,
  ].join("\n");
}

function sectionRange(documentText) {
  const start = documentText.indexOf(generatedStart);
  const end = documentText.indexOf(generatedEnd);
  const startCount = documentText.split(generatedStart).length - 1;
  const endCount = documentText.split(generatedEnd).length - 1;
  if (startCount !== 1 || endCount !== 1 || start === -1 || end === -1 || end < start) {
    throw new Error("Ribbon destination ledger markers are missing or out of order.");
  }
  return { start, end: end + generatedEnd.length };
}

export function replaceGeneratedSection(documentText, generatedSection) {
  const { start, end } = sectionRange(documentText);
  return `${documentText.slice(0, start)}${generatedSection}${documentText.slice(end)}`;
}

function checkModeFromArgs(args) {
  if (args.length === 0) return false;
  if (args.length === 1 && args[0] === "--check") return true;
  throw new Error(
    "Usage: node --import tsx devel/generate_ribbon_destination_ledger.mjs [--check]",
  );
}

export function main(args = process.argv.slice(2)) {
  const checkMode = checkModeFromArgs(args);
  const ledgerPath = ledgerPathFor(getRepoRoot());
  if (!fs.existsSync(ledgerPath)) {
    throw new Error("Ribbon destination ledger is missing.");
  }
  const committedDocument = fs.readFileSync(ledgerPath, "utf8");
  const generatedSection = renderGeneratedLedger();
  const expectedDocument = replaceGeneratedSection(committedDocument, generatedSection);
  if (checkMode) {
    if (committedDocument !== expectedDocument) {
      throw new Error("Ribbon destination ledger generated section is stale. Run the generator.");
    }
    console.log("Ribbon destination ledger generated section is current.");
    return;
  }
  fs.writeFileSync(ledgerPath, expectedDocument);
  console.log("Wrote docs/ux/RIBBON_DESTINATION_LEDGER.md.");
}

if (
  process.argv[1] !== undefined &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main();
}
