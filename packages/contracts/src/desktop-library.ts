/** Desktop library v0.1: an explicit local action, with no renderer-supplied path. */
export interface DesktopLibraryApiV01 {
  openFolder(): Promise<{ readonly opened: boolean }>;
  openEntryFolder(entryId: string): Promise<{ readonly opened: boolean }>;
  openBoothProduct(productId: string): Promise<{ readonly opened: boolean }>;
  /** Preload extracts native File paths; Main validates them, only for active AMF. */
  resolveDroppedFiles(files: readonly File[]): Promise<readonly string[]>;
}
