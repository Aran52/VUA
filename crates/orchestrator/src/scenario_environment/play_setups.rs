//! Static play setup catalog and composed SOP v0.2 (`vua.scenario-environment/v0.2`).
//! Mirrors the desktop launch setup rules so the UI and MCP read one deterministic source.
//! Performs zero filesystem, network, process, or installation operations.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const SCENARIO_SCHEMA_V02: &str = "vua.scenario-environment/v0.2";
pub const SOP_SEGMENT_VERSION_V02: &str = "0.2.0";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlayPlatformId {
    Vrchat,
    Chilloutvr,
    Reborn,
}

impl PlayPlatformId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vrchat => "vrchat",
            Self::Chilloutvr => "chilloutvr",
            Self::Reborn => "reborn",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Vrchat => "VRChat",
            Self::Chilloutvr => "ChilloutVR",
            Self::Reborn => "Reborn",
        }
    }

    pub fn steam_url(self) -> &'static str {
        match self {
            Self::Vrchat => "https://store.steampowered.com/app/438100/VRChat/",
            Self::Chilloutvr => "https://store.steampowered.com/app/661130/ChilloutVR/",
            Self::Reborn => "https://store.steampowered.com/app/2089520/Reborn/",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlayDeviceId {
    Pc,
    Pico,
    Quest,
    SteamFrame,
    ViveStandalone,
    ViveWired,
    Index,
    Psvr2,
    Bigscreen,
    Pimax,
    Varjo,
    Meganex,
    RiftS,
    Wmr,
    OtherHeadset,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlayConnectionId {
    PicoConnect,
    Standalone,
    QuestLink,
    SteamLink,
    VirtualDesktop,
    Alvr,
    SteamFrame,
    ViveHub,
    SteamvrDirect,
    Psvr2App,
    BigscreenDriver,
    PimaxPlay,
    VarjoBase,
    MeganexConfigurator,
    RiftApp,
    WmrSteamvr,
    SteamvrOther,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HeadsetStore {
    Pico,
    Meta,
    Viveport,
}

impl HeadsetStore {
    fn store_label(self) -> &'static str {
        match self {
            Self::Pico => "PICO Store",
            Self::Meta => "Meta Horizon Store",
            Self::Viveport => "VIVEPORT",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayStepActor {
    Vua,
    Open,
    Player,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaySetupSelection {
    pub platform: PlayPlatformId,
    pub device: PlayDeviceId,
    pub connection: Option<PlayConnectionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlayEngine {
    Route {
        route: String,
    },
    Tool {
        #[serde(rename = "toolId")]
        tool_id: String,
    },
    Headset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaySopStepV02 {
    pub id: String,
    pub actor: PlayStepActor,
    pub title: String,
    pub explanation: String,
    pub expected_result: String,
    pub automatic: bool,
    pub verification: String,
    pub on_failure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaySopVersions {
    pub platform: String,
    pub device: String,
    pub connection: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaySopVerification {
    pub date: String,
    pub environment: String,
    pub software_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaySopSource {
    pub label: String,
    pub url: String,
}

struct DeviceDef {
    id: PlayDeviceId,
    name: &'static str,
    connections: &'static [PlayConnectionId],
    store: Option<HeadsetStore>,
    other: bool,
}

struct ConnectionDef {
    id: PlayConnectionId,
    name: &'static str,
    component: Option<&'static str>,
    url: &'static str,
}

const PLATFORMS: [PlayPlatformId; 3] = [
    PlayPlatformId::Vrchat,
    PlayPlatformId::Chilloutvr,
    PlayPlatformId::Reborn,
];

const DEVICES: [DeviceDef; 15] = [
    DeviceDef {
        id: PlayDeviceId::Pc,
        name: "",
        connections: &[],
        store: None,
        other: false,
    },
    DeviceDef {
        id: PlayDeviceId::Pico,
        name: "PICO",
        connections: &[
            PlayConnectionId::PicoConnect,
            PlayConnectionId::SteamLink,
            PlayConnectionId::VirtualDesktop,
            PlayConnectionId::Alvr,
            PlayConnectionId::Standalone,
        ],
        store: Some(HeadsetStore::Pico),
        other: false,
    },
    DeviceDef {
        id: PlayDeviceId::Quest,
        name: "Meta Quest",
        connections: &[
            PlayConnectionId::QuestLink,
            PlayConnectionId::SteamLink,
            PlayConnectionId::VirtualDesktop,
            PlayConnectionId::Alvr,
            PlayConnectionId::Standalone,
        ],
        store: Some(HeadsetStore::Meta),
        other: false,
    },
    DeviceDef {
        id: PlayDeviceId::SteamFrame,
        name: "Steam Frame",
        connections: &[PlayConnectionId::SteamFrame],
        store: None,
        other: false,
    },
    DeviceDef {
        id: PlayDeviceId::ViveStandalone,
        name: "",
        connections: &[
            PlayConnectionId::ViveHub,
            PlayConnectionId::SteamLink,
            PlayConnectionId::VirtualDesktop,
            PlayConnectionId::Alvr,
            PlayConnectionId::Standalone,
        ],
        store: Some(HeadsetStore::Viveport),
        other: false,
    },
    DeviceDef {
        id: PlayDeviceId::ViveWired,
        name: "",
        connections: &[PlayConnectionId::SteamvrDirect],
        store: None,
        other: false,
    },
    DeviceDef {
        id: PlayDeviceId::Index,
        name: "Valve Index",
        connections: &[PlayConnectionId::SteamvrDirect],
        store: None,
        other: false,
    },
    DeviceDef {
        id: PlayDeviceId::Psvr2,
        name: "PS VR2",
        connections: &[PlayConnectionId::Psvr2App],
        store: None,
        other: false,
    },
    DeviceDef {
        id: PlayDeviceId::Bigscreen,
        name: "Bigscreen Beyond",
        connections: &[PlayConnectionId::BigscreenDriver],
        store: None,
        other: true,
    },
    DeviceDef {
        id: PlayDeviceId::Pimax,
        name: "Pimax",
        connections: &[PlayConnectionId::PimaxPlay],
        store: None,
        other: true,
    },
    DeviceDef {
        id: PlayDeviceId::Varjo,
        name: "Varjo",
        connections: &[PlayConnectionId::VarjoBase],
        store: None,
        other: true,
    },
    DeviceDef {
        id: PlayDeviceId::Meganex,
        name: "MeganeX",
        connections: &[PlayConnectionId::MeganexConfigurator],
        store: None,
        other: true,
    },
    DeviceDef {
        id: PlayDeviceId::RiftS,
        name: "Oculus Rift S",
        connections: &[PlayConnectionId::RiftApp],
        store: None,
        other: true,
    },
    DeviceDef {
        id: PlayDeviceId::Wmr,
        name: "Windows MR",
        connections: &[PlayConnectionId::WmrSteamvr],
        store: None,
        other: true,
    },
    DeviceDef {
        id: PlayDeviceId::OtherHeadset,
        name: "",
        connections: &[PlayConnectionId::SteamvrOther],
        store: None,
        other: true,
    },
];

const CONNECTIONS: [ConnectionDef; 17] = [
    ConnectionDef {
        id: PlayConnectionId::PicoConnect,
        name: "PICO Connect",
        component: Some("pico_runtime"),
        url: "https://www.picoxr.com/",
    },
    ConnectionDef {
        id: PlayConnectionId::Standalone,
        name: "",
        component: None,
        url: "https://wiki.vrchat.com/wiki/Getting_Started",
    },
    ConnectionDef {
        id: PlayConnectionId::QuestLink,
        name: "Quest Link / Air Link",
        component: Some("oculus_runtime"),
        url: "https://www.meta.com/quest/setup/",
    },
    ConnectionDef {
        id: PlayConnectionId::SteamLink,
        name: "Steam Link",
        component: None,
        url: "https://help.steampowered.com/en/faqs/view/0E2C-406B-9135-38A4",
    },
    ConnectionDef {
        id: PlayConnectionId::VirtualDesktop,
        name: "Virtual Desktop",
        component: Some("virtual_desktop"),
        url: "https://www.vrdesktop.net/",
    },
    ConnectionDef {
        id: PlayConnectionId::Alvr,
        name: "ALVR",
        component: Some("alvr"),
        url: "https://github.com/alvr-org/ALVR",
    },
    ConnectionDef {
        id: PlayConnectionId::SteamFrame,
        name: "SteamVR",
        component: Some("steamvr"),
        url: "https://store.steampowered.com/hardware/steamframe",
    },
    ConnectionDef {
        id: PlayConnectionId::ViveHub,
        name: "VIVE Hub",
        component: Some("vive_runtime"),
        url: "https://www.vive.com/us/vive-hub/download/",
    },
    ConnectionDef {
        id: PlayConnectionId::SteamvrDirect,
        name: "SteamVR",
        component: Some("steamvr"),
        url: "https://store.steampowered.com/app/250820/",
    },
    ConnectionDef {
        id: PlayConnectionId::Psvr2App,
        name: "PlayStation VR2 App",
        component: Some("psvr2"),
        url: "https://www.playstation.com/en-us/support/hardware/pc-prepare-ps-vr2/",
    },
    ConnectionDef {
        id: PlayConnectionId::BigscreenDriver,
        name: "Bigscreen Beyond Utility",
        component: Some("bigscreen_beyond"),
        url: "https://bigscreenvr.com/mybeyond/",
    },
    ConnectionDef {
        id: PlayConnectionId::PimaxPlay,
        name: "Pimax Play",
        component: Some("pimax_runtime"),
        url: "https://www.pimax.com/support/downloads",
    },
    ConnectionDef {
        id: PlayConnectionId::VarjoBase,
        name: "Varjo Base",
        component: Some("varjo_runtime"),
        url: "https://support.varjo.com/hc/en-us/downloads",
    },
    ConnectionDef {
        id: PlayConnectionId::MeganexConfigurator,
        name: "MeganeX Configurator",
        component: None,
        url: "https://shiftall.notion.site/meganex8k-manual-en",
    },
    ConnectionDef {
        id: PlayConnectionId::RiftApp,
        name: "Meta Horizon Link",
        component: Some("oculus_runtime"),
        url: "https://www.meta.com/quest/setup/",
    },
    ConnectionDef {
        id: PlayConnectionId::WmrSteamvr,
        name: "Windows Mixed Reality for SteamVR",
        component: None,
        url: "https://store.steampowered.com/app/719950/",
    },
    ConnectionDef {
        id: PlayConnectionId::SteamvrOther,
        name: "SteamVR",
        component: Some("steamvr"),
        url: "https://store.steampowered.com/app/250820/",
    },
];

fn device_def(id: PlayDeviceId) -> &'static DeviceDef {
    DEVICES
        .iter()
        .find(|d| d.id == id)
        .expect("closed PlayDeviceId enum")
}

fn connection_def(id: PlayConnectionId) -> &'static ConnectionDef {
    CONNECTIONS
        .iter()
        .find(|c| c.id == id)
        .expect("closed PlayConnectionId enum")
}

pub fn connections_for_setup(
    platform: PlayPlatformId,
    device: PlayDeviceId,
) -> Vec<PlayConnectionId> {
    device_def(device)
        .connections
        .iter()
        .copied()
        .filter(|c| *c != PlayConnectionId::Standalone || platform == PlayPlatformId::Vrchat)
        .collect()
}

pub fn setup_engine(selection: &PlaySetupSelection) -> PlayEngine {
    if selection.connection == Some(PlayConnectionId::Standalone) {
        return PlayEngine::Headset;
    }
    if selection.platform == PlayPlatformId::Vrchat && selection.device == PlayDeviceId::Pc {
        return PlayEngine::Route {
            route: "desktop_play".into(),
        };
    }
    if selection.platform == PlayPlatformId::Vrchat
        && selection.connection == Some(PlayConnectionId::PicoConnect)
    {
        return PlayEngine::Route {
            route: "pico_pcvr".into(),
        };
    }
    PlayEngine::Tool {
        tool_id: selection.platform.as_str().into(),
    }
}

pub fn setup_requirements(selection: &PlaySetupSelection) -> Vec<String> {
    if selection.connection == Some(PlayConnectionId::Standalone) {
        return Vec::new();
    }
    let mut set = BTreeSet::new();
    set.insert("steam");
    set.insert(selection.platform.as_str());
    if selection.device != PlayDeviceId::Pc {
        set.insert("steamvr");
    }
    if let Some(conn) = selection.connection {
        if let Some(component) = connection_def(conn).component {
            set.insert(component);
        }
    }
    set.into_iter().map(str::to_owned).collect()
}

pub fn list_play_setups() -> Value {
    let platforms = PLATFORMS
        .iter()
        .map(|p| json!({ "id": p, "name": p.display_name() }))
        .collect::<Vec<_>>();
    let devices = DEVICES
        .iter()
        .map(|d| {
            let mut obj = json!({
                "id": d.id,
                "name": d.name,
                "connections": d.connections,
            });
            if let Some(store) = d.store {
                obj["store"] = json!(store);
            }
            if d.other {
                obj["other"] = json!(true);
            }
            obj
        })
        .collect::<Vec<_>>();
    let connections = CONNECTIONS
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "component": c.component,
                "url": c.url,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schemaVersion": SCENARIO_SCHEMA_V02,
        "platforms": platforms,
        "devices": devices,
        "connections": connections,
    })
}

fn step_v02(
    id: &str,
    actor: PlayStepActor,
    title: impl Into<String>,
    explanation: impl Into<String>,
    expected_result: impl Into<String>,
    verification: impl Into<String>,
    on_failure: impl Into<String>,
) -> PlaySopStepV02 {
    PlaySopStepV02 {
        id: id.into(),
        actor,
        title: title.into(),
        explanation: explanation.into(),
        expected_result: expected_result.into(),
        automatic: actor == PlayStepActor::Vua,
        verification: verification.into(),
        on_failure: on_failure.into(),
    }
}

pub fn scenario_sop_v02(selection: &PlaySetupSelection) -> Result<Value, &'static str> {
    if selection.platform == PlayPlatformId::Reborn && selection.device == PlayDeviceId::Pc {
        return Err("vua.scenario.invalid_params");
    }
    let allowed = connections_for_setup(selection.platform, selection.device);
    match (allowed.is_empty(), selection.connection) {
        (true, None) => {}
        (true, Some(_)) => return Err("vua.scenario.invalid_params"),
        (false, Some(conn)) if allowed.contains(&conn) => {}
        (false, _) => return Err("vua.scenario.invalid_params"),
    }

    let engine = setup_engine(selection);
    let requirements = setup_requirements(selection);
    let platform_name = selection.platform.display_name();
    let device = device_def(selection.device);

    let steps = if selection.connection == Some(PlayConnectionId::Standalone) {
        let store_name = device
            .store
            .map(HeadsetStore::store_label)
            .unwrap_or("the headset store");
        vec![
            step_v02(
                "store-install",
                PlayStepActor::Player,
                format!("Install {platform_name} on the headset"),
                format!("Open {store_name} inside the headset and install {platform_name}. No PC streaming software is needed."),
                format!("{platform_name} appears in the headset's installed library."),
                format!("Check the headset library in {store_name} to confirm {platform_name} finished downloading."),
                format!("Check headset Wi-Fi connectivity and available storage in {store_name}, then retry the download."),
            ),
            step_v02(
                "store-launch",
                PlayStepActor::Player,
                format!("Open {platform_name} in the headset"),
                format!("Launch {platform_name} directly on the headset and sign in through the official in-headset flow."),
                format!("{platform_name} opens in standalone mode and enters the home world."),
                "Confirm audio, controller tracking and sign-in inside the headset.",
                "Restart the application on the headset and review official account linking guidance if sign-in fails.",
            ),
        ]
    } else {
        let mut list = vec![
            step_v02(
                "steam",
                PlayStepActor::Open,
                "Install Steam on this computer",
                "Install the official Steam client if it is not already present on Windows.",
                "Steam is installed and can open on Windows.",
                "Check local software presence for component `steam`.",
                "Open the official Steam installer page from VUA and complete the Windows installation.",
            ),
            step_v02(
                "steam-sign-in",
                PlayStepActor::Player,
                "Sign in to Steam",
                "Sign in with your own Steam account inside the official Steam window. VUA never receives your password.",
                "Steam shows your signed-in library.",
                "Confirm the Steam client main library window is signed in and online.",
                "Complete Steam Guard verification or password recovery in the official Steam client.",
            ),
            step_v02(
                "platform",
                PlayStepActor::Vua,
                format!("Install {platform_name} in Steam"),
                format!("Ensure {platform_name} is installed in your Steam library before starting play."),
                format!("{platform_name} is installed and ready in Steam."),
                format!("Check local software presence for component `{}`.", selection.platform.as_str()),
                format!("Open {platform_name} in Steam and finish downloading any pending update."),
            ),
        ];
        if let Some(conn_id) = selection.connection {
            let conn = connection_def(conn_id);
            let conn_label = if conn.name.is_empty() {
                "the headset connection software"
            } else {
                conn.name
            };
            let pc_actor = if conn.component.is_none() {
                PlayStepActor::Player
            } else {
                PlayStepActor::Open
            };
            list.push(step_v02(
                "connection-pc",
                pc_actor,
                format!("Prepare {conn_label} on the computer"),
                format!("Install and open {conn_label} (and SteamVR) on this PC following {}.", conn.url),
                format!("{conn_label} and SteamVR are ready on the PC."),
                match conn.component {
                    Some(comp) => format!("Check local software presence for `{comp}` and `steamvr`."),
                    None => "Check local software presence for `steamvr` and confirm the companion app is ready.".into(),
                },
                format!("Open {} to install or repair {conn_label} and verify SteamVR is installed in Steam.", conn.url),
            ));
            list.push(step_v02(
                "connection-headset",
                PlayStepActor::Player,
                format!("Connect the headset through {conn_label}"),
                format!("Put on the headset, connect by cable or local network as required by {conn_label}, and enter the SteamVR environment."),
                "SteamVR reports the headset as connected and displays the PC VR view inside the headset.",
                "Confirm the headset icon is active in SteamVR and the VR compositor is visible in the headset.",
                format!("Reconnect the cable or local Wi-Fi session in {conn_label}, then restart SteamVR before launching the game."),
            ));
        }
        list.push(step_v02(
            "launch",
            PlayStepActor::Vua,
            if selection.device == PlayDeviceId::Pc {
                format!("Launch {platform_name} in desktop mode")
            } else {
                format!("Launch {platform_name} in VR")
            },
            if selection.device == PlayDeviceId::Pc {
                format!("Start {platform_name} on your computer screen without a headset.")
            } else {
                format!("Start {platform_name} once the headset is active in SteamVR.")
            },
            if selection.device == PlayDeviceId::Pc {
                format!("The {platform_name} desktop window opens.")
            } else {
                format!("{platform_name} opens inside the connected headset.")
            },
            "Observe the running play session or external tool process state in VUA.",
            "Recheck the preceding connection and Steam steps before requesting launch again.",
        ));
        list
    };

    let mut sources = vec![PlaySopSource {
        label: format!("{platform_name} on Steam"),
        url: selection.platform.steam_url().into(),
    }];
    if let Some(conn_id) = selection.connection {
        let conn = connection_def(conn_id);
        let label = if conn.name.is_empty() {
            format!("{platform_name} standalone getting started")
        } else {
            conn.name.into()
        };
        sources.push(PlaySopSource {
            label,
            url: conn.url.into(),
        });
    }

    let is_route = matches!(engine, PlayEngine::Route { .. });
    let next_action = if matches!(engine, PlayEngine::Headset) {
        "follow_sop"
    } else {
        "check_environment"
    };
    let last_verification = PlaySopVerification {
        date: "2026-11-07".into(),
        environment: if is_route {
            "Windows 11 play-route flow and official upstream documentation".into()
        } else {
            "Official upstream documentation catalog review (no unverified hardware session claim)".into()
        },
        software_versions: Vec::new(),
    };

    Ok(json!({
        "schemaVersion": SCENARIO_SCHEMA_V02,
        "selection": selection,
        "engine": engine,
        "requirements": requirements,
        "steps": steps,
        "performed": "sop_only",
        "nextAction": next_action,
        "sopVersions": PlaySopVersions {
            platform: SOP_SEGMENT_VERSION_V02.into(),
            device: SOP_SEGMENT_VERSION_V02.into(),
            connection: selection.connection.map(|_| SOP_SEGMENT_VERSION_V02.into()),
        },
        "lastVerification": last_verification,
        "sources": sources,
    }))
}
