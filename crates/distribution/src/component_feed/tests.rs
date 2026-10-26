use super::*;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{json, Value};

fn baseline() -> Manifest {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../schemas/windows-distribution/v0.1/vectors/cases.json"
    ))
    .unwrap();
    serde_json::from_value(
        cases
            .into_iter()
            .find(|case| case["schema"] == "manifest" && case["valid"] == true)
            .unwrap()["value"]
            .clone(),
    )
    .unwrap()
}
fn signed(value: &impl Serialize, schema: &str) -> Vec<u8> {
    let payload = serde_json::to_string(value).unwrap();
    let signature = SigningKey::from_bytes(&[7; 32])
        .sign(payload.as_bytes())
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    serde_json::to_vec(&json!({"schemaVersion":schema,"algorithm":"ed25519","payload":payload,"signature":signature})).unwrap()
}
fn release(manifest: &Manifest, kind: ComponentKind) -> (Release, Vec<u8>) {
    let raw = signed(manifest, "0.1");
    let component = match kind {
        ComponentKind::Host => &manifest.host,
        ComponentKind::Amf => &manifest.amf,
    };
    (
        Release {
            version: component.version.clone(),
            build: component.build.clone(),
            release_url: "https://ph-r.com/vua/releases/v0.6.7/".into(),
            manifest: ManifestObject {
                sha256: crate::paths::digest(&raw),
                bytes: raw.len() as u64,
                sources: vec!["https://ph-r.com/vua/objects/manifest.json".into()],
            },
            compatibility: manifest.compatibility.clone(),
            activation: if kind == ComponentKind::Host {
                Activation::Restart
            } else {
                Activation::Reload
            },
            runtime_version: if kind == ComponentKind::Host {
                Some(manifest.runtime.version.clone())
            } else {
                None
            },
        },
        raw,
    )
}
fn feed(component: ComponentKind, releases: Vec<Release>) -> ComponentFeed {
    ComponentFeed {
        schema_version: SCHEMA.into(),
        product: "VUA".into(),
        platform: "windows".into(),
        arch: "x64".into(),
        channel: "beta".into(),
        component,
        sequence: 3,
        releases,
    }
}
fn key() -> [u8; 32] {
    SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes()
}

#[test]
fn component_feed_vectors_validate_schema_and_native_consumer() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../schemas/windows-distribution/v0.2/vectors/cases.json"
    ))
    .unwrap();
    for case in cases {
        let schema: Value = serde_json::from_str(match case["schema"].as_str().unwrap() {
            "index" => {
                include_str!("../../../../schemas/windows-distribution/v0.2/index.schema.json")
            }
            "envelope" => {
                include_str!("../../../../schemas/windows-distribution/v0.2/envelope.schema.json")
            }
            other => panic!("Unknown schema {other}"),
        })
        .unwrap();
        assert_eq!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&case["value"]),
            case["schemaValid"].as_bool().unwrap(),
            "schema: {}",
            case["name"]
        );
        let kind = if case["component"] == "host" {
            ComponentKind::Host
        } else {
            ComponentKind::Amf
        };
        let valid = if case["schema"] == "envelope" {
            verify(
                &serde_json::to_vec(&case["value"]).unwrap(),
                &key(),
                kind,
                0,
            )
            .is_ok()
        } else {
            serde_json::from_value::<ComponentFeed>(case["value"].clone())
                .is_ok_and(|feed| feed.validate().is_ok())
        };
        assert_eq!(
            valid,
            case["nativeValid"].as_bool().unwrap(),
            "native: {}",
            case["name"]
        );
    }
}

#[test]
fn component_feed_authentication_binds_role_sequence_and_exact_payload() {
    let index = feed(
        ComponentKind::Amf,
        vec![release(&baseline(), ComponentKind::Amf).0],
    );
    let raw = signed(&index, SCHEMA);
    assert_eq!(
        verify(&raw, &key(), ComponentKind::Amf, 3)
            .unwrap()
            .sequence,
        3
    );
    assert_eq!(
        verify(&raw, &key(), ComponentKind::Host, 0).unwrap_err().0,
        "wrong_component_feed"
    );
    assert_eq!(
        verify(&raw, &key(), ComponentKind::Amf, 4).unwrap_err().0,
        "component_feed_replayed"
    );
    assert!(verify(
        &raw,
        &SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes(),
        ComponentKind::Amf,
        0
    )
    .is_err());
    let mut tampered: Value = serde_json::from_slice(&raw).unwrap();
    tampered["payload"] = Value::String(format!("{} ", tampered["payload"].as_str().unwrap()));
    assert_eq!(
        verify(
            &serde_json::to_vec(&tampered).unwrap(),
            &key(),
            ComponentKind::Amf,
            0
        )
        .unwrap_err()
        .0,
        "invalid_release_signature"
    );
    assert!(verify(
        &vec![b' '; MAX_MANIFEST_BYTES + 1],
        &key(),
        ComponentKind::Amf,
        0
    )
    .is_err());
}

#[test]
fn component_feed_selects_latest_compatible_amf_without_selecting_host_or_runtime() {
    let active = baseline();
    let mut older = active.clone();
    older.amf.version = "0.1.1".into();
    let mut latest = active.clone();
    latest.host.version = "0.7.0".into();
    latest.release_version = "0.7.0".into();
    latest.amf.version = "0.2.0".into();
    latest.compatibility.host = ">=0.7.0, <0.8.0".into();
    let index = feed(
        ComponentKind::Amf,
        vec![
            release(&older, ComponentKind::Amf).0,
            release(&latest, ComponentKind::Amf).0,
        ],
    );
    let selected = index.select(&active, &active).unwrap();
    assert_eq!(selected.latest.unwrap().version, "0.2.0");
    assert_eq!(selected.target.unwrap().version, "0.1.1");
    assert!(selected.target.unwrap().runtime_version.is_none());
    assert_eq!(
        index
            .select(&active, &older)
            .unwrap()
            .target
            .map(|r| &r.version),
        None
    );
}

#[test]
fn component_feed_does_not_offer_host_update_that_strands_current_amf() {
    let active = baseline();
    let mut target = active.clone();
    target.host.version = "0.7.0".into();
    target.release_version = "0.7.0".into();
    target.compatibility.host = ">=0.7.0, <0.8.0".into();
    let index = feed(
        ComponentKind::Host,
        vec![release(&target, ComponentKind::Host).0],
    );
    assert!(index.select(&active, &active).unwrap().target.is_none());
}

#[test]
fn component_feed_host_patch_preserves_compatible_amf_selection() {
    let active = baseline();
    let mut target = active.clone();
    target.host.version = "0.6.8".into();
    target.release_version = "0.6.8".into();
    let index = feed(
        ComponentKind::Host,
        vec![release(&target, ComponentKind::Host).0],
    );
    let selection = index.select(&active, &active).unwrap();
    assert_eq!(selection.target.unwrap().version, "0.6.8");
    assert_eq!(selection.target.unwrap().activation, Activation::Restart);
    assert_eq!(active.amf.version, "0.1.0");
}

#[test]
fn component_feed_inventory_is_separately_verified_and_bound_to_the_offered_build() {
    let baseline = baseline();
    let (mut entry, raw) = release(&baseline, ComponentKind::Amf);
    assert!(entry
        .verify_inventory(ComponentKind::Amf, &raw, &key())
        .is_ok());
    entry.build = "b".repeat(40);
    assert_eq!(
        entry
            .verify_inventory(ComponentKind::Amf, &raw, &key())
            .unwrap_err()
            .0,
        "component_manifest_identity_mismatch"
    );
    entry.manifest.sha256 = "a".repeat(64);
    assert_eq!(
        entry
            .verify_inventory(ComponentKind::Amf, &raw, &key())
            .unwrap_err()
            .0,
        "component_manifest_integrity_failed"
    );
}

#[test]
fn component_feed_semver_order_is_numeric_and_data_migrations_cannot_hot_activate() {
    let active = baseline();
    let mut a = active.clone();
    a.amf.version = "0.1.9".into();
    let mut b = active.clone();
    b.amf.version = "0.1.10".into();
    let index = feed(
        ComponentKind::Amf,
        vec![
            release(&a, ComponentKind::Amf).0,
            release(&b, ComponentKind::Amf).0,
        ],
    );
    assert_eq!(
        index
            .select(&active, &active)
            .unwrap()
            .target
            .unwrap()
            .version,
        "0.1.10"
    );
    let mut entry = release(&b, ComponentKind::Amf).0;
    assert_eq!(
        entry.activation_for(ComponentKind::Amf, &active),
        Activation::Reload
    );
    entry.compatibility.bdl_write = "new-data-format".into();
    assert_eq!(
        entry.activation_for(ComponentKind::Amf, &active),
        Activation::Restart
    );
    assert_eq!(
        entry.activation_for(ComponentKind::Host, &active),
        Activation::Restart
    );
}
