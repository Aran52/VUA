#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    ZhCn,
    ZhTw,
    Ja,
    Ko,
}

pub struct CopyTable {
    pub install_title: &'static str,
    pub install_sub_prep: &'static str,
    pub install_sub_download: &'static str,
    pub install_phase_unpack: &'static str,
    pub install_phase_verify: &'static str,
    pub install_phase_download: &'static str,
    pub install_finishing: &'static str,
    pub install_error_title: &'static str,
    pub install_error_sub: &'static str,
    pub install_error_busy: &'static str,
    pub install_error_general: &'static str,
    pub uninstall_confirm_title: &'static str,
    pub uninstall_confirm_sub: &'static str,
    pub uninstall_confirm_body: &'static str,
    pub uninstall_running_title: &'static str,
    pub uninstall_running_sub: &'static str,
    pub uninstall_phase_remove: &'static str,
    pub uninstall_error_sub: &'static str,
    pub uninstall_error_busy_title: &'static str,
    pub uninstall_error_busy_body: &'static str,
    pub uninstall_error_general_title: &'static str,
    pub uninstall_error_general_body: &'static str,
    pub failure_title: &'static str,
    pub failure_sub: &'static str,
    pub failure_code_prefix: &'static str,
    pub failure_body: &'static str,
    pub btn_cancel: &'static str,
    pub btn_cancelling: &'static str,
    pub btn_retry: &'static str,
    pub btn_close: &'static str,
    pub btn_uninstall: &'static str,
    pub btn_removing: &'static str,
}

const EN: CopyTable = CopyTable {
    install_title: "Installing VUA",
    install_sub_prep: "Preparing and verifying program files…",
    install_sub_download: "Downloading runtime components…",
    install_phase_unpack: "Unpacking installer package",
    install_phase_verify: "Verifying signature & installing",
    install_phase_download: "Downloading verified runtime",
    install_finishing: "Ready — opening VUA…",
    install_error_title: "Couldn't finish installing VUA",
    install_error_sub: "Your existing app and personal data were not changed",
    install_error_busy: "VUA is currently running. Close VUA (including the system tray icon) and click Retry.",
    install_error_general: "Check your network connection and click Retry, or use the offline installer instead.",
    uninstall_confirm_title: "Uninstall VUA",
    uninstall_confirm_sub: "Remove VUA program files from this computer?",
    uninstall_confirm_body: "Only program and runtime files will be removed. Your material library, Recipes, Unity projects, and account settings will stay untouched.",
    uninstall_running_title: "Uninstalling VUA",
    uninstall_running_sub: "Cleaning up local program directory…",
    uninstall_phase_remove: "Removing program & runtime files",
    uninstall_error_sub: "Program files and personal data were kept in place",
    uninstall_error_busy_title: "Please close VUA first",
    uninstall_error_busy_body: "VUA is still running. Close VUA (including the system tray icon) and click Retry.",
    uninstall_error_general_title: "Couldn't finish uninstalling",
    uninstall_error_general_body: "Close any running VUA processes or open files in the install folder, then click Retry.",
    failure_title: "VUA couldn't start",
    failure_sub: "Local program files are missing or need repair",
    failure_code_prefix: "Diagnostic code: ",
    failure_body: "Run the VUA installer again to repair program files. Your personal materials, Recipes, and settings will be kept safe.",
    btn_cancel: "Cancel",
    btn_cancelling: "Cancelling…",
    btn_retry: "Retry",
    btn_close: "Close",
    btn_uninstall: "Uninstall",
    btn_removing: "Removing…",
};

const ZH_CN: CopyTable = CopyTable {
    install_title: "正在安装 VUA",
    install_sub_prep: "正在准备并校验程序文件…",
    install_sub_download: "正在下载运行环境组件…",
    install_phase_unpack: "解压安装包组件",
    install_phase_verify: "校验签名与完整性并写入目录",
    install_phase_download: "下载并校验运行环境",
    install_finishing: "安装完成，正在打开 VUA…",
    install_error_title: "安装未能完成",
    install_error_sub: "原有版本与个人数据均未受影响",
    install_error_busy: "检测到 VUA 正在运行。请先退出 VUA（含右下角托盘图标），再点击「重试」。",
    install_error_general: "请检查网络连接后点击「重试」，或改用离线安装包进行安装。",
    uninstall_confirm_title: "卸载 VUA",
    uninstall_confirm_sub: "要从这台电脑移除 VUA 程序文件吗？",
    uninstall_confirm_body: "仅移除程序与运行环境文件。你的素材库、改模配方、Unity 工程与账号设置均会完整保留，不受影响。",
    uninstall_running_title: "正在卸载 VUA",
    uninstall_running_sub: "正在清理本地程序目录…",
    uninstall_phase_remove: "移除程序与运行环境文件",
    uninstall_error_sub: "程序文件与个人数据均暂未改动",
    uninstall_error_busy_title: "请先退出正在运行的 VUA",
    uninstall_error_busy_body: "检测到 VUA 仍在运行。请先退出 VUA（含右下角托盘图标），再点击「重试」。",
    uninstall_error_general_title: "卸载未能完成",
    uninstall_error_general_body: "请先关闭正在运行的 VUA 进程或占用的安装目录文件，再点击「重试」。",
    failure_title: "VUA 未能启动",
    failure_sub: "检测到本地程序文件缺失或损坏",
    failure_code_prefix: "诊断代码：",
    failure_body: "请重新运行 VUA 安装包以修复程序文件。你的个人素材、配方与设置均会安全保留。",
    btn_cancel: "取消",
    btn_cancelling: "正在取消…",
    btn_retry: "重试",
    btn_close: "关闭",
    btn_uninstall: "卸载",
    btn_removing: "正在卸载…",
};

const ZH_TW: CopyTable = CopyTable {
    install_title: "正在安裝 VUA",
    install_sub_prep: "正在準備並驗證程式檔案…",
    install_sub_download: "正在下載執行環境元件…",
    install_phase_unpack: "解壓縮安裝套件元件",
    install_phase_verify: "驗證簽章與完整性並寫入目錄",
    install_phase_download: "下載並驗證執行環境",
    install_finishing: "安裝完成，正在開啟 VUA…",
    install_error_title: "安裝未能完成",
    install_error_sub: "原有版本與個人資料均未受影響",
    install_error_busy: "偵測到 VUA 正在執行。請先結束 VUA（含右下角系統匣圖示），再按「重試」。",
    install_error_general: "請檢查網路連線後按「重試」，或改用離線安裝程式進行安裝。",
    uninstall_confirm_title: "解除安裝 VUA",
    uninstall_confirm_sub: "要從這部電腦移除 VUA 程式檔案嗎？",
    uninstall_confirm_body: "僅移除程式與執行環境檔案。你的素材庫、改模配方、Unity 專案與帳號設定皆會完整保留，不受影響。",
    uninstall_running_title: "正在解除安裝 VUA",
    uninstall_running_sub: "正在清理本機程式目錄…",
    uninstall_phase_remove: "移除程式與執行環境檔案",
    uninstall_error_sub: "程式檔案與個人資料均暫未變更",
    uninstall_error_busy_title: "請先關閉正在執行的 VUA",
    uninstall_error_busy_body: "偵測到 VUA 仍在執行。請先關閉 VUA（含右下角系統匣圖示），再按「重試」。",
    uninstall_error_general_title: "解除安裝未能完成",
    uninstall_error_general_body: "請先關閉正在執行的 VUA 程序或遭佔用的安裝目錄檔案，再按「重試」。",
    failure_title: "VUA 無法啟動",
    failure_sub: "偵測到本機程式檔案缺失或損毀",
    failure_code_prefix: "診斷代碼：",
    failure_body: "請重新執行 VUA 安裝程式以修復程式檔案。你的個人素材、配方與設定皆會安全保留。",
    btn_cancel: "取消",
    btn_cancelling: "正在取消…",
    btn_retry: "重試",
    btn_close: "關閉",
    btn_uninstall: "解除安裝",
    btn_removing: "正在移除…",
};

const JA: CopyTable = CopyTable {
    install_title: "VUA をインストールしています",
    install_sub_prep: "プログラムファイルを準備・確認しています…",
    install_sub_download: "ランタイムコンポーネントをダウンロード中…",
    install_phase_unpack: "パッケージを展開中",
    install_phase_verify: "署名・整合性の確認と配置",
    install_phase_download: "ランタイムを取得中",
    install_finishing: "完了しました — VUA を起動しています…",
    install_error_title: "インストールを完了できませんでした",
    install_error_sub: "既存の環境と個人データは安全に保持されています",
    install_error_busy: "VUA が実行中です。タスクトレイを含め VUA を終了してから「再試行」を押してください。",
    install_error_general: "通信環境を確認して「再試行」を押すか、オフライン版インストーラーをご利用ください。",
    uninstall_confirm_title: "VUA のアンインストール",
    uninstall_confirm_sub: "この PC から VUA のプログラムファイルを削除しますか？",
    uninstall_confirm_body: "プログラムとランタイムのみを削除します。素材ライブラリ、レシピ、Unity プロジェクト、アカウント設定はそのまま安全に残ります。",
    uninstall_running_title: "VUA を削除しています",
    uninstall_running_sub: "プログラムフォルダーを整理しています…",
    uninstall_phase_remove: "プログラムとランタイムを削除中",
    uninstall_error_sub: "プログラムファイルと個人データは変更されていません",
    uninstall_error_busy_title: "先に VUA を終了してください",
    uninstall_error_busy_body: "VUA が実行中です。タスクトレイを含め VUA を終了してから「再試行」を押してください。",
    uninstall_error_general_title: "アンインストールを完了できませんでした",
    uninstall_error_general_body: "実行中の VUA プロセスや使用中のファイルを閉じてから「再試行」を押してください。",
    failure_title: "VUA を起動できませんでした",
    failure_sub: "プログラムファイルの不足または破損が検出されました",
    failure_code_prefix: "診断コード: ",
    failure_body: "VUA インストーラーをもう一度実行してプログラムファイルを修復してください。素材・レシピ・個人設定はそのまま保持されます。",
    btn_cancel: "キャンセル",
    btn_cancelling: "キャンセル中…",
    btn_retry: "再試行",
    btn_close: "閉じる",
    btn_uninstall: "アンインストール",
    btn_removing: "削除中…",
};

const KO: CopyTable = CopyTable {
    install_title: "VUA 설치 중",
    install_sub_prep: "프로그램 파일을 준비하고 확인하는 중…",
    install_sub_download: "런타임 구성 요소를 다운로드하는 중…",
    install_phase_unpack: "설치 패키지 압축 해제 중",
    install_phase_verify: "서명 및 무결성 확인 후 설치 중",
    install_phase_download: "실행 환경 다운로드 중",
    install_finishing: "설치 완료 — VUA를 여는 중…",
    install_error_title: "VUA 설치를 완료하지 못했어요",
    install_error_sub: "기존 버전과 개인 데이터는 안전하게 유지됩니다",
    install_error_busy: "VUA가 실행 중입니다. 시스템 트레이를 포함해 VUA를 먼저 종료한 뒤 '다시 시도'를 눌러 주세요.",
    install_error_general: "네트워크 연결을 확인한 뒤 '다시 시도'를 누르거나 오프라인 설치 파일을 사용해 보세요.",
    uninstall_confirm_title: "VUA 제거",
    uninstall_confirm_sub: "이 컴퓨터에서 VUA 프로그램 파일을 제거할까요?",
    uninstall_confirm_body: "프로그램 및 런타임 파일만 제거됩니다. 에셋 라이브러리, 레시피, Unity 프로젝트와 계정 설정은 그대로 안전하게 보존됩니다.",
    uninstall_running_title: "VUA 제거 중",
    uninstall_running_sub: "프로그램 폴더를 정리하는 중…",
    uninstall_phase_remove: "프로그램 및 런타임 파일 정리 중",
    uninstall_error_sub: "프로그램 파일과 개인 데이터는 변경되지 않았습니다",
    uninstall_error_busy_title: "먼저 VUA를 종료해 주세요",
    uninstall_error_busy_body: "VUA가 아직 실행 중입니다. 시스템 트레이를 포함해 VUA를 종료한 뒤 '다시 시도'를 눌러 주세요.",
    uninstall_error_general_title: "제거를 완료하지 못했어요",
    uninstall_error_general_body: "실행 중인 VUA 프로세스나 열려 있는 설치 폴더 파일을 닫은 뒤 '다시 시도'를 눌러 주세요.",
    failure_title: "VUA를 시작할 수 없어요",
    failure_sub: "프로그램 파일이 누락되었거나 복구가 필요합니다",
    failure_code_prefix: "진단 코드: ",
    failure_body: "VUA 설치 프로그램을 다시 실행해 프로그램 파일을 복구해 주세요. 개인 에셋, 레시피, 설정은 안전하게 유지됩니다.",
    btn_cancel: "취소",
    btn_cancelling: "취소하는 중…",
    btn_retry: "다시 시도",
    btn_close: "닫기",
    btn_uninstall: "제거",
    btn_removing: "제거 중…",
};

impl Lang {
    pub fn detect(explicit: Option<&str>) -> Self {
        let preferred = query_user_preferred_ui_languages();
        let lcid = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
        Self::resolve_with_preferences(explicit, &preferred, lcid)
    }

    pub fn resolve_with_preferences<S: AsRef<str>>(
        explicit: Option<&str>,
        preferred_tags: &[S],
        fallback_lcid: u16,
    ) -> Self {
        if let Some(raw) = explicit {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                if let Ok(parsed) = trimmed.parse::<u16>() {
                    return Self::from_lcid(parsed);
                }
                if let Some(matched) = Self::from_bcp47(trimmed) {
                    return matched;
                }
            }
        }
        if !preferred_tags.is_empty() {
            for tag in preferred_tags {
                if let Some(matched) = Self::from_bcp47(tag.as_ref()) {
                    return matched;
                }
            }
            return Self::En;
        }
        Self::from_lcid(fallback_lcid)
    }

    pub fn from_bcp47(tag: &str) -> Option<Self> {
        let normalized = tag.trim().to_ascii_lowercase().replace('_', "-");
        let mut parts = normalized.split('-').filter(|part| !part.is_empty());
        match parts.next()? {
            "zh" => match parts.next() {
                Some("hant" | "cht" | "tw" | "hk" | "mo") => Some(Self::ZhTw),
                _ => Some(Self::ZhCn),
            },
            "en" => Some(Self::En),
            "ja" => Some(Self::Ja),
            "ko" => Some(Self::Ko),
            _ => None,
        }
    }

    pub fn from_lcid(lcid: u16) -> Self {
        let primary = lcid & 0x3ff;
        let sub = (lcid >> 10) & 0x3f;
        match primary {
            0x04 => {
                // 0x02 = Chinese Simplified (China), 0x04 = Singapore
                if sub == 0x02 || sub == 0x04 || sub == 0x00 {
                    Self::ZhCn
                } else {
                    Self::ZhTw
                }
            }
            0x11 => Self::Ja,
            0x12 => Self::Ko,
            _ => Self::En,
        }
    }

    pub fn copy(self) -> &'static CopyTable {
        match self {
            Self::En => &EN,
            Self::ZhCn => &ZH_CN,
            Self::ZhTw => &ZH_TW,
            Self::Ja => &JA,
            Self::Ko => &KO,
        }
    }

    pub fn font_family(self) -> &'static str {
        match self {
            Self::En => "Segoe UI",
            Self::ZhCn => "Microsoft YaHei UI",
            Self::ZhTw => "Microsoft JhengHei UI",
            Self::Ja => "Yu Gothic UI",
            Self::Ko => "Malgun Gothic",
        }
    }
}

fn query_user_preferred_ui_languages() -> Vec<String> {
    use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    let mut num_languages: u32 = 0;
    let mut buffer_len: u32 = 0;
    let sized = unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut num_languages,
            std::ptr::null_mut(),
            &mut buffer_len,
        )
    };
    if sized == 0 || buffer_len == 0 {
        return Vec::new();
    }
    let mut buffer = vec![0u16; buffer_len as usize];
    let ok = unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut num_languages,
            buffer.as_mut_ptr(),
            &mut buffer_len,
        )
    };
    if ok == 0 {
        return Vec::new();
    }
    buffer.truncate(buffer_len as usize);
    parse_multi_sz(&buffer)
}

fn parse_multi_sz(buffer: &[u16]) -> Vec<String> {
    buffer
        .split(|&unit| unit == 0)
        .filter(|segment| !segment.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_explicit_and_fallback_lcids_across_five_locales() {
        let empty: [&str; 0] = [];
        assert_eq!(
            Lang::resolve_with_preferences(Some("2052"), &empty, 1033),
            Lang::ZhCn
        );
        assert_eq!(
            Lang::resolve_with_preferences(Some("1028"), &empty, 1033),
            Lang::ZhTw
        );
        assert_eq!(
            Lang::resolve_with_preferences(Some("3076"), &empty, 1033),
            Lang::ZhTw
        );
        assert_eq!(
            Lang::resolve_with_preferences(Some("1041"), &empty, 1033),
            Lang::Ja
        );
        assert_eq!(
            Lang::resolve_with_preferences(Some("1042"), &empty, 1033),
            Lang::Ko
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &empty, 0x0804),
            Lang::ZhCn
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &empty, 0x0404),
            Lang::ZhTw
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &empty, 0x0c04),
            Lang::ZhTw
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &empty, 0x0411),
            Lang::Ja
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &empty, 0x0412),
            Lang::Ko
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &empty, 0x0409),
            Lang::En
        );
    }

    #[test]
    fn resolves_ordered_preferred_bcp47_tags_like_desktop_main() {
        assert_eq!(
            Lang::resolve_with_preferences(None, &["fr-FR", "ja-JP", "en-US"], 0x0409),
            Lang::Ja
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &["de-DE", "zh-Hant-HK"], 0x0409),
            Lang::ZhTw
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &["zh-Hans-SG", "en-US"], 0x0409),
            Lang::ZhCn
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &["es-ES", "ko-KR"], 0x0409),
            Lang::Ko
        );
        assert_eq!(
            Lang::resolve_with_preferences(None, &["fr-FR", "kok-IN"], 0x0411),
            Lang::En
        );
        assert_eq!(
            Lang::resolve_with_preferences(Some("zh-HK"), &["en-US"], 0x0409),
            Lang::ZhTw
        );
    }

    #[test]
    fn parses_double_nul_terminated_utf16_language_lists() {
        let mut raw: Vec<u16> = Vec::new();
        for tag in ["fr-FR", "ja-JP", "zh-Hant-TW"] {
            raw.extend(tag.encode_utf16());
            raw.push(0);
        }
        raw.push(0);
        assert_eq!(parse_multi_sz(&raw), vec!["fr-FR", "ja-JP", "zh-Hant-TW"]);
        assert!(parse_multi_sz(&[0, 0]).is_empty());
    }

    #[test]
    fn every_locale_table_is_populated() {
        for lang in [Lang::En, Lang::ZhCn, Lang::ZhTw, Lang::Ja, Lang::Ko] {
            let c = lang.copy();
            assert!(!c.install_title.is_empty());
            assert!(!c.uninstall_confirm_body.is_empty());
            assert!(!c.failure_body.is_empty());
            assert!(!c.btn_uninstall.is_empty());
            assert!(!lang.font_family().is_empty());
        }
    }
}
