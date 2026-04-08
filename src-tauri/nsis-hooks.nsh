!macro NSIS_HOOK_PREINSTALL
  ; Stop ZecBox app and all sidecar processes before extracting files.
  ; Errors are silently ignored — processes may not be running.
  nsExec::Exec 'taskkill /F /IM zecbox.exe'
  nsExec::Exec 'taskkill /F /IM zebrad.exe'
  nsExec::Exec 'taskkill /F /IM arti.exe'
  nsExec::Exec 'taskkill /F /IM zaino.exe'
  nsExec::Exec 'taskkill /F /IM zecbox-firewall-helper.exe'
  nsExec::Exec 'sc.exe stop ZecBoxFirewall'
  Sleep 2000
!macroend
