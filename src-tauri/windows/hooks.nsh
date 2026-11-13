; Hooks NSIS de l'installateur Lanceur (bundle.windows.nsis.installerHooks).
;
; Données utilisateur :
;   %APPDATA%\com.lanceur.desktop       (roaming)
;   %LOCALAPPDATA%\com.lanceur.desktop  (config.json et cache WebView2)
;
; DESINSTALLATION (Paramètres Windows, désinstallateur lancé SANS /UPDATE) :
;   les deux dossiers ci-dessus sont supprimés, que la case « supprimer les données de
;   l'application » soit cochée ou non (ils n'ont aucun intérêt sans l'application).
;
; DESINSTALLATION AVEC /UPDATE ($UpdateMode = 1) : le hook ne fait RIEN, les données sont conservées
;   (la case du template est elle aussi ignorée avec /UPDATE).
;
; Mises à jour : notre template (windows/installer.nsi) ne lance aucun désinstallateur. Le setup
; (Lanceur-Setup.exe) refuse de s'exécuter si le Lanceur est déjà installé ; l'exe de mise à jour
; (Lanceur-Update.exe, compilé avec /DLANCEUR_UPDATER par scripts/build-updater.mjs) copie les fichiers
; par-dessus l'installation existante. Les données ne sont donc jamais touchées par une mise à jour.
;
; Ne pas renommer /UPDATE : c'est le drapeau que connaissent les désinstallateurs déjà installés.
;
; Contexte : installMode currentUser (tauri.conf.json) => l'installateur et le désinstallateur sont
; déjà en « SetShellVarContext current » (macro SetContext de utils.nsh) : $APPDATA et $LOCALAPPDATA
; sont ceux de l'utilisateur courant. Le programme est installé dans %LOCALAPPDATA%\Programs\Lanceur,
; dossier DISTINCT des données %LOCALAPPDATA%\com.lanceur.desktop : seul ce hook (vraie
; désinstallation) supprime les données. Les deux !if repassent en « current » si l'on revenait en
; perMachine, puis restaurent « all » ; en currentUser, ils ne produisent aucune instruction.

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    !if "${INSTALLMODE}" == "perMachine"
      SetShellVarContext current
    !endif
    RMDir /r "$APPDATA\${BUNDLEID}"
    RMDir /r "$LOCALAPPDATA\${BUNDLEID}"
    !if "${INSTALLMODE}" == "perMachine"
      SetShellVarContext all
    !endif
  ${EndIf}
!macroend

; Page de fin : la case « Lancer Lanceur » est COCHÉE par défaut (setup et update) ;
; MUI_FINISHPAGE_RUN_NOTCHECKED n'est défini nulle part. L'update lance lui-même la nouvelle version
; à la fin de la copie et saute sa page de fin, qui n'apparaît (avec cette case) que si ce lancement
; a échoué (voir LanceurUpdateFinishPre dans installer.nsi).

; Case « Créer un raccourci sur le Bureau » décochée par défaut ; l'utilisateur peut la cocher.
; Cette case n'existe que dans le setup ; l'exe de mise à jour n'en a pas (sans effet pour lui).
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED
