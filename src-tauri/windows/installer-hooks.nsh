; NSIS hooks for the Windows installer (see tauri.windows.conf.json's
; bundle.windows.nsis.installerHooks). Hooked into the generated
; installer.nsi by Tauri's bundler -- not a standalone script.
;
; Tauri's own `fileAssociations` config (tauri.conf.json) registers .pdf
; under Classes so "Open With" and per-extension defaults work, but Windows
; only lists an application in Settings > Apps > Default apps > "Set
; defaults for applications" if it *also* registers itself under
; `Software\RegisteredApplications` with a `Capabilities` key -- the
; classic "Default Programs" mechanism. That extra registration is what
; this hook adds. Same pattern other native Windows PDF viewers use.
;
; SHCTX is set by Tauri's NSIS template (via the MultiUser plugin) before
; hooks run, and already points at HKLM or HKCU to match whatever install
; mode (currentUser/perMachine) this build was configured with -- using it
; here keeps this consistent with wherever Tauri itself registered the
; .pdf association, rather than hardcoding a hive.

!macro NSIS_HOOK_POSTINSTALL
  ; Our own ProgID for the capabilities entry to point at. Deliberately
  ; separate from whatever internal ProgID Tauri's own fileAssociations
  ; registration uses (that name isn't part of its public config surface),
  ; so this doesn't silently break if that changes.
  WriteRegStr SHCTX "Software\Classes\PathPDF.Document" "" "PDF Document"
  WriteRegStr SHCTX "Software\Classes\PathPDF.Document\DefaultIcon" "" "$INSTDIR\path-pdf.exe,0"
  WriteRegStr SHCTX "Software\Classes\PathPDF.Document\shell\open\command" "" '"$INSTDIR\path-pdf.exe" "%1"'

  ; The Capabilities block Windows' Default Apps UI actually reads.
  WriteRegStr SHCTX "Software\Path PDF\Capabilities" "ApplicationName" "Path PDF"
  WriteRegStr SHCTX "Software\Path PDF\Capabilities" "ApplicationDescription" "A fast, lightweight PDF viewer and light editor."
  WriteRegStr SHCTX "Software\Path PDF\Capabilities" "ApplicationIcon" "$INSTDIR\path-pdf.exe,0"
  WriteRegStr SHCTX "Software\Path PDF\Capabilities\FileAssociations" ".pdf" "PathPDF.Document"

  ; The pointer Windows enumerates to build the "Set defaults for
  ; applications" list in the first place.
  WriteRegStr SHCTX "Software\RegisteredApplications" "Path PDF" "Software\Path PDF\Capabilities"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey SHCTX "Software\Classes\PathPDF.Document"
  DeleteRegKey SHCTX "Software\Path PDF\Capabilities"
  DeleteRegValue SHCTX "Software\RegisteredApplications" "Path PDF"
!macroend
