import path from "node:path";

interface FileObject { readonly path: string; readonly bytes: number; readonly sha256: string; readonly offset: number }
export interface InstalledManifest {
  readonly releaseVersion: string;
  readonly host: { readonly version: string; readonly build: string; readonly files: readonly FileObject[] };
  readonly amf: { readonly version: string; readonly build: string; readonly files: readonly FileObject[] };
  readonly runtime: { readonly version: string };
}
export interface InstalledContext {
  readonly schemaVersion: "0.1";
  readonly root: string; readonly hostRoot: string; readonly amfRoot: string; readonly runtimeRoot: string;
  readonly host: InstalledManifest; readonly amf: InstalledManifest; readonly amfAvailable: boolean;
}

/** Set only by the verified packaged bootstrap, never by an environment variable
 * or Renderer. Legacy ZIP and development retain their existing resource paths. */
export function installedContext(): InstalledContext | undefined {
  const value = (globalThis as typeof globalThis & { __VUA_DISTRIBUTION_V01__?: InstalledContext }).__VUA_DISTRIBUTION_V01__;
  if (!value) return undefined;
  if (value.schemaVersion !== "0.1" || !Object.isFrozen(value)
      || path.resolve(process.resourcesPath) !== path.join(value.runtimeRoot, "resources")
      || path.dirname(path.dirname(value.runtimeRoot)) !== value.root) throw new Error("invalid_installed_context");
  return value;
}
