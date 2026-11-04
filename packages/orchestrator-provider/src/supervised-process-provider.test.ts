import { EventEmitter, once } from "node:events";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import path from "node:path";
import { PassThrough } from "node:stream";
import { describe, expect, it } from "vitest";
import {
  APPLICATION_CONTRACT_VERSION,
  type ApplicationRequestV01,
} from "@vua/contracts";
import {
  providerEnvironment,
  SupervisedProcessProviderV01,
  type ProviderProcessFactoryV01,
} from "./supervised-process-provider.js";

class FakeProviderProcess extends EventEmitter {
  readonly stdin = new PassThrough();
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  readonly stdio = [this.stdin, this.stdout, this.stderr];
  killed = false;
  shutdownAfterExit = false;
  #buffer = "";
  readonly #handshakePayload: unknown;

  constructor(handshakePayload: unknown = {
    contractVersion: "0.1",
    supportedContractVersions: ["0.1"],
    providerBuildId: "fake-build",
    providerInstanceId: "fake-instance",
    downloadIngest: false,
  }) {
    super();
    this.#handshakePayload = handshakePayload;
    this.stdin.on("data", (chunk: Buffer) => {
      this.#buffer += chunk.toString("utf8");
      let newline = this.#buffer.indexOf("\n");
      while (newline >= 0) {
        const line = this.#buffer.slice(0, newline);
        this.#buffer = this.#buffer.slice(newline + 1);
        this.#handle(JSON.parse(line) as Record<string, unknown>);
        newline = this.#buffer.indexOf("\n");
      }
    });
  }

  kill(): boolean {
    this.killed = true;
    queueMicrotask(() => this.finish(0));
    return true;
  }

  crash(): void {
    this.finish(23);
  }

  finish(code: number): void {
    this.emit("exit", code, null);
    this.emit("close", code, null);
  }

  #handle(frame: Record<string, unknown>): void {
    const frameId = frame.frameId as string;
    const kind = frame.kind as string;
    if (kind === "handshake") {
      this.#respond(frameId, this.#handshakePayload);
      return;
    }
    if (kind === "prepare_shutdown" || kind === "continue_shutdown") {
      const respond = () => this.#respond(frameId, {
        contractVersion: "0.1",
        outcome: "safe_to_stop",
        blockingTasks: [],
      });
      if (this.shutdownAfterExit) {
        this.emit("exit", 0, null);
        queueMicrotask(() => { respond(); this.emit("close", 0, null); });
      } else {
        respond();
        queueMicrotask(() => this.finish(0));
      }
      return;
    }
    const request = frame.payload as ApplicationRequestV01;
    this.#respond(frameId, {
      contractVersion: "0.1",
      requestId: request.requestId,
      ok: true,
      value: {
        contractVersion: "0.1",
        taskId: "task-recovered",
        revision: 4,
        correlationId: request.correlationId,
        state: "running",
        cancellationRequested: false,
        recoveryDisposition: "inspect_required",
        updatedAt: "2026-09-02T00:00:00.000Z",
      },
    });
    this.stdout.write(`${JSON.stringify({
      frameVersion: "0.1",
      frameId: "event-1",
      kind: "event",
      payload: {
        contractVersion: "0.1",
        eventId: "event-1",
        taskId: "task-recovered",
        revision: 4,
        occurredAt: "2026-09-02T00:00:00.000Z",
        correlationId: request.correlationId,
        kind: "task.stateChanged",
        state: "running",
        payload: {},
      },
    })}\n`);
  }

  #respond(frameId: string, payload: unknown): void {
    this.stdout.write(`${JSON.stringify({
      frameVersion: "0.1",
      frameId,
      kind: "response",
      payload,
    })}\n`);
  }
}

function harness(handshakePayload?: unknown): {
  readonly provider: SupervisedProcessProviderV01;
  readonly processes: FakeProviderProcess[];
} {
  const processes: FakeProviderProcess[] = [];
  const factory: ProviderProcessFactoryV01 = () => {
    const process = new FakeProviderProcess(handshakePayload);
    processes.push(process);
    return process as never;
  };
  return {
    provider: new SupervisedProcessProviderV01({
      executablePath: path.resolve("fake-provider.exe"),
      databasePath: path.resolve("fake-provider.db"),
    }, factory),
    processes,
  };
}

function getRequest(): ApplicationRequestV01 {
  return {
    contractVersion: APPLICATION_CONTRACT_VERSION,
    requestId: "request-get",
    correlationId: "correlation-get",
    kind: "query",
    method: "task.get",
    params: { taskId: "task-recovered" },
  };
}

describe("supervised process Provider v0.1", () => {
  it("drains the final shutdown response after native exit before settling pending calls", async () => {
    let child: ChildProcessWithoutNullStreams;
    const provider = new SupervisedProcessProviderV01({ executablePath: process.execPath, databasePath: path.resolve("unused.db") }, () => {
      child = spawn(process.execPath, ["-e", `
        const readline = require('node:readline');
        readline.createInterface({ input: process.stdin }).on('line', line => {
          const frame = JSON.parse(line);
          const payload = frame.kind === 'handshake'
            ? { contractVersion: '0.1', supportedContractVersions: ['0.1'], providerBuildId: 'stream-order', providerInstanceId: 'isolated', downloadIngest: false }
            : { contractVersion: '0.1', outcome: 'safe_to_stop', blockingTasks: [] };
          process.stdout.write(JSON.stringify({ frameVersion: '0.1', frameId: frame.frameId, kind: 'response', payload }) + '\\n', () => {
            if (frame.kind !== 'handshake') process.exit(0);
          });
        });
      `], { windowsHide: true, stdio: "pipe" });
      return child;
    });
    try {
      await provider.start();
      // Force the documented exit-before-stdio-close ordering with a real OS
      // process, without depending on CI scheduling or a compiled Rust binary.
      child!.stdout.pause();
      const exited = once(child!, "exit");
      const shutdown = provider.prepareShutdown({ timeoutMs: 1_000 }).then(value => value, error => ({ error: String(error) }));
      await exited;
      const streamsDrained = provider.waitForExit(1_000);
      child!.stdout.resume();
      await expect(shutdown).resolves.toMatchObject({ outcome: "safe_to_stop" });
      await expect(streamsDrained).resolves.toBe(true);
      expect(provider.status()).toMatchObject({ state: "stopped", acceptingCalls: false });
    } finally {
      child!?.stdout.resume();
      child!?.kill();
    }
  });
  it("publishes failed rather than staying starting when process creation throws", async () => {
    const provider = new SupervisedProcessProviderV01({ executablePath: path.resolve("absent.exe"), databasePath: path.resolve("absent.db") }, () => { throw new Error("spawn refused"); });
    const states: string[] = [];
    const unsubscribe = provider.subscribeStatus(status => states.push(status.state));
    await expect(provider.start()).rejects.toThrow("spawn refused");
    expect(provider.status().state).toBe("failed");
    expect(provider.status().acceptingCalls).toBe(false);
    expect(states).toEqual(["starting", "failed"]);
    unsubscribe();
  });
  it("keeps admission closed through handshake and forwards responses and events", async () => {
    const { provider } = harness();
    expect(provider.status()).toMatchObject({ state: "stopped", acceptingCalls: false });
    await expect(provider.start()).resolves.toMatchObject({ providerInstanceId: "fake-instance" });
    expect(provider.status()).toMatchObject({ state: "ready", acceptingCalls: true });
    const events: string[] = [];
    provider.subscribe((event) => events.push(event.kind));
    await expect(provider.invoke(getRequest())).resolves.toMatchObject({
      ok: true,
      value: { recoveryDisposition: "inspect_required" },
    });
    expect(events).toEqual(["task.stateChanged"]);
    await expect(provider.prepareShutdown({ timeoutMs: 1_000 }))
      .resolves.toMatchObject({ outcome: "safe_to_stop" });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(provider.status()).toMatchObject({ state: "stopped", acceptingCalls: false });
  });

  it("drains the final shutdown response when exit arrives before stdout", async () => {
    const { provider, processes } = harness();
    await provider.start();
    processes[0]!.shutdownAfterExit = true;
    const shutdown = provider.prepareShutdown({ timeoutMs: 1_000 });
    expect(provider.status().acceptingCalls).toBe(false);
    await expect(shutdown).resolves.toMatchObject({ outcome: "safe_to_stop" });
    await expect(provider.waitForExit(1_000)).resolves.toBe(true);
    expect(provider.status()).toMatchObject({ state: "stopped", acceptingCalls: false });
  });

  it("rejects an unanswered request after an unexpected exit closes its streams", async () => {
    const { provider, processes } = harness();
    await provider.start();
    processes[0]!.stdin.removeAllListeners("data");
    const request = provider.invoke(getRequest());
    processes[0]!.crash();
    await expect(request).rejects.toThrow("Provider exited");
    expect(provider.status()).toMatchObject({ state: "failed", acceptingCalls: false });
  });

  it("reports an unexpected process exit and permits an explicit restart", async () => {
    const { provider, processes } = harness();
    await provider.start();
    processes[0]?.crash();
    expect(provider.status()).toMatchObject({ state: "failed", acceptingCalls: false });
    await expect(provider.start()).resolves.toMatchObject({ providerBuildId: "fake-build" });
    expect(processes).toHaveLength(2);
    await provider.prepareShutdown({ timeoutMs: 1_000 });
  });

  it("keeps a rejected handshake in the failed state after terminating the child", async () => {
    const { provider } = harness({ contractVersion: "unknown" });
    await expect(provider.start()).rejects.toThrow("invalid handshake");
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(provider.status()).toMatchObject({ state: "failed", acceptingCalls: false });
  });

  it("waits for a failed handshake's process to exit before reporting it gone", async () => {
    const { provider, processes } = harness({ contractVersion: "unknown" });
    await expect(provider.start()).rejects.toThrow("invalid handshake");
    await expect(provider.waitForExit(1_000)).resolves.toBe(true);
    expect(processes[0]?.killed).toBe(true);
  });

  it("reports a timeout while the Provider process is still running", async () => {
    const { provider } = harness();
    await provider.start();
    await expect(provider.waitForExit(20)).resolves.toBe(false);
    await provider.prepareShutdown({ timeoutMs: 1_000 });
  });

  it("does not pass credential-like environment variables to the Provider", () => {
    expect(providerEnvironment({
      SystemRoot: "C:\\Windows",
      TEMP: "C:\\Temp",
      VUA_TOKEN: "secret",
      GITHUB_TOKEN: "secret",
      PATH: "unneeded",
    })).toEqual({ SystemRoot: "C:\\Windows", TEMP: "C:\\Temp" });
  });

  it("passes the machine folders Unity's Package Manager needs", () => {
    expect(providerEnvironment({
      ProgramData: "C:\\ProgramData",
      ALLUSERSPROFILE: "C:\\ProgramData",
      USERNAME: "player",
    })).toEqual({ ProgramData: "C:\\ProgramData", ALLUSERSPROFILE: "C:\\ProgramData" });
  });
});
