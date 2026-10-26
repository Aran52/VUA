import semver from "semver";
export const OFFICIAL_BASE = "https://ph-r.com/vua/releases";

function version(value) {
  if (typeof value !== "string" || value.length > 160) throw Error("Invalid release version");
  const parsed = semver.parse(value);
  if (!parsed || parsed.version + (parsed.build.length ? `+${parsed.build.join(".")}` : "") !== value) throw Error("Invalid release version");
  return value;
}
export function officialReleaseUrl(releaseVersion) {
  return `${OFFICIAL_BASE}/v${encodeURIComponent(version(releaseVersion))}/`;
}
// v0.6.7 Main accepts only the canonical GitHub release-page form. Keep v0.1
// envelopes readable there; the new helper projects a page from its selected feed.
export function canonicalReleaseUrl(releaseVersion) {
  return `https://github.com/VUA-Project/VUA/releases/tag/v${encodeURIComponent(version(releaseVersion))}`;
}
export function componentSources(releaseVersion, name) {
  version(releaseVersion);
  const match = typeof name === "string" && /^vua-(host|amf)-(.+)\.bin$/.exec(name);
  if (!match) throw Error("Invalid component asset name");
  version(match[2]);
  return [
    `${officialReleaseUrl(releaseVersion)}${encodeURIComponent(name)}`,
    `https://github.com/VUA-Project/VUA/releases/download/v${encodeURIComponent(releaseVersion)}/${encodeURIComponent(name)}`,
  ];
}
