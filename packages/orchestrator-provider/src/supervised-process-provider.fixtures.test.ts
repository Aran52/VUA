import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import path from "node:path";
import { PassThrough } from "node:stream";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  SupervisedProcessProviderV01,
  type ProviderProcessFactoryV01,
} from "./supervised-process-provider.js";

/**
 * Handshake wire-contract vectors (proposal 001, provider-frame-v0.1).
 *
 * The TypeScript supervisor and the Rust host (crates/provider-host tests)
 * consume the SAME positive and negative vectors from
 * schemas/orchestrator/provider-frame-v0.1/fixtures. Division of labor:
 * the supervisor produces handshake requests and consumes handshake
 * responses, so the request-positive vector pins the produced frame shape
 * and every response vector pins start() admission; the request-negative
 * vectors are consumed on the Rust side (the host rejects them with
 * protocol_error). Changing a handshake shape without updating schema,
 * vectors and both consumers fails here first.
 */

const here = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.resolve(here, "../../../schemas/orchestrator/provider-frame-v0.1/fixtures");

function vector(name: string): Record<string, unknown> {
  return JSON.parse(readFileSync(path.join(fixturesDir, name), "utf8")) as Record<string, unknown>;
}

class VectorFakeProcess extends EventEmitter {
  readonly stdin = new PassThrough();
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  readonly stdio = [this.stdin, this.stdout, this.stderr];
  readonly sentFrames: Array<Record<string, unknown>> = [];
  #buffer = "";
  readonly #respond: (frame: Record<string, unknown>) => void;

  constructor(respond: (frame: Record<string, unknown>) => void) {
    super();
    this.#respond = respond;
    this.stdin.on("data", (chunk: Buffer) => {
      this.#buffer += chunk.toString("utf8");
      let newline = this.#buffer.indexOf("\n");
      while (newline >= 0) {
        const line = this.#buffer.slice(0, newline);
        this.#buffer = this.#buffer.slice(newline + 1);
        const frame = JSON.parse(line) as Record<string, unknown>;
        this.sentFrames.push(frame);
        this.#respond(frame);
        newline = this.#buffer.indexOf("\n");
      }
    });
  }

  kill(): boolean {
    queueMicrotask(() => { this.emit("exit", 0, null); this.emit("close", 0, null); });
    return true;
  }

  /// Replays a frozen response vector: the wire face (frameVersion/kind/
  /// payload) comes from the vector, the frameId reuses the request's.
  replyWithVector(requestFrameId: string, responseVector: Record<string, unknown>): void {
    this.stdout.write(`${JSON.stringify({
      frameVersion: responseVector.frameVersion,
      frameId: requestFrameId,
      kind: responseVector.kind,
      payload: responseVector.payload,
    })}\n`);
  }
}

function harness(
  respond: (frame: Record<string, unknown>) => void,
): { readonly provider: SupervisedProcessProviderV01; readonly processes: VectorFakeProcess[] } {
  const processes: VectorFakeProcess[] = [];
  const factory: ProviderProcessFactoryV01 = () => {
    const process = new VectorFakeProcess(respond);
    processes.push(process);
    return process as never;
  };
  return {
    provider: new SupervisedProcessProviderV01({
      executablePath: path.resolve("fake-provider.exe"),
      databasePath: path.resolve("fake-provider.db"),
      handshakeTimeoutMs: 2_000,
    }, factory),
    processes,
  };
}

const RESPONSE_POSITIVE_VECTORS = [
  "handshake.response.valid.json",
  "handshake.response.valid-no-downloads.json",
];

const RESPONSE_NEGATIVE_PAYLOAD_VECTORS = [
  "handshake.response.invalid-missing-download-ingest.json",
  "handshake.response.invalid-contract-version.json",
  "handshake.response.invalid-unsupported-contract-only.json",
  "handshake.response.invalid-empty-provider-build-id.json",
];

describe("handshake wire-contract vectors (provider-frame-v0.1)", () => {
  it("produces the frozen request shape and accepts both positive response vectors", async () => {
    for (const name of RESPONSE_POSITIVE_VECTORS) {
      const responseVector = vector(name);
      const { provider, processes } = harness((frame) => {
        processes[0]?.replyWithVector(frame.frameId as string, responseVector);
      });
      const handshake = await provider.start();
      // The produced request must equal the frozen request vector in every
      // wire field except the generated frameId.
      const sent = processes[0]?.sentFrames[0];
      const requestVector = vector("handshake.request.valid.json");
      expect(sent?.frameVersion).toBe(requestVector.frameVersion);
      expect(sent?.kind).toBe(requestVector.kind);
      expect(sent?.payload).toBeNull();
      expect(sent?.frameId).toMatch(/^main-\d+$/);
      expect(handshake).toEqual(responseVector.payload);
    }
  });

  it("rejects every negative response payload vector and lands in failed", async () => {
    for (const name of RESPONSE_NEGATIVE_PAYLOAD_VECTORS) {
      const responseVector = vector(name);
      const { provider, processes } = harness((frame) => {
        processes[0]?.replyWithVector(frame.frameId as string, responseVector);
      });
      await expect(provider.start()).rejects.toThrow("invalid handshake");
      await new Promise((resolve) => setTimeout(resolve, 0));
      expect(provider.status()).toMatchObject({ state: "failed", acceptingCalls: false });
    }
  });

  it("never admits a handshake through a non-response frame", async () => {
    const responseVector = vector("handshake.response.invalid-kind-event.json");
    const { provider, processes } = harness((frame) => {
      processes[0]?.replyWithVector(frame.frameId as string, responseVector);
    });
    await expect(provider.start()).rejects.toThrow();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(provider.status()).toMatchObject({ state: "failed", acceptingCalls: false });
  });
});
