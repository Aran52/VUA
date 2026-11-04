import { isLibraryViewParamsV01, isLibraryListV01, type LibraryListParamsV01, type LibraryListV01, type LibraryRowV01 } from "./library-view-v01.js";
import { TERMS_PRESET_IDS, type TermsPresetId } from "./terms-v01.js";

export const LIBRARY_USE_STATUSES = ["allowed","conditional","contact","forbidden","unknown","no_terms"] as const;
export type LibraryUseStatus = typeof LIBRARY_USE_STATUSES[number];
export interface LibraryUseV02 { readonly presetId: TermsPresetId; readonly status: LibraryUseStatus }
export type LibraryListParamsV02 = Omit<LibraryListParamsV01,"schemaVersion"> & {
  readonly schemaVersion: "0.2"; readonly use?: TermsPresetId; readonly useMode?: "available" | "all";
};
export type LibraryRowV02 = LibraryRowV01 & { readonly use?: LibraryUseV02 };
export type LibraryListV02 = Omit<LibraryListV01,"schemaVersion"|"items"> & { readonly schemaVersion:"0.2"; readonly items:readonly LibraryRowV02[] };
const record = (value: unknown): value is Record<string,unknown> => value !== null && typeof value === "object" && !Array.isArray(value);
export function isLibraryUseV02(value: unknown): value is LibraryUseV02 {
  return record(value) && Object.keys(value).length === 2 && Object.hasOwn(value,"presetId") && Object.hasOwn(value,"status")
    && (TERMS_PRESET_IDS as readonly unknown[]).includes(value.presetId) && (LIBRARY_USE_STATUSES as readonly unknown[]).includes(value.status);
}
export function isLibraryListParamsV02(value: unknown): value is LibraryListParamsV02 {
  if (!record(value) || value.schemaVersion !== "0.2") return false;
  const {use,useMode,...previous} = value;
  return isLibraryViewParamsV01("library.list",{...previous,schemaVersion:"0.1"})
    && (!Object.hasOwn(value,"use") || (TERMS_PRESET_IDS as readonly unknown[]).includes(use))
    && (!Object.hasOwn(value,"useMode") || (use !== undefined && (useMode === "available" || useMode === "all")));
}
export function isLibraryListV02(value: unknown): value is LibraryListV02 {
  if (!record(value) || value.schemaVersion !== "0.2" || !Array.isArray(value.items)) return false;
  const items: unknown[]=[];
  for (const row of value.items) {
    if (!record(row) || (Object.hasOwn(row,"use") && !isLibraryUseV02(row.use))) return false;
    const {use: _annotation,...previous} = row; items.push(previous);
  }
  return isLibraryListV01({...value,schemaVersion:"0.1",items});
}
