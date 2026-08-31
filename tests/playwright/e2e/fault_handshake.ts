// Private owner-child protocol for one lifecycle-controlled browser recovery journey.
import { closeSync, fsyncSync, lstatSync, openSync, writeSync } from "node:fs";
import { createConnection, type Socket } from "node:net";
import { basename, dirname, join } from "node:path";

const PHASES = {
  gateway_submit_outage: {
    child: ["response_selected", "network_recovery_visible", "completed"],
    owner: ["gateway_stopped", "gateway_recovered"],
  },
} as const;
const MAXIMUM_MESSAGE_BYTES = 256;
const MAXIMUM_SOCKET_PATH_BYTES = 100;
const SOCKET_TIMEOUT_MS = 600_000;
const SOCKET_NAME = /^fault-[0-9a-f]{24}\.sock$/u;
const TOKEN = /^[A-Za-z0-9_-]{43}$/u;
export type FaultTransition = keyof typeof PHASES;
type ChildPhase<T extends FaultTransition> = (typeof PHASES)[T]["child"][number];
type OwnerPhase<T extends FaultTransition> = (typeof PHASES)[T]["owner"][number];

export interface FaultHandshake<T extends FaultTransition = "gateway_submit_outage"> {
  notify(phase: ChildPhase<T>): void;
  waitFor(phase: OwnerPhase<T>): Promise<void>;
  close(): void;
}

function required(environment: NodeJS.ProcessEnv, name: string): string {
  const value = environment[name];
  if (value === undefined || value === "") throw new Error(`fault protocol requires ${name}`);
  return value;
}

function invokingUserOwns(uid: number): boolean {
  return typeof process.getuid !== "function" || uid === process.getuid();
}

function privateDirectory(path: string): void {
  const details = lstatSync(path);
  if (!details.isDirectory() || details.isSymbolicLink() || (details.mode & 0o777) !== 0o700) {
    throw new Error("fault protocol directory must be owner-private");
  }
  if (!invokingUserOwns(details.uid)) {
    throw new Error("fault protocol directory must be owned by the invoking user");
  }
}

function token(value: string): string {
  if (!TOKEN.test(value)) throw new Error("fault protocol token is invalid");
  return value;
}

function privateSocket(value: string): { directory: string; path: string } {
  if (!value.startsWith("/") || Buffer.byteLength(value, "utf8") > MAXIMUM_SOCKET_PATH_BYTES) {
    throw new Error("fault protocol socket path is invalid");
  }
  if (!SOCKET_NAME.test(basename(value))) {
    throw new Error("fault protocol socket name is invalid");
  }
  const directory = dirname(value);
  privateDirectory(directory);
  const details = lstatSync(value);
  if (!details.isSocket() || details.isSymbolicLink() || (details.mode & 0o777) !== 0o600) {
    throw new Error("fault protocol socket must be owner-private");
  }
  if (!invokingUserOwns(details.uid)) {
    throw new Error("fault protocol socket must be owned by the invoking user");
  }
  return { directory, path: value };
}

function message(phase: string, tokenValue: string): string {
  return JSON.stringify({ kind: "phase", phase, token: tokenValue, version: 1 });
}

function authentication(scenarioId: string, namespace: string, tokenValue: string): string {
  return JSON.stringify({ kind: "hello", namespace, scenarioId, token: tokenValue, version: 1 });
}

function accepted(tokenValue: string): string {
  return JSON.stringify({ kind: "accepted", token: tokenValue, version: 1 });
}

function readExact(channel: Socket, expected: string): Promise<void> {
  return new Promise((resolve, reject) => {
    let buffered = Buffer.alloc(0);
    let finished = false;

    function finish(error: Error | undefined): void {
      if (finished) return;
      finished = true;
      channel.off("data", onData);
      channel.off("error", onError);
      channel.off("close", onClose);
      channel.off("timeout", onTimeout);
      if (error === undefined) resolve();
      else reject(error);
    }

    function onData(chunk: Buffer): void {
      if (chunk.length > MAXIMUM_MESSAGE_BYTES - buffered.length) {
        finish(new Error("fault protocol socket message is too large"));
        return;
      }
      buffered = Buffer.concat([buffered, chunk]);
      const boundary = buffered.indexOf(0x0a);
      if (boundary < 0) return;
      if (boundary !== buffered.length - 1) {
        finish(new Error("fault protocol socket message has trailing data"));
        return;
      }
      if (buffered.toString("ascii") !== `${expected}\n`) {
        finish(new Error("fault protocol socket message identity is invalid"));
        return;
      }
      finish(undefined);
    }

    function onError(error: Error): void {
      finish(error);
    }

    function onClose(): void {
      finish(new Error("fault protocol socket closed"));
    }

    function onTimeout(): void {
      finish(new Error("fault protocol socket timed out"));
    }

    channel.on("data", onData);
    channel.once("error", onError);
    channel.once("close", onClose);
    channel.once("timeout", onTimeout);
  });
}

async function connected(path: string): Promise<Socket> {
  const channel = createConnection(path);
  channel.setTimeout(SOCKET_TIMEOUT_MS);
  await new Promise<void>((resolve, reject) => {
    function onConnect(): void {
      channel.off("error", onError);
      channel.off("timeout", onTimeout);
      resolve();
    }

    function onError(error: Error): void {
      channel.off("connect", onConnect);
      channel.off("timeout", onTimeout);
      reject(error);
    }

    function onTimeout(): void {
      channel.off("connect", onConnect);
      channel.off("error", onError);
      reject(new Error("fault protocol socket connection timed out"));
    }

    channel.once("connect", onConnect);
    channel.once("error", onError);
    channel.once("timeout", onTimeout);
  });
  return channel;
}

function writePrivateMarker(
  directory: string,
  phase: string,
  scenarioId: string,
  namespace: string,
  tokenValue: string,
): void {
  const marker = JSON.stringify({
    kind: "phase",
    namespace,
    phase,
    scenarioId,
    token: tokenValue,
    version: 1,
  });
  const descriptor = openSync(join(directory, `fault-${phase}.json`), "wx", 0o600);
  try {
    const bytes = Buffer.from(marker, "ascii");
    let written = 0;
    while (written < bytes.length) written += writeSync(descriptor, bytes, written);
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

export function faultHandshakeFromEnvironment(
  environment: NodeJS.ProcessEnv,
  scenarioId: string,
  namespace: string,
): Promise<FaultHandshake<"gateway_submit_outage">>;
export function faultHandshakeFromEnvironment<T extends FaultTransition>(
  environment: NodeJS.ProcessEnv,
  scenarioId: string,
  namespace: string,
  transition: T,
): Promise<FaultHandshake<T>>;
export async function faultHandshakeFromEnvironment(
  environment: NodeJS.ProcessEnv,
  scenarioId: string,
  namespace: string,
  transition: FaultTransition = "gateway_submit_outage",
): Promise<FaultHandshake<FaultTransition>> {
  const tokenValue = token(required(environment, "PLE_BROWSER_SUITE_FAULT_TOKEN"));
  const socket = privateSocket(required(environment, "PLE_BROWSER_SUITE_FAULT_SOCKET_PATH"));
  const channel = await connected(socket.path);
  channel.write(`${authentication(scenarioId, namespace, tokenValue)}\n`);
  await readExact(channel, accepted(tokenValue));
  const phases = PHASES[transition];
  let childIndex = 0;
  let ownerIndex = 0;

  return {
    notify(phase: ChildPhase<FaultTransition>): void {
      if (phases.child[childIndex] !== phase) {
        throw new Error("fault protocol child phase order is invalid");
      }
      childIndex += 1;
      writePrivateMarker(socket.directory, phase, scenarioId, namespace, tokenValue);
      channel.write(`${message(phase, tokenValue)}\n`);
    },
    async waitFor(phase: OwnerPhase<FaultTransition>): Promise<void> {
      if (phases.owner[ownerIndex] !== phase) {
        throw new Error("fault protocol owner phase order is invalid");
      }
      ownerIndex += 1;
      await readExact(channel, message(phase, tokenValue));
    },
    close(): void {
      channel.destroy();
    },
  };
}
