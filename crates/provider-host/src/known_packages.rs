//! The third-party packages AMF recognizes in selected materials, and how each
//! one is delivered. `docs/development/known-packages.md` records the license
//! evidence and shows how to add an entry.
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::Path,
};
use vua_orchestrator::{Diagnostic, DiagnosticSeverity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Delivery {
    /// Preparation installs the package from its official feed when materials need it.
    Auto,
    /// Preparation never installs it; a diagnostic points at the official page.
    Guide,
    /// Ships with the VRChat SDK; materials are detected but nothing is installed.
    Bundled,
}

pub(crate) struct KnownPackage {
    pub id: &'static str,
    pub display_name: &'static str,
    pub feed_url: &'static str,
    /// Where the player obtains the package; used by the `amf.package.guide` diagnostic.
    pub official_url: &'static str,
    /// License evidence is review data recorded alongside the table, not runtime input.
    #[allow(dead_code)]
    pub license: &'static str,
    #[allow(dead_code)]
    pub license_url: &'static str,
    /// Day the license text was read, as YYYY-MM-DD.
    #[allow(dead_code)]
    pub license_checked: &'static str,
    pub delivery: Delivery,
    /// Serialized material properties only this package writes.
    pub material_markers: &'static [&'static str],
    /// Lower-case shader GUIDs read from the official package's `.meta` files.
    pub shader_guids: &'static [&'static str],
    /// Legacy install locations used when the cached feed declares none.
    pub legacy_folders: &'static [&'static str],
    pub legacy_files: &'static [&'static str],
}

impl KnownPackage {
    fn detects_materials(&self) -> bool {
        !self.material_markers.is_empty() || !self.shader_guids.is_empty()
    }
}

pub(crate) const MODULAR_AVATAR_FEED: &str = "https://vpm.nadena.dev/vpm.json";
const LILTOON_FEED: &str = "https://lilxyzw.github.io/vpm-repos/vpm.json";
const POIYOMI_FEED: &str = "https://poiyomi.github.io/vpm/index.json";
const UNLITWF_FEED: &str = "https://whiteflare.github.io/vpm-repos/vpm.json";
const VRCHAT_FEED: &str = "https://vrchat.github.io/packages/index.json";
const VRCHAT_SDK_LICENSE: &str = "https://hello.vrchat.com/legal/sdk";
const CHECKED: &str = "2026-10-18";

pub(crate) static KNOWN_PACKAGES: &[KnownPackage] = &[
    KnownPackage {
        id: "jp.lilxyzw.liltoon",
        display_name: "lilToon",
        feed_url: LILTOON_FEED,
        official_url: "https://lilxyzw.github.io/lilToon/",
        license: "MIT",
        license_url: "https://github.com/lilxyzw/lilToon/blob/master/LICENSE",
        license_checked: CHECKED,
        delivery: Delivery::Auto,
        material_markers: &["_lilToonVersion"],
        shader_guids: &[],
        legacy_folders: &["Assets\\lilToon"],
        legacy_files: &[],
    },
    KnownPackage {
        id: "com.poiyomi.toon",
        display_name: "Poiyomi Toon Shader",
        feed_url: POIYOMI_FEED,
        official_url: "https://www.poiyomi.com/",
        license: "MIT",
        license_url: "https://github.com/poiyomi/PoiyomiToonShader/blob/master/LICENSE.txt",
        license_checked: CHECKED,
        delivery: Delivery::Auto,
        material_markers: &[],
        shader_guids: POIYOMI_TOON_GUIDS,
        legacy_folders: &["Assets\\_PoiyomiShaders"],
        legacy_files: &[],
    },
    KnownPackage {
        id: "com.poiyomi.pro",
        display_name: "Poiyomi Pro",
        feed_url: POIYOMI_FEED,
        official_url: "https://www.poiyomi.com/",
        license: "Proprietary/EULA",
        license_url: "https://www.poiyomi.com/",
        license_checked: CHECKED,
        delivery: Delivery::Guide,
        material_markers: &[],
        shader_guids: &[],
        legacy_folders: &[],
        legacy_files: &[],
    },
    KnownPackage {
        id: "jp.whiteflare.unlitwf",
        display_name: "UnlitWF Shader",
        feed_url: UNLITWF_FEED,
        official_url: "https://github.com/whiteflare/Unlit_WF_ShaderSuite",
        license: "Zlib",
        license_url: "https://github.com/whiteflare/Unlit_WF_ShaderSuite/blob/master/LICENSE",
        license_checked: CHECKED,
        delivery: Delivery::Auto,
        material_markers: &[],
        shader_guids: UNLITWF_GUIDS,
        legacy_folders: &["Assets\\Unlit_WF_ShaderSuite"],
        legacy_files: &[],
    },
    KnownPackage {
        id: "com.vrchat.base",
        display_name: "VRChat Toon Standard",
        feed_url: VRCHAT_FEED,
        official_url: "https://creators.vrchat.com/avatars/shaders/",
        license: "Proprietary/EULA",
        license_url: VRCHAT_SDK_LICENSE,
        license_checked: CHECKED,
        delivery: Delivery::Bundled,
        material_markers: &[],
        shader_guids: TOON_STANDARD_GUIDS,
        legacy_folders: &[],
        legacy_files: &[],
    },
    KnownPackage {
        id: "com.vrchat.avatars",
        display_name: "VRChat SDK - Avatars",
        feed_url: VRCHAT_FEED,
        official_url: "https://creators.vrchat.com/sdk/",
        license: "Proprietary/EULA",
        license_url: VRCHAT_SDK_LICENSE,
        license_checked: CHECKED,
        delivery: Delivery::Auto,
        material_markers: &[],
        shader_guids: &[],
        legacy_folders: &[],
        legacy_files: &[],
    },
    KnownPackage {
        id: "nadena.dev.modular-avatar",
        display_name: "Modular Avatar",
        feed_url: MODULAR_AVATAR_FEED,
        official_url: "https://modular-avatar.nadena.dev/",
        license: "MIT",
        license_url: "https://github.com/bdunderscore/modular-avatar/blob/main/COPYING.md",
        license_checked: CHECKED,
        delivery: Delivery::Auto,
        material_markers: &[],
        shader_guids: &[],
        legacy_folders: &[],
        legacy_files: &[],
    },
    KnownPackage {
        id: "nadena.dev.ndmf",
        display_name: "Non-Destructive Modular Framework",
        feed_url: MODULAR_AVATAR_FEED,
        official_url: "https://ndmf.nadena.dev/",
        license: "MIT",
        license_url: "https://github.com/bdunderscore/ndmf/blob/main/COPYING.md",
        license_checked: CHECKED,
        delivery: Delivery::Auto,
        material_markers: &[],
        shader_guids: &[],
        legacy_folders: &[],
        legacy_files: &[],
    },
];

/// The package a serialized `.mat` needs, and the shader GUID the material references.
pub(crate) struct MaterialMatch {
    pub package: usize,
    pub shader_guid: Option<String>,
}

/// Marker properties win over the GUID table, keeping lilToon's historical
/// `_lilToonVersion` rule; other packages match their shader GUIDs.
pub(crate) fn detect_material(table: &[KnownPackage], material: &[u8]) -> Option<MaterialMatch> {
    let text = String::from_utf8_lossy(material);
    let guid = shader_guid(&text);
    for (index, package) in table.iter().enumerate() {
        if package.material_markers.iter().any(|marker| text.contains(marker)) {
            return Some(MaterialMatch { package: index, shader_guid: guid });
        }
    }
    let guid = guid?;
    table
        .iter()
        .enumerate()
        .find(|(_, package)| package.shader_guids.contains(&guid.as_str()))
        .map(|(index, _)| MaterialMatch { package: index, shader_guid: Some(guid) })
}

/// The referenced shader GUID in lower case, parsed like the historical lilToon rule.
fn shader_guid(text: &str) -> Option<String> {
    let reference = text.lines().find_map(|line| line.trim_start().strip_prefix("m_Shader:"))?;
    let guid = reference.split("guid:").nth(1)?.split([',', '}']).next()?.trim().to_ascii_lowercase();
    (guid.len() == 32 && guid.bytes().all(|c| c.is_ascii_hexdigit())).then_some(guid)
}

/// What one package needs from preparation: a bundled legacy copy arrived, and the
/// shader GUIDs the selected materials reference.
#[derive(Default)]
pub(crate) struct PackageNeed {
    pub bundled_copy: bool,
    pub shaders: BTreeSet<String>,
}
pub(crate) type PackageNeeds = BTreeMap<usize, PackageNeed>;

/// The `auto` packages to install, in table order. A package is required when a
/// bundled legacy copy arrived or a detected shader is not provided by the selected
/// sources. Recipe pins win, and a package the project already carries is not reinstalled.
pub(crate) fn auto_install_plan(
    table: &[KnownPackage],
    needs: &PackageNeeds,
    provided_guids: &HashMap<String, String>,
    package_pins: &BTreeMap<String, String>,
    installed: &serde_json::Value,
) -> Vec<usize> {
    needs
        .iter()
        .filter(|(index, need)| {
            let package = &table[**index];
            matches!(package.delivery, Delivery::Auto)
                && (need.bundled_copy || need.shaders.iter().any(|guid| !provided_guids.contains_key(guid)))
                && !package_pins.contains_key(package.id)
                && installed.get(package.id).is_none()
        })
        .map(|(index, _)| *index)
        .collect()
}

/// The Recipe-pinned packages whose feeds project creation registers. Packages project
/// creation already installs through its own path (the VRChat SDK, Modular Avatar and
/// NDMF) keep that path; only material-driven `auto` packages need their feed here.
pub(crate) fn pinned_auto_packages<'a>(
    table: &'a [KnownPackage],
    package_pins: &'a BTreeMap<String, String>,
) -> impl Iterator<Item = &'a KnownPackage> {
    table.iter().filter(move |package| {
        matches!(package.delivery, Delivery::Auto)
            && package.detects_materials()
            && package_pins.contains_key(package.id)
    })
}

/// One package's legacy install locations, lower case with `/` separators.
pub(crate) struct LegacyEntry {
    package: usize,
    folders: Vec<String>,
    files: Vec<String>,
}

/// Legacy declarations of the packages whose bundled copies preparation replaces with
/// the official package. Files of every other package import unchanged, so entries
/// limited to detection, guidance or the SDK's own installation keep their old behavior.
pub(crate) fn legacy_entries(table: &[KnownPackage], environment_root: Option<&Path>) -> Vec<LegacyEntry> {
    table
        .iter()
        .enumerate()
        .filter(|(_, package)| matches!(package.delivery, Delivery::Auto) && package.detects_materials())
        .map(|(index, package)| {
            let cached = environment_root.and_then(|root| cached_legacy_declaration(root, package.feed_url, package.id));
            let (folders, files) = effective_legacy_paths(package, cached);
            LegacyEntry { package: index, folders, files }
        })
        .filter(|entry| !entry.folders.is_empty() || !entry.files.is_empty())
        .collect()
}

/// The package whose legacy install location holds this folded asset path.
pub(crate) fn legacy_owner(entries: &[LegacyEntry], folded_path: &str) -> Option<usize> {
    entries
        .iter()
        .find(|entry| {
            entry.files.iter().any(|file| folded_path == file)
                || entry.folders.iter().any(|folder| {
                    folded_path == folder
                        || (folded_path.len() > folder.len()
                            && folded_path.starts_with(folder)
                            && folded_path.as_bytes()[folder.len()] == b'/')
                })
        })
        .map(|entry| entry.package)
}

/// A feed declaration wins when it names paths; an empty or absent declaration falls
/// back to the table so a feed without legacy metadata keeps the default rule.
fn effective_legacy_paths(
    package: &KnownPackage,
    cached: Option<(Vec<String>, Vec<String>)>,
) -> (Vec<String>, Vec<String>) {
    let normalize = |paths: Vec<String>| -> Vec<String> {
        paths
            .into_iter()
            .map(|path| path.replace('\\', "/").trim_end_matches('/').to_ascii_lowercase())
            .filter(|path| !path.is_empty())
            .collect()
    };
    let (folders, files) = cached.unwrap_or_default();
    let folders = if folders.is_empty() {
        package.legacy_folders.iter().map(|path| (*path).to_owned()).collect()
    } else {
        folders
    };
    let files = if files.is_empty() {
        package.legacy_files.iter().map(|path| (*path).to_owned()).collect()
    } else {
        files
    };
    (normalize(folders), normalize(files))
}

/// The `legacyFolders`/`legacyFiles` paths a feed manifest declares for one package,
/// joined across its cached versions. `None` means the manifest does not carry the package.
pub(crate) fn legacy_declaration(feed: &serde_json::Value, package_id: &str) -> Option<(Vec<String>, Vec<String>)> {
    let versions = feed.get("packages")?.get(package_id)?.get("versions")?.as_object()?;
    let mut folders = BTreeSet::new();
    let mut files = BTreeSet::new();
    for version in versions.values() {
        collect_legacy_paths(version.get("legacyFolders"), &mut folders);
        collect_legacy_paths(version.get("legacyFiles"), &mut files);
    }
    Some((folders.into_iter().collect(), files.into_iter().collect()))
}

fn collect_legacy_paths(value: Option<&serde_json::Value>, into: &mut BTreeSet<String>) {
    match value {
        Some(serde_json::Value::Object(map)) => into.extend(map.keys().cloned()),
        Some(serde_json::Value::Array(list)) => {
            into.extend(list.iter().filter_map(|item| item.as_str().map(str::to_owned)))
        }
        _ => {}
    }
}

/// Reads a subscribed feed's local cache without network access. The cache file name
/// matches `VrcGetLibBackend::add_remote_repo`; a missing or unreadable cache simply
/// leaves the table's fallback declarations in charge.
fn cached_legacy_declaration(environment_root: &Path, feed_url: &str, package_id: &str) -> Option<(Vec<String>, Vec<String>)> {
    let path = environment_root
        .join("Repos")
        .join(format!("{}.json", vua_orchestrator::fnv1a_hex(feed_url.as_bytes())));
    let feed: serde_json::Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    legacy_declaration(&feed, package_id)
}

/// One `amf.package.guide` warning per package the missing materials point at: every
/// recognized `guide` package, and every recognized `auto` package the project does
/// not carry, whose automatic installation has not landed.
pub(crate) fn guide_diagnostics<'a>(
    table: &[KnownPackage],
    materials: impl IntoIterator<Item = &'a [u8]>,
    installed_versions: &BTreeMap<String, String>,
) -> Vec<Diagnostic> {
    let mut counts = BTreeMap::<usize, usize>::new();
    for material in materials {
        if let Some(found) = detect_material(table, material) {
            *counts.entry(found.package).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(index, count)| {
            let package = &table[index];
            let guided = match package.delivery {
                Delivery::Guide => true,
                Delivery::Auto => !installed_versions.contains_key(package.id),
                Delivery::Bundled => false,
            };
            guided.then(|| Diagnostic {
                code: "amf.package.guide".into(),
                severity: DiagnosticSeverity::Warning,
                message: format!(
                    "{count} material{} need{} {}; get it from {}.",
                    if count == 1 { "" } else { "s" },
                    if count == 1 { "s" } else { "" },
                    package.display_name,
                    package.official_url
                ),
            })
        })
        .collect()
}

// Read from com.poiyomi.toon 9.3.64: every `_PoiyomiShaders/Shaders/**.shader.meta`.
const POIYOMI_TOON_GUIDS: &[&str] = &[
    "00a679886ff24e74585a2a9d37b3c8d7",
    "034460dc851505a42baca2ba6514f9f0",
    "104003ab15a9f7a47b6ef0597225e771",
    "1513582543e8f6c44b1981e8689f7a52",
    "18de0e547c911de4e804ceaaa97a0b26",
    "1b58d372fe360874a8fb767d53f955b0",
    "21e56665a7e45184b9013fb9e4f7bc96",
    "231d95ff7213dd74f82af136b256fb32",
    "23f6705aff8bf964c87bfb3dd66ab345",
    "24edb77705ad65a49afb831f72747592",
    "3210d19aa4851e148b6848177a017f02",
    "3eefd743f727e99419fa501e889c4998",
    "47c3851517c96c84aae06d328a2581f6",
    "5ab467a3a3d657b40836c4493ff3e51d",
    "5ca92f1e9fc35504aba297fd1acd62df",
    "68d4032c05798c44eafe02f272e6895c",
    "73da03e2cd0afa940968d9c487e2474d",
    "7845a1ca48a75804c8925b2bd5680f7f",
    "784867d15973a4847b1a6048d42c1565",
    "7bca35b6e5a09ee4cbdff5cc54a9bd49",
    "917b37092bae034459c28c00a3a19b54",
    "9444ce77bf4418748b1e8591b9d97f85",
    "9cdbe9241ea4aaa48944028272700834",
    "a49dd2fba12886c499261512c6eaaa80",
    "a6839ad792f95aa41a941c7317aa1fe1",
    "aa7e588ee69e46f429e3daa4b3068311",
    "add1d40d10695a24b9e91e172ccb9525",
    "bdbac715113578048ab7114aa3379a1d",
    "c0aca4001823bf74e8a9538b3075bf56",
    "c933bce24d9d90747a5def016febd042",
    "cef5dc16b30fdbb46b08ab424d41d1c0",
    "d0091594032d90d46b8d5eaeaf065a17",
    "d141019541e4e44479fbd71ad42a2cf6",
    "e99d2df55dac44a4c921122fe7cf6ba7",
    "eda2412ac7ab2db45a47a521f6d7d8a6",
    "fc2c2d7b9ec11d34785997ccbb86f496",
];

// Read from jp.whiteflare.unlitwf 2.9.0: every `Runtime/Shaders/**.shader.meta`.
const UNLITWF_GUIDS: &[&str] = &[
    "0299954f2a9b0994f8c9587945948766",
    "06e9294a93df4474cac2f4157b5e1d1d",
    "0733cfc88032e8d4eafce250263c497c",
    "0a7a6cdca16a38548a5d81aca8d4e3ba",
    "0b53cf0bcd0f9db4fa9d1297d255d06d",
    "0c20402a658b66444a95461b76f367b2",
    "1332d868874e61e44b48c1f59bf4bd11",
    "1435581bcf13e7a47b5bf5636f8d8252",
    "15212414cba0c7a4aac92d94a4ae8750",
    "152ebc8c9a60c0e43820c71a0f0dfbb6",
    "17124f35e2c0c5244a712e8654a994f9",
    "188bb4cd384ba624892e50b5b35f7297",
    "21f6eaa1dd1f25c4cb29a42c4ff5d98f",
    "2210f95a2274e9d4faf8a14dac933fdb",
    "28e73745c5d1e464a876038f875501b4",
    "2cf66b0706c40744baab089297afa895",
    "2d294f328149d944eb0899b452ff879c",
    "2dafe14658b486b4bb44869606d30d67",
    "2efe527cfcbf0e1408b67463225f552f",
    "3276a740671679f44b1141523bf73e33",
    "3bde56820d1aece41bd22966876a061c",
    "3c07b964e541eef45bc195a029b878b3",
    "45e1aee78764bec499e99106c330f95a",
    "46e58649f5c3de74a9f578af4e6d30b5",
    "4ba701b07ccc81e4aae7f053bf332eab",
    "4bd76f6599a5b8e4d88d81300fb74c37",
    "4e4be4aab63a2bd4fbcea2390ae92fdf",
    "4eef00f52cc21b04e9e34e4caefa6bbf",
    "5498b01615002d948bea7542f55e0c07",
    "5523e041d29d259439fa14bd131f5c82",
    "58bb80b63bec29d4384e105c53ca6970",
    "58ccf9c912b226146a25726b8a1f04db",
    "5c1fc3340b9976949b12562190be5db0",
    "5c54a3a3c81772549819db795008c986",
    "64bf3ca653a7b274fab3e8a87016bfb0",
    "660abd485057f4740ac9050f7ab3237d",
    "6bf04fb5e50a2de4dbf06063f8592ab0",
    "7240817400475dd41b084e32b1264d6f",
    "747bf283d686334469fb662b2fc4a5c2",
    "78d2e3fa0b8eb674aa9cf9e048f79c93",
    "7be712530731fbd44ae755f12dbc6d07",
    "871fd7a51a8ea3e4980c3fe7b8347619",
    "8bc13844fa3debd40aa4cd72fdc3900b",
    "8c7888a4ac175584f81e0b6e7d4af5a7",
    "90cac9ec3b2a7524eb99b36ab87f25f1",
    "92675e22926880147a67c25086c3bb70",
    "9350854c6e88f3f4eb873d2f94ff3328",
    "94ee7f8988740fd4887f8b1ce41f0c1c",
    "98bb3de5d5444094aa041a65f8a85708",
    "9d5847e3fa0700f4dab38227bf565df2",
    "a220e3e0675cc3f4f817a485688788d6",
    "a3641cf3b665bec468ac868660864d64",
    "a3678756e883b9349ac22fce33313139",
    "a3777cac8e82d944bba542a87aa5368c",
    "a5ae7f40ac53e274ea0bc1262e1f6895",
    "aab16a18215b5e2488031de410fda510",
    "ab4eb87c406a22f46887cf72178e2685",
    "ad88000744b4fb241835ba6ec106caf4",
    "af3422dc9372a89449a9f44d409d9714",
    "af51615040dcdad4cb01c29ea34dbb03",
    "bd20e788383cdb943b829f230eb1c5ec",
    "c0f75d3ed420fd144a74722588d3bc21",
    "c6e2e8e77eaf9bb4b82f757fe9b823f0",
    "c7e5995223250464cac205689e058693",
    "ccf60bd2b8af3e94ebc1a310962dac61",
    "cda050f4096b1df478fdc6fcc89586c7",
    "d01a5c313ada49e488b2ef8c6b00f56d",
    "d1e7b0a18e221a1409ad59065ec157e4",
    "d242cb83664caae4f957030870dd801d",
    "d279a88eda1ae0e4c89e92539639eb16",
    "d52eb08feafe8de4386757d285f9c26a",
    "dd3a683002b3a6f43bdb6c97bd0985c1",
    "dfb821bc7afadc14591e4338a8ec865f",
    "e0b93fdad2eeedf42baccbc0975cdd1d",
    "e4b04fed7c539d84887123c0beeffaca",
    "e7263331a8ee0a04aa4a271fc1fef104",
    "ec023985bf0f3f44bb6fe6bf962c2c69",
    "f19f31eb9bfd5154ba2c1a41c54a908b",
    "f3f80636c64e389498b3b19e2ee218da",
    "f5d5b7934ab1854418baf5042b0f7453",
    "fb639668f92376a4786d635258ae96da",
    "fc2c3716646379d47809d35f53238f82",
    "fcbd21eca211262498a416d29e6b050e",
    "ffa6a9ecd57d50a4db625f7ed4fabeeb",
];

// Read from com.vrchat.base 3.10.5: `Runtime/VRCSDK/Sample Assets/Shaders/Mobile/
// ToonStandard/ToonStandard.shader.meta` and `ToonStandardOutline.shader.meta`.
const TOON_STANDARD_GUIDS: &[&str] = &[
    "051a0ed2f2aedd741aa8186ae92f97e0",
    "e765db0afa7ecfc44ade2e4e2491f65a",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn index(table: &[KnownPackage], id: &str) -> usize {
        table.iter().position(|package| package.id == id).unwrap()
    }

    fn material(guid: &str, extra: &str) -> Vec<u8> {
        format!("Material:\n  m_Shader: {{fileID: 4800000, guid: {guid}, type: 3}}\n{extra}").into_bytes()
    }

    fn need(bundled_copy: bool, shaders: &[&str]) -> PackageNeed {
        PackageNeed {
            bundled_copy,
            shaders: shaders.iter().map(|shader| (*shader).to_owned()).collect(),
        }
    }

    #[test]
    fn shader_guid_signals_name_poiyomi_unlitwf_and_toon_standard() {
        for (guid, id) in [
            ("9444CE77BF4418748B1E8591B9D97F85", "com.poiyomi.toon"),
            ("a3678756e883b9349ac22fce33313139", "jp.whiteflare.unlitwf"),
            ("e765db0afa7ecfc44ade2e4e2491f65a", "com.vrchat.base"),
        ] {
            let found = detect_material(KNOWN_PACKAGES, &material(guid, "")).unwrap();
            assert_eq!(KNOWN_PACKAGES[found.package].id, id);
            assert_eq!(found.shader_guid.as_deref(), Some(guid.to_ascii_lowercase().as_str()));
        }
        assert!(detect_material(KNOWN_PACKAGES, &material("0123456789abcdef0123456789abcdef", "")).is_none());
        assert!(detect_material(KNOWN_PACKAGES, b"Material:\n  m_Shader: {fileID: 46, type: 0}\n").is_none());
    }

    #[test]
    fn legacy_copies_match_feed_declarations_then_table_fallbacks() {
        let entries = legacy_entries(KNOWN_PACKAGES, None);
        let owners: Vec<_> = entries.iter().map(|entry| KNOWN_PACKAGES[entry.package].id).collect();
        assert_eq!(owners, ["jp.lilxyzw.liltoon", "com.poiyomi.toon", "jp.whiteflare.unlitwf"]);
        for (path, id) in [
            ("assets/liltoon", "jp.lilxyzw.liltoon"),
            ("assets/liltoon/shader/lts.shader", "jp.lilxyzw.liltoon"),
            ("assets/_poiyomishaders/shaders/9.3/toon/poiyomi toon.shader", "com.poiyomi.toon"),
            ("assets/unlit_wf_shadersuite/runtime/shaders/unlit_wf_gem_opaque.shader", "jp.whiteflare.unlitwf"),
        ] {
            let owner = legacy_owner(&entries, path).unwrap();
            assert_eq!(KNOWN_PACKAGES[owner].id, id, "{path}");
        }
        for path in ["assets/liltoonextras/readme.txt", "assets/amenbo/tshirt&shorts/material/liltoon.mat", "assets/_poiyomishaders.txt"] {
            assert_eq!(legacy_owner(&entries, path), None, "{path}");
        }
    }

    #[test]
    fn a_bundled_legacy_copy_is_skipped_and_still_installs_the_official_package() {
        let entries = legacy_entries(KNOWN_PACKAGES, None);
        let liltoon = index(KNOWN_PACKAGES, "jp.lilxyzw.liltoon");
        // The TshirtShorts package's bundled lilToon_1.4.0.unitypackage sits under Assets/lilToon.
        assert_eq!(legacy_owner(&entries, "assets/liltoon/liltoon_1.4.0.unitypackage"), Some(liltoon));
        let needs = PackageNeeds::from([(liltoon, need(true, &[]))]);
        assert_eq!(
            auto_install_plan(KNOWN_PACKAGES, &needs, &HashMap::new(), &BTreeMap::new(), &serde_json::json!({})),
            [liltoon]
        );
    }

    #[test]
    fn table_entries_carry_evidence_and_pin_only_material_driven_feeds() {
        for package in KNOWN_PACKAGES {
            assert!(package.id.contains('.'), "{}", package.id);
            assert!(package.feed_url.starts_with("https://"), "{}", package.id);
            assert!(package.official_url.starts_with("https://"), "{}", package.id);
            assert!(!package.license.is_empty(), "{}", package.id);
            assert!(package.license_url.starts_with("https://"), "{}", package.id);
            assert_eq!(package.license_checked.len(), 10, "{}", package.id);
            assert!(
                package.shader_guids.iter().all(|guid| {
                    guid.len() == 32 && guid.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                }),
                "{}",
                package.id
            );
        }
        // SDK and Modular Avatar pins keep their existing registration path during
        // project creation; only material-driven auto packages register their feed.
        let pins = BTreeMap::from([
            ("com.vrchat.avatars".to_owned(), "3.10.5".to_owned()),
            ("nadena.dev.modular-avatar".to_owned(), "1.18.7".to_owned()),
            ("jp.lilxyzw.liltoon".to_owned(), "1.10.0".to_owned()),
            ("com.poiyomi.toon".to_owned(), "9.3.64".to_owned()),
        ]);
        let ids: Vec<_> = pinned_auto_packages(KNOWN_PACKAGES, &pins).map(|package| package.id).collect();
        assert_eq!(ids, ["jp.lilxyzw.liltoon", "com.poiyomi.toon"]);
    }

    #[test]
    fn cached_feed_manifests_override_legacy_defaults_without_network() {
        let root = std::env::temp_dir().join(format!("vua-known-packages-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let liltoon = index(KNOWN_PACKAGES, "jp.lilxyzw.liltoon");
        let feed_url = KNOWN_PACKAGES[liltoon].feed_url;
        let cache = root.join("Repos").join(format!("{}.json", vua_orchestrator::fnv1a_hex(feed_url.as_bytes())));
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::write(&cache, serde_json::to_vec(&serde_json::json!({
            "packages": {"jp.lilxyzw.liltoon": {"versions": {
                "1.0.0": {"legacyFolders": {"Assets\\CustomLil": ""}},
                "2.0.0": {"legacyFiles": {"Assets\\Lil\\Old.shader": ""}}
            }}}
        })).unwrap()).unwrap();
        let entries = legacy_entries(KNOWN_PACKAGES, Some(&root));
        let entry = entries.iter().find(|entry| entry.package == liltoon).unwrap();
        assert_eq!(entry.folders, ["assets/customlil"]);
        assert_eq!(entry.files, ["assets/lil/old.shader"]);
        assert_eq!(legacy_owner(&entries, "assets/customlil/one.shader"), Some(liltoon));
        assert_eq!(legacy_owner(&entries, "assets/liltoon/one.shader"), None, "the feed declaration replaces the default folder");

        std::fs::write(&cache, b"{\"packages\":{\"jp.lilxyzw.liltoon\":{\"versions\":{\"1.0.0\":{}}}}}").unwrap();
        let entries = legacy_entries(KNOWN_PACKAGES, Some(&root));
        assert_eq!(legacy_owner(&entries, "assets/liltoon"), Some(liltoon), "an empty declaration falls back to the table");
        std::fs::write(&cache, b"not json").unwrap();
        assert_eq!(legacy_owner(&legacy_entries(KNOWN_PACKAGES, Some(&root)), "assets/liltoon"), Some(liltoon));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn auto_installs_cover_first_preparation_and_material_changes() {
        let liltoon = index(KNOWN_PACKAGES, "jp.lilxyzw.liltoon");
        let poiyomi = index(KNOWN_PACKAGES, "com.poiyomi.toon");
        let shader = "df12117ecd77c31469c224178886498e";
        let no_pins = BTreeMap::new();
        let no_guids = HashMap::new();
        let needs = PackageNeeds::from([(liltoon, need(false, &[shader]))]);

        // First preparation: the project carries nothing yet.
        assert_eq!(auto_install_plan(KNOWN_PACKAGES, &needs, &no_guids, &no_pins, &serde_json::json!({})), [liltoon]);
        // A material change on a line that already installed lilToon installs nothing.
        let installed = serde_json::json!({"jp.lilxyzw.liltoon": "1.10.0"});
        assert!(auto_install_plan(KNOWN_PACKAGES, &needs, &no_guids, &no_pins, &installed).is_empty());
        // A Recipe pin owns the version instead.
        let pins = BTreeMap::from([("jp.lilxyzw.liltoon".to_owned(), "1.8.0".to_owned())]);
        assert!(auto_install_plan(KNOWN_PACKAGES, &needs, &no_guids, &pins, &serde_json::json!({})).is_empty());
        // A selected source carrying the shader provides it already.
        let provided = HashMap::from([(shader.to_owned(), "Assets/x.shader".to_owned())]);
        assert!(auto_install_plan(KNOWN_PACKAGES, &needs, &provided, &no_pins, &serde_json::json!({})).is_empty());
        // A bundled legacy copy alone still installs the official package.
        let bundled = PackageNeeds::from([(liltoon, need(true, &[]))]);
        assert_eq!(auto_install_plan(KNOWN_PACKAGES, &bundled, &no_guids, &no_pins, &serde_json::json!({})), [liltoon]);
        // Several missing auto packages install in table order.
        let both = PackageNeeds::from([(poiyomi, need(false, &["9444ce77bf4418748b1e8591b9d97f85"])), (liltoon, need(false, &[shader]))]);
        assert_eq!(auto_install_plan(KNOWN_PACKAGES, &both, &no_guids, &no_pins, &serde_json::json!({})), [liltoon, poiyomi]);
        // Guide and SDK-bundled packages are never installed by this path.
        let guided = PackageNeeds::from([
            (index(KNOWN_PACKAGES, "com.poiyomi.pro"), need(true, &[])),
            (index(KNOWN_PACKAGES, "com.vrchat.base"), need(false, &["e765db0afa7ecfc44ade2e4e2491f65a"])),
        ]);
        assert!(auto_install_plan(KNOWN_PACKAGES, &guided, &no_guids, &no_pins, &serde_json::json!({})).is_empty());
    }

    #[test]
    fn guide_diagnostics_name_the_package_material_count_and_official_page() {
        const AUTO_GUID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        const BUNDLED_GUID: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let entry = |id: &'static str, display_name: &'static str, delivery: Delivery, markers: &'static [&'static str], guids: &'static [&'static str]| KnownPackage {
            id, display_name, feed_url: "", official_url: "https://example.com/get", license: "MIT", license_url: "",
            license_checked: "", delivery, material_markers: markers, shader_guids: guids, legacy_folders: &[], legacy_files: &[],
        };
        let table = [
            entry("example.auto", "Example Auto", Delivery::Auto, &[], &[AUTO_GUID]),
            entry("example.guide", "Example Guide", Delivery::Guide, &["_ExampleGuideVersion"], &[]),
            entry("example.bundled", "Example Bundled", Delivery::Bundled, &[], &[BUNDLED_GUID]),
        ];
        let auto = material(AUTO_GUID, "");
        let guide_one = material("0123456789abcdef0123456789abcdef", "  m_Floats:\n    - _ExampleGuideVersion: 3\n");
        let guide_two = material("0123456789abcdef0123456789abcdef", "    - _ExampleGuideVersion: 4\n");
        let bundled = material(BUNDLED_GUID, "");
        let unknown = material("cccccccccccccccccccccccccccccccc", "");
        let materials = [auto.as_slice(), guide_one.as_slice(), guide_two.as_slice(), bundled.as_slice(), unknown.as_slice()];

        let nothing_installed = BTreeMap::new();
        let diagnostics = guide_diagnostics(&table, materials, &nothing_installed);
        assert_eq!(diagnostics.len(), 2, "the bundled and unknown materials produce no guide");
        assert_eq!(diagnostics[0].code, "amf.package.guide");
        assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
        assert_eq!(diagnostics[0].message, "1 material needs Example Auto; get it from https://example.com/get.");
        assert_eq!(diagnostics[1].message, "2 materials need Example Guide; get it from https://example.com/get.");

        let installed = BTreeMap::from([("example.auto".to_owned(), "1.0.0".to_owned())]);
        let diagnostics = guide_diagnostics(&table, materials, &installed);
        assert_eq!(diagnostics.len(), 1, "an installed auto package needs no guide");
        assert_eq!(diagnostics[0].message, "2 materials need Example Guide; get it from https://example.com/get.");
    }
}
