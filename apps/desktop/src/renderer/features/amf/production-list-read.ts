/** The production list checks every material file, and the AMF module answers one request at a time, so a read
 * holds up whichever page opens next. Pages share one read and reuse it for as long as they can. */
import type { AmfProductionList, AmfProductionPort } from "../../gateway/index.ts";

export const PRODUCTION_LIST_FRESH_MS = 60_000;
interface SharedRead { readonly at: number; readonly read: Promise<AmfProductionList> }
const reads = new WeakMap<AmfProductionPort, SharedRead>();
const latest = new WeakMap<AmfProductionPort, AmfProductionList>();

/** The newest list this session has seen, for drawing a page at once while a newer read runs. */
export function latestProductionList(port: AmfProductionPort | undefined): AmfProductionList | null {
  return port === undefined ? null : latest.get(port) ?? null;
}

/** A read no older than `maxAgeMs`; without one the module is asked again. */
export function readProductionList(port: AmfProductionPort, maxAgeMs = 0): Promise<AmfProductionList> {
  const known = reads.get(port);
  if (known !== undefined && Date.now() - known.at < maxAgeMs) return known.read;
  const read = port.list().then(list => { latest.set(port, list); return list; });
  const entry = { at: Date.now(), read };
  reads.set(port, entry);
  read.catch(() => { if (reads.get(port) === entry) reads.delete(port); });
  return read;
}
