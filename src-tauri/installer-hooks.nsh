; installer-hooks.nsh
; Registers a WinRAR-style "Zarc" cascading context menu for files, folders,
; and the desktop background, plus a dedicated menu for archive files
; themselves (Extract Here / Extract to.../ Test archive).
;
; Tauri's NSIS bundler calls NSIS.hookAddFiles / NSIS.hookPreInstall etc. via
; tauri.conf.json -> bundle.windows.nsis.installerHooks. This file plugs into
; the custom install/uninstall macros it expects.

!macro NSIS_HOOK_POSTINSTALL
  ; --- Cascading "Zarc" menu on any file / folder ---
  WriteRegStr HKCR "*\shell\Zarc" "MUIVerb" "Zarc"
  WriteRegStr HKCR "*\shell\Zarc" "SubCommands" ""
  WriteRegStr HKCR "*\shell\Zarc" "Icon" "$INSTDIR\Zarc.exe"

  WriteRegStr HKCR "*\shell\Zarc\shell\01_add" "" "Add to archive..."
  WriteRegStr HKCR "*\shell\Zarc\shell\01_add\command" "" '"$INSTDIR\Zarc.exe" --add "%1"'

  WriteRegStr HKCR "*\shell\Zarc\shell\02_add_default" "" "Add to ""%1"".zip"
  WriteRegStr HKCR "*\shell\Zarc\shell\02_add_default\command" "" '"$INSTDIR\Zarc.exe" --add-default "%1"'

  WriteRegStr HKCR "*\shell\Zarc\shell\03_add_mail" "" "Compress and email..."
  WriteRegStr HKCR "*\shell\Zarc\shell\03_add_mail\command" "" '"$INSTDIR\Zarc.exe" --add-mail "%1"'

  ; --- Same menu for folders ---
  WriteRegStr HKCR "Directory\shell\Zarc" "MUIVerb" "Zarc"
  WriteRegStr HKCR "Directory\shell\Zarc" "SubCommands" ""
  WriteRegStr HKCR "Directory\shell\Zarc" "Icon" "$INSTDIR\Zarc.exe"

  WriteRegStr HKCR "Directory\shell\Zarc\shell\01_add" "" "Add to archive..."
  WriteRegStr HKCR "Directory\shell\Zarc\shell\01_add\command" "" '"$INSTDIR\Zarc.exe" --add "%1"'

  WriteRegStr HKCR "Directory\shell\Zarc\shell\02_add_default" "" "Add to ""%1"".zip"
  WriteRegStr HKCR "Directory\shell\Zarc\shell\02_add_default\command" "" '"$INSTDIR\Zarc.exe" --add-default "%1"'

  WriteRegStr HKCR "Directory\shell\Zarc\shell\03_add_mail" "" "Compress and email..."
  WriteRegStr HKCR "Directory\shell\Zarc\shell\03_add_mail\command" "" '"$INSTDIR\Zarc.exe" --add-mail "%1"'

  ; --- Dedicated menu when right-clicking an archive Zarc already owns ---
  !define ZARC_ARCHIVE_EXTS "SystemFileAssociations\.zip;SystemFileAssociations\.tar;SystemFileAssociations\.7z"

  WriteRegStr HKCR "SystemFileAssociations\.zip\shell\ZarcExtractHere" "" "Extract Here"
  WriteRegStr HKCR "SystemFileAssociations\.zip\shell\ZarcExtractHere\command" "" '"$INSTDIR\Zarc.exe" --extract-here "%1"'
  WriteRegStr HKCR "SystemFileAssociations\.zip\shell\ZarcOpen" "" "Open with Zarc"
  WriteRegStr HKCR "SystemFileAssociations\.zip\shell\ZarcOpen\command" "" '"$INSTDIR\Zarc.exe" "%1"'

  WriteRegStr HKCR "SystemFileAssociations\.7z\shell\ZarcExtractHere" "" "Extract Here"
  WriteRegStr HKCR "SystemFileAssociations\.7z\shell\ZarcExtractHere\command" "" '"$INSTDIR\Zarc.exe" --extract-here "%1"'
  WriteRegStr HKCR "SystemFileAssociations\.7z\shell\ZarcOpen" "" "Open with Zarc"
  WriteRegStr HKCR "SystemFileAssociations\.7z\shell\ZarcOpen\command" "" '"$INSTDIR\Zarc.exe" "%1"'

  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCR "*\shell\Zarc"
  DeleteRegKey HKCR "Directory\shell\Zarc"
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\ZarcExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\ZarcOpen"
  DeleteRegKey HKCR "SystemFileAssociations\.7z\shell\ZarcExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.7z\shell\ZarcOpen"

  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend
