/** Flat cut-away interiors of the factory rooms. Colors come from the room's --room and the theme tokens. */
import { strings } from "../../i18n/index.ts";
import { STATIONS, type StationKey, type StationState } from "./factory-model.ts";

const copy = strings.factory;
const svg = { viewBox: "0 0 400 190", preserveAspectRatio: "xMidYMax meet", "aria-hidden": true, className: "vua-interior" } as const;

export function ControlInterior({ recipes }: { recipes: number }) {
  const nodes = [[60, 34], [104, 22], [140, 46], [96, 60], [176, 30]] as const;
  const shown = nodes.slice(0, Math.max(2, Math.min(nodes.length, recipes + 1)));
  return <svg {...svg}>
    <rect className="vua-interior__wall" x="20" y="10" width="190" height="72" rx="3"/>
    <g className="vua-interior__graph">
      {shown.slice(1).map(([x, y], i) => <line key={`l${i}`} x1={shown[i]![0]} y1={shown[i]![1]} x2={x} y2={y}/>)}
      {shown.map(([x, y], i) => <circle key={i} cx={x} cy={y} r={i === 0 ? 7 : 5}/>)}
    </g>
    {[0, 1, 2].map(i => <g key={i} transform={`translate(${30 + i * 120} 112)`}>
      <rect className="vua-interior__screen" x="18" y="0" width="44" height="28" rx="2"/>
      <rect className="vua-interior__line" x="37" y="28" width="6" height="8"/>
      <rect className="vua-interior__surface" x="0" y="36" width="96" height="6" rx="1"/>
      <rect className="vua-interior__line" x="6" y="42" width="4" height="30"/><rect className="vua-interior__line" x="86" y="42" width="4" height="30"/>
      <path className="vua-interior__chair" d="M70 50 h18 v16 h-18 z M74 66 v12 M84 66 v12"/>
    </g>)}
    <line className="vua-interior__floor" x1="0" y1="186" x2="400" y2="186"/>
  </svg>;
}

export function MaterialsInterior({ packages }: { packages: number }) {
  const boxes = Math.min(packages, 24);
  return <svg {...svg}>
    {[0, 1, 2].map(unit => <g key={unit} transform={`translate(${24 + unit * 126} 16)`}>
      <rect className="vua-interior__line" x="0" y="0" width="4" height="170"/><rect className="vua-interior__line" x="104" y="0" width="4" height="170"/>
      {[0, 1, 2, 3].map(shelf => <g key={shelf}>
        <rect className="vua-interior__surface" x="0" y={40 + shelf * 42} width="108" height="4"/>
        {Array.from({ length: 2 }, (_, slot) => unit * 8 + shelf * 2 + slot < boxes
          ? <rect key={slot} className="vua-interior__box" x={12 + slot * 46} y={16 + shelf * 42} width={38} height={24} rx="2"/> : null)}
      </g>)}
    </g>)}
    <line className="vua-interior__floor" x1="0" y1="186" x2="400" y2="186"/>
  </svg>;
}

export function ReleaseInterior({ image }: { image: string | null }) {
  return <svg {...svg}>
    <path className="vua-interior__curtain" d="M0 0 h46 q-10 90 0 186 h-46 z M400 0 h-46 q10 90 0 186 h46 z"/>
    <path className="vua-interior__light" d="M150 0 l-60 160 h120 z"/><path className="vua-interior__light" d="M250 0 l-60 160 h120 z"/>
    {image ? <image href={image} x="140" y="14" width="120" height="150" preserveAspectRatio="xMidYMax meet"/>
      : <path className="vua-interior__figure" d="M200 40 a12 12 0 1 1 0.1 0 M188 58 h24 l6 52 h-10 l-2 48 h-12 l-2 -48 h-10 z"/>}
    <path className="vua-interior__stage" d="M70 160 h260 l28 26 h-316 z"/>
  </svg>;
}

export function WorkshopInterior({ states, running }: { states: Record<StationKey, StationState> | null; running: boolean }) {
  return <svg {...svg} data-running={running || undefined}>
    {STATIONS.map((key, i) => <g key={key} transform={`translate(${14 + i * 76} 30)`} data-state={states?.[key] ?? "todo"} className="vua-interior__machine">
      <rect x="0" y="0" width="64" height="70" rx="3"/>
      <rect className="vua-interior__lamp" x="24" y="8" width="16" height="6" rx="2"/>
      <text x="32" y="44" textAnchor="middle">{copy.stations[key]}</text>
      <rect className="vua-interior__line" x="28" y="70" width="8" height="40"/>
    </g>)}
    <rect className="vua-interior__belt" x="6" y="140" width="388" height="18" rx="9"/>
    <line className="vua-interior__belt-flow" x1="16" y1="149" x2="384" y2="149"/>
    {states && (() => {
      const at = STATIONS.findIndex(key => ["running", "waiting", "failed"].includes(states[key]));
      const index = at >= 0 ? at : STATIONS.length - 1;
      return <rect className="vua-interior__box" x={34 + index * 76} y="118" width="24" height="22" rx="2"/>;
    })()}
    <line className="vua-interior__floor" x1="0" y1="186" x2="400" y2="186"/>
  </svg>;
}
