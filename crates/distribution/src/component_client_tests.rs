use super::*;
use crate::{
    commands::active_component_versions,
    component_feed::{ComponentFeed, ComponentKind},
    component_stage::ActivationJournal,
    layout::Active,
    transfer::distribution_user_agent,
    update_plan::select_update_pair,
};

fn index(manifest: &Manifest, kind: ComponentKind, sequence: u64) -> ComponentFeed {
    let envelope: SignedManifest =
        serde_json::from_slice(&component_index(manifest, kind, sequence)).unwrap();
    serde_json::from_str(&envelope.payload).unwrap()
}

fn sign_index(index: &ComponentFeed) -> Vec<u8> {
    let payload = serde_json::to_string(index).unwrap();
    serde_json::to_vec(&SignedManifest {
        schema_version: "0.2".into(),
        algorithm: "ed25519".into(),
        signature: hex(&SigningKey::from_bytes(&[7; 32])
            .sign(payload.as_bytes())
            .to_bytes()),
        payload,
    })
    .unwrap()
}

struct Staged {
    _temporary: tempfile::TempDir,
    installation: Installation,
    installed: Manifest,
    target: Manifest,
    base: Active,
    id: String,
}

impl Staged {
    async fn new(kind: ComponentKind) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let input = temporary.path().join("input");
        let (installed, raw, key) = fixture(&input);
        let installation =
            Installation::with_key(&temporary.path().join("installation"), true, key).unwrap();
        installation
            .install(&raw, &input, &mut transfer())
            .await
            .unwrap();
        let base = installation.active().unwrap();
        let mut target = installed.clone();
        match kind {
            ComponentKind::Host => {
                target.host.version = "0.6.8".into();
                target.host.build = "b".repeat(40);
                target.release_version = "0.6.8".into();
            }
            ComponentKind::Amf => {
                target.amf.version = "0.1.1".into();
                target.amf.build = "b".repeat(40);
            }
        }
        let id = installation.store_manifest(&signed(&target).0).unwrap().0;
        installation
            .accept_component_index(kind, &component_index(&target, kind, 10))
            .unwrap();
        installation
            .stage_component(
                kind,
                &id,
                &mut transfer(),
                Some(&input.join(format!("{}.bin", kind.name()))),
            )
            .await
            .unwrap();
        Self {
            _temporary: temporary,
            installation,
            installed,
            target,
            base,
            id,
        }
    }

    fn pending(&self, kind: ComponentKind) -> Vec<u8> {
        fs::read(
            self.installation
                .root
                .join(format!("updates-v02/{}-pending.json", kind.name())),
        )
        .unwrap()
    }

    fn withdrawn(&self, kind: ComponentKind) -> Vec<u8> {
        let mut feed = index(&self.target, kind, 13);
        feed.releases.clear();
        sign_index(&feed)
    }
}

#[tokio::test]
async fn new_signed_checks_keep_retained_pending_but_withdraw_removed_targets_without_deleting_files(
) {
    for kind in [ComponentKind::Host, ComponentKind::Amf] {
        let h = Staged::new(kind).await;
        let pending = h.pending(kind);
        let retained = component_index(&h.target, kind, 11);
        let (_, view) = h
            .installation
            .accept_component_check(kind, &retained, &h.base)
            .unwrap();
        assert_eq!(view["state"], "staged");
        assert!(view["error"].is_null());
        assert_eq!(h.pending(kind), pending);

        let (directory, files) = match kind {
            ComponentKind::Host => (
                h.installation.host_root(&h.target).unwrap(),
                &h.target.host.files,
            ),
            ComponentKind::Amf => (
                h.installation.amf_root(&h.target).unwrap(),
                &h.target.amf.files,
            ),
        };
        let saved = files
            .iter()
            .map(|file| {
                (
                    file.path.clone(),
                    fs::read(directory.join(&file.path)).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let withdrawn = h.withdrawn(kind);
        let mut tampered: SignedManifest = serde_json::from_slice(&withdrawn).unwrap();
        tampered.signature = "0".repeat(128);
        assert_eq!(
            h.installation
                .accept_component_check(kind, &serde_json::to_vec(&tampered).unwrap(), &h.base)
                .unwrap_err()
                .0,
            "invalid_release_signature"
        );
        assert_eq!(h.pending(kind), pending);
        assert_eq!(
            h.installation
                .cached_component_index(kind)
                .unwrap()
                .unwrap()
                .0
                .sequence,
            11
        );
        let (_, view) = h
            .installation
            .accept_component_check(kind, &withdrawn, &h.base)
            .unwrap();
        assert_eq!(view["state"], "idle");
        assert_eq!(view["error"], "update_withdrawn");
        assert!(view["target"].is_null());
        assert!(h.installation.component_pending(kind).unwrap().is_none());
        assert_eq!(h.installation.active().unwrap(), h.base);
        assert!(h.installation.manifest(&h.id).is_ok());
        for (path, bytes) in saved {
            assert_eq!(fs::read(directory.join(path)).unwrap(), bytes);
        }
        assert_eq!(
            h.installation
                .request_component_activation(kind)
                .unwrap_err()
                .0,
            "update_not_staged"
        );
        assert_eq!(
            h.installation
                .cached_component_index(kind)
                .unwrap()
                .unwrap()
                .0
                .sequence,
            13
        );
        assert_eq!(
            h.installation
                .accept_component_check(kind, &retained, &h.base)
                .unwrap_err()
                .0,
            "component_feed_replayed"
        );
        let (_, repeated) = h
            .installation
            .accept_component_check(kind, &withdrawn, &h.base)
            .unwrap();
        assert_eq!(repeated["state"], "idle");
        assert!(repeated["error"].is_null());
    }
}

#[tokio::test]
async fn withdrawal_does_not_mutate_pending_or_unfinished_activation_journals() {
    for kind in [ComponentKind::Host, ComponentKind::Amf] {
        for state in ["prepared", "verifying"] {
            let h = Staged::new(kind).await;
            let mut next = h.base.clone();
            match kind {
                ComponentKind::Host => next.host_manifest = h.id.clone(),
                ComponentKind::Amf => next.amf_manifest = h.id.clone(),
            }
            let journal = ActivationJournal {
                schema_version: "0.2".into(),
                before: h.base.clone(),
                next: next.clone(),
                state: state.into(),
            };
            h.installation
                .save_update("updates-v02/activation.json", &journal)
                .unwrap();
            if state == "verifying" {
                atomic_json(&h.installation.root.join("active.json"), &next).unwrap();
            }
            let active = h.installation.active().unwrap();
            let pending = h.pending(kind);
            let journal_path = h.installation.root.join("updates-v02/activation.json");
            let journal_bytes = fs::read(&journal_path).unwrap();
            let (_, view) = h
                .installation
                .accept_component_check(kind, &h.withdrawn(kind), &active)
                .unwrap();
            assert_ne!(view["error"], "update_withdrawn");
            assert_eq!(h.pending(kind), pending);
            assert_eq!(fs::read(journal_path).unwrap(), journal_bytes);
            assert_eq!(h.installation.active().unwrap(), active);
            assert_eq!(
                h.installation
                    .cached_component_index(kind)
                    .unwrap()
                    .unwrap()
                    .0
                    .sequence,
                13
            );
        }
    }
}

#[tokio::test]
async fn a_busy_activation_preserves_pending_and_a_later_check_can_apply_the_authenticated_withdrawal(
) {
    let kind = ComponentKind::Amf;
    let h = Staged::new(kind).await;
    let pending = h.pending(kind);
    let activation = h.installation.lock("operation.lock").unwrap();
    let withdrawn = h.withdrawn(kind);
    assert_eq!(
        h.installation
            .accept_component_check(kind, &withdrawn, &h.base)
            .unwrap_err()
            .0,
        "installation_busy"
    );
    assert_eq!(h.pending(kind), pending);
    drop(activation);
    let (_, view) = h
        .installation
        .accept_component_check(kind, &withdrawn, &h.base)
        .unwrap();
    assert_eq!(view["error"], "update_withdrawn");
    assert!(h.installation.component_pending(kind).unwrap().is_none());
}

#[tokio::test]
async fn user_agent_reads_each_active_component_and_falls_back_when_installation_records_are_unavailable(
) {
    let empty = tempfile::tempdir().unwrap();
    let (_, _, key) = fixture(&empty.path().join("input"));
    let absent = Installation::with_key(&empty.path().join("not-installed"), true, key).unwrap();
    assert_eq!(active_component_versions(&absent), None);

    let h = Staged::new(ComponentKind::Amf).await;
    assert_eq!(
        active_component_versions(&h.installation),
        Some((
            h.installed.host.version.clone(),
            h.installed.amf.version.clone()
        ))
    );
    let mut independent = h.target.clone();
    independent.host.version = "0.6.9".into();
    independent.release_version = "0.6.9".into();
    let id = h
        .installation
        .store_manifest(&signed(&independent).0)
        .unwrap()
        .0;
    let mut active = h.base.clone();
    active.amf_manifest = id.clone();
    atomic_json(&h.installation.root.join("active.json"), &active).unwrap();
    let versions = active_component_versions(&h.installation).unwrap();
    assert_eq!(versions, ("0.6.7".into(), "0.1.1".into()));
    assert_eq!(
        distribution_user_agent(Some((&versions.0, &versions.1))),
        "VUA-distribution/0.2 (host=0.6.7; amf=0.1.1)"
    );
    fs::write(
        h.installation.root.join(format!("manifests/{id}.json")),
        b"corrupt",
    )
    .unwrap();
    let versions = active_component_versions(&h.installation);
    assert_eq!(versions, None);
    assert_eq!(
        distribution_user_agent(
            versions
                .as_ref()
                .map(|(host, amf)| (host.as_str(), amf.as_str()))
        ),
        "VUA-distribution/0.2"
    );
}

#[test]
fn unsupported_abi_or_application_entries_do_not_hide_compatible_single_or_pair_updates() {
    let temporary = tempfile::tempdir().unwrap();
    let (installed, _, _) = fixture(temporary.path());
    for unsupported_abi in [true, false] {
        let mut supported = installed.clone();
        supported.host.version = "0.6.8".into();
        supported.release_version = "0.6.8".into();
        supported.amf.version = "0.1.1".into();
        let mut future = supported.clone();
        future.host.version = "0.7.0".into();
        future.release_version = "0.7.0".into();
        future.amf.version = "0.2.0".into();
        future.compatibility.host = ">=0.7.0, <0.8.0".into();
        if unsupported_abi {
            future.compatibility.ui_abi = "amf-ui/0.2".into();
        } else {
            future.compatibility.application = "0.2".into();
        }
        let mut host = index(&supported, ComponentKind::Host, 4);
        host.releases
            .extend(index(&future, ComponentKind::Host, 4).releases);
        let mut amf = index(&supported, ComponentKind::Amf, 4);
        amf.releases
            .extend(index(&future, ComponentKind::Amf, 4).releases);
        host.validate().unwrap();
        amf.validate().unwrap();
        let selected_host = host.select(&installed, &installed).unwrap();
        assert_eq!(selected_host.latest.unwrap().version, "0.7.0");
        assert_eq!(selected_host.target.unwrap().version, "0.6.8");
        let selected_amf = amf.select(&installed, &installed).unwrap();
        assert_eq!(selected_amf.latest.unwrap().version, "0.2.0");
        assert_eq!(selected_amf.target.unwrap().version, "0.1.1");
        let (next_host, next_amf) =
            select_update_pair(&host, &amf, &installed, &installed).unwrap();
        assert_eq!(
            (&*next_host.version, &*next_amf.version),
            ("0.6.8", "0.1.1")
        );
        host.releases.remove(0);
        amf.releases.remove(0);
        assert_eq!(
            select_update_pair(&host, &amf, &installed, &installed)
                .unwrap_err()
                .0,
            "no_compatible_update_plan"
        );
    }
}
