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
        "VUA-distribution/0.3 (host=0.6.7; amf=0.1.1)"
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
        "VUA-distribution/0.3"
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

#[tokio::test]
async fn installation_hardlinks_runtime_alias_and_cleans_runtime_archive_cache() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let (installed, raw, key) = fixture(&input);
    let installation =
        Installation::with_key(&temporary.path().join("installation"), true, key).unwrap();
    installation
        .install(&raw, &input, &mut transfer())
        .await
        .unwrap();
    let runtime = installation.runtime_root(&installed).unwrap();
    assert!(crate::paths::same_regular_file(
        &runtime.join("electron.exe"),
        &runtime.join("VUA.Runtime.exe"),
    ));
    let copy_path = temporary.path().join("separate-copy.exe");
    fs::copy(runtime.join("electron.exe"), &copy_path).unwrap();
    assert!(!crate::paths::same_regular_file(
        &runtime.join("electron.exe"),
        &copy_path,
    ));
    assert!(!installation
        .root
        .join(format!("cache/{}.zip", installed.runtime.archive.sha256))
        .exists());
}

#[tokio::test]
async fn v03_compressed_staging_reuses_unchanged_files_prevents_downgrade_and_bounds_decompression()
{
    use crate::component_feed::{CompressedFileObject, CompressedTransfer, CompressionEncoding};
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let (installed, raw_base, key) = fixture(&input);
    let installation =
        Installation::with_key(&temporary.path().join("installation"), true, key).unwrap();
    installation
        .install(&raw_base, &input, &mut transfer())
        .await
        .unwrap();

    // Build an AMF update where provider exe changes (compressible payload) and other files stay unchanged.
    let mut updated = installed.clone();
    updated.amf.version = "0.1.1".into();
    updated.amf.build = "c".repeat(40);
    let changed_bytes = b"VUA-AMF-PROVIDER-EXECUTABLE-".repeat(2048);
    let mut raw_pack = Vec::new();
    let mut offset = 0u64;
    let mut file_payloads = Vec::new();
    for file in &mut updated.amf.files {
        let content = if file.path == "provider/vua-amf-provider.exe" {
            changed_bytes.clone()
        } else {
            fs::read(installation.amf_root(&installed).unwrap().join(&file.path)).unwrap()
        };
        file.offset = offset;
        file.bytes = content.len() as u64;
        file.sha256 = digest(&content);
        offset += file.bytes;
        raw_pack.extend_from_slice(&content);
        file_payloads.push((file.clone(), content));
    }
    updated.amf.pack.bytes = raw_pack.len() as u64;
    updated.amf.pack.sha256 = digest(&raw_pack);

    for encoding in [CompressionEncoding::Zstd, CompressionEncoding::Gzip] {
        let mut compressed_pack = Vec::new();
        let mut compressed_files = Vec::new();
        let mut comp_offset = 0u64;
        for (file, content) in &file_payloads {
            let frame = if content.is_empty() {
                Vec::new()
            } else {
                match encoding {
                    CompressionEncoding::Zstd => ruzstd::encoding::compress_to_vec(
                        content.as_slice(),
                        ruzstd::encoding::CompressionLevel::Fastest,
                    ),
                    CompressionEncoding::Gzip => {
                        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                        encoder.write_all(content).unwrap();
                        encoder.finish().unwrap()
                    }
                }
            };
            let comp_len = frame.len() as u64;
            let comp_sha = digest(&frame);
            compressed_files.push(CompressedFileObject {
                path: file.path.clone(),
                bytes: file.bytes,
                sha256: file.sha256.clone(),
                compressed_offset: comp_offset,
                compressed_bytes: comp_len,
                compressed_sha256: comp_sha,
            });
            comp_offset += comp_len;
            compressed_pack.extend_from_slice(&frame);
        }

        // Loopback HTTP server serving ranges from compressed_pack to test real HTTP 206 compressed staging!
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let pack_url = format!("http://127.0.0.1:{port}/object.pack");
        let served_pack = std::sync::Arc::new(compressed_pack.clone());
        let range_requests = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let range_requests_server = range_requests.clone();
        let server = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let pack = served_pack.clone();
                let counter = range_requests_server.clone();
                tokio::spawn(async move {
                    let mut req_buf = vec![0u8; 4096];
                    let n = stream.read(&mut req_buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&req_buf[..n]);
                    if let Some(range_line) = req
                        .lines()
                        .find(|line| line.to_ascii_lowercase().starts_with("range: bytes="))
                    {
                        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let spec = range_line.split('=').nth(1).unwrap().trim();
                        let mut parts = spec.split('-');
                        let start: usize = parts.next().unwrap().parse().unwrap();
                        let end: usize = parts.next().unwrap().parse().unwrap();
                        let body = &pack[start..=end];
                        let header = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                            body.len(),
                            pack.len()
                        );
                        let _ = stream.write_all(header.as_bytes()).await;
                        let _ = stream.write_all(body).await;
                    }
                });
            }
        });

        let mut target_manifest = updated.clone();
        target_manifest.amf.version = match encoding {
            CompressionEncoding::Zstd => "0.1.1".into(),
            CompressionEncoding::Gzip => "0.1.2".into(),
        };
        let (raw_inventory, _) = signed(&target_manifest);
        let transfer_doc = CompressedTransfer {
            schema_version: "0.3".into(),
            product: "VUA".into(),
            platform: "windows".into(),
            arch: "x64".into(),
            component: ComponentKind::Amf,
            manifest_sha256: digest(&raw_inventory),
            encoding,
            pack: crate::manifest::Pack {
                bytes: compressed_pack.len() as u64,
                sha256: digest(&compressed_pack),
                sources: vec![pack_url],
            },
            files: compressed_files.clone(),
        };
        let staged_dir = temporary.path().join(format!("staged-{encoding:?}"));
        let mut client = crate::transfer::Transfer::loopback(
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            |_| {},
        );
        client
            .component_compressed(
                &target_manifest.amf.files,
                &target_manifest.amf.pack,
                &transfer_doc,
                Some(&installation.amf_root(&installed).unwrap()),
                &staged_dir,
                &temporary.path().join("cache.pack"),
                &temporary.path().join("cache.bin"),
            )
            .await
            .unwrap();
        server.abort();
        // Only the single changed file should have been fetched over HTTP Range!
        assert_eq!(range_requests.load(std::sync::atomic::Ordering::Relaxed), 1);
        installation
            .verify_files(&staged_dir, &target_manifest.amf.files)
            .unwrap();

        let changed_cf = compressed_files
            .iter()
            .find(|f| f.path == "provider/vua-amf-provider.exe")
            .unwrap();
        assert!(changed_cf.compressed_bytes < changed_cf.bytes / 4);
        let frame_slice = &compressed_pack[changed_cf.compressed_offset as usize
            ..(changed_cf.compressed_offset + changed_cf.compressed_bytes) as usize];
        let out_file = temporary.path().join("decoded.bin");
        crate::transfer::decompress_bounded(
            frame_slice,
            encoding,
            changed_cf.bytes,
            &changed_cf.sha256,
            &out_file,
        )
        .unwrap();
        assert_eq!(fs::read(&out_file).unwrap(), changed_bytes);

        // Zip-bomb / length-overflow defense: claiming expected_bytes smaller than actual output aborts immediately.
        let bomb_err = crate::transfer::decompress_bounded(
            frame_slice,
            encoding,
            changed_cf.bytes - 1,
            &changed_cf.sha256,
            &out_file,
        )
        .unwrap_err();
        assert_eq!(bomb_err.0, "unexpected_decompressed_length");
        assert!(!out_file.exists());
    }

    // Anti-downgrade: accept v0.2 seq 1, upgrade to v0.3 seq 2, then reject v0.2 seq 3!
    let v02_seq1 = component_index(&updated, ComponentKind::Amf, 1);
    installation
        .accept_component_index(ComponentKind::Amf, &v02_seq1)
        .unwrap();
    let mut v03_feed = index(&updated, ComponentKind::Amf, 2);
    v03_feed.schema_version = "0.3".into();
    let payload_v03 = serde_json::to_string(&v03_feed).unwrap();
    let raw_v03 = serde_json::to_vec(&SignedManifest {
        schema_version: "0.3".into(),
        algorithm: "ed25519".into(),
        signature: hex(&SigningKey::from_bytes(&[7; 32])
            .sign(payload_v03.as_bytes())
            .to_bytes()),
        payload: payload_v03,
    })
    .unwrap();
    installation
        .accept_component_index(ComponentKind::Amf, &raw_v03)
        .unwrap();
    assert_eq!(
        installation
            .cached_component_index(ComponentKind::Amf)
            .unwrap()
            .unwrap()
            .0
            .schema_version,
        "0.3"
    );
    let v02_seq3 = component_index(&updated, ComponentKind::Amf, 3);
    assert_eq!(
        installation
            .accept_component_index(ComponentKind::Amf, &v02_seq3)
            .unwrap_err()
            .0,
        "component_feed_downgraded"
    );
}
