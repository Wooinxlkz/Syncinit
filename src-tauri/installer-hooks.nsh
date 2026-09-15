; Registers the Windows Explorer integration used by Zarc.
; %* lets the command receive every selected path instead of only the
; first selected item. The app parses those arguments into one archive job.

!macro ZARC_WRITE_CONTEXT_MENU ROOT
  WriteRegStr HKCR "${ROOT}\shell\Zarc" "MUIVerb" "Zarc"
  WriteRegStr HKCR "${ROOT}\shell\Zarc" "SubCommands" ""
  WriteRegStr HKCR "${ROOT}\shell\Zarc" "Icon" "$INSTDIR\Zarc.exe"
  WriteRegStr HKCR "${ROOT}\shell\Zarc" "MultiSelectModel" "Player"

  WriteRegStr HKCR "${ROOT}\shell\Zarc\shell\01_add" "" "Add to archive..."
  WriteRegStr HKCR "${ROOT}\shell\Zarc\shell\01_add\command" "" '"$INSTDIR\Zarc.exe" --add %*'

  WriteRegStr HKCR "${ROOT}\shell\Zarc\shell\02_add_default" "" "Add to .arc archive"
  WriteRegStr HKCR "${ROOT}\shell\Zarc\shell\02_add_default\command" "" '"$INSTDIR\Zarc.exe" --add-default %*'

  WriteRegStr HKCR "${ROOT}\shell\Zarc\shell\03_add_mail" "" "Compress and email..."
  WriteRegStr HKCR "${ROOT}\shell\Zarc\shell\03_add_mail\command" "" '"$INSTDIR\Zarc.exe" --add-mail %*'
!macroend

!macro ZARC_WRITE_ARCHIVE_MENU EXT
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\ZarcExtractHere" "" "Extract Here"
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\ZarcExtractHere\command" "" '"$INSTDIR\Zarc.exe" --extract-here "%1"'
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\ZarcOpen" "" "Open with Zarc"
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\ZarcOpen\command" "" '"$INSTDIR\Zarc.exe" "%1"'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro ZARC_WRITE_CONTEXT_MENU "*"
  !insertmacro ZARC_WRITE_CONTEXT_MENU "Directory"

  ; Right-clicking empty space inside a directory archives that directory.
  WriteRegStr HKCR "Directory\Background\shell\Zarc" "MUIVerb" "Zarc"
  WriteRegStr HKCR "Directory\Background\shell\Zarc" "Icon" "$INSTDIR\Zarc.exe"
  WriteRegStr HKCR "Directory\Background\shell\Zarc\command" "" '"$INSTDIR\Zarc.exe" --add-default "%V"'

  !insertmacro ZARC_WRITE_ARCHIVE_MENU "arc"
  !insertmacro ZARC_WRITE_ARCHIVE_MENU "zip"
  !insertmacro ZARC_WRITE_ARCHIVE_MENU "7z"

  ; Make .arc a first-class Zarc archive type and use the bundled logo.
  WriteRegStr HKCR ".arc" "" "Zarc.Archive"
  WriteRegStr HKCR ".arc" "Content Type" "application/x-zarc"
  WriteRegStr HKCR "Zarc.Archive" "" "Zarc Archive"
  WriteRegStr HKCR "Zarc.Archive\DefaultIcon" "" "$INSTDIR\Zarc.exe,0"
  WriteRegStr HKCR "Zarc.Archive\shell\open\command" "" '"$INSTDIR\Zarc.exe" "%1"'

  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCR "*\shell\Zarc"
  DeleteRegKey HKCR "Directory\shell\Zarc"
  DeleteRegKey HKCR "Directory\Background\shell\Zarc"
  DeleteRegKey HKCR "SystemFileAssociations\.arc\shell\ZarcExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.arc\shell\ZarcOpen"
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\ZarcExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\ZarcOpen"
  DeleteRegKey HKCR "SystemFileAssociations\.7z\shell\ZarcExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.7z\shell\ZarcOpen"
  DeleteRegKey HKCR "Zarc.Archive"
  DeleteRegValue HKCR ".arc" ""
  DeleteRegValue HKCR ".arc" "Content Type"

  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend