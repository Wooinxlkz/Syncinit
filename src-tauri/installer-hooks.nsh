; Registers the Windows Explorer integration used by Tugur.
; %* lets the command receive every selected path instead of only the
; first selected item. The app parses those arguments into one archive job.

!macro TUGUR_WRITE_CONTEXT_MENU ROOT
  WriteRegStr HKCR "${ROOT}\shell\Tugur" "MUIVerb" "Tugur"
  WriteRegStr HKCR "${ROOT}\shell\Tugur" "SubCommands" ""
  WriteRegStr HKCR "${ROOT}\shell\Tugur" "Icon" "$INSTDIR\Tugur.exe"
  WriteRegStr HKCR "${ROOT}\shell\Tugur" "MultiSelectModel" "Player"

  WriteRegStr HKCR "${ROOT}\shell\Tugur\shell\01_add" "" "Add to archive..."
  WriteRegStr HKCR "${ROOT}\shell\Tugur\shell\01_add\command" "" '"$INSTDIR\Tugur.exe" --add %*'

  WriteRegStr HKCR "${ROOT}\shell\Tugur\shell\02_add_default" "" "Add to .arc archive"
  WriteRegStr HKCR "${ROOT}\shell\Tugur\shell\02_add_default\command" "" '"$INSTDIR\Tugur.exe" --add-default %*'

  WriteRegStr HKCR "${ROOT}\shell\Tugur\shell\03_add_mail" "" "Compress and email..."
  WriteRegStr HKCR "${ROOT}\shell\Tugur\shell\03_add_mail\command" "" '"$INSTDIR\Tugur.exe" --add-mail %*'
!macroend

!macro TUGUR_WRITE_ARCHIVE_MENU EXT
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\TugurExtractHere" "" "Extract Here"
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\TugurExtractHere\command" "" '"$INSTDIR\Tugur.exe" --extract-here "%1"'
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\TugurOpen" "" "Open with Tugur"
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\TugurOpen\command" "" '"$INSTDIR\Tugur.exe" "%1"'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro TUGUR_WRITE_CONTEXT_MENU "*"
  !insertmacro TUGUR_WRITE_CONTEXT_MENU "Directory"

  ; Right-clicking empty space inside a directory archives that directory.
  WriteRegStr HKCR "Directory\Background\shell\Tugur" "MUIVerb" "Tugur"
  WriteRegStr HKCR "Directory\Background\shell\Tugur" "Icon" "$INSTDIR\Tugur.exe"
  WriteRegStr HKCR "Directory\Background\shell\Tugur\command" "" '"$INSTDIR\Tugur.exe" --add-default "%V"'

  !insertmacro TUGUR_WRITE_ARCHIVE_MENU "arc"
  !insertmacro TUGUR_WRITE_ARCHIVE_MENU "zip"
  !insertmacro TUGUR_WRITE_ARCHIVE_MENU "7z"

  ; Make .arc a first-class Tugur archive type and use the bundled logo.
  WriteRegStr HKCR ".arc" "" "Tugur.Archive"
  WriteRegStr HKCR ".arc" "Content Type" "application/x-tugur"
  WriteRegStr HKCR "Tugur.Archive" "" "Tugur Archive"
  WriteRegStr HKCR "Tugur.Archive\DefaultIcon" "" "$INSTDIR\Tugur.exe,0"
  WriteRegStr HKCR "Tugur.Archive\shell\open\command" "" '"$INSTDIR\Tugur.exe" "%1"'

  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCR "*\shell\Tugur"
  DeleteRegKey HKCR "Directory\shell\Tugur"
  DeleteRegKey HKCR "Directory\Background\shell\Tugur"
  DeleteRegKey HKCR "SystemFileAssociations\.arc\shell\TugurExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.arc\shell\TugurOpen"
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\TugurExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\TugurOpen"
  DeleteRegKey HKCR "SystemFileAssociations\.7z\shell\TugurExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.7z\shell\TugurOpen"
  DeleteRegKey HKCR "Tugur.Archive"
  DeleteRegValue HKCR ".arc" ""
  DeleteRegValue HKCR ".arc" "Content Type"

  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend