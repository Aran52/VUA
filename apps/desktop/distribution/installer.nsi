Unicode true
ManifestDPIAware true
!include "LogicLib.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "FileFunc.nsh"

Icon "${VUA_ICON}"
UninstallIcon "${VUA_ICON}"

Var Isolated
Var Headless
Var UnHeadless

!ifdef PREVIEW
  Name "VUA ${VERSION} — CI PREVIEW"
!else
  Name "VUA ${VERSION}"
!endif
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\VUA"
InstallDirRegKey HKCU "Software\VUA" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 64
BrandingText "VUA"

LoadLanguageFile "${NSISDIR}\Contrib\Language files\English.nlf"
LoadLanguageFile "${NSISDIR}\Contrib\Language files\SimpChinese.nlf"
LoadLanguageFile "${NSISDIR}\Contrib\Language files\TradChinese.nlf"
LoadLanguageFile "${NSISDIR}\Contrib\Language files\Japanese.nlf"
LoadLanguageFile "${NSISDIR}\Contrib\Language files\Korean.nlf"

!ifndef VERSION_NUMBER
  !define VERSION_NUMBER "${VERSION}.0"
!endif
VIProductVersion "${VERSION_NUMBER}"
VIAddVersionKey /LANG=1033 "ProductName" "VUA"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileDescription" "VUA Installer"
VIAddVersionKey /LANG=1033 "LegalCopyright" "VUA Project contributors"

Function .onInit
  ${If} ${Silent}
    StrCpy $Headless 1
  ${Else}
    StrCpy $Headless 0
    SetSilent silent
  ${EndIf}
  ${GetParameters} $0
  ${GetOptions} $0 "/ISOLATED" $1
  ${IfNot} ${Errors}
    StrCpy $Isolated 1
  ${EndIf}
  ${IfNot} ${RunningX64}
    ${If} $Headless == 0
      MessageBox MB_OK|MB_ICONSTOP "VUA requires Windows x64."
    ${EndIf}
    SetErrorLevel 1
    Abort
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    ${If} $Headless == 0
      MessageBox MB_OK|MB_ICONSTOP "VUA requires Windows 10 or later."
    ${EndIf}
    SetErrorLevel 1
    Abort
  ${EndIf}
FunctionEnd

Function un.onInit
  ${If} ${Silent}
    StrCpy $UnHeadless 1
  ${Else}
    StrCpy $UnHeadless 0
    SetSilent silent
  ${EndIf}
FunctionEnd

Section "VUA"
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\vua-input"
  ; Extract the signed manifest and native helper first from the solid LZMA stream
  ; so the modern native card appears immediately while larger packs unpack.
  File /oname=vua-release.json "${INPUT}\vua-release.json"
  File /oname=vua-distribution.exe "${INPUT}\VUA.exe"

  ${If} $Headless == 1
    File /oname=host.bin "${INPUT}\host.bin"
    File /oname=amf.bin "${INPUT}\amf.bin"
    !ifdef OFFLINE
      File /oname=runtime.zip "${INPUT}\runtime.zip"
    !endif
    SetOutPath "$TEMP"
    ExecWait '"$PLUGINSDIR\vua-input\vua-distribution.exe" install --root "$INSTDIR" --input "$PLUGINSDIR\vua-input" --headless' $0
  ${Else}
    SetOutPath "$TEMP"
    ExecWait '"$PLUGINSDIR\vua-input\vua-distribution.exe" ui-spawn --session "$PLUGINSDIR\vua-ui" --root "$INSTDIR" --input "$PLUGINSDIR\vua-input"' $2
    SetOutPath "$PLUGINSDIR\vua-input"
    ${If} $2 == 0
      IfFileExists "$PLUGINSDIR\vua-ui\cancel" cancelUnpack
      File /oname=host.bin "${INPUT}\host.bin"
      IfFileExists "$PLUGINSDIR\vua-ui\cancel" cancelUnpack
      File /oname=amf.bin "${INPUT}\amf.bin"
      !ifdef OFFLINE
        IfFileExists "$PLUGINSDIR\vua-ui\cancel" cancelUnpack
        File /oname=runtime.zip "${INPUT}\runtime.zip"
      !endif
      FileOpen $1 "$PLUGINSDIR\vua-ui\input-ready" w
      FileWrite $1 "1"
      FileClose $1
      cancelUnpack:
      SetOutPath "$TEMP"
      ExecWait '"$PLUGINSDIR\vua-input\vua-distribution.exe" ui-wait --session "$PLUGINSDIR\vua-ui"' $0
    ${Else}
      File /oname=host.bin "${INPUT}\host.bin"
      File /oname=amf.bin "${INPUT}\amf.bin"
      !ifdef OFFLINE
        File /oname=runtime.zip "${INPUT}\runtime.zip"
      !endif
      SetOutPath "$TEMP"
      ExecWait '"$PLUGINSDIR\vua-input\vua-distribution.exe" install --root "$INSTDIR" --input "$PLUGINSDIR\vua-input"' $0
    ${EndIf}
  ${EndIf}

  ${If} $0 != 0
    SetOutPath "$TEMP"
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
    SetOutPath "$TEMP"
    Goto finished
  ${EndIf}

  SetShellVarContext current
  CreateDirectory "$SMPROGRAMS\VUA"
  CreateShortCut "$SMPROGRAMS\VUA\VUA.lnk" "$INSTDIR\VUA.exe"
  WriteRegStr HKCU "Software\VUA" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "DisplayName" "VUA"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "DisplayIcon" "$INSTDIR\VUA.exe,0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "Publisher" "VUA Project contributors"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "URLInfoAbout" "https://github.com/VUA-Project/VUA"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\VUA" "NoRepair" 1

  ${If} $Headless == 0
    SetOutPath "$INSTDIR"
    Exec '"$INSTDIR\VUA.exe"'
  ${EndIf}
  SetOutPath "$TEMP"
  Goto finished

  copyFailed:
    SetOutPath "$TEMP"
    SetErrorLevel 1
    Abort
  finished:
SectionEnd

Section "Uninstall"
  SetOutPath "$TEMP"
  ${If} $UnHeadless == 1
    ExecWait '"$INSTDIR\VUA.exe" remove --root "$INSTDIR" --headless' $0
  ${Else}
    IfFileExists "$INSTDIR\VUA.exe" runInteractiveRemove checkOrphanedPayload
    runInteractiveRemove:
      ExecWait '"$INSTDIR\VUA.exe" remove --root "$INSTDIR"' $0
      Goto removeChecked
    checkOrphanedPayload:
      IfFileExists "$INSTDIR\payloads\*.*" 0 orphanCleanOk
      StrCpy $0 1
      Goto removeChecked
    orphanCleanOk:
      StrCpy $0 0
    removeChecked:
  ${EndIf}

  ${If} $0 != 0
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
