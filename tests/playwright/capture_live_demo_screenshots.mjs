// Thin JavaScript entry point for the typed screenshot corpus runner.

import process from "node:process";

import { main } from "./screenshot_corpus/cli.ts";

try {
  await main(process.argv.slice(2));
} catch (error) {
  console.error(error);
  process.exitCode = 1;
}
