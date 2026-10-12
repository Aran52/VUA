export const OFFICIAL_BASE = "https://ph-r.com/vua/releases";

function version(value) {
  if (!/^\d+\.\d+\.\d+$/.test(value)) throw Error("Invalid release version");
  return value;
}
export function officialReleaseUrl(releaseVersion) {
  return `${OFFICIAL_BASE}/v${version(releaseVersion)}/`;
}
export function componentSources(releaseVersion, name) {
  version(releaseVersion);
  if (!/^vua-(host|amf)-\d+\.\d+\.\d+\.bin$/.test(name)) throw Error("Invalid component asset name");
  return [
    `${officialReleaseUrl(releaseVersion)}${name}`,
    `https://github.com/VUA-Project/VUA/releases/download/v${releaseVersion}/${name}`,
  ];
}
