; Registers the Windows Explorer integration used by Syncinit.
; %* lets the command receive every selected path instead of only the
; first selected item. The app parses those arguments into one archive job.

!macro SYNCINIT_WRITE_CONTEXT_MENU ROOT
  WriteRegStr HKCR "${ROOT}\shell\Syncinit" "MUIVerb" "Syncinit"
  WriteRegStr HKCR "${ROOT}\shell\Syncinit" "SubCommands" ""
  WriteRegStr HKCR "${ROOT}\shell\Syncinit" "Icon" "$INSTDIR\Syncinit.exe"
  WriteRegStr HKCR "${ROOT}\shell\Syncinit" "MultiSelectModel" "Player"

  WriteRegStr HKCR "${ROOT}\shell\Syncinit\shell\01_add" "" "Add to archive..."
  WriteRegStr HKCR "${ROOT}\shell\Syncinit\shell\01_add\command" "" '"$INSTDIR\Syncinit.exe" --add %*'

  WriteRegStr HKCR "${ROOT}\shell\Syncinit\shell\02_add_default" "" "Add to .init archive"
  WriteRegStr HKCR "${ROOT}\shell\Syncinit\shell\02_add_default\command" "" '"$INSTDIR\Syncinit.exe" --add-default %*'

  WriteRegStr HKCR "${ROOT}\shell\Syncinit\shell\03_add_mail" "" "Compress and email..."
  WriteRegStr HKCR "${ROOT}\shell\Syncinit\shell\03_add_mail\command" "" '"$INSTDIR\Syncinit.exe" --add-mail %*'
!macroend

!macro SYNCINIT_WRITE_ARCHIVE_MENU EXT
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\SyncinitExtractHere" "" "Extract Here"
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\SyncinitExtractHere\command" "" '"$INSTDIR\Syncinit.exe" --extract-here "%1"'
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\SyncinitOpen" "" "Open with Syncinit"
  WriteRegStr HKCR "SystemFileAssociations\.${EXT}\shell\SyncinitOpen\command" "" '"$INSTDIR\Syncinit.exe" "%1"'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro SYNCINIT_WRITE_CONTEXT_MENU "*"
  !insertmacro SYNCINIT_WRITE_CONTEXT_MENU "Directory"

  ; Right-clicking empty space inside a directory archives that directory.
  WriteRegStr HKCR "Directory\Background\shell\Syncinit" "MUIVerb" "Syncinit"
  WriteRegStr HKCR "Directory\Background\shell\Syncinit" "Icon" "$INSTDIR\Syncinit.exe"
  WriteRegStr HKCR "Directory\Background\shell\Syncinit\command" "" '"$INSTDIR\Syncinit.exe" --add-default "%V"'

  !insertmacro SYNCINIT_WRITE_ARCHIVE_MENU "arc"
  !insertmacro SYNCINIT_WRITE_ARCHIVE_MENU "zip"
  !insertmacro SYNCINIT_WRITE_ARCHIVE_MENU "7z"

  ; Make .init a first-class Syncinit archive type and use the bundled logo.
  WriteRegStr HKCR ".init" "" "Syncinit.Archive"
  WriteRegStr HKCR ".init" "Content Type" "application/x-syncinit"
  WriteRegStr HKCR "Syncinit.Archive" "" "Syncinit Archive"
  WriteRegStr HKCR "Syncinit.Archive\DefaultIcon" "" "$INSTDIR\Syncinit.exe,0"
  WriteRegStr HKCR "Syncinit.Archive\shell\open\command" "" '"$INSTDIR\Syncinit.exe" "%1"'

  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCR "*\shell\Syncinit"
  DeleteRegKey HKCR "Directory\shell\Syncinit"
  DeleteRegKey HKCR "Directory\Background\shell\Syncinit"
  DeleteRegKey HKCR "SystemFileAssociations\.init\shell\SyncinitExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.init\shell\SyncinitOpen"
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\SyncinitExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\SyncinitOpen"
  DeleteRegKey HKCR "SystemFileAssociations\.7z\shell\SyncinitExtractHere"
  DeleteRegKey HKCR "SystemFileAssociations\.7z\shell\SyncinitOpen"
  DeleteRegKey HKCR "Syncinit.Archive"
  DeleteRegValue HKCR ".init" ""
  DeleteRegValue HKCR ".init" "Content Type"

  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend