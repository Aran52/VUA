use std::{
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use vua_distribution::transfer::Progress;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Controls::*, WindowsAndMessaging::*},
};
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}
struct State {
    cancel: Arc<AtomicBool>,
    latest: Arc<Mutex<Option<Progress>>>,
    done: Arc<Mutex<Option<bool>>>,
    label: HWND,
    progress: HWND,
    language: usize,
}
const COPY: [[&str; 5]; 4] = [
    ["Installing VUA", "Checking and preparing files…", "Cancel", "Installation failed. Retry the installer or use the offline installer. Your current installation and personal data are retained.", "Cancelling…"],
    ["正在安装 VUA", "正在检查并准备文件…", "取消", "安装未完成。请重试安装器，或使用离线安装包。原有版本和个人数据已保留。", "正在取消…"],
    ["VUA をインストール", "ファイルを確認・準備中…", "キャンセル", "インストールできませんでした。再試行するか、オフライン版を使用してください。既存のアプリと個人データは保持されます。", "キャンセル中…"],
    ["VUA 설치", "파일 확인 및 준비 중…", "취소", "설치하지 못했습니다. 다시 시도하거나 오프라인 설치 파일을 사용하세요. 기존 앱과 개인 데이터는 유지됩니다.", "취소 중…"],
];
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut State;
    if !state.is_null() {
        let state = unsafe { &*state };
        if message == WM_CLOSE || (message == WM_COMMAND && w & 0xffff == 1) {
            state.cancel.store(true, Ordering::Relaxed);
            return 0;
        }
        if message == WM_TIMER {
            let finished = *state.done.lock().expect("install completion");
            if let Some(success) = finished {
                if !success && !state.cancel.load(Ordering::Relaxed) {
                    unsafe {
                        MessageBoxW(
                            hwnd,
                            wide(COPY[state.language][3]).as_ptr(),
                            wide("VUA").as_ptr(),
                            MB_OK | MB_ICONERROR,
                        );
                    }
                }
                unsafe {
                    DestroyWindow(hwnd);
                }
                return 0;
            }
            let latest = state.latest.lock().expect("install progress").clone();
            let text = if state.cancel.load(Ordering::Relaxed) {
                COPY[state.language][4].into()
            } else if let Some(progress) = &latest {
                format!(
                    "{}\n{:.1} / {:.1} MB  ·  {:.1} MB/s",
                    COPY[state.language][1],
                    progress.completed as f64 / 1_048_576.0,
                    progress.total as f64 / 1_048_576.0,
                    progress.bytes_per_second as f64 / 1_048_576.0
                )
            } else {
                COPY[state.language][1].into()
            };
            unsafe {
                SetWindowTextW(state.label, wide(&text).as_ptr());
            }
            if let Some(progress) = latest {
                unsafe {
                    SendMessageW(
                        state.progress,
                        PBM_SETPOS,
                        ((progress.completed.saturating_mul(100) / progress.total.max(1)).min(100))
                            as usize,
                        0,
                    );
                }
            }
            return 0;
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
pub fn install(args: Vec<String>, cancel: Arc<AtomicBool>) -> i32 {
    let language = match vua_distribution::commands::option(&args, "--lang") {
        Some("2052") => 1,
        Some("1041") => 2,
        Some("1042") => 3,
        _ => 0,
    };
    let latest = Arc::new(Mutex::new(None));
    let done = Arc::new(Mutex::new(None));
    let state_ptr = Box::into_raw(Box::new(State {
        cancel: cancel.clone(),
        latest: latest.clone(),
        done: done.clone(),
        label: ptr::null_mut(),
        progress: ptr::null_mut(),
        language,
    }));
    unsafe {
        let common = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS,
        };
        InitCommonControlsEx(&common);
        let instance = GetModuleHandleW(ptr::null());
        let class = wide("VUA.Installation.Progress");
        let definition = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
            ..std::mem::zeroed()
        };
        RegisterClassW(&definition);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide(COPY[language][0]).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            (GetSystemMetrics(SM_CXSCREEN) - 520) / 2,
            (GetSystemMetrics(SM_CYSCREEN) - 230) / 2,
            520,
            230,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null(),
        );
        if hwnd.is_null() {
            drop(Box::from_raw(state_ptr));
            return 1;
        }
        (*state_ptr).label = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(COPY[language][1]).as_ptr(),
            WS_CHILD | WS_VISIBLE,
            26,
            24,
            456,
            64,
            hwnd,
            ptr::null_mut(),
            instance,
            ptr::null(),
        );
        (*state_ptr).progress = CreateWindowExW(
            0,
            wide("msctls_progress32").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE,
            26,
            96,
            456,
            22,
            hwnd,
            ptr::null_mut(),
            instance,
            ptr::null(),
        );
        CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide(COPY[language][2]).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            372,
            136,
            110,
            32,
            hwnd,
            1usize as HMENU,
            instance,
            ptr::null(),
        );
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
        SetTimer(hwnd, 1, 100, None);
        ShowWindow(hwnd, SW_SHOW);
        let worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("install runtime");
            let result = runtime.block_on(vua_distribution::commands::execute(
                &args,
                cancel,
                move |progress| {
                    *latest.lock().expect("install progress") = Some(progress);
                },
            ));
            *done.lock().expect("install completion") = Some(result.is_ok());
            result.is_ok()
        });
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        drop(Box::from_raw(state_ptr));
        if worker.join().unwrap_or(false) {
            0
        } else {
            1
        }
    }
}
pub fn failure(code: &str) {
    unsafe {
        MessageBoxW(ptr::null_mut(), wide(&format!("VUA could not start ({code}).\nRun the VUA installer again to repair program files. Personal data will be retained.")).as_ptr(), wide("VUA").as_ptr(), MB_OK|MB_ICONERROR);
    }
}
