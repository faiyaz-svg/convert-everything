; Adds "Convert with Convert Everything" to the File Explorer right-click menu for
; files and folders, for the current user only. The app keeps these keys in step
; with its settings; the uninstaller removes them.

!macro CE_ADD_VERB ROOT
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\ConvertEverything" "MUIVerb" "Convert with Convert Everything"
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\ConvertEverything" "Icon" '"$INSTDIR\convert-everything.exe",0'
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\ConvertEverything" "MultiSelectModel" "Player"
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\ConvertEverything\command" "" '"$INSTDIR\convert-everything.exe" "%1"'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro CE_ADD_VERB "*"
  !insertmacro CE_ADD_VERB "Directory"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Classes\*\shell\ConvertEverything"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\ConvertEverything"
  RMDir /r "$TEMP\convert-everything"
!macroend
