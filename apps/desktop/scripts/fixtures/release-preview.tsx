// Synthetic release room for visual review: finished Avatars, a drawn turntable and upload states.
// The pictures are drawn figures, not renders of real materials; nothing here claims a real upload.
import React from "react";
import { AmfRoom } from "../../src/renderer/features/amf/AmfRoom.tsx";
import { ReleaseAlbumPage } from "../../src/renderer/features/amf/release/ReleaseAlbumPage.tsx";
import { AlbumBook, type AlbumSpread } from "../../src/renderer/features/amf/release/ReleaseAlbum.tsx";
import { ReleaseStage } from "../../src/renderer/features/amf/release/ReleaseStage.tsx";
import { ReleaseSheet } from "../../src/renderer/features/amf/release/ReleaseSheet.tsx";
import { avatarTitle } from "../../src/renderer/features/amf/release/release-model.ts";
import "../../src/renderer/features/amf/release/release-album.css";
import type { AmfProductionPort, AmfProductionRun, AmfReleasePort, AmfReleaseState, AmfUpload } from "../../src/renderer/gateway/index.ts";

const avatarId = "avtr_6f1c2d3e-4a5b-4c6d-8e7f-90a1b2c3d4e5";
const palette = [
  { hair: "#6d5bd0", shirt: "#f4f4f6", shorts: "#2f3a5a", skin: "#f2d4c4" },
  { hair: "#d06b8f", shirt: "#1f2a44", shorts: "#c94f6d", skin: "#f5dccd" },
  { hair: "#3d8f7a", shirt: "#f0c75e", shorts: "#3a3f4b", skin: "#eccab6" },
];

/** A drawn figure seen from camera yaw `index * 6` degrees on the grey preview background. */
function frame(look: number, index: number): string {
  const c = palette[look % palette.length]!;
  const angle = (index * 6 * Math.PI) / 180;
  const face = Math.cos(angle), side = Math.sin(angle);
  const width = 0.55 + 0.45 * Math.abs(face);
  const body = (w: number) => (w * width).toFixed(1);
  const eyes = face > 0.15 ? `<circle cx="${(256 - 14 * width + side * 22).toFixed(1)}" cy="150" r="5" fill="#2b2233"/><circle cx="${(256 + 14 * width + side * 22).toFixed(1)}" cy="150" r="5" fill="#2b2233"/>` : "";
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
<rect width="512" height="512" fill="#9e9e9e"/><ellipse cx="256" cy="470" rx="120" ry="16" fill="#8a8a8a"/>
<rect x="${(256 - 24 * width).toFixed(1)}" y="330" width="${body(20)}" height="130" rx="9" fill="${c.skin}"/>
<rect x="${(256 + 4 * width).toFixed(1)}" y="330" width="${body(20)}" height="130" rx="9" fill="${c.skin}"/>
<rect x="${(256 - 58 * width).toFixed(1)}" y="300" width="${body(116)}" height="70" rx="16" fill="${c.shorts}"/>
<rect x="${(256 - 62 * width).toFixed(1)}" y="200" width="${body(124)}" height="118" rx="26" fill="${c.shirt}" stroke="#00000022"/>
<rect x="${(256 - 86 * width).toFixed(1)}" y="214" width="${body(24)}" height="104" rx="12" fill="${c.skin}"/>
<rect x="${(256 + 62 * width).toFixed(1)}" y="214" width="${body(24)}" height="104" rx="12" fill="${c.skin}"/>
<ellipse cx="256" cy="148" rx="${body(54)}" ry="58" fill="${c.skin}"/>
<path d="M ${(256 - 60 * width).toFixed(1)} 150 Q 256 ${face < 0 ? 40 : 70} ${(256 + 60 * width).toFixed(1)} 150 L ${(256 + 64 * width).toFixed(1)} ${face < 0 ? 230 : 190} Q 256 ${face < 0 ? 250 : 120} ${(256 - 64 * width).toFixed(1)} ${face < 0 ? 230 : 190} Z" fill="${c.hair}"/>
${eyes}</svg>`;
  return `data:image/svg+xml;base64,${btoa(svg)}`;
}

const sources = [
  { sourceId: "avatar", itemName: "Meiyun_v2.1.0.zip" }, { sourceId: "outfit", itemName: "TshirtShorts_Meiyun_v1.00.zip" },
  { sourceId: "avatar", itemName: "Karin_v1.4.zip" }, { sourceId: "outfit", itemName: "SailorDress_Karin.zip" },
  { sourceId: "avatar", itemName: "Rusk_v3.0.zip" }, { sourceId: "outfit", itemName: "Hoodie_Rusk.zip" },
];
function finished(index: number, name: string, outfit: string, toggle: string, extra: Partial<NonNullable<NonNullable<AmfProductionRun["result"]>["facts"]>> = {}): AmfProductionRun {
  const runId = `n3-release-${index}`;
  return {
    schemaVersion: "0.1", runId, phase: "succeeded", taskId: `task-${runId}`, editor: { path: "C:/Unity/Unity.exe", version: "2022.3.22f1" }, projectRoot: `C:/work/${runId}`,
    choices: [], sources: [sources[index * 2]!, sources[index * 2 + 1]!].map((source, at) => ({ ...source, role: at === 0 ? "avatar" as const : "outfit" as const, name: source.itemName, sha256: "a".repeat(64) })),
    prefabs: [], steps: [{ key: "assembly", name: "assembly", state: "succeeded", detail: "" }], error: null, recovery: null, snapshotId: "snap",
    completedAt: `2026-10-1${7 - index}T0${3 + index}:2${index}:00Z`,
    selection: { avatarPrefab: `Assets/${name}/${name}.prefab`, outfits: [{ sourceId: "outfit", prefabPath: `Assets/Outfit/${outfit}.prefab`, mode: "merge_armature", bone: "", toggleName: toggle, active: true }] },
    result: { facts: { sdkDescriptorRecognized: true, missingReferences: 0, avatarName: name, instances: [{ sourceId: "outfit", bindingVerified: true, menuVerified: true, menuModified: true }],
      missingShaderMaterials: [], sdkSignedIn: "yes", hasMissingScripts: false,
      previews: (["front", "side", "back"] as const).map(view => ({ view, path: `.vua/previews/${runId}/${view}.png` })), ...extra } },
  };
}
const runs = [finished(0, "Meiyun", "TshirtShorts", "T-shirt"), finished(1, "Karin", "SailorDress", "Sailor dress", { sdkSignedIn: "no", missingShaderMaterials: ["Assets/Karin/Ribbon.mat"] }), finished(2, "Rusk", "Hoodie", "")];
const lookOf = (runId: string) => Math.max(0, runs.findIndex(run => run.runId === runId));

const upload = (patch: Partial<AmfUpload>): AmfUpload => ({ uploadId: "upload-1", avatarName: "Meiyun TshirtShorts", startedAt: "2026-10-17T06:00:00Z", endedAt: "2026-10-17T06:07:00Z",
  state: "succeeded", stage: "finishing", progress: 1, status: "", blueprintId: avatarId, firstUpload: true, reason: null, detail: "", errors: [], ...patch });

export function releaseViewNames(): string[] {
  return ["release-album", "release-stills", "release-turn", "release-confirm", "release-uploading", "release-uploaded", "release-failed", "release-empty"];
}

export function releasePorts(view: string): { production: AmfProductionPort; release: AmfReleasePort } {
  const state = (runId: string): AmfReleaseState => {
    const first = runId === runs[0]!.runId;
    const uploads: AmfUpload[] = !first ? [] : view === "release-uploading" ? [upload({ uploadId: "upload-2", state: "running", stage: "uploading", progress: 0.46, endedAt: null, blueprintId: null, firstUpload: null, status: "Uploading the Avatar bundle" })]
      : view === "release-uploaded" ? [upload({}), upload({ uploadId: "upload-0", state: "failed", blueprintId: null, reason: "sign_in_required", startedAt: "2026-10-17T05:40:00Z", endedAt: "2026-10-17T05:41:00Z" })]
      : view === "release-failed" ? [upload({ state: "failed", stage: "building", blueprintId: null, reason: "validation", detail: "Avatar validation failed",
        errors: ["This avatar uses 3 materials with the same name.", "Polygon count is over the Poor limit for PC (70,000)."] })]
      : [];
    return { runId, editorOpen: false, uploads, turntable: runId === runs[2]!.runId ? null : { frames: 60, size: 512, bakedAt: "2026-10-17T05:30:00Z", current: true } };
  };
  const production = {
    async list() { return { sources: [], editors: [], busy: false, runs: (view === "release-empty" ? [] : runs).map(run => ({ runId: run.runId, phase: run.phase, sourceNames: run.sources.map(s => s.itemName) })) }; },
    async get(runId: string) { return structuredClone(runs.find(run => run.runId === runId)!); },
    async previewImage(runId: string, path: string) { return frame(lookOf(runId), path.endsWith("side.png") ? 15 : path.endsWith("back.png") ? 30 : 0); },
    async reinspect(runId: string) { return { runId, taskId: "task" }; },
  } as unknown as AmfProductionPort;
  const release: AmfReleasePort = {
    async state(runId) { return state(runId); },
    async turntableFrame(runId, index) { return frame(lookOf(runId), index); },
    async bakeTurntable(runId) { return { runId, taskId: "task" }; },
    async upload(runId) { return { runId, taskId: "task" }; },
  };
  return { production, release };
}

/** The mid-turn frame: the first spread's right page lifted 42% of the way to the next spread. */
function TurningAlbum({ production, release }: { production: AmfProductionPort; release: AmfReleasePort }) {
  const states = runs.map(run => ({ runId: run.runId, editorOpen: false, uploads: [], turntable: null }));
  const spreads: AlbumSpread[] = runs.map((run, at) => ({
    key: run.runId, tab: avatarTitle(run),
    left: () => <div className="vua-album__left"><ReleaseStage production={production} release={release} run={run} turntable={null} title={avatarTitle(run)} live={false} /></div>,
    right: () => <ReleaseSheet run={run} state={states[at]!} number={at + 1} live={false} />,
  }));
  return <div className="vua-page vua-release-room"><section className="vua-album"><div className="vua-album__binder">
    <AlbumBook spreads={spreads} index={0} turn={{ from: 0, to: 1, direction: "next" }} progress={0.42} />
  </div></section></div>;
}

export function ReleasePreview({ view }: { view: string }) {
  const { production, release } = releasePorts(view);
  React.useEffect(() => {
    if (view !== "release-stills") return;
    // Turns to the third Avatar, which has its three views and no turntable yet.
    const timer = setInterval(() => {
      const tab = document.querySelector<HTMLButtonElement>(".vua-album__tabs li:nth-child(3) button");
      if (!tab) return;
      clearInterval(timer);
      tab.click();
    }, 100);
    return () => clearInterval(timer);
  }, [view]);
  React.useEffect(() => {
    if (view !== "release-confirm") return;
    // Opens the upload confirmation the way the player does and ticks the rights statement.
    const timer = setInterval(() => {
      const button = document.querySelector<HTMLButtonElement>('[data-nav-id="release-upload"]');
      if (!button || button.disabled) return;
      clearInterval(timer);
      button.click();
      setTimeout(() => document.querySelector<HTMLInputElement>('[data-nav-id="release-rights"]')?.click(), 150);
    }, 100);
    return () => clearInterval(timer);
  }, [view]);
  if (view === "release-turn") return <AmfRoom page="release" navigate={() => {}}><TurningAlbum production={production} release={release} /></AmfRoom>;
  return <AmfRoom page="release" navigate={() => {}}><ReleaseAlbumPage onNavigate={() => {}} /></AmfRoom>;
}
