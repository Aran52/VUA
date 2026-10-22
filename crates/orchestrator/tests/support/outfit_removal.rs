use super::*;

fn removal_line(h: &Harness) -> AssemblySelection {
    h.platform.removal_mode.store(1, Ordering::SeqCst);
    h.engine.begin("n3-test", choices(&["avatar", "outfit", "outfit2"]), "synthetic-editor").unwrap();
    assert!(h.job(|e, ctx| e.prepare("n3-test", ctx)));
    let mut both = selection(); both.outfits.extend(selection2().outfits);
    let chosen = both.clone();
    let digest = h.engine.plan("n3-test", &both).unwrap()["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |e, ctx| e.assemble("n3-test", chosen, &digest, ctx)));
    both
}
fn removal_digest(h: &Harness) -> String {
    h.engine.plan_outfit_removal("n3-test", "outfit").unwrap()["planDigest"].as_str().unwrap().to_owned()
}

#[test]
fn explicit_removal_retains_the_other_outfit_and_allows_adding_the_first_back() {
    let h = Harness::new(); let both = removal_line(&h);
    let before = h.engine.get("n3-test").unwrap();
    let cleared = h.platform.cleared.load(Ordering::SeqCst);
    let plan = h.engine.plan_outfit_removal("n3-test", "outfit").unwrap();
    assert_eq!(plan["removal"]["objects"][2]["manual"], true);
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &digest, true, ctx)));
    let after = h.engine.get("n3-test").unwrap();
    assert_eq!(after.phase, "succeeded"); assert_eq!(after.selection, Some(selection2()));
    assert_eq!(after.sources.len(), before.sources.len());
    assert_eq!(after.result["facts"]["instances"].as_array().unwrap().len(), 1);
    assert_eq!(after.result["facts"]["instances"][0]["globalObjectId"], "id-outfit2");
    assert_eq!(after.revisions[0].before, RevisionKind::OutfitRemoval);
    assert_eq!(after.revisions[0].state.selection, Some(both.clone()));
    assert_eq!(h.platform.cleared.load(Ordering::SeqCst), cleared);
    let digest = h.engine.plan("n3-test", &both).unwrap()["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |e, ctx| e.assemble("n3-test", both, &digest, ctx)));
    assert_eq!(h.engine.get("n3-test").unwrap().selection.unwrap().outfits.len(), 2);
}

#[test]
fn removal_refuses_unconfirmed_stale_unknown_last_and_opaque_patch_cases_before_snapshot() {
    let h = Harness::new(); removal_line(&h);
    let digest = removal_digest(&h);
    let snapshots = h.platform.snapshots.load(Ordering::SeqCst);
    let d = digest.clone();
    assert_eq!(h.run_job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &d, false, ctx)).unwrap_err(), "vua.amf_production.confirmation_required");
    assert_eq!(h.engine.plan_outfit_removal("n3-test", "unknown").unwrap_err().code, "vua.amf_production.outfit_invalid");
    assert!(h.job(|e, ctx| e.reinspect("n3-test", ctx)));
    assert_eq!(h.run_job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &digest, true, ctx)).unwrap_err(), "vua.amf_production.plan_drift");
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots);
    let run = h.engine.get("n3-test").unwrap();
    let marker = Path::new(&run.project_root).join("ProjectSettings/VUA.FaceTracking.json");
    fs::create_dir_all(marker.parent().unwrap()).unwrap(); fs::write(&marker, "{}").unwrap();
    assert_eq!(h.engine.plan_outfit_removal("n3-test", "outfit").unwrap_err().code, "vua.amf_production.face_tracking_manual_replay");
    let d = "a".repeat(64);
    assert_eq!(h.run_job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &d, true, ctx)).unwrap_err(), "vua.amf_production.face_tracking_manual_replay");
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots);
    fs::remove_file(marker).unwrap();
    let digest = removal_digest(&h);
    assert!(h.job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &digest, true, ctx)));
    assert_eq!(h.engine.plan_outfit_removal("n3-test", "outfit2").unwrap_err().code, "vua.amf_production.last_outfit");
}

#[test]
fn removal_failure_cancellation_unverified_result_and_missing_proof_restore_prior_facts() {
    for mode in [2, 3, 4, 5] {
        let h = Harness::new(); removal_line(&h);
        let before = h.engine.get("n3-test").unwrap(); let digest = removal_digest(&h);
        h.platform.removal_mode.store(mode, Ordering::SeqCst);
        assert!(!h.job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &digest, true, ctx)));
        let after = h.engine.get("n3-test").unwrap();
        assert_eq!(after.recovery.as_deref(), Some("restored"));
        assert_eq!(after.selection, before.selection); assert_eq!(after.result, before.result);
        assert_eq!(after.revision, before.revision); assert_eq!(after.applied_fits, before.applied_fits);
        assert_eq!(after.revisions.len(), before.revisions.len());
        assert_eq!(h.platform.fingerprint(Path::new(&after.project_root)).unwrap(), "after");
    }
}

#[test]
fn removal_restore_failure_keeps_its_durable_version_and_recovery_point() {
    let h = Harness::new(); removal_line(&h); let digest = removal_digest(&h);
    h.platform.removal_mode.store(2, Ordering::SeqCst); h.platform.fail_restore.store(true, Ordering::SeqCst);
    assert!(!h.job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &digest, true, ctx)));
    let after = h.engine.get("n3-test").unwrap();
    assert!(after.recovery.unwrap().starts_with("restore_failed:"));
    assert_eq!(after.revisions[0].before, RevisionKind::OutfitRemoval); assert!(after.snapshot_id.is_some());
}

#[test]
fn optional_old_inventory_and_malformed_new_inventory_are_distinguished() {
    validate_outfit_removal_facts(&serde_json::json!({})).unwrap();
    let inventory = synthetic_removal("outfit");
    assert!(valid_outfit_removal(&inventory));
    let mut malformed = inventory.clone(); malformed.objects.push(malformed.objects[0].clone());
    assert!(!valid_outfit_removal(&malformed));
    let mut blocked = inventory; blocked.blocked_reason = "contains_owned_object".into();
    assert!(valid_outfit_removal(&blocked));
    assert!(validate_outfit_removal_facts(&serde_json::json!({"outfitRemoval":[blocked.clone(),blocked]})).is_err());
}

#[test]
fn removal_keeps_the_retained_garments_applied_fit_and_pending_manual_adjustment() {
    let h = Harness::new(); removal_line(&h);
    let mut run = h.engine.get("n3-test").unwrap();
    let fits: Vec<_> = ["outfit", "outfit2"].into_iter().map(|id| AppliedFit {
        source_id: id.into(), fit_id: None, name: None, objects: vec![fit_object("Clip", 0.5)],
    }).collect();
    run.applied_fits = fits.clone();
    fs::write(h.root.join("production/runs/n3-test.json"), serde_json::to_vec(&run).unwrap()).unwrap();
    let pending = GarmentFitChanges { source_id:"outfit2".into(), moved:vec![fit_object("Clip",0.8)], missing:Vec::new() };
    *h.platform.fit_changes.lock().unwrap() = Some(serde_json::json!([pending]));
    assert!(h.job(|e, ctx| e.reinspect("n3-test", ctx)));
    let digest = removal_digest(&h);
    assert!(h.job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &digest, true, ctx)));
    let after = h.engine.get("n3-test").unwrap();
    assert_eq!(after.applied_fits, vec![fits[1].clone()]);
    assert_eq!(after.fit_changes, vec![pending]);
    assert_eq!(after.revisions[0].state.fits, fits);
}

#[test]
fn changed_warnings_invalidate_the_digest_and_missing_or_blocked_inventory_cannot_delete() {
    let h = Harness::new(); removal_line(&h);
    let digest = removal_digest(&h); let snapshots = h.platform.snapshots.load(Ordering::SeqCst);
    let file = h.root.join("production/runs/n3-test.json");
    let mut run = h.engine.get("n3-test").unwrap();
    run.result["facts"]["outfitRemoval"][0]["objects"][2]["manual"] = serde_json::json!(false);
    fs::write(&file, serde_json::to_vec(&run).unwrap()).unwrap();
    assert_ne!(removal_digest(&h), digest);
    assert_eq!(h.run_job(move |e, ctx| e.remove_outfit("n3-test", "outfit", &digest, true, ctx)).unwrap_err(), "vua.amf_production.plan_drift");
    run.result["facts"]["outfitRemoval"][0]["blockedReason"] = serde_json::json!("contains_owned_object");
    fs::write(&file, serde_json::to_vec(&run).unwrap()).unwrap();
    assert_eq!(h.engine.plan_outfit_removal("n3-test", "outfit").unwrap_err().code, "vua.amf_production.outfit_removal_blocked");
    run.result["facts"].as_object_mut().unwrap().remove("outfitRemoval");
    fs::write(&file, serde_json::to_vec(&run).unwrap()).unwrap();
    assert_eq!(h.engine.plan_outfit_removal("n3-test", "outfit").unwrap_err().code, "vua.amf_production.inspect_required");
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots);
}
