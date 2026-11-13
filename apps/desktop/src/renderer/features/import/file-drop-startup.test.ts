import { describe, expect, it, vi } from "vitest";
import type { AmfModuleSnapshotV01 } from "@vua/contracts";
import { waitForAmfStartup } from "./FileDropImporter.tsx";

const snap = (
  installed: boolean,
  state: AmfModuleSnapshotV01["state"],
): AmfModuleSnapshotV01 => ({
  schemaVersion: "0.1",
  moduleId: "amf",
  installed,
  state,
});

describe("waitForAmfStartup", () => {
  it("waits for deferred AMF startup when installed and starting, then resolves ready", async () => {
    let current = snap(true, "starting");
    const listeners = new Set<(next: AmfModuleSnapshotV01) => void>();
    const unsubscribeSpy = vi.fn();
    const port = {
      snapshot: vi.fn(async () => current),
      subscribe: vi.fn((listener: (next: AmfModuleSnapshotV01) => void) => {
        listeners.add(listener);
        return () => {
          listeners.delete(listener);
          unsubscribeSpy();
        };
      }),
    };

    const pending = waitForAmfStartup(port, current, 5_000);
    current = snap(true, "ready");
    for (const listener of listeners) listener(current);

    await expect(pending).resolves.toEqual(snap(true, "ready"));
    expect(unsubscribeSpy).toHaveBeenCalledTimes(1);
  });

  it("returns immediately without subscribing when AMF is disabled (absent)", async () => {
    const absent = snap(false, "absent");
    const port = {
      snapshot: vi.fn(async () => absent),
      subscribe: vi.fn(() => () => {}),
    };

    await expect(waitForAmfStartup(port, absent, 5_000)).resolves.toEqual(absent);
    expect(port.subscribe).not.toHaveBeenCalled();
    expect(port.snapshot).not.toHaveBeenCalled();
  });

  it("catches transitions that complete before subscribe settles via post-subscribe snapshot()", async () => {
    const starting = snap(true, "starting");
    const ready = snap(true, "ready");
    const unsubscribeSpy = vi.fn();
    const port = {
      snapshot: vi.fn(async () => ready),
      subscribe: vi.fn(() => unsubscribeSpy),
    };

    await expect(waitForAmfStartup(port, starting, 5_000)).resolves.toEqual(ready);
    expect(unsubscribeSpy).toHaveBeenCalledTimes(1);
  });
});
