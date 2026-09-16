// local_demo_authenticator.ts - private child-process handoff, never a browser seed.

import { spawn } from "node:child_process";

let lastCounter = -1;

export async function localDemoAuthenticationCode(setupFile: string): Promise<string> {
  // ASVS 1.2.5, 14.2.4: no shell, argv/environment credentials, or child diagnostics.
  return new Promise((resolve, reject) => {
    const child = spawn(
      "python3",
      ["-m", "devel.local_demo_totp", setupFile, "--after-counter", String(lastCounter)],
      { stdio: ["ignore", "pipe", "ignore"] },
    );
    let output = "";
    const timer = setTimeout(() => child.kill(), 40_000);
    child.stdout.on("data", (chunk: Buffer) => {
      if (output.length > 128) return;
      output += chunk.toString("ascii");
      if (output.length > 128) child.kill();
    });
    child.on("error", () => {
      clearTimeout(timer);
      reject(new Error("Local demonstration authenticator unavailable."));
    });
    child.on("close", (status) => {
      clearTimeout(timer);
      try {
        if (status !== 0 || output.length > 128) throw new Error();
        const decoded: unknown = JSON.parse(output);
        if (typeof decoded !== "object" || decoded === null) throw new Error();
        const value = decoded as Record<string, unknown>;
        if (
          Object.keys(value).sort().join(",") !== "code,counter" ||
          typeof value["code"] !== "string" ||
          !/^[0-9]{6}$/u.test(value["code"]) ||
          typeof value["counter"] !== "number" ||
          !Number.isSafeInteger(value["counter"]) ||
          value["counter"] <= lastCounter
        )
          throw new Error();
        lastCounter = value["counter"];
        resolve(value["code"]);
      } catch {
        reject(new Error("Local demonstration authenticator unavailable."));
      }
    });
  });
}
