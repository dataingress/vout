import { createServer } from "node:net";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { expect } from "bun:test";
import {
  SSMClient,
  type SSMClientConfig,
} from "@aws-sdk/client-ssm";

export const credentials = {
  accessKeyId: "AKIAVOUTAMZAPI0001",
  secretAccessKey: "vout-local-secret-key",
};

export const region = "local";

const keyringKeyVersion = "amz_api_tests";

export type TestServer = {
  endpoint: string;
  ssm: SSMClient;
  stop: () => Promise<void>;
};

const repoRoot = resolve(import.meta.dir, "../../..");
const testTargetDir = join(repoRoot, "target", "amz-api-tests");

function cargoEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    VOUT_KEYRING_KEY_VERSION: keyringKeyVersion,
  };
}

async function unusedPort(): Promise<number> {
  return await new Promise((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") {
          resolvePort(address.port);
        } else {
          reject(new Error("Failed to allocate an unused port"));
        }
      });
    });
  });
}

async function waitUntilReady(endpoint: string, process: Bun.Subprocess): Promise<void> {
  const deadline = Date.now() + 30_000;
  let lastError: unknown;

  while (Date.now() < deadline) {
    if (process.exitCode !== null) {
      throw new Error(`vout exited before becoming ready with code ${process.exitCode}`);
    }

    try {
      const response = await fetch(`${endpoint}/ready`);
      if (response.ok) {
        const body = await response.json() as { ready?: boolean };
        if (body.ready === true) {
          return;
        }
      }
    } catch (error) {
      lastError = error;
    }

    await Bun.sleep(100);
  }

  throw new Error(`Timed out waiting for vout /ready: ${String(lastError)}`);
}

async function buildDefaultBinary(): Promise<string> {
  const configured = Bun.env.VOUT_TEST_BINARY;
  if (configured) {
    return configured;
  }

  const clearKeyring = Bun.spawnSync(
    ["cargo", "run", "-p", "clear-keyring", "--target-dir", testTargetDir],
    {
      cwd: repoRoot,
      env: cargoEnv(),
      stdout: "inherit",
      stderr: "inherit",
    },
  );

  if (!clearKeyring.success) {
    throw new Error(`cargo run -p clear-keyring failed with code ${clearKeyring.exitCode}`);
  }

  const build = Bun.spawnSync(["cargo", "build", "-p", "vout", "--target-dir", testTargetDir], {
    cwd: repoRoot,
    env: cargoEnv(),
    stdout: "inherit",
    stderr: "inherit",
  });

  if (!build.success) {
    throw new Error(`cargo build -p vout failed with code ${build.exitCode}`);
  }

  return join(testTargetDir, "debug", process.platform === "win32" ? "vout.exe" : "vout");
}

async function drainLogs(stream: ReadableStream<Uint8Array> | null, lines: string[]) {
  if (!stream) return;

  const decoder = new TextDecoder();
  for await (const chunk of stream) {
    const text = decoder.decode(chunk);
    lines.push(text);
    if (Bun.env.VOUT_TEST_LOGS === "1") {
      process.stderr.write(text);
    }
    if (lines.length > 100) {
      lines.splice(0, lines.length - 100);
    }
  }
}

export async function startVout(): Promise<TestServer> {
  const port = await unusedPort();
  const endpoint = `http://127.0.0.1:${port}`;
  const workdir = await mkdtemp(join(tmpdir(), "vout-amz-api-tests-"));
  const dbPath = join(workdir, "vout.sqlite");
  const configPath = join(workdir, "vout.toml");
  const binary = await buildDefaultBinary();

  await writeFile(configPath, `
db_dsn = "sqlite://${dbPath}?mode=rwc"
amz_error_on_unsupported = true

[first_user]
access_key = "${credentials.accessKeyId}"
secret_key = "${credentials.secretAccessKey}"

[server]
listen_address = "127.0.0.1:${port}"
concurrent = 16
`);

  const process = Bun.spawn([binary, "--config", configPath], {
    cwd: repoRoot,
    env: cargoEnv(),
    stdout: "pipe",
    stderr: "pipe",
  });
  const logs: string[] = [];

  void drainLogs(process.stdout, logs);
  void drainLogs(process.stderr, logs);

  try {
    await waitUntilReady(endpoint, process);
  } catch (error) {
    process.kill();
    await process.exited.catch(() => undefined);
    await rm(workdir, { recursive: true, force: true });
    throw new Error(`${(error as Error).message}\n\nvout output:\n${logs.join("")}`);
  }

  const clientConfig: SSMClientConfig = {
    endpoint,
    region,
    credentials,
  };

  return {
    endpoint,
    ssm: new SSMClient(clientConfig),
    stop: async () => {
      process.kill();
      await process.exited.catch(() => undefined);
      await rm(workdir, { recursive: true, force: true });
    },
  };
}

export async function expectAwsError(
  operation: Promise<unknown>,
  name: string,
): Promise<Error & { name: string }> {
  try {
    await operation;
  } catch (error) {
    const awsError = error as Error & { name: string };
    expect(awsError.name).toBe(name);
    return awsError;
  }

  throw new Error(`Expected AWS error ${name}`);
}
