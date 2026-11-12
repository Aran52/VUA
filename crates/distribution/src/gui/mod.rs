pub mod copy;
pub mod theme;

use copy::Lang;
use std::{
    ffi::c_void,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::Command,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use theme::{
    render_card, BodySpec, ButtonAction, ButtonRole, ButtonSpec, CardModel, CardTone, DibSurface,
    Palette, RenderedLayout, LOGICAL_HEIGHT, LOGICAL_WIDTH,
};
use vua_distribution::{commands::option, layout::Installation, transfer::Progress};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{
        LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
        Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    },
    UI::{
        Input::KeyboardAndMouse::{
            ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE,
            VK_LEFT, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB,
        },
        WindowsAndMessaging::*,
    },
};

const WM_APP_WORKER_UPDATE: u32 = WM_APP + 1;
const WM_MOUSELEAVE_MSG: u32 = 0x02A3;
const TIMER_ANIM: usize = 1;
const ICON_RESOURCE_ID: usize = 1;

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

type SetProcessDpiAwarenessContextFn = unsafe extern "system" fn(isize) -> i32;
type SetProcessDpiAwareFn = unsafe extern "system" fn() -> i32;
type GetDpiForWindowFn = unsafe extern "system" fn(HWND) -> u32;
type GetDpiForSystemFn = unsafe extern "system" fn() -> u32;
type AdjustWindowRectExForDpiFn = unsafe extern "system" fn(*mut RECT, u32, i32, u32, u32) -> i32;
type DwmSetWindowAttributeFn = unsafe extern "system" fn(HWND, u32, *const c_void, u32) -> i32;

fn user32_proc(name: &std::ffi::CStr) -> Option<unsafe extern "system" fn() -> isize> {
    unsafe {
        let module = GetModuleHandleW(wide("user32.dll").as_ptr());
        if module.is_null() {
            return None;
        }
        GetProcAddress(module, name.as_ptr().cast())
    }
}

fn enable_per_monitor_dpi_v2() {
    unsafe {
        if let Some(proc) = user32_proc(c"SetProcessDpiAwarenessContext") {
            let func: SetProcessDpiAwarenessContextFn = std::mem::transmute(proc);
            // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4
            if func(-4) != 0 {
                return;
            }
        }
        if let Some(proc) = user32_proc(c"SetProcessDPIAware") {
            let func: SetProcessDpiAwareFn = std::mem::transmute(proc);
            let _ = func();
        }
    }
}

fn system_dpi() -> u32 {
    unsafe {
        if let Some(proc) = user32_proc(c"GetDpiForSystem") {
            let func: GetDpiForSystemFn = std::mem::transmute(proc);
            let dpi = func();
            if dpi >= 96 {
                return dpi;
            }
        }
        let dc = GetDC(ptr::null_mut());
        if !dc.is_null() {
            let dpi = GetDeviceCaps(dc, LOGPIXELSX as i32);
            ReleaseDC(ptr::null_mut(), dc);
            if dpi >= 96 {
                return dpi as u32;
            }
        }
        96
    }
}

fn window_dpi(hwnd: HWND) -> u32 {
    unsafe {
        if !hwnd.is_null() {
            if let Some(proc) = user32_proc(c"GetDpiForWindow") {
                let func: GetDpiForWindowFn = std::mem::transmute(proc);
                let dpi = func(hwnd);
                if dpi >= 96 {
                    return dpi;
                }
            }
        }
    }
    system_dpi()
}

fn outer_size_for_client(
    client_w: i32,
    client_h: i32,
    style: u32,
    ex_style: u32,
    dpi: u32,
) -> (i32, i32) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: client_w,
        bottom: client_h,
    };
    unsafe {
        let adjusted = if let Some(proc) = user32_proc(c"AdjustWindowRectExForDpi") {
            let func: AdjustWindowRectExForDpiFn = std::mem::transmute(proc);
            func(&mut rect, style, 0, ex_style, dpi) != 0
        } else {
            false
        };
        if !adjusted {
            let _ = AdjustWindowRectEx(&mut rect, style, 0, ex_style);
        }
    }
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn apply_dwm_chrome(hwnd: HWND, palette: &Palette) {
    static DWM_SET_ATTR: OnceLock<Option<DwmSetWindowAttributeFn>> = OnceLock::new();
    let Some(dwm_set) = *DWM_SET_ATTR.get_or_init(|| unsafe {
        let lib = LoadLibraryW(wide("dwmapi.dll").as_ptr());
        if lib.is_null() {
            return None;
        }
        GetProcAddress(lib, c"DwmSetWindowAttribute".as_ptr().cast()).map(|proc| {
            std::mem::transmute::<unsafe extern "system" fn() -> isize, DwmSetWindowAttributeFn>(
                proc,
            )
        })
    }) else {
        return;
    };

    unsafe {
        let dark: i32 = if palette.is_dark { 1 } else { 0 };
        // DWMWA_USE_IMMERSIVE_DARK_MODE (20, fallback 19 on early Win10)
        if dwm_set(hwnd, 20, (&dark as *const i32).cast(), 4) != 0 {
            let _ = dwm_set(hwnd, 19, (&dark as *const i32).cast(), 4);
        }
        // DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2 (Windows 11)
        let round: u32 = 2;
        let _ = dwm_set(hwnd, 33, (&round as *const u32).cast(), 4);
        // DWMWA_CAPTION_COLOR = 35, DWMWA_TEXT_COLOR = 36 (Windows 11)
        let caption_color: u32 = palette.canvas.colorref();
        let text_color: u32 = palette.text_strong.colorref();
        let _ = dwm_set(hwnd, 35, (&caption_color as *const u32).cast(), 4);
        let _ = dwm_set(hwnd, 36, (&text_color as *const u32).cast(), 4);
    }
}

fn load_embedded_icon(size_px: i32) -> HICON {
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        LoadImageW(
            instance,
            ICON_RESOURCE_ID as *const u16,
            IMAGE_ICON,
            size_px.max(16),
            size_px.max(16),
            0,
        ) as HICON
    }
}

fn read_manifest_version(input_dir: &Path) -> Option<String> {
    let raw = fs::read(input_dir.join("vua-release.json")).ok()?;
    let signed: vua_distribution::manifest::SignedManifest = serde_json::from_slice(&raw).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&signed.payload).ok()?;
    let ver = parsed
        .get("releaseVersion")
        .or_else(|| parsed.get("host").and_then(|h| h.get("version")))
        .and_then(serde_json::Value::as_str)?;
    if !ver.is_empty()
        && ver.len() <= 24
        && ver
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
    {
        Some(format!("v{ver}"))
    } else {
        None
    }
}

fn read_installed_version(root: &Path) -> Option<String> {
    let installation = Installation::open(root, false).ok()?;
    let snap = installation.snapshot().ok()?;
    if snap.host_version.is_empty() {
        None
    } else {
        Some(format!("v{}", snap.host_version))
    }
}

#[derive(Clone, Debug)]
enum WorkerOutcome {
    Succeeded,
    Cancelled,
    Failed(String),
}

#[derive(Default)]
struct SharedWorker {
    unpacking: bool,
    latest: Option<Progress>,
    outcome: Option<WorkerOutcome>,
}

enum Screen {
    InstallRunning { started: Instant },
    InstallFinishing { started: Instant },
    InstallError { busy: bool },
    UninstallConfirm,
    UninstallRunning { started: Instant },
    UninstallError { busy: bool },
    Failure { code_line: String },
}

impl Screen {
    fn is_animated(&self) -> bool {
        matches!(
            self,
            Self::InstallRunning { .. }
                | Self::InstallFinishing { .. }
                | Self::UninstallRunning { .. }
        )
    }
}

enum FlowKind {
    Install {
        args: Vec<String>,
        session_dir: Option<PathBuf>,
    },
    Uninstall {
        args: Vec<String>,
    },
    Failure,
}

struct WindowState {
    flow: FlowKind,
    screen: Screen,
    lang: Lang,
    palette: Palette,
    version_badge: Option<String>,
    dpi: u32,
    surface: DibSurface,
    card_icon: HICON,
    big_icon: HICON,
    small_icon: HICON,
    layout: RenderedLayout,
    hovered_idx: Option<usize>,
    pressed_idx: Option<usize>,
    focused_idx: Option<usize>,
    tracking_mouse: bool,
    cancel: Arc<AtomicBool>,
    shared: Arc<Mutex<SharedWorker>>,
    worker: Option<JoinHandle<()>>,
    exit_code: i32,
}

impl Drop for WindowState {
    fn drop(&mut self) {
        unsafe {
            for icon in [self.card_icon, self.big_icon, self.small_icon] {
                if !icon.is_null() {
                    DestroyIcon(icon);
                }
            }
        }
    }
}

impl WindowState {
    fn set_timer_state(&self, hwnd: HWND) {
        unsafe {
            if self.screen.is_animated() {
                SetTimer(hwnd, TIMER_ANIM, 16, None);
            } else {
                KillTimer(hwnd, TIMER_ANIM);
            }
        }
    }

    fn reload_card_icon(&mut self) {
        let icon_px = ((32.0 * self.dpi as f32) / 96.0).round() as i32;
        let next = load_embedded_icon(icon_px);
        if !next.is_null() {
            if !self.card_icon.is_null() {
                unsafe {
                    DestroyIcon(self.card_icon);
                }
            }
            self.card_icon = next;
        }
    }

    fn spawn_install_attempt(&mut self, hwnd: HWND) {
        let FlowKind::Install { args, session_dir } = &self.flow else {
            return;
        };
        let args = args.clone();
        let session_dir = session_dir.clone();
        self.cancel.store(false, Ordering::Relaxed);
        {
            let mut guard = self.shared.lock().expect("worker state");
            guard.unpacking = session_dir
                .as_ref()
                .is_some_and(|d| !d.join("input-ready").exists());
            guard.latest = None;
            guard.outcome = None;
        }
        let cancel = self.cancel.clone();
        let shared = self.shared.clone();
        let hwnd_raw = hwnd as isize;

        self.worker = Some(std::thread::spawn(move || {
            let post_update = || unsafe {
                let _ = PostMessageW(hwnd_raw as HWND, WM_APP_WORKER_UPDATE, 0, 0);
            };

            if let Some(dir) = &session_dir {
                let ready = dir.join("input-ready");
                let failed = dir.join("input-failed");
                let external_cancel = dir.join("cancel");
                while !ready.exists() {
                    if cancel.load(Ordering::Relaxed) || external_cancel.exists() {
                        let _ = fs::write(&external_cancel, b"1");
                        shared.lock().expect("worker state").outcome =
                            Some(WorkerOutcome::Cancelled);
                        post_update();
                        return;
                    }
                    if failed.exists() {
                        shared.lock().expect("worker state").outcome =
                            Some(WorkerOutcome::Failed("incomplete_release_pack".into()));
                        post_update();
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                shared.lock().expect("worker state").unpacking = false;
                post_update();
            }

            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => {
                    shared.lock().expect("worker state").outcome =
                        Some(WorkerOutcome::Failed("runtime_unavailable".into()));
                    post_update();
                    return;
                }
            };

            let shared_progress = shared.clone();
            let result = runtime.block_on(vua_distribution::commands::execute(
                &args,
                cancel.clone(),
                move |progress| {
                    let mut guard = shared_progress.lock().expect("install progress");
                    guard.unpacking = false;
                    guard.latest = Some(progress);
                    drop(guard);
                    unsafe {
                        let _ = PostMessageW(hwnd_raw as HWND, WM_APP_WORKER_UPDATE, 0, 0);
                    }
                },
            ));

            let outcome = match result {
                Ok(_) => {
                    if let Some(dir) = &session_dir {
                        let _ = fs::write(dir.join("succeeded"), b"1");
                    }
                    WorkerOutcome::Succeeded
                }
                Err(err) if err.0 == "cancelled" || cancel.load(Ordering::Relaxed) => {
                    if let Some(dir) = &session_dir {
                        let _ = fs::write(dir.join("cancel"), b"1");
                    }
                    WorkerOutcome::Cancelled
                }
                Err(err) => WorkerOutcome::Failed(err.0),
            };
            shared.lock().expect("worker state").outcome = Some(outcome);
            post_update();
        }));
    }

    fn spawn_uninstall_attempt(&mut self, hwnd: HWND) {
        let FlowKind::Uninstall { args } = &self.flow else {
            return;
        };
        let args = args.clone();
        self.cancel.store(false, Ordering::Relaxed);
        {
            let mut guard = self.shared.lock().expect("worker state");
            guard.unpacking = false;
            guard.latest = None;
            guard.outcome = None;
        }
        let cancel = self.cancel.clone();
        let shared = self.shared.clone();
        let hwnd_raw = hwnd as isize;

        self.worker = Some(std::thread::spawn(move || {
            let started = Instant::now();
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => {
                    shared.lock().expect("worker state").outcome =
                        Some(WorkerOutcome::Failed("runtime_unavailable".into()));
                    unsafe {
                        let _ = PostMessageW(hwnd_raw as HWND, WM_APP_WORKER_UPDATE, 0, 0);
                    }
                    return;
                }
            };
            let result =
                runtime.block_on(vua_distribution::commands::execute(&args, cancel, |_| {}));
            if result.is_ok() {
                // Keep progress visible briefly so the card transition feels smooth rather than a 1-frame flash
                let min_visible = Duration::from_millis(220);
                if let Some(rem) = min_visible.checked_sub(started.elapsed()) {
                    std::thread::sleep(rem);
                }
            }
            let outcome = match result {
                Ok(_) => WorkerOutcome::Succeeded,
                Err(err) => WorkerOutcome::Failed(err.0),
            };
            shared.lock().expect("worker state").outcome = Some(outcome);
            unsafe {
                let _ = PostMessageW(hwnd_raw as HWND, WM_APP_WORKER_UPDATE, 0, 0);
            }
        }));
    }

    fn poll_worker_transition(&mut self, hwnd: HWND) {
        let outcome = self.shared.lock().expect("worker state").outcome.clone();
        let Some(outcome) = outcome else {
            return;
        };

        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
        self.shared.lock().expect("worker state").outcome = None;
        self.hovered_idx = None;
        self.pressed_idx = None;
        self.focused_idx = None;

        match (&self.flow, outcome) {
            (FlowKind::Install { .. }, WorkerOutcome::Succeeded) => {
                self.exit_code = 0;
                self.screen = Screen::InstallFinishing {
                    started: Instant::now(),
                };
                self.set_timer_state(hwnd);
            }
            (FlowKind::Install { .. }, WorkerOutcome::Cancelled) => {
                self.exit_code = 1;
                unsafe {
                    DestroyWindow(hwnd);
                }
            }
            (FlowKind::Install { .. }, WorkerOutcome::Failed(code)) => {
                self.exit_code = 1;
                self.screen = Screen::InstallError {
                    busy: code == "installation_busy",
                };
                self.set_timer_state(hwnd);
            }
            (FlowKind::Uninstall { .. }, WorkerOutcome::Succeeded) => {
                self.exit_code = 0;
                unsafe {
                    DestroyWindow(hwnd);
                }
            }
            (FlowKind::Uninstall { .. }, WorkerOutcome::Cancelled) => {
                self.exit_code = 1;
                unsafe {
                    DestroyWindow(hwnd);
                }
            }
            (FlowKind::Uninstall { .. }, WorkerOutcome::Failed(code)) => {
                self.exit_code = 1;
                self.screen = Screen::UninstallError {
                    busy: code == "installation_busy",
                };
                self.set_timer_state(hwnd);
            }
            (FlowKind::Failure, _) => {}
        }
    }

    fn request_cancel_or_close(&mut self, hwnd: HWND) {
        match &self.screen {
            Screen::InstallRunning { .. } => {
                self.cancel.store(true, Ordering::Relaxed);
                if let FlowKind::Install {
                    session_dir: Some(dir),
                    ..
                } = &self.flow
                {
                    let _ = fs::write(dir.join("cancel"), b"1");
                }
                unsafe {
                    InvalidateRect(hwnd, ptr::null(), 0);
                }
            }
            Screen::InstallFinishing { .. } | Screen::UninstallRunning { .. } => {}
            Screen::InstallError { .. }
            | Screen::UninstallConfirm
            | Screen::UninstallError { .. }
            | Screen::Failure { .. } => {
                self.exit_code = 1;
                unsafe {
                    DestroyWindow(hwnd);
                }
            }
        }
    }

    fn trigger_action(&mut self, hwnd: HWND, action: ButtonAction) {
        match action {
            ButtonAction::Cancel | ButtonAction::Close => self.request_cancel_or_close(hwnd),
            ButtonAction::Retry => match &self.flow {
                FlowKind::Install { .. } => {
                    self.hovered_idx = None;
                    self.pressed_idx = None;
                    self.focused_idx = None;
                    self.screen = Screen::InstallRunning {
                        started: Instant::now(),
                    };
                    self.set_timer_state(hwnd);
                    self.spawn_install_attempt(hwnd);
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                }
                FlowKind::Uninstall { .. } => {
                    self.hovered_idx = None;
                    self.pressed_idx = None;
                    self.focused_idx = None;
                    self.screen = Screen::UninstallRunning {
                        started: Instant::now(),
                    };
                    self.set_timer_state(hwnd);
                    self.spawn_uninstall_attempt(hwnd);
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                }
                FlowKind::Failure => {}
            },
            ButtonAction::ConfirmUninstall => {
                self.hovered_idx = None;
                self.pressed_idx = None;
                self.focused_idx = None;
                self.screen = Screen::UninstallRunning {
                    started: Instant::now(),
                };
                self.set_timer_state(hwnd);
                self.spawn_uninstall_attempt(hwnd);
                unsafe {
                    InvalidateRect(hwnd, ptr::null(), 0);
                }
            }
        }
    }

    fn paint(&mut self, dc: HDC, client_w: i32, client_h: i32) {
        self.surface.ensure_size(client_w.max(1), client_h.max(1));
        let copy = self.lang.copy();
        let cancelling = self.cancel.load(Ordering::Relaxed);
        let ver_ref = self.version_badge.as_deref();

        let button_style = |idx: usize,
                            label: &'static str,
                            role: ButtonRole,
                            action: ButtonAction,
                            enabled: bool|
         -> ButtonSpec<'static> {
            ButtonSpec {
                label,
                role,
                action,
                enabled,
                hovered: enabled && self.hovered_idx == Some(idx),
                pressed: enabled && self.pressed_idx == Some(idx) && self.hovered_idx == Some(idx),
                focused: enabled && self.focused_idx == Some(idx),
            }
        };

        match &self.screen {
            Screen::InstallRunning { started } => {
                let (unpacking, latest) = {
                    let guard = self.shared.lock().expect("worker state");
                    (guard.unpacking, guard.latest.clone())
                };
                let anim_phase = (started.elapsed().as_secs_f32() / 1.30).rem_euclid(1.0);
                let mut metric_buf = String::new();
                let (subtitle, phase_label, fraction): (&str, &str, Option<f32>) = if cancelling {
                    (copy.install_sub_prep, copy.btn_cancelling, None)
                } else if unpacking {
                    (copy.install_sub_prep, copy.install_phase_unpack, None)
                } else if let Some(p) = &latest {
                    match p.phase.as_str() {
                        "downloading" => {
                            let frac = (p.completed as f32 / p.total.max(1) as f32).clamp(0.0, 1.0);
                            if p.bytes_per_second > 0 {
                                metric_buf = format!(
                                    "{:.1} / {:.1} MB  ·  {:.1} MB/s",
                                    p.completed as f64 / 1_048_576.0,
                                    p.total as f64 / 1_048_576.0,
                                    p.bytes_per_second as f64 / 1_048_576.0
                                );
                            } else {
                                metric_buf = format!(
                                    "{:.1} / {:.1} MB",
                                    p.completed as f64 / 1_048_576.0,
                                    p.total as f64 / 1_048_576.0
                                );
                            }
                            (
                                copy.install_sub_download,
                                copy.install_phase_download,
                                Some(frac),
                            )
                        }
                        "staging" | "verifying" | "extracting" => {
                            let frac = (p.completed as f32 / p.total.max(1) as f32).clamp(0.0, 1.0);
                            metric_buf = format!("{:.0}%", (frac * 100.0).round());
                            (copy.install_sub_prep, copy.install_phase_verify, Some(frac))
                        }
                        "runtime-reused" | "ready" => {
                            metric_buf = "100%".into();
                            (copy.install_sub_prep, copy.install_phase_verify, Some(1.0))
                        }
                        _ => (copy.install_sub_prep, copy.install_phase_verify, None),
                    }
                } else {
                    (copy.install_sub_prep, copy.install_phase_verify, None)
                };

                let btns = [button_style(
                    0,
                    if cancelling {
                        copy.btn_cancelling
                    } else {
                        copy.btn_cancel
                    },
                    ButtonRole::Secondary,
                    ButtonAction::Cancel,
                    !cancelling,
                )];
                let model = CardModel {
                    lang: self.lang,
                    palette: self.palette,
                    title: copy.install_title,
                    version_badge: ver_ref,
                    subtitle,
                    body: BodySpec::Progress {
                        phase_label,
                        metric_label: (!metric_buf.is_empty()).then_some(metric_buf.as_str()),
                        fraction,
                        anim_phase,
                    },
                    buttons: &btns,
                };
                self.layout = render_card(&mut self.surface, self.dpi, self.card_icon, &model);
            }
            Screen::InstallFinishing { .. } => {
                let btns = [button_style(
                    0,
                    copy.btn_cancel,
                    ButtonRole::Secondary,
                    ButtonAction::Cancel,
                    false,
                )];
                let model = CardModel {
                    lang: self.lang,
                    palette: self.palette,
                    title: copy.install_title,
                    version_badge: ver_ref,
                    subtitle: copy.install_finishing,
                    body: BodySpec::Progress {
                        phase_label: copy.install_finishing,
                        metric_label: Some("100%"),
                        fraction: Some(1.0),
                        anim_phase: 0.0,
                    },
                    buttons: &btns,
                };
                self.layout = render_card(&mut self.surface, self.dpi, self.card_icon, &model);
            }
            Screen::InstallError { busy } => {
                let btns = [
                    button_style(
                        0,
                        copy.btn_close,
                        ButtonRole::Secondary,
                        ButtonAction::Close,
                        true,
                    ),
                    button_style(
                        1,
                        copy.btn_retry,
                        ButtonRole::Primary,
                        ButtonAction::Retry,
                        true,
                    ),
                ];
                let model = CardModel {
                    lang: self.lang,
                    palette: self.palette,
                    title: copy.install_error_title,
                    version_badge: ver_ref,
                    subtitle: copy.install_error_sub,
                    body: BodySpec::Message {
                        tone: CardTone::Danger,
                        code_badge: None,
                        body: if *busy {
                            copy.install_error_busy
                        } else {
                            copy.install_error_general
                        },
                    },
                    buttons: &btns,
                };
                self.layout = render_card(&mut self.surface, self.dpi, self.card_icon, &model);
            }
            Screen::UninstallConfirm => {
                let btns = [
                    button_style(
                        0,
                        copy.btn_cancel,
                        ButtonRole::Secondary,
                        ButtonAction::Cancel,
                        true,
                    ),
                    button_style(
                        1,
                        copy.btn_uninstall,
                        ButtonRole::Danger,
                        ButtonAction::ConfirmUninstall,
                        true,
                    ),
                ];
                let model = CardModel {
                    lang: self.lang,
                    palette: self.palette,
                    title: copy.uninstall_confirm_title,
                    version_badge: ver_ref,
                    subtitle: copy.uninstall_confirm_sub,
                    body: BodySpec::Message {
                        tone: CardTone::Neutral,
                        code_badge: None,
                        body: copy.uninstall_confirm_body,
                    },
                    buttons: &btns,
                };
                self.layout = render_card(&mut self.surface, self.dpi, self.card_icon, &model);
            }
            Screen::UninstallRunning { started } => {
                let anim_phase = (started.elapsed().as_secs_f32() / 1.30).rem_euclid(1.0);
                let btns = [button_style(
                    0,
                    copy.btn_removing,
                    ButtonRole::Secondary,
                    ButtonAction::Cancel,
                    false,
                )];
                let model = CardModel {
                    lang: self.lang,
                    palette: self.palette,
                    title: copy.uninstall_running_title,
                    version_badge: ver_ref,
                    subtitle: copy.uninstall_running_sub,
                    body: BodySpec::Progress {
                        phase_label: copy.uninstall_phase_remove,
                        metric_label: None,
                        fraction: None,
                        anim_phase,
                    },
                    buttons: &btns,
                };
                self.layout = render_card(&mut self.surface, self.dpi, self.card_icon, &model);
            }
            Screen::UninstallError { busy } => {
                let btns = [
                    button_style(
                        0,
                        copy.btn_close,
                        ButtonRole::Secondary,
                        ButtonAction::Close,
                        true,
                    ),
                    button_style(
                        1,
                        copy.btn_retry,
                        ButtonRole::Primary,
                        ButtonAction::Retry,
                        true,
                    ),
                ];
                let model = CardModel {
                    lang: self.lang,
                    palette: self.palette,
                    title: if *busy {
                        copy.uninstall_error_busy_title
                    } else {
                        copy.uninstall_error_general_title
                    },
                    version_badge: ver_ref,
                    subtitle: copy.uninstall_error_sub,
                    body: BodySpec::Message {
                        tone: CardTone::Danger,
                        code_badge: None,
                        body: if *busy {
                            copy.uninstall_error_busy_body
                        } else {
                            copy.uninstall_error_general_body
                        },
                    },
                    buttons: &btns,
                };
                self.layout = render_card(&mut self.surface, self.dpi, self.card_icon, &model);
            }
            Screen::Failure { code_line } => {
                let btns = [button_style(
                    0,
                    copy.btn_close,
                    ButtonRole::Primary,
                    ButtonAction::Close,
                    true,
                )];
                let model = CardModel {
                    lang: self.lang,
                    palette: self.palette,
                    title: copy.failure_title,
                    version_badge: None,
                    subtitle: copy.failure_sub,
                    body: BodySpec::Message {
                        tone: CardTone::Danger,
                        code_badge: Some(code_line.as_str()),
                        body: copy.failure_body,
                    },
                    buttons: &btns,
                };
                self.layout = render_card(&mut self.surface, self.dpi, self.card_icon, &model);
            }
        }

        unsafe {
            let _ = BitBlt(
                dc,
                0,
                0,
                self.surface.width,
                self.surface.height,
                self.surface.dc,
                0,
                0,
                SRCCOPY,
            );
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut WindowState;
    if !ptr.is_null() {
        let state = unsafe { &mut *ptr };
        match message {
            WM_ERASEBKGND => return 1,
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
                let dc = unsafe { BeginPaint(hwnd, &mut ps) };
                let mut rc: RECT = unsafe { std::mem::zeroed() };
                unsafe {
                    GetClientRect(hwnd, &mut rc);
                }
                state.paint(dc, rc.right - rc.left, rc.bottom - rc.top);
                unsafe {
                    EndPaint(hwnd, &ps);
                }
                return 0;
            }
            WM_APP_WORKER_UPDATE => {
                state.poll_worker_transition(hwnd);
                unsafe {
                    InvalidateRect(hwnd, ptr::null(), 0);
                }
                return 0;
            }
            WM_TIMER if w == TIMER_ANIM => {
                state.poll_worker_transition(hwnd);
                if let Screen::InstallFinishing { started } = &state.screen {
                    if started.elapsed() >= Duration::from_millis(260) {
                        unsafe {
                            KillTimer(hwnd, TIMER_ANIM);
                            DestroyWindow(hwnd);
                        }
                        return 0;
                    }
                }
                unsafe {
                    InvalidateRect(hwnd, ptr::null(), 0);
                }
                return 0;
            }
            WM_MOUSEMOVE => {
                let x = (l as i16) as i32;
                let y = ((l >> 16) as i16) as i32;
                if !state.tracking_mouse {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    if unsafe { TrackMouseEvent(&mut tme) } != 0 {
                        state.tracking_mouse = true;
                    }
                }
                let next_hover = state
                    .layout
                    .buttons
                    .iter()
                    .position(|&(_, enabled, rect)| enabled && rect.contains(x, y));
                if next_hover != state.hovered_idx {
                    state.hovered_idx = next_hover;
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                }
                return 0;
            }
            WM_MOUSELEAVE_MSG => {
                state.tracking_mouse = false;
                if state.hovered_idx.is_some() {
                    state.hovered_idx = None;
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                }
                return 0;
            }
            WM_SETCURSOR => {
                if (l & 0xffff) as u32 == HTCLIENT {
                    let cursor_id = if state.hovered_idx.is_some() {
                        IDC_HAND
                    } else {
                        IDC_ARROW
                    };
                    unsafe {
                        SetCursor(LoadCursorW(ptr::null_mut(), cursor_id));
                    }
                    return 1;
                }
            }
            WM_LBUTTONDOWN => {
                let x = (l as i16) as i32;
                let y = ((l >> 16) as i16) as i32;
                if let Some(idx) = state
                    .layout
                    .buttons
                    .iter()
                    .position(|&(_, enabled, rect)| enabled && rect.contains(x, y))
                {
                    state.pressed_idx = Some(idx);
                    state.focused_idx = Some(idx);
                    unsafe {
                        SetCapture(hwnd);
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                }
                return 0;
            }
            WM_LBUTTONUP => {
                let x = (l as i16) as i32;
                let y = ((l >> 16) as i16) as i32;
                if let Some(pressed) = state.pressed_idx.take() {
                    unsafe {
                        let _ = ReleaseCapture();
                    }
                    let clicked =
                        state
                            .layout
                            .buttons
                            .get(pressed)
                            .and_then(|&(action, enabled, rect)| {
                                (enabled && rect.contains(x, y)).then_some(action)
                            });
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                    if let Some(action) = clicked {
                        state.trigger_action(hwnd, action);
                    }
                }
                return 0;
            }
            WM_KEYDOWN => {
                let vk = (w & 0xffff) as u16;
                if vk == VK_ESCAPE {
                    state.request_cancel_or_close(hwnd);
                    return 0;
                }
                if vk == VK_TAB || vk == VK_LEFT || vk == VK_RIGHT {
                    let enabled_indices: Vec<usize> = state
                        .layout
                        .buttons
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &(_, enabled, _))| enabled.then_some(i))
                        .collect();
                    if !enabled_indices.is_empty() {
                        let next = match state
                            .focused_idx
                            .and_then(|cur| enabled_indices.iter().position(|&i| i == cur))
                        {
                            Some(pos) if vk == VK_LEFT => {
                                enabled_indices
                                    [(pos + enabled_indices.len() - 1) % enabled_indices.len()]
                            }
                            Some(pos) => enabled_indices[(pos + 1) % enabled_indices.len()],
                            None => *enabled_indices.last().unwrap(),
                        };
                        state.focused_idx = Some(next);
                        unsafe {
                            InvalidateRect(hwnd, ptr::null(), 0);
                        }
                    }
                    return 0;
                }
                if vk == VK_RETURN || vk == VK_SPACE {
                    let target_action = if let Some(idx) = state.focused_idx {
                        state
                            .layout
                            .buttons
                            .get(idx)
                            .and_then(|&(act, en, _)| en.then_some(act))
                    } else if vk == VK_RETURN {
                        // Enter defaults to Retry or Close on static error/failure cards, never auto-confirms destructive Uninstall without focus
                        state.layout.buttons.iter().find_map(|&(act, en, _)| {
                            (en && matches!(act, ButtonAction::Retry | ButtonAction::Close))
                                .then_some(act)
                        })
                    } else {
                        None
                    };
                    if let Some(action) = target_action {
                        state.trigger_action(hwnd, action);
                    }
                    return 0;
                }
            }
            WM_SETTINGCHANGE => {
                let next_palette = Palette::system();
                if next_palette.is_dark != state.palette.is_dark {
                    state.palette = next_palette;
                    apply_dwm_chrome(hwnd, &state.palette);
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                }
            }
            WM_DPICHANGED => {
                let new_dpi = (w & 0xffff) as u32;
                if new_dpi >= 96 {
                    state.dpi = new_dpi;
                    state.reload_card_icon();
                    if l != 0 {
                        let suggested = unsafe { &*(l as *const RECT) };
                        unsafe {
                            SetWindowPos(
                                hwnd,
                                ptr::null_mut(),
                                suggested.left,
                                suggested.top,
                                suggested.right - suggested.left,
                                suggested.bottom - suggested.top,
                                SWP_NOZORDER | SWP_NOACTIVATE,
                            );
                        }
                    }
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                }
                return 0;
            }
            WM_CLOSE => {
                state.request_cancel_or_close(hwnd);
                return 0;
            }
            _ => {}
        }
    }
    if message == WM_DESTROY {
        unsafe {
            PostQuitMessage(0);
        }
        return 0;
    }
    unsafe { DefWindowProcW(hwnd, message, w, l) }
}

fn run_card_window(
    flow: FlowKind,
    initial_screen: Screen,
    lang: Lang,
    version_badge: Option<String>,
    cancel: Arc<AtomicBool>,
) -> i32 {
    enable_per_monitor_dpi_v2();
    let palette = Palette::system();
    let dpi = system_dpi();
    let scale = dpi as f32 / 96.0;
    let client_w = (LOGICAL_WIDTH as f32 * scale).round() as i32;
    let client_h = (LOGICAL_HEIGHT as f32 * scale).round() as i32;

    let Some(surface) = DibSurface::new(client_w, client_h) else {
        return 1;
    };

    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let ex_style = 0u32;
    let (outer_w, outer_h) = outer_size_for_client(client_w, client_h, style, ex_style, dpi);

    let card_icon = load_embedded_icon((32.0 * scale).round() as i32);
    let big_icon = load_embedded_icon(unsafe { GetSystemMetrics(SM_CXICON) });
    let small_icon = load_embedded_icon(unsafe { GetSystemMetrics(SM_CXSMICON) });

    let mut boxed_state = Box::new(WindowState {
        flow,
        screen: initial_screen,
        lang,
        palette,
        version_badge,
        dpi,
        surface,
        card_icon,
        big_icon,
        small_icon,
        layout: RenderedLayout::default(),
        hovered_idx: None,
        pressed_idx: None,
        focused_idx: None,
        tracking_mouse: false,
        cancel,
        shared: Arc::new(Mutex::new(SharedWorker::default())),
        worker: None,
        exit_code: 1,
    });

    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        let class_name = wide("VUA.ModernSetup.Card");
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            hIcon: boxed_state.big_icon,
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            hbrBackground: ptr::null_mut(),
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);

        let pos_x = ((GetSystemMetrics(SM_CXSCREEN) - outer_w) / 2).max(0);
        let pos_y = ((GetSystemMetrics(SM_CYSCREEN) - outer_h) / 2).max(0);
        let hwnd = CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            wide("VUA").as_ptr(),
            style,
            pos_x,
            pos_y,
            outer_w,
            outer_h,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null(),
        );
        if hwnd.is_null() {
            return 1;
        }

        if !boxed_state.big_icon.is_null() {
            SendMessageW(
                hwnd,
                WM_SETICON,
                ICON_BIG as usize,
                boxed_state.big_icon as isize,
            );
        }
        if !boxed_state.small_icon.is_null() {
            SendMessageW(
                hwnd,
                WM_SETICON,
                ICON_SMALL as usize,
                boxed_state.small_icon as isize,
            );
        }

        let actual_dpi = window_dpi(hwnd);
        if actual_dpi != boxed_state.dpi {
            boxed_state.dpi = actual_dpi;
            boxed_state.reload_card_icon();
            let s = actual_dpi as f32 / 96.0;
            let cw = (LOGICAL_WIDTH as f32 * s).round() as i32;
            let ch = (LOGICAL_HEIGHT as f32 * s).round() as i32;
            let (ow, oh) = outer_size_for_client(cw, ch, style, ex_style, actual_dpi);
            SetWindowPos(
                hwnd,
                ptr::null_mut(),
                ((GetSystemMetrics(SM_CXSCREEN) - ow) / 2).max(0),
                ((GetSystemMetrics(SM_CYSCREEN) - oh) / 2).max(0),
                ow,
                oh,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        apply_dwm_chrome(hwnd, &boxed_state.palette);

        let state_ptr = Box::into_raw(boxed_state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        (*state_ptr).set_timer_state(hwnd);
        if matches!((*state_ptr).flow, FlowKind::Install { .. }) {
            (*state_ptr).spawn_install_attempt(hwnd);
        }

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        let _ = SetForegroundWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let mut finished = Box::from_raw(state_ptr);
        if let Some(worker) = finished.worker.take() {
            finished.cancel.store(true, Ordering::Relaxed);
            let _ = worker.join();
        }
        finished.exit_code
    }
}

pub fn ui_spawn(args: &[String]) -> i32 {
    let Some(session) = option(args, "--session").map(PathBuf::from) else {
        return 1;
    };
    let Some(root) = option(args, "--root") else {
        return 1;
    };
    let Some(input) = option(args, "--input") else {
        return 1;
    };
    if fs::create_dir_all(&session).is_err() {
        return 1;
    }
    for stale in [
        "spawned",
        "succeeded",
        "cancel",
        "input-ready",
        "input-failed",
        "worker.pid",
    ] {
        let _ = fs::remove_file(session.join(stale));
    }

    let Ok(exe) = std::env::current_exe() else {
        return 1;
    };
    let mut cmd = Command::new(exe);
    cmd.current_dir(std::env::temp_dir());
    cmd.args(["install", "--root", root, "--input", input, "--session"]);
    cmd.arg(&session);
    if let Some(lang) = option(args, "--lang") {
        cmd.args(["--lang", lang]);
    }

    let Ok(mut child) = cmd.spawn() else {
        return 1;
    };
    let _ = fs::write(session.join("worker.pid"), child.id().to_string());

    let spawned_marker = session.join("spawned");
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if spawned_marker.exists() {
            return 0;
        }
        if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
            return if spawned_marker.exists() { 0 } else { 1 };
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    if spawned_marker.exists() {
        0
    } else {
        1
    }
}

pub fn ui_wait(args: &[String]) -> i32 {
    let Some(session) = option(args, "--session").map(PathBuf::from) else {
        return 1;
    };
    let lock_path = session.join("ui.lock");
    if let Ok(lock_file) = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
    {
        let _ = lock_file.lock();
    }

    // Ensure the worker process image section has finished kernel rundown before NSIS deletes $PLUGINSDIR
    if let Ok(pid_text) = fs::read_to_string(session.join("worker.pid")) {
        if let Ok(pid) = pid_text.trim().parse::<u32>() {
            unsafe {
                let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
                if !handle.is_null() {
                    let _ = WaitForSingleObject(handle, 5_000);
                    let _ = CloseHandle(handle);
                }
            }
        }
    }

    if session.join("succeeded").exists() {
        0
    } else {
        1
    }
}

pub fn install(args: Vec<String>, cancel: Arc<AtomicBool>) -> i32 {
    let session_dir = option(&args, "--session").map(PathBuf::from);
    let mut session_lock: Option<File> = None;
    if let Some(dir) = &session_dir {
        if fs::create_dir_all(dir).is_err() {
            return 1;
        }
        let Ok(file) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("ui.lock"))
        else {
            return 1;
        };
        if file.lock().is_err() {
            return 1;
        }
        session_lock = Some(file);
        let _ = fs::write(dir.join("spawned"), b"1");
    }

    let lang = Lang::detect(option(&args, "--lang"));
    let version_badge = option(&args, "--input")
        .map(Path::new)
        .and_then(read_manifest_version);

    let code = run_card_window(
        FlowKind::Install { args, session_dir },
        Screen::InstallRunning {
            started: Instant::now(),
        },
        lang,
        version_badge,
        cancel,
    );

    // Let OS process exit release the kernel file lock so ui-wait wakes right as process terminates
    std::mem::forget(session_lock);
    code
}

pub fn uninstall(args: Vec<String>, cancel: Arc<AtomicBool>) -> i32 {
    let lang = Lang::detect(option(&args, "--lang"));
    let version_badge = option(&args, "--root")
        .map(Path::new)
        .and_then(read_installed_version);

    run_card_window(
        FlowKind::Uninstall { args },
        Screen::UninstallConfirm,
        lang,
        version_badge,
        cancel,
    )
}

pub fn failure(code: &str, explicit_lang: Option<&str>) {
    let lang = Lang::detect(explicit_lang);
    let copy = lang.copy();
    let code_line = format!("{}{code}", copy.failure_code_prefix);
    let _ = run_card_window(
        FlowKind::Failure,
        Screen::Failure { code_line },
        lang,
        None,
        Arc::new(AtomicBool::new(false)),
    );
}
