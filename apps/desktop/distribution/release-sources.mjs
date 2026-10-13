export const OFFICIAL_BASE = "https://ph-r.com/vua/releases";

function version(value) {
  if (!/^\d+\.\d+\.\d+$/.test(value)) throw Error("Invalid release version");
  return value;
}
export function officialReleaseUrl(releaseVersion) {
  return `${OFFICIAL_BASE}/v${version(releaseVersion)}/`;
}
// v0.6.7 Main accepts only the canonical GitHub release-page form. Keep v0.1
// envelopes readable there; the new helper projects a page from its selected feed.
export function canonicalReleaseUrl(releaseVersion) {
  return `https://github.com/VUA-Project/VUA/releases/tag/v${version(releaseVersion)}`;
}
export function componentSources(releaseVersion, name) {
  version(releaseVersion);
  if (!/^vua-(host|amf)-\d+\.\d+\.\d+\.bin$/.test(name)) throw Error("Invalid component asset name");
  return [
    `${officialReleaseUrl(releaseVersion)}${name}`,
    `https://github.com/VUA-Project/VUA/releases/download/v${releaseVersion}/${name}`,
  ];
}
