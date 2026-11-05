import { useCallback, useEffect, useRef, useState } from "react";
import { PLAY_ROUTES, type PlayRoute, type PlaySession } from "@vua/contracts";
import { useGateway } from "../../gateway/index.ts";
/** Play follows sessions closely; Home only needs them about right. */
export function usePlaySessions(pollMs = 1500) {
  const port = useGateway().environment.play;
  const [sessions, setSessions] = useState<Partial<Record<PlayRoute, PlaySession>>>({});
  const [unavailable, setUnavailable] = useState<Partial<Record<PlayRoute, boolean>>>({});
  const [pending, setPending] = useState<Partial<Record<PlayRoute, boolean>>>({});
  const mounted = useRef(false); const epochs = useRef({ desktop_play: 0, pico_pcvr: 0 });
  const busy = useRef(new Set<PlayRoute>());
  const refresh = useCallback(async () => {
    await Promise.all(PLAY_ROUTES.map(async route => {
      if (busy.current.has(route)) return;
      const epoch = epochs.current[route];
      try {
        if (!port) throw new Error("unavailable");
        const snapshot = await port.observe(route);
        if (mounted.current && epoch === epochs.current[route]) { setSessions(s => ({ ...s, [route]: snapshot })); setUnavailable(s => ({ ...s, [route]: false })); }
      } catch { if (mounted.current && epoch === epochs.current[route]) setUnavailable(s => ({ ...s, [route]: true })); }
    }));
  }, [port]);
  useEffect(() => {
    mounted.current = true;
    let cancelled = false; let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      if (!document.hidden && document.querySelector(".vua-play-page, .vua-home")?.getClientRects().length) await refresh();
      if (!cancelled) timer = setTimeout(() => { void poll(); }, document.documentElement.dataset.effects === "off" ? Math.max(pollMs, 5000) : pollMs);
    };
    void refresh().then(() => { if (!cancelled) timer = setTimeout(() => { void poll(); }, pollMs); });
    return () => { mounted.current = false; cancelled = true; clearTimeout(timer); };
  }, [refresh, pollMs]);
  const act = async (route: PlayRoute, stop: boolean) => {
    if (!port || busy.current.has(route)) return;
    busy.current.add(route); const epoch = ++epochs.current[route]; setPending(p => ({ ...p, [route]: true }));
    try {
      const snapshot = await port[stop ? "stop" : "start"](route, crypto.randomUUID());
      if (mounted.current && epoch === epochs.current[route]) { setSessions(s => ({ ...s, [route]: snapshot })); setUnavailable(s => ({ ...s, [route]: false })); }
    } catch { if (mounted.current) setUnavailable(s => ({ ...s, [route]: true })); }
    finally { busy.current.delete(route); if (mounted.current) setPending(p => ({ ...p, [route]: false })); }
  };
  return { sessions, unavailable, pending, refresh, act };
}
