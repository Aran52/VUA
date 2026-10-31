/** Regional reference latency as tiers: the tile orders regions by closeness and
 * speaks in tiers (closest, about the same, then tier numbers); milliseconds stay
 * in the expanded rows. Pure presentation — measurements arrive from NetworkPanel. */
import type { WebsiteObservation } from "@vua/contracts";
import { Tooltip } from "../../components/primitives/Tooltip.tsx";
import { format, strings } from "../../i18n/index.ts";
import {
  REGIONAL_REFERENCES, referenceElapsed, regionalTierPhrase, regionalTiers,
  type RegionalReference,
} from "./regional-reference.ts";

const copy = strings.websiteTests;
export type RegionalCardState = { readonly busy: true } | { readonly busy: false; readonly result?: WebsiteObservation };
type RegionalStates = Record<string, RegionalCardState | undefined>;

const resultOf = (state: RegionalCardState | undefined): WebsiteObservation | undefined => state && !state.busy ? state.result : undefined;
const tiersOf = (states: RegionalStates) =>
  regionalTiers(REGIONAL_REFERENCES.map(reference => {
    const observation = resultOf(states[reference.url]);
    return observation ? { reference, observation } : { reference };
  }));
const referenceOf = (id: string) => REGIONAL_REFERENCES.find(region => region.id === id)!;

/** True once a successful regional measurement reports a proxy on this computer. */
export function regionalMeasuredViaProxy(states: RegionalStates): boolean {
  return REGIONAL_REFERENCES.some(reference => {
    const state = states[reference.url];
    return !!state && !state.busy && !!state.result && referenceElapsed(reference, state.result) !== null && state.result.regional?.viaProxy === true;
  });
}

function phraseText(phrase: "closest" | "about_same" | number): string {
  return phrase === "closest" ? copy.regionClosest : phrase === "about_same" ? copy.regionAboutSame : format(copy.regionTierN, { n: phrase });
}

/** Status text for a region without a tier: mid-test, never tested, or failed. */
function stateLabel(state: RegionalCardState | undefined): string {
  if (state?.busy) return copy.testing;
  if (!state) return "—";
  return state.result?.httpStatus !== null && state.result?.httpStatus !== undefined
    ? format(copy.httpStatus, { code: state.result.httpStatus })
    : copy.statuses[state.result?.status ?? "probe_error"];
}

/** The tile rows: every region in closeness order with its tier phrase. The tile
 * sits inside one toggle button, so the unsteady marker uses a hover title. */
export function RegionalTierSummary({ states }: { states: RegionalStates }) {
  const tiers = tiersOf(states);
  return <span className="vua-network-tile__results" aria-live="polite">
    {tiers.map((row, index) => {
      const phrase = regionalTierPhrase(tiers, index);
      return <span key={row.id} title={copy.regionCities[row.id]}>
        <span>{copy.regionShort[row.id]}</span>
        <span>{phrase === null ? stateLabel(states[referenceOf(row.id).url]) : phraseText(phrase)}</span>
        {row.unsteady ? <span className="vua-network-tile__unsteady" title={copy.regionUnsteadyHint}>{copy.regionUnsteady}</span> : null}
      </span>;
    })}
  </span>;
}

/** The expanded rows: the tile's order, with the measured milliseconds and a
 * per-region retest. */
export function RegionalTierList({ states, ready, onTest }: {
  states: RegionalStates;
  ready: boolean | null;
  onTest: (region: RegionalReference) => void;
}) {
  const tiers = tiersOf(states);
  return <section className="vua-network__references" aria-label={copy.regionTitle}>
    <p className="vua-text-secondary">{copy.regionScope}</p>
    <ul>{tiers.map((row, index) => {
      const region = referenceOf(row.id);
      const state = states[region.url];
      const phrase = regionalTierPhrase(tiers, index);
      const ms = state && !state.busy ? referenceElapsed(region, state.result) : null;
      return <li key={row.id}>
        <span title={`Oracle Cloud · ${row.id.toUpperCase()}`}><strong>{copy.regionCities[row.id]}</strong></span>
        {phrase !== null ? <span className="vua-network__tier">{phraseText(phrase)}{row.unsteady
          ? <Tooltip content={copy.regionUnsteadyHint}><span tabIndex={0} className="vua-network__unsteady">{copy.regionUnsteady}</span></Tooltip>
          : null}</span> : null}
        <button type="button" className="vua-network__region-test" data-region-test={row.id} title={region.url} disabled={!ready || state?.busy}
          aria-label={format(copy.testSite, { name: copy.regionCities[row.id] })} onClick={() => onTest(region)}>
          <span role="status">{state?.busy ? copy.testing : ms !== null ? format(copy.approximateMilliseconds, { ms }) : state ? stateLabel(state) : copy.test}</span>
        </button>
      </li>;
    })}</ul>
  </section>;
}
