export const amfUpdateCopy = {
  en: { apply: "Apply AMF update", restore: "Roll back", busy: "Wait for active AMF tasks, downloads, and unsaved edits to finish before updating.", restart: "Applying this update restarts the service. Confirm when you're ready.", verifying: "Verifying the updated AMF module and service…", checkFailed: "Couldn't refresh version info, though your downloaded update is still ready to apply." },
  "zh-CN": { apply: "应用 AMF 更新", restore: "回退版本", busy: "请先等进行中的 AMF 任务、下载和未保存修改完成后再应用更新。", restart: "应用此版本需要重启服务，确认无误后再继续。", verifying: "正在校验新版 AMF 界面与后台服务…", checkFailed: "版本在线检查未成功，但已下载好的更新包仍可直接应用。" },
  "zh-TW": { apply: "套用 AMF 更新", restore: "還原舊版", busy: "請先完成進行中的 AMF 任務、下載與未儲存的編輯，再套用更新。", restart: "此版本需要重新啟動服務，請確認後再繼續。", verifying: "正在驗證新版 AMF 介面與背景服務…", checkFailed: "版本線上檢查失敗，但已下載完成的更新仍可套用。" },
  ja: { apply: "AMF の更新を適用", restore: "元の版に戻す", busy: "更新を適用する前に、進行中の AMF タスク・ダウンロード・未保存の編集を完了してください。", restart: "適用には再起動が必要です。作業の区切りを確認してから進めてください。", verifying: "新しい AMF 画面とバックグラウンドサービスを確認中…", checkFailed: "オンライン確認に失敗しましたが、ダウンロード済みの更新はそのまま適用できます。" },
  ko: { apply: "AMF 업데이트 적용", restore: "이전 버전 복원", busy: "업데이트를 적용하기 전에 진행 중인 AMF 작업, 다운로드, 저장하지 않은 편집을 먼저 마쳐 주세요.", restart: "이 버전을 적용하려면 다시 시작해야 해요. 확인한 뒤 계속 진행해 주세요.", verifying: "새 AMF 화면과 백그라운드 서비스를 점검하는 중…", checkFailed: "버전 확인에는 실패했지만 이미 받아 둔 업데이트는 바로 적용할 수 있어요." },
} as const;
export const amfDataMigrationCopy = {
  en: "Run the VUA installer to complete this data-format upgrade.",
  "zh-CN": "本次更新涉及本地数据格式升级，请运行 VUA 安装包完成更新。",
  "zh-TW": "此次更新涉及本機資料格式升級，請執行 VUA 安裝程式完成更新。",
  ja: "データ形式の移行を伴うため、VUA インストーラーから更新を完了してください。",
  ko: "이번 업데이트는 데이터 형식 업그레이드가 포함되어 있어 VUA 설치 프로그램으로 완료해 주세요.",
} as const;
