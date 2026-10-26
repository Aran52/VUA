Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "FileFunc.nsh"
Var Isolated
Name "VUA ${VERSION}"
!ifdef PREVIEW
  Name "VUA ${VERSION} — CI PREVIEW"
!endif
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\VUA"
InstallDirRegKey HKCU "Software\VUA" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 64
BrandingText "VUA · Ibis Beta"
!ifndef VERSION_NUMBER
  !define VERSION_NUMBER "${VERSION}.0"
!endif
VIProductVersion "${VERSION_NUMBER}"
VIAddVersionKey /LANG=1033 "ProductName" "VUA"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileDescription" "VUA Installer"
VIAddVersionKey /LANG=1033 "LegalCopyright" "VUA Project contributors"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\VUA.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "Japanese"
!insertmacro MUI_LANGUAGE "Korean"
LangString Failed ${LANG_ENGLISH} "Installation was not completed. Finish any running VUA work, then retry or use the offline installer. Your personal data is retained."
LangString Failed ${LANG_SIMPCHINESE} "安装未完成。请先结束正在运行的 VUA 任务，再重试或使用离线安装包。个人数据已保留。"
LangString Failed ${LANG_JAPANESE} "インストールできませんでした。実行中の VUA の作業を終了して再試行するか、オフライン版を使用してください。個人データは保持されます。"
LangString Failed ${LANG_KOREAN} "설치하지 못했습니다. 실행 중인 VUA 작업을 마친 후 다시 시도하거나 오프라인 설치 파일을 사용하세요. 개인 데이터는 유지됩니다."
LangString RemovalFailed ${LANG_ENGLISH} "Close VUA before uninstalling. Program files were retained."
LangString RemovalFailed ${LANG_SIMPCHINESE} "请先关闭 VUA 再卸载。程序文件已保留。"
LangString RemovalFailed ${LANG_JAPANESE} "アンインストール前に VUA を終了してください。プログラムのファイルは保持されました。"
LangString RemovalFailed ${LANG_KOREAN} "제거하기 전에 VUA를 닫으세요. 프로그램 파일은 유지됩니다."
Function .onInit
  ${GetParameters} $0
  ${GetOptions} $0 "/ISOLATED" $1
  ${IfNot} ${Errors}
    StrCpy $Isolated 1
  ${EndIf}
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "VUA requires Windows x64."
    Abort
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_OK|MB_ICONSTOP "VUA requires Windows 10 or later."
    Abort
  ${EndIf}
FunctionEnd
Section "VUA"
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\vua-input"
  File /oname=vua-release.json "${INPUT}\vua-release.json"
  File /oname=host.bin "${INPUT}\host.bin"
  File /oname=amf.bin "${INPUT}\amf.bin"
  !ifdef OFFLINE
    File /oname=runtime.zip "${INPUT}\runtime.zip"
  !endif
  File /oname=vua-distribution.exe "${INPUT}\VUA.exe"
  IfSilent silent visible
  silent:
    ExecWait '"$PLUGINSDIR\vua-input\vua-distribution.exe" install --root "$INSTDIR" --input "$PLUGINSDIR\vua-input" --headless' $0
    Goto installed
  visible:
    ExecWait '"$PLUGINSDIR\vua-input\vua-distribution.exe" install --root "$INSTDIR" --input "$PLUGINSDIR\vua-input" --lang $LANGUAGE' $0
  installed:
  ${If} $0 != 0
    IfSilent skipMessage
    MessageBox MB_OK|MB_ICONEXCLAMATION "$(Failed)"
    skipMessage:
    SetErrorLevel 1
    Abort
  ${EndIf}
  SetOutPath "$INSTDIR"
  CopyFiles /SILENT "$PLUGINSDIR\vua-input\vua-distribution.exe" "$INSTDIR\VUA.exe"
  IfErrors copyFailed
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  ${If} $Isolated == 1
    FileOpen $1 "$INSTDIR\nsis-isolated" w
    FileWrite $1 "VUA isolated installer test"
    FileClose $1
    Goto finished
  ${EndIf}
  SetShellVarContext current
  CreateDirectory "$SMPROGRAMS\VUA"
  CreateShortCut "$SMPROGRAMS\VUA\VUA.lnk" "$INSTDIR\VUA.exe"
  WriteRegStr HKCU "Software\VUA" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "DisplayName" "VUA"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "Publisher" "VUA Project contributors"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "URLInfoAbout" "https://github.com/VUA-Project/VUA"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "NoRepair" 1
  Goto finished
  copyFailed:
    SetErrorLevel 1
    Abort
  finished:
SectionEnd
Section "Uninstall"
  ExecWait '"$INSTDIR\VUA.exe" remove --root "$INSTDIR" --headless' $0
  ${If} $0 != 0
    IfSilent skipRemovalMessage
    MessageBox MB_OK|MB_ICONEXCLAMATION "$(RemovalFailed)"
    skipRemovalMessage:
    SetErrorLevel 1
    Abort
  ${EndIf}
  IfFileExists "$INSTDIR\nsis-isolated" skipRegistration
  SetShellVarContext current
  Delete "$SMPROGRAMS\VUA\VUA.lnk"
  RMDir "$SMPROGRAMS\VUA"
  DeleteRegKey HKCU "Software\VUA"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA"
  skipRegistration:
  Delete "$INSTDIR\nsis-isolated"
  Delete "$INSTDIR\active.json"
  Delete "$INSTDIR\pending.json"
  Delete "$INSTDIR\previous.json"
  Delete "$INSTDIR\operation.lock"
  Delete "$INSTDIR\running.lock"
  Delete "$INSTDIR\vua-installation.json"
  Delete "$INSTDIR\VUA.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
