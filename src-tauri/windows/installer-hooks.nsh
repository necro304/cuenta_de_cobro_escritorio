; Hooks del instalador NSIS de CuentaCobro (Tauri).
; Retira CuentaCobro 1.x (Electron) instalado SOLO para el usuario actual antes de
; copiar archivos y crear accesos directos. Los datos (%APPDATA%\cuenta-cobro) NO se
; tocan: no se pasa --delete-app-data y el instalador 1.x se generó con
; deleteAppDataOnUninstall=false. No pasar --updated: con _?= el desinstalador abortaría.

!define CC_LEGACY_GUID "672ad896-9caf-52a9-ae28-073b6304d939"
!define CC_LEGACY_INSTALL_KEY "Software\${CC_LEGACY_GUID}"
!define CC_LEGACY_UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${CC_LEGACY_GUID}"
!define CC_LEGACY_UNINSTALLER "Uninstall CuentaCobro.exe"

!macro NSIS_HOOK_PREINSTALL
  Push $R7
  Push $R8

  ; 1) Instalación por usuario (HKCU): se desinstala en silencio y se espera.
  ReadRegStr $R7 HKCU "${CC_LEGACY_INSTALL_KEY}" "InstallLocation"
  ${If} $R7 != ""
  ${AndIf} ${FileExists} "$R7\${CC_LEGACY_UNINSTALLER}"
  cc_reintentar_desinstalacion:
    DetailPrint "Retirando CuentaCobro 1.x de $R7 (tus datos se conservan)..."
    ClearErrors
    ExecWait '"$R7\${CC_LEGACY_UNINSTALLER}" /currentuser /S _?=$R7' $R8
    ${If} ${Errors}
      StrCpy $R8 "no se pudo iniciar"
    ${EndIf}
    ${If} $R8 != "0"
      DetailPrint "No se pudo retirar CuentaCobro 1.x ($R8)."
      MessageBox MB_RETRYCANCEL|MB_ICONSTOP "No se pudo retirar CuentaCobro 1.x ($R8). Cierra la aplicación anterior y vuelve a intentarlo." /SD IDCANCEL IDRETRY cc_reintentar_desinstalacion
      Pop $R8
      Pop $R7
      SetErrorLevel 1
      Abort
    ${EndIf}
    Delete "$R7\${CC_LEGACY_UNINSTALLER}"
    RMDir "$R7"
  ${EndIf}

  ; 2) Instalación para todos los usuarios (HKLM): requiere administrador; solo se avisa.
  ${If} ${RunningX64}
    SetRegView 64
  ${EndIf}
  ReadRegStr $R7 HKLM "${CC_LEGACY_UNINSTALL_KEY}" "DisplayVersion"
  SetRegView default
  ${If} $R7 != ""
  ${AndIfNot} ${Silent}
    MessageBox MB_OK|MB_ICONINFORMATION "Se encontró CuentaCobro $R7 instalado para todos los usuarios.$\r$\n$\r$\nCuando termine esta instalación, desinstálalo desde Configuración > Aplicaciones. Tus datos se conservan y esta versión ya los importa."
  ${EndIf}

  Pop $R8
  Pop $R7
!macroend
