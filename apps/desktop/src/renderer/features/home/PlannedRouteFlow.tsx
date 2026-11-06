/** A headset route VUA explains and observes, ahead of installing and starting it: choose a connection,
 * see what the PC and the headset need, and open the official page. The route's real flow replaces this
 * when its backend arrives; see the planned connections in the tool catalog. */
import { useRef, useState } from "react";
import { Icon } from "@vua/design-system";
import { openExternalUrl } from "../../app/open-external.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { useSwapFade } from "../../components/primitives/motion.ts";
import { currentLocale } from "../../i18n/index.ts";
import { plannedRoutesCopy } from "../../i18n/planned-routes-copy.ts";
import { initialConnection, PLANNED_CONNECTIONS, type ConnectionId } from "./planned-routes.ts";

export function PlannedConnections({ id, connections, detected }: { id: string; connections: readonly ConnectionId[]; detected: readonly string[] }) {
  const copy = plannedRoutesCopy[currentLocale];
  const [chosen, setChosen] = useState<ConnectionId | null>(null);
  const active = chosen ?? initialConnection(connections, detected);
  const connection = PLANNED_CONNECTIONS[active];
  const text = copy.connections[active];
  const found = connection.component !== null && detected.includes(connection.component);
  const detail = useRef<HTMLDivElement>(null);
  useSwapFade(active, detail);
  return <div className="vua-planned" data-planned-route={id}>
    {connections.length > 1 ? <fieldset className="vua-segmented">
      <legend>{copy.connection}</legend>
      {connections.map(option => <label key={option} data-checked={option === active}>
        <input type="radio" name={`planned-${id}`} checked={option === active} data-nav-id={`planned-${id}-${option}`} onChange={() => setChosen(option)} />
        {PLANNED_CONNECTIONS[option].name}
      </label>)}
    </fieldset> : null}
    <div ref={detail} className="vua-planned__detail" data-connection={active}>
      <div className="vua-planned__head">
        <strong>{connection.name}</strong>
        <span className="vua-planned__tag">{text.tag}</span>
        {connection.component !== null ? <span className="vua-planned__state" data-found={found || undefined} role="status">
          {found ? <Icon name="check" size={16} /> : null}{found ? copy.detected : copy.notDetected}
        </span> : null}
      </div>
      <p>{text.summary}</p>
      <dl className="vua-planned__needs">
        <dt>{copy.pc}</dt><dd>{text.pc}</dd>
        <dt>{copy.headset}</dt><dd>{text.headset}</dd>
      </dl>
      <div className="vua-planned__actions">
        <Button data-nav-id={`planned-official-${active}`} onClick={() => { void openExternalUrl(connection.url); }}>{copy.official}</Button>
      </div>
    </div>
  </div>;
}

export function PlannedRouteFlow({ id, connections, detected }: { id: string; connections: readonly ConnectionId[]; detected: readonly string[] }) {
  const copy = plannedRoutesCopy[currentLocale];
  return <div className="vua-play-flow">
    <PlannedConnections id={id} connections={connections} detected={detected} />
    <p className="vua-play-flow__notice">{copy.note}</p>
  </div>;
}
