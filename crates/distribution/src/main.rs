#![cfg_attr(windows, windows_subsystem = "windows")]
use std::sync::{atomic::AtomicBool, Arc};
fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_none_or(|arg| arg.starts_with("--")) {
        args.insert(0, "launch".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    if !["launch", "apply-update", "install"].contains(&args[0].as_str()) {
        let request_cancel = cancel.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::stdin()
                .lock()
                .lines()
                .map_while(std::result::Result::ok)
            {
                if line == "cancel" {
                    request_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    break;
                }
            }
        });
    }
    #[cfg(windows)]
    if args[0] == "install" && !args.iter().any(|arg| arg == "--headless") {
        std::process::exit(gui::install(args, cancel));
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("distribution runtime");
    let result = runtime.block_on(vua_distribution::commands::execute(
        &args,
        cancel,
        vua_distribution::commands::report,
    ));
    match result {
        Ok(value) => println!(
            "{}",
            serde_json::json!({"schemaVersion":"0.1","kind":"result","value":value})
        ),
        Err(error) => {
            let code = if error.0.len() < 80
                && error
                    .0
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
            {
                error.0
            } else {
                "distribution_failed".into()
            };
            println!(
                "{}",
                serde_json::json!({"schemaVersion":"0.1","kind":"error","code":code})
            );
            #[cfg(windows)]
            if (args[0] == "launch" || args[0] == "apply-update")
                && !args.iter().any(|a| a.starts_with("--vua-smoke-test="))
            {
                gui::failure(&code);
            }
            std::process::exit(1);
        }
    }
}
#[cfg(windows)]
mod gui;
