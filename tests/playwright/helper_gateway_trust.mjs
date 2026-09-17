// Scoped trust for the current canonical Live Demo gateway only.
import { lstatSync, readFileSync, realpathSync } from "node:fs";
import { X509Certificate } from "node:crypto";
import path from "node:path";

import { REPO_ROOT } from "./repo_root.mjs";

/** @param {string} origin @returns {string[]} */
export function liveDemoChromiumArgs(origin) {
  const workspace = path.join(REPO_ROOT, "local_stack_state/live_demo_browser/workspace");
  const certificatePath = path.join(workspace, "gateway-root.crt");
  const receiptPath = path.join(workspace, "gateway-browser-trust.json");
  for (const file of [certificatePath, receiptPath]) {
    const stat = lstatSync(file);
    if (!stat.isFile() || stat.isSymbolicLink() || (stat.mode & 0o777) !== 0o600)
      throw new Error("Live Demo browser trust requires private owner files");
  }
  const receipt = JSON.parse(readFileSync(receiptPath, "utf8"));
  const expected = new URL(origin);
  if (
    expected.protocol !== "https:" ||
    expected.hostname !== "localhost" ||
    expected.port === "" ||
    receipt.origin !== `${expected.origin}/` ||
    typeof receipt.spki !== "string" ||
    !/^[A-Za-z0-9+/]{43}=$/u.test(receipt.spki)
  )
    throw new Error("Live Demo browser trust does not match the selected HTTPS owner");
  const root = new X509Certificate(readFileSync(certificatePath));
  if (!root.ca || Date.parse(root.validFrom) > Date.now() || Date.parse(root.validTo) <= Date.now())
    throw new Error("Live Demo browser CA is invalid or expired");
  if (
    process.env.NODE_EXTRA_CA_CERTS === undefined ||
    realpathSync(process.env.NODE_EXTRA_CA_CERTS) !== realpathSync(certificatePath)
  )
    throw new Error(
      "Launch browser tests with NODE_EXTRA_CA_CERTS set to the owner's gateway-root.crt",
    );
  // ASVS 12.3.4: Chromium accepts only this gateway's intermediate public key.
  return [`--ignore-certificate-errors-spki-list=${receipt.spki}`];
}
