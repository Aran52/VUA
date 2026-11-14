import { useCallback, useEffect, useRef, useState } from "react";
import type { PoseGraphSnapshotV01, PoseSourceStateV01 } from "@vua/contracts";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { useGateway } from "../../gateway/index.ts";
import { format, strings } from "../../i18n/index.ts";

const copy = strings.poseGraph;

function sourceTone(state: PoseSourceStateV01): "success" | "neutral" {
  return state === "active" || state === "ready" ? "success" : "neutral";
}

export function PoseGraphSection() {
  const port = useGateway().environment.poseGraph;
  const [snapshot, setSnapshot] = useState<PoseGraphSnapshotV01 | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const generation = useRef(0);
  const alive = useRef(false);

  const load = useCallback(async () => {
    if (!port) return;
    const ticket = ++generation.current;
    setBusy(true);
    setFailed(false);
    try {
      const next = await port.observe();
      if (alive.current && ticket === generation.current) {
        setSnapshot(next);
      }
    } catch {
      if (alive.current && ticket === generation.current) {
        setFailed(true);
      }
    } finally {
      if (alive.current && ticket === generation.current) {
        setBusy(false);
      }
    }
  }, [port]);

  useEffect(() => {
    alive.current = true;
    if (port) void load();
    return () => {
      alive.current = false;
      generation.current += 1;
    };
  }, [port, load]);

  if (!port) return null;

  return (
    <section className="vua-environment-section vua-pose-graph" data-pose-graph>
      <header className="vua-pose-graph__header">
        <div>
          <h2>{copy.title}</h2>
          <p className="vua-pose-graph__subtitle">{copy.subtitle}</p>
        </div>
        <Button
          variant="subtle"
          data-nav-id="tools-refresh-pose-graph"
          disabled={busy}
          onClick={() => void load()}
        >
          {busy ? copy.refreshing : copy.refresh}
        </Button>
      </header>

      {failed ? <p role="status" className="vua-pose-graph__status">{copy.failed}</p> : null}

      {snapshot ? (
        <div className="vua-pose-graph__body">
          <div className="vua-pose-graph__summary">
            <div className="vua-pose-graph__panel">
              <h3>{copy.openxrTitle}</h3>
              <div className="vua-pose-graph__row">
                <strong>{snapshot.openxr.runtimeName ?? copy.openxrUnset}</strong>
                <Badge tone={snapshot.openxr.presence === "found" ? "success" : "neutral"}>
                  {copy.openxrPresence[snapshot.openxr.presence]}
                </Badge>
              </div>
              {snapshot.lastHeadsetHint ? (
                <p className="vua-pose-graph__meta">
                  {format(copy.headsetHint, { name: snapshot.lastHeadsetHint })}
                </p>
              ) : null}
            </div>

            <div className="vua-pose-graph__panel">
              <h3>{copy.pharosTitle}</h3>
              <div className="vua-pose-graph__row">
                <span>
                  {snapshot.pharosHub.activeModes.length > 0
                    ? snapshot.pharosHub.activeModes.map((mode) => copy.pharosModes[mode]).join(" · ")
                    : copy.pharosNoActiveModes}
                </span>
                <Badge tone={snapshot.pharosHub.state === "running" ? "success" : "neutral"}>
                  {copy.pharosStates[snapshot.pharosHub.state]}
                </Badge>
              </div>
              {snapshot.pharosHub.wearableNodes.some((node) => node.state === "connected") ? (
                <div className="vua-pose-graph__badges">
                  {snapshot.pharosHub.wearableNodes
                    .filter((node) => node.state === "connected")
                    .map((node) => (
                      <Badge key={node.slot} tone="success">
                        {copy.pharosSlots[node.slot]}
                      </Badge>
                    ))}
                </div>
              ) : null}
            </div>
          </div>

          <div className="vua-pose-graph__panel">
            <h3>{copy.sourcesTitle}</h3>
            <ul className="vua-pose-graph__sources">
              {snapshot.poseSources.map((source) => (
                <li key={source.sourceId} className="vua-pose-graph__source" data-pose-source={source.sourceId}>
                  <div className="vua-pose-graph__source-main">
                    <strong>{copy.sourceNames[source.sourceId]}</strong>
                  </div>
                  <div className="vua-pose-graph__badges">
                    <Badge tone={sourceTone(source.state)}>
                      {copy.sourceStates[source.state]}
                    </Badge>
                  </div>
                </li>
              ))}
            </ul>
          </div>

          <div className="vua-pose-graph__panel">
            <h3>{copy.driversTitle}</h3>
            {snapshot.steamvrDrivers.length === 0 ? (
              <p className="vua-pose-graph__meta">{copy.driversEmpty}</p>
            ) : (
              <ul className="vua-pose-graph__drivers">
                {snapshot.steamvrDrivers.map((driverId) => (
                  <li key={driverId} className="vua-pose-graph__driver">
                    <span>{copy.driverNames[driverId]}</span>
                    <Badge tone="success">{copy.sourceStates.installed}</Badge>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      ) : null}
    </section>
  );
}
