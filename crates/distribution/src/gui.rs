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
const COPY: [[&str; 5]; 5] = [
    [
        "Installing VUA",
        "Checking and preparing files…",
        "Cancel",
        "Couldn't finish installing VUA. Run the installer again, or try the offline installer instead. Your existing app and personal data were kept safe.",
        "Cancelling…",
    ],
    [
        "正在安装 VUA",
        "正在检查并准备文件…",
        "取消",
        "安装未能完成。请重新运行安装程序，或改用离线安装包。原有版本与个人数据均已保留。",
        "正在取消…",
    ],
    [
        "正在安裝 VUA",
        "正在檢查並準備檔案…",
        "取消",
        "安裝未能完成。請重新執行安裝程式，或改用離線安裝包。原有版本與個人資料均已保留。",
        "正在取消…",
    ],
    [
        "VUA をインストール中",
        "ファイルを確認・準備しています…",
        "キャンセル",
        "インストールを完了できませんでした。もう一度実行するか、オフライン版インストーラーをお試しください。既存のアプリと個人データはそのまま保持されています。",
        "キャンセル中…",
    ],
    [
        "VUA 설치 중",
        "파일을 확인하고 준비하는 중…",
        "취소",
        "설치를 완료하지 못했어요. 설치 프로그램을 다시 실행하거나 오프라인 설치 파일을 사용해 보세요. 기존 앱과 개인 데이터는 그대로 유지돼요.",
        "취소하는 중…",
    ],
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
        Some("1028" | "3076" | "5124") => 2,
        Some("1041") => 3,
        Some("1042") => 4,
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
        MessageBoxW(ptr::null_mut(), wide(&format!("VUA couldn't start ({code}).\nRun the VUA installer again to repair program files — your personal data will be kept safe.")).as_ptr(), wide("VUA").as_ptr(), MB_OK|MB_ICONERROR);
    }
}
