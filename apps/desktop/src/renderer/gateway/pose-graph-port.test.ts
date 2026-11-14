import { describe, expect, it } from "vitest";
import type { DesktopGatewayRequestV1, GlbInspectionResultV01, PoseGraphResultV01, TermsUseCheckV02 } from "@vua/contracts";
import type { GatewayClient } from "./gateway-client.ts";
import { createLivePoseGraphPort } from "./pose-graph-port.ts";
import { createLiveLibraryBrowserPort } from "./library-browser-port.ts";
import { createTermsPort } from "./terms-port.ts";

describe("forward-compatibility gateway ports", () => {
  it("observes pose graph snapshot via environment.observePoseGraph", async () => {
    const calls: DesktopGatewayRequestV1[] = [];
    const payload: PoseGraphResultV01 = {
      poseGraph: {
        schemaVersion: "vua.pose-graph/v0.1",
        capturedAt: "2026-11-14T00:00:00Z",
        openxr: { presence: "found", runtimeName: "SteamVR OpenXR" },
        steamvrDrivers: ["space_calibrator", "pharos"],
        lastHeadsetHint: "Quest 3",
        poseSources: [
          { sourceId: "headset_6dof", state: "ready", detailCode: null },
          { sourceId: "pharos", state: "active", detailCode: "hub_running" },
          { sourceId: "slimevr", state: "not_detected", detailCode: null },
          { sourceId: "pico_motion", state: "not_detected", detailCode: null },
          { sourceId: "lighthouse", state: "installed", detailCode: null },
          { sourceId: "eye_face_osc", state: "not_detected", detailCode: null },
        ],
        pharosHub: {
          state: "running",
          activeModes: ["optical_stereo_2x2x2", "imu_fusion"],
          wearableNodes: [
            { slot: "left_foot", state: "connected", footHeadingAvailable: true },
            { slot: "right_foot", state: "connected", footHeadingAvailable: true },
            { slot: "waist", state: "connected", footHeadingAvailable: false },
            { slot: "chest", state: "not_connected", footHeadingAvailable: false },
          ],
          stereoBaseStationCount: 2,
          panoramicAnchorCount: 1,
        },
      },
    };
    const client: GatewayClient = {
      async invoke(request) {
        calls.push(request);
        return { ok: true, value: payload };
      },
      subscribe: () => () => {},
    };
    const port = createLivePoseGraphPort(client);
    const snapshot = await port.observe();
    expect(calls[0]).toMatchObject({
      method: "environment.observePoseGraph",
      params: { schemaVersion: "0.1" },
    });
    expect(snapshot.pharosHub.state).toBe("running");
    expect(snapshot.poseSources.find((s) => s.sourceId === "pharos")?.state).toBe("active");
  });

  it("inspects GLB/VRM artifacts via library.inspectGlb", async () => {
    const calls: DesktopGatewayRequestV1[] = [];
    const payload: GlbInspectionResultV01 = {
      schemaVersion: "vua.glb-inspection/v0.1",
      truncated: false,
      items: [
        {
          copyId: "cpy-1",
          fileName: "head.glb",
          sizeBytes: 2048,
          format: "glb_2_0",
          extensionsUsed: [],
          vrm: null,
          boundsMeters: { size: [0.35, 0.40, 0.42], min: [-0.175, 0, -0.21], max: [0.175, 0.4, 0.21] },
          objectHeadClearance: {
            status: "passes",
            referenceHeadHmdMeters: [0.22, 0.26, 0.28],
            marginsMeters: [0.13, 0.14, 0.14],
          },
          phrReadiness: "ready_object_head",
        },
      ],
    };
    const client: GatewayClient = {
      async invoke(request) {
        calls.push(request);
        return { ok: true, value: payload };
      },
      subscribe: () => () => {},
    };
    const port = createLiveLibraryBrowserPort(client);
    const result = await port.inspectGlb!({ productId: "booth:12345" });
    expect(calls[0]).toMatchObject({
      method: "library.inspectGlb",
      params: { schemaVersion: "0.1", productId: "booth:12345" },
    });
    expect(result.items[0]?.phrReadiness).toBe("ready_object_head");
  });

  it("checks PH-R VN3 presets via terms.checkUse v0.2", async () => {
    const calls: DesktopGatewayRequestV1[] = [];
    const payload: TermsUseCheckV02 = {
      schemaVersion: "0.2",
      presetId: "phr_personal_avatar",
      matchingProductIds: [],
      combination: {
        decision: "contact_licensor",
        blockingItems: [{ productId: "booth:12345", item: "X", termKey: null, reason: "special_note_unknown" }],
        conditions: [],
        advisories: [],
        hasSpecialNote: true,
        specialNotes: [{ productId: "booth:12345", sourceId: "src-1", language: "ja", text: "VRChat専用モデルです" }],
        platformRestrictionHint: true,
      },
      products: [
        {
          productId: "booth:12345",
          sourceState: "parsed",
          decision: "contact_licensor",
          blockingItems: [{ productId: "booth:12345", item: "X", termKey: null, reason: "special_note_unknown" }],
          conditions: [],
          advisories: [],
          hasSpecialNote: true,
          specialNotes: [{ productId: "booth:12345", sourceId: "src-1", language: "ja", text: "VRChat専用モデルです" }],
          platformRestrictionHint: true,
        },
      ],
    };
    const client: GatewayClient = {
      async invoke(request) {
        calls.push(request);
        return { ok: true, value: payload };
      },
      subscribe: () => () => {},
    };
    const port = createTermsPort(client);
    const check = await port.checkUseV02!("phr_personal_avatar", ["booth:12345"]);
    expect(calls[0]).toMatchObject({
      method: "terms.checkUse",
      params: { schemaVersion: "0.2", presetId: "phr_personal_avatar", productIds: ["booth:12345"] },
    });
    expect(check.combination.platformRestrictionHint).toBe(true);
  });
});
