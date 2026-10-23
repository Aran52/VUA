import type { WebsiteObservation } from "@vua/contracts";

/** Public regional service origins, not VRChat/Photon room endpoints.
 * Region locations and API origins checked against Oracle's docs on 2026-10-11.
 * The native adapter pins these origins, warms three connections, then reports
 * the minimum of nine interleaved HEAD responses; redirects are never followed. */
export const REGIONAL_REFERENCES = [
  { id: "usw", url: "https://objectstorage.us-sanjose-1.oraclecloud.com/" },
  { id: "use", url: "https://objectstorage.us-ashburn-1.oraclecloud.com/" },
  { id: "jp", url: "https://objectstorage.ap-tokyo-1.oraclecloud.com/" },
  { id: "eu", url: "https://objectstorage.eu-amsterdam-1.oraclecloud.com/" },
] as const;
export type RegionalReference = typeof REGIONAL_REFERENCES[number];

/** An anonymous HEAD to the service root normally returns 404 with no body.
 * That is a timed response, not a successful API call. Redirects, rate limits
 * and transport failures never become a regional latency number. */
export function referenceElapsed(reference: RegionalReference, observation?: WebsiteObservation): number | null {
  if (!observation || observation.url !== reference.url) return null;
  return observation.status === "reachable" || (observation.status === "http_error" && observation.httpStatus === 404)
    ? observation.elapsedMs : null;
}

/** Tier and steadiness calibration for the regional tile; the author adjusts
 * these after comparing the tile with in-game Ping. */
export const REGIONAL_TIER_ABSOLUTE_MS = 15;
export const REGIONAL_TIER_RELATIVE = 0.1;
export const REGIONAL_UNSTEADY_ABSOLUTE_MS = 20;
export const REGIONAL_UNSTEADY_RELATIVE = 0.5;

/** One region's place in the ordered tier model: `value` is the measured
 * minimum in ms, `tier` groups regions close to the tier's first region, and
 * `unsteady` marks a wide spread between the minimum and the p90. Regions
 * without a successful measurement sort last without a tier. */
export interface RegionalTier {
  readonly id: RegionalReference["id"];
  readonly value: number | null;
  readonly tier: number | null;
  readonly unsteady: boolean;
}

/** Orders the four regions by value and groups close ones into tiers. Within
 * one tier every region stays inside the wider of 15 ms or 10 % of the tier's
 * first region. Pure model; the tile wires it up separately. */
export function regionalTiers(
  inputs: readonly { readonly reference: RegionalReference; readonly observation?: WebsiteObservation }[],
): readonly RegionalTier[] {
  const rows = inputs.map(({ reference, observation }) => ({
    id: reference.id,
    value: referenceElapsed(reference, observation),
    regional: observation?.regional ?? null,
  }));
  const measured = rows
    .filter((row): row is typeof row & { value: number } => row.value !== null)
    .sort((left, right) => left.value - right.value);
  const unmeasured = rows.filter(row => row.value === null);
  const result: RegionalTier[] = [];
  let tier = -1;
  let tierFirstValue = 0;
  for (const { id, value, regional } of measured) {
    const boundary = Math.max(REGIONAL_TIER_ABSOLUTE_MS, REGIONAL_TIER_RELATIVE * tierFirstValue);
    if (tier < 0 || value - tierFirstValue > boundary) {
      tier += 1;
      tierFirstValue = value;
    }
    result.push({
      id,
      value,
      tier,
      unsteady: regional !== null
        && regional.p90Ms - value > Math.max(REGIONAL_UNSTEADY_ABSOLUTE_MS, REGIONAL_UNSTEADY_RELATIVE * value),
    });
  }
  return [...result, ...unmeasured.map(row => ({ id: row.id, value: null, tier: null, unsteady: false }))];
}

/** Display phrase for one row of `regionalTiers` output: the first row (the
 * closest region) reads "closest", a row sharing the previous row's tier reads
 * "about the same", and a row opening a later tier reads as that tier's number
 * (the second tier is 2). Rows without a measurement have no phrase. */
export function regionalTierPhrase(tiers: readonly RegionalTier[], index: number): "closest" | "about_same" | number | null {
  const row = tiers[index];
  if (!row || row.tier === null) return null;
  if (index === 0) return "closest";
  return row.tier === tiers[index - 1]!.tier ? "about_same" : row.tier + 1;
}
