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
        blockedSteamvrDrivers: [],
        lastHeadsetHint: "Quest 3",
        poseSources: [
          { sourceId: "headset_6dof", state: "ready", detailCode: null },
          { sourceId: "pharos", state: "active", detailCode: "telemetry_live" },
          { sourceId: "slimevr", state: "not_detected", detailCode: null },
          { sourceId: "pico_motion", state: "not_detected", detailCode: null },
          { sourceId: "lighthouse", state: "installed", detailCode: null },
          { sourceId: "space_calibrator", state: "ready", detailCode: "driver_registered" },
          { sourceId: "mocopi", state: "not_detected", detailCode: null },
          { sourceId: "haritorax", state: "not_detected", detailCode: null },
          { sourceId: "rebocap", state: "not_detected", detailCode: null },
          { sourceId: "eye_face_osc", state: "not_detected", detailCode: null },
        ],
        pharosHub: {
          state: "running",
          telemetryState: "live",
          calibrationState: "calibrated",
          spatialAnchorReady: true,
          activeModes: ["optical_stereo_2x2x2", "imu_fusion"],
          wearableNodes: [
            { slot: "left_foot", state: "connected", trackingMode: "optical_imu", batteryPercent: 85, footHeadingAvailable: true },
            { slot: "right_foot", state: "connected", trackingMode: "passive_retroreflective", batteryPercent: 0, footHeadingAvailable: true },
            { slot: "waist", state: "connected", trackingMode: "optical_imu", batteryPercent: 72, footHeadingAvailable: false },
            { slot: "chest", state: "not_connected", trackingMode: null, batteryPercent: null, footHeadingAvailable: false },
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
    expect(snapshot.pharosHub.spatialAnchorReady).toBe(true);
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
          fileName: "Props/head.glb",
          sourceKind: "zip_member",
          containerFileName: "head_bundle.zip",
          sizeBytes: 2048,
          format: "glb_2_0",
          extensionsUsed: [],
          contentSummary: {
            meshCount: 1,
            primitiveCount: 1,
            triangleEstimate: 1200,
            skinCount: 0,
            nodeCount: 1,
            animationCount: 0,
          },
          materialSummary: {
            materialCount: 1,
            unlitMaterialCount: 0,
            mtoonMaterialCount: 0,
          },
          textureSummary: {
            textureCount: 0,
            embeddedImageCount: 0,
            externalUriTextureCount: 0,
          },
          secondaryMotion: {
            springBoneJointCount: 0,
            nodeConstraintCount: 0,
            expressionCount: 0,
          },
          vrm: null,
          boundsMeters: { size: [0.35, 0.40, 0.42], min: [-0.175, 0, -0.21], max: [0.175, 0.4, 0.21] },
          objectHeadClearance: {
            status: "passes",
            referenceHeadHmdMeters: [0.22, 0.26, 0.28],
            marginsMeters: [0.065, 0.07, 0.07],
            mountOffsetCenterMeters: [0, 0.2, 0],
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
    expect(result.items[0]?.sourceKind).toBe("zip_member");
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
