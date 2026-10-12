/** Desktop library v0.1: an explicit local action, with no renderer-supplied path. */
export interface DesktopLibraryApiV01 {
  openFolder(): Promise<{ readonly opened: boolean }>;
  openEntryFolder(entryId: string): Promise<{ readonly opened: boolean }>;
  openBoothProduct(productId: string): Promise<{ readonly opened: boolean }>;
  /** Preload extracts native File paths; Main validates them, only for active AMF. */
  resolveDroppedFiles(files: readonly File[]): Promise<readonly string[]>;
  previewDirectory(path: string): Promise<DesktopLibraryDirectoryPreviewV01>;
}

export interface DesktopLibraryDirectoryPreviewV01 {
  readonly path: string;
  readonly state: "ready" | "unavailable";
  readonly truncated: boolean;
  readonly items: readonly { readonly name: string; readonly path: string; readonly kind: "directory" | "file" | "link" | "other"; readonly sizeBytes: number | null }[];
}
