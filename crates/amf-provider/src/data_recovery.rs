use std::path::Path;

/// Closed maintenance entry, before provider startup, BDL opening or migration.
pub fn run() -> Option<Result<(), Box<dyn std::error::Error>>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let action = match args.first().map(String::as_str) {
        Some("--data-snapshot") => "snapshot",
        Some("--data-migrate") => "migrate",
        Some("--data-restore") => "restore",
        Some("--data-commit") => "commit",
        _ => return None,
    };
    Some((|| {
        if args.len() != (if action == "snapshot" { 7 } else { 9 })
            || args[1] != "--profile"
            || args[3] != "--token"
            || args[5] != "--binding"
            || (action != "snapshot" && args[7] != "--receipt")
        {
            return Err("invalid_data_maintenance_arguments".into());
        }
        let expected = if action == "snapshot" {
            None
        } else {
            Some(args[8].as_str())
        };
        let state = vua_bdl_store::recovery::maintain(
            Path::new(&args[2]),
            &args[4],
            &args[6],
            action,
            expected,
        )?;
        let receipt =
            vua_bdl_store::recovery::backup_receipt(Path::new(&args[2]), &args[4], &args[6])?;
        println!(
            "{}",
            serde_json::json!({"schemaVersion":"0.1", "dataRecoveryAbi":vua_bdl_store::recovery::DATA_RECOVERY_ABI, "token":args[4], "binding":args[6], "state":state, "receipt":receipt})
        );
        Ok(())
    })())
}
