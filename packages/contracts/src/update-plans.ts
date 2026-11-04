/** Explicit compatible pair updates; independent actions keep their own component. */
import type { ComponentUpdateTargetV02, ComponentUpdateSnapshotV02, VuaDesktopApiV5 } from "./component-updates.js";
export interface UpdatePlanV01 {
  readonly id: string;
  readonly installed: { readonly host: string; readonly amf: string };
  readonly targets: readonly [
    { readonly component: "host"; readonly target: ComponentUpdateTargetV02 },
    { readonly component: "amf"; readonly target: ComponentUpdateTargetV02 },
  ];
  readonly activation: "restart";
}
export interface UpdatePlanSnapshotV01 {
  readonly schemaVersion: "0.1";
  readonly available: boolean;
  readonly state: "idle" | "checking" | "proposed" | "downloading" | "staged" | "waiting-for-idle" | "activating" | "interrupted" | "failed";
  readonly plan?: UpdatePlanV01 | undefined;
  readonly progress?: (NonNullable<ComponentUpdateSnapshotV02["progress"]> & { readonly component: "host" | "amf" }) | undefined;
  readonly rollbackAvailable: boolean;
  readonly error?: string | undefined;
}
export interface DesktopUpdatePlansApiV01 {
  snapshot(): Promise<UpdatePlanSnapshotV01>;
  propose(): Promise<UpdatePlanSnapshotV01>;
  stage(id: string): Promise<UpdatePlanSnapshotV01>;
  cancel(): Promise<UpdatePlanSnapshotV01>;
  apply(id: string): Promise<UpdatePlanSnapshotV01>;
  restore(): Promise<UpdatePlanSnapshotV01>;
  subscribe(listener: (value: UpdatePlanSnapshotV01) => void): () => void;
}
export interface VuaDesktopApiV6 extends VuaDesktopApiV5 {
  readonly updatePlans?: DesktopUpdatePlansApiV01;
}
