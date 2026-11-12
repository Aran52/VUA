use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use vua_orchestrator::{
    deployment::*, scenario_environment::*, NanosTaskIdGenerator, SqliteTaskStore, SystemClock,
    TaskRuntime,
};
struct CheckOnly {
    observations: AtomicUsize,
}
impl DeploymentAdapter for CheckOnly {
    fn observe(&self, _: &DeploymentIntent) -> Vec<DeploymentObservation> {
        self.observations.fetch_add(1, Ordering::SeqCst);
        vec![DeploymentObservation {
            component: "unity_editor".into(),
            presence: DeploymentPresence::Verified,
            version: Some("2022.3.22f1c1".into()),
            location: Some(r"C:\Editors\2022.3.22f1c1".into()),
        }]
    }
    fn download_region(&self) -> DownloadRegion {
        panic!("a software check must not probe download regions")
    }
    fn installer(&self, _: &DeploymentIntent) -> Option<DeploymentInstaller> {
        panic!("a check must not inspect installation authority")
    }
    fn install(
        &self,
        _: &DeploymentIntent,
        _: DeploymentAction,
        _: Option<&DeploymentInstaller>,
        _: &mut DeploymentReporter<'_>,
    ) -> Result<(), &'static str> {
        panic!("a check must not install")
    }
}
fn intent(purposes: Vec<DeploymentPurpose>) -> DeploymentIntent {
    DeploymentIntent {
        purposes,
        tools: Vec::new(),
        reinstall_tools: Vec::new(),
        editor_root: r"C:\Editors".into(),
        use_mirrors: true,
        pico_region: None,
    }
}
fn service() -> (Arc<CheckOnly>, DeploymentService, TaskRuntime) {
    let adapter = Arc::new(CheckOnly {
        observations: AtomicUsize::new(0),
    });
    let runtime = TaskRuntime::with_sqlite(
        Arc::new(SqliteTaskStore::open_in_memory().unwrap()),
        Arc::new(SystemClock),
        Arc::new(NanosTaskIdGenerator::default()),
    )
    .unwrap();
    (
        adapter.clone(),
        DeploymentService::new(adapter, runtime.clone()),
        runtime,
    )
}
#[test]
fn sop_is_available_without_services_or_a_software_check() {
    for scenario in [
        DeploymentPurpose::DesktopPlay,
        DeploymentPurpose::PicoPcvr,
        DeploymentPurpose::PcAvatar,
        DeploymentPurpose::QuestAvatar,
    ] {
        let result = scenario_sop(&ScenarioSopParams {
            scenario,
            connection: None,
        })
        .unwrap();
        assert_eq!(result["performed"], "sop_only");
        assert_eq!(
            result["steps"].as_array().unwrap().last().unwrap()["id"],
            "recovery"
        );
        assert!(result["steps"].as_array().unwrap().len() >= 5);
    }
    assert!(scenario_sop(&ScenarioSopParams {
        scenario: DeploymentPurpose::DesktopPlay,
        connection: Some(ScenarioConnection::Wifi)
    })
    .is_err());
    for connection in [ScenarioConnection::Usb, ScenarioConnection::Wifi] {
        let result = scenario_sop(&ScenarioSopParams {
            scenario: DeploymentPurpose::PicoPcvr,
            connection: Some(connection),
        })
        .unwrap();
        assert_eq!(
            result["connection"],
            serde_json::to_value(connection).unwrap()
        );
    }
}
#[test]
fn check_observes_c1_without_an_installer_license_probe_or_created_task() {
    let (adapter, service, runtime) = service();
    let result = check_scenario(&service, &intent(vec![DeploymentPurpose::PcAvatar])).unwrap();
    assert_eq!(adapter.observations.load(Ordering::SeqCst), 1);
    assert_eq!(result["softwareReady"], true);
    assert_eq!(result["observations"][0]["version"], "2022.3.22f1c1");
    assert_eq!(result["licenseState"], "not_checked");
    assert_eq!(result["installationStarted"], false);
    assert_eq!(runtime.snapshot_all().len(), 0);
}
#[test]
fn missing_observations_are_unknown_and_invalid_intent_never_reaches_io() {
    let (adapter, service, _) = service();
    let result = check_scenario(&service, &intent(vec![DeploymentPurpose::QuestAvatar])).unwrap();
    assert_eq!(result["softwareReady"], false);
    assert_eq!(result["observations"][0]["component"], "android_modules");
    assert_eq!(result["observations"][0]["presence"], "detection_failed");
    assert_eq!(result["nextAction"], "plan_installation");
    let mut invalid = intent(vec![DeploymentPurpose::PcAvatar]);
    invalid.editor_root = "relative".into();
    assert!(check_scenario(&service, &invalid).is_err());
    assert_eq!(adapter.observations.load(Ordering::SeqCst), 1);
}

#[test]
fn play_setups_v02_enforces_exact_combinations_engines_and_step_ids() {
    let catalog = list_play_setups();
    assert_eq!(catalog["schemaVersion"], SCENARIO_SCHEMA_V02);
    assert_eq!(catalog["platforms"].as_array().unwrap().len(), 3);
    assert_eq!(catalog["devices"].as_array().unwrap().len(), 15);
    assert_eq!(catalog["connections"].as_array().unwrap().len(), 17);

    // Reborn has no PC mode; standalone is VRChat-only; mismatched or missing headset connection fails.
    for bad in [
        PlaySetupSelection {
            platform: PlayPlatformId::Reborn,
            device: PlayDeviceId::Pc,
            connection: None,
        },
        PlaySetupSelection {
            platform: PlayPlatformId::Chilloutvr,
            device: PlayDeviceId::Quest,
            connection: Some(PlayConnectionId::Standalone),
        },
        PlaySetupSelection {
            platform: PlayPlatformId::Reborn,
            device: PlayDeviceId::Pico,
            connection: Some(PlayConnectionId::Standalone),
        },
        PlaySetupSelection {
            platform: PlayPlatformId::Vrchat,
            device: PlayDeviceId::Quest,
            connection: Some(PlayConnectionId::PicoConnect),
        },
        PlaySetupSelection {
            platform: PlayPlatformId::Vrchat,
            device: PlayDeviceId::Quest,
            connection: None,
        },
        PlaySetupSelection {
            platform: PlayPlatformId::Vrchat,
            device: PlayDeviceId::Pc,
            connection: Some(PlayConnectionId::SteamLink),
        },
    ] {
        assert_eq!(scenario_sop_v02(&bad), Err("vua.scenario.invalid_params"));
    }

    let desktop = scenario_sop_v02(&PlaySetupSelection {
        platform: PlayPlatformId::Vrchat,
        device: PlayDeviceId::Pc,
        connection: None,
    })
    .unwrap();
    assert_eq!(desktop["engine"]["kind"], "route");
    assert_eq!(desktop["engine"]["route"], "desktop_play");
    assert_eq!(
        desktop["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["steam", "steam-sign-in", "platform", "launch"]
    );
    assert_eq!(
        desktop["sopVersions"]["connection"],
        serde_json::Value::Null
    );

    let pico = scenario_sop_v02(&PlaySetupSelection {
        platform: PlayPlatformId::Vrchat,
        device: PlayDeviceId::Pico,
        connection: Some(PlayConnectionId::PicoConnect),
    })
    .unwrap();
    assert_eq!(pico["engine"]["kind"], "route");
    assert_eq!(pico["engine"]["route"], "pico_pcvr");
    assert_eq!(
        pico["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "steam",
            "steam-sign-in",
            "platform",
            "connection-pc",
            "connection-headset",
            "launch"
        ]
    );

    let standalone = scenario_sop_v02(&PlaySetupSelection {
        platform: PlayPlatformId::Vrchat,
        device: PlayDeviceId::Quest,
        connection: Some(PlayConnectionId::Standalone),
    })
    .unwrap();
    assert_eq!(standalone["engine"]["kind"], "headset");
    assert_eq!(standalone["nextAction"], "follow_sop");
    assert_eq!(
        standalone["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["store-install", "store-launch"]
    );

    let cvr = scenario_sop_v02(&PlaySetupSelection {
        platform: PlayPlatformId::Chilloutvr,
        device: PlayDeviceId::SteamFrame,
        connection: Some(PlayConnectionId::SteamFrame),
    })
    .unwrap();
    assert_eq!(cvr["engine"]["kind"], "tool");
    assert_eq!(cvr["engine"]["toolId"], "chilloutvr");
}
