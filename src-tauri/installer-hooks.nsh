; Firewall rules, added at install rather than left to the first-run prompt.
;
; Windows only prompts for the profiles it feels like offering — in practice
; Private — so the same install that works at a desk fails on a domain or on a
; network Windows has decided is Public, with no error anywhere to say why.
; Worse, cancelling that prompt writes a *block* rule that outranks everything
; afterwards and leaves no visible trace. Removing the prompt removes both.
;
; The rule is scoped to the **program**, not to port 9001. The port is a setting
; the user can change, and a port-scoped rule would silently stop matching the
; moment they did. Program-scoped also covers mDNS discovery on 5353 for free,
; since that is the same executable.
;
; Deleted first so reinstalling and upgrading do not stack duplicates — netsh
; happily adds a second rule with the same name.

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Adding the Windows Firewall rule for ${PRODUCTNAME}..."
  nsExec::ExecToLog '"$SYSDIR\netsh.exe" advfirewall firewall delete rule name="${PRODUCTNAME}"'
  Pop $0
  nsExec::ExecToLog '"$SYSDIR\netsh.exe" advfirewall firewall add rule name="${PRODUCTNAME}" dir=in action=allow program="$INSTDIR\${MAINBINARYNAME}.exe" protocol=udp profile=any enable=yes'
  Pop $0
  ; Not fatal. An install that succeeded except for the firewall is far better
  ; than a rolled-back install, and the app still works on any network where
  ; the rule was not the thing standing in the way.
  ${If} $0 != 0
    DetailPrint "Could not add the firewall rule (netsh returned $0). Windows will ask on first start instead."
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Removing the Windows Firewall rule for ${PRODUCTNAME}..."
  nsExec::ExecToLog '"$SYSDIR\netsh.exe" advfirewall firewall delete rule name="${PRODUCTNAME}"'
  Pop $0
!macroend
