// Thin JavaScript entry point for the typed screenshot corpus runner.

import process from "node:process";

try {
  // ASVS 14.2.4: Playwright API debug can log form credentials before an error is caught.
  // Refuse direct debug runs before importing Playwright; the wrapper clears only its child env.
  if (process.env.DEBUG || process.env.PWDEBUG) {
    throw new Error("Screenshot capture requires DEBUG and PWDEBUG to be unset or empty.");
  }
  const { main } = await import("./screenshot_corpus/cli.ts");
  await main(process.argv.slice(2));
} catch (error) {
  console.error(error);
  process.exitCode = 1;
}
