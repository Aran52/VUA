import fs from "node:fs";
import path from "node:path";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { EAC_CONCLUSIONS_V02, type EacProbeSnapshotV02 } from "@vua/contracts";
import { EacDialog } from "./EacDialog.tsx";

const { gateway } = vi.hoisted(() => ({
  gateway: {
    environment: {
      eac: {
        inspect: vi.fn(),
        repairCapability: vi.fn(() => Promise.resolve({ state: "unavailable" as const })),
        terminateResidual: vi.fn(),
        launchRepair: vi.fn(),
        openRepairFolder: vi.fn(() => Promise.resolve({ opened: true })),
        getExperimentalTermination: vi.fn(() => Promise.resolve({ enabled: false })),
        setExperimentalTermination: vi.fn(() => Promise.resolve({ enabled: false })),
      },
    },
  },
}));

vi.mock("../../gateway/index.ts", () => ({ useGateway: () => gateway }));

const FIXTURES_DIR = path.resolve(
  __dirname,
  "../../../../../../schemas/eac-probe/v0.2/fixtures",
);

function loadFixtures(): readonly EacProbeSnapshotV02[] {
  return fs
    .readdirSync(FIXTURES_DIR)
    .filter((file) => file.endsWith(".json"))
    .sort()
    .map((file) => JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, file), "utf8")) as EacProbeSnapshotV02);
}

describe("EacDialog rendering across all eight conclusions", () => {
  const fixtures = loadFixtures();

  it("renders a valid dialog view for every v0.2 conclusion fixture", () => {
    expect(fixtures.length).toBe(EAC_CONCLUSIONS_V02.length);
    const seen = new Set<string>();

    for (const snapshot of fixtures) {
      seen.add(snapshot.conclusion);
      const html = renderToStaticMarkup(
        createElement(EacDialog, {
          open: true,
          initialSnapshot: snapshot,
          onClose: vi.fn(),
        }),
      );

      expect(html).toContain(`data-conclusion="${snapshot.conclusion}"`);
      expect(html).toContain('data-nav-id="eac-open-guide"');

      if (snapshot.plan.length === 0) {
        expect(html).not.toContain('data-nav-id="eac-step-');
      } else {
        // Only one primary ladder step button is shown at a time.
        const stepMatches = [...html.matchAll(/data-nav-id="eac-step-([a-z_]+)"/g)];
        expect(stepMatches).toHaveLength(1);
      }

      // Full process paths only appear inside the collapsible details section.
      for (const proc of snapshot.processes) {
        if (proc.path) {
          const detailsStart = html.indexOf('<details class="vua-eac-dialog__details"');
          expect(detailsStart).toBeGreaterThan(0);
          expect(html.indexOf(proc.path)).toBeGreaterThan(detailsStart);
        }
      }
    }

    expect([...seen].sort()).toEqual([...EAC_CONCLUSIONS_V02].sort());
  });

  it("renders nothing when closed", () => {
    const html = renderToStaticMarkup(
      createElement(EacDialog, {
        open: false,
        initialSnapshot: fixtures[0] ?? null,
        onClose: vi.fn(),
      }),
    );
    expect(html).toBe("");
  });
});
