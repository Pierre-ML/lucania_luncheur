# Lanceur

Application Windows (Tauri v2, frontend Astro) qui liste des applications publiées sur GitHub Releases. Pour chaque application, le Lanceur affiche si elle est installée, la dernière version disponible, et permet de l'installer, de la mettre à jour, de la lancer ou de la désinstaller. Il est lui-même installé par un installateur NSIS pour l'utilisateur courant (sans droits administrateur) et se met à jour avec `Lanceur-Update.exe`.

## Prérequis et commandes

Prérequis : Node.js 22, Rust (toolchain stable), WebView2 (présent sur Windows 11).

```
npm install
npm run desktop:dev          # lance l'app en développement
npm run check                # vérifications du frontend
npm run desktop:build        # tauri build + installateurs à nom fixe
npm run version:set X.Y.Z    # change la version dans les 4 fichiers qui la portent
```

`npm run desktop:build` produit dans `src-tauri/target/release/bundle/release/` :
`Lanceur-Setup.exe` (première installation) et `Lanceur-Update.exe` (mise à jour). La source de la version est `src-tauri/tauri.conf.json` ; `package.json`, `package-lock.json` et `src-tauri/Cargo.toml` doivent porter la même (le workflow le vérifie, `version:set` les met à jour ensemble).

Données de l'app : `%LOCALAPPDATA%\com.lanceur.desktop` (config.json, cache WebView2) et `%APPDATA%\com.lanceur.desktop`. Elles ne sont supprimées que par une vraie désinstallation.

## Ajouter une application

Modifier seulement `apps.json` à la racine. Un objet par application :

| Champ | Rôle |
| --- | --- |
| `id` | identifiant unique, stable |
| `enabled` | `true` pour l'afficher |
| `name` | nom affiché |
| `description.fr` / `description.en` | description courte |
| `icon` | nom d'un fichier de `public/app-icons/`, ou URL `raw.githubusercontent.com` |
| `repo` | dépôt GitHub `owner/repo` |
| `setup` | nom du fichier d'installation dans la release |
| `updater` | nom du fichier de mise à jour dans la release |
| `identifier` | identifiant Tauri de l'app (dossier de données `%LOCALAPPDATA%\<identifier>`) |
| `productName` | nom de la clé `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\<productName>` écrite par l'installateur |
| `exe` | nom de l'exécutable principal installé |
| `waitPidArg` (facultatif) | argument qui reçoit le PID à attendre au lancement de l'updater (ex. `/LANCEUR_WAITPID=`) |
| `keepDataArg` (facultatif) | argument passé au désinstallateur pour conserver les données |
| `installArgs` (facultatif) | arguments passés à l'installateur |
| `updateArgs` (facultatif) | arguments passés à l'exe de mise à jour |
| `maxSizeMb` (facultatif) | taille maximale acceptée pour le téléchargement |

Le `apps.json` de la branche `main` du dépôt `Pierre-ML/Lanceur` surcharge celui embarqué dans l'app : ajouter une application ne demande donc pas de nouvelle version du Lanceur. Seuls les dépôts des mêmes propriétaires que ceux déjà acceptés sont pris en compte ; les autres entrées sont ignorées.

### Ce que chaque application doit publier

- Des fichiers à NOM FIXE (sans numéro de version) dans les GitHub Releases du dépôt, correspondant à `setup` et `updater` (par exemple `MonApp-Setup.exe` et `MonApp-Update.exe`), pour que les liens `releases/latest/download/<fichier>` restent valides.
- Un installateur NSIS en mode currentUser (installation dans `%LOCALAPPDATA%\Programs\<productName>`, clé de désinstallation dans `HKCU`), avec `DisplayVersion` renseigné : le Lanceur en déduit l'état installé et la version.
- Un tag `vX.Y.Z` par release.

## Release

Le workflow `.github/workflows/build.yml` se lance à chaque push sur `main` : il vérifie que les versions concordent, construit sous Windows, vérifie la présence des deux installateurs, puis publie une release `Lanceur vX.Y.Z` (tag `vX.Y.Z`) avec `Lanceur-Setup.exe` et `Lanceur-Update.exe`, sauf si ce tag existe déjà. Pour sortir une version : `npm run version:set X.Y.Z`, commit, push.

### Signature SignPath (facultative)

Tant que rien n'est configuré, les installateurs sont publiés non signés. Pour activer la signature, dans Settings > Secrets and variables > Actions du dépôt :

- secret `SIGNPATH_API_TOKEN` ;
- variables `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG`, `SIGNPATH_SIGNING_POLICY_SLUG`, `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`.

Le workflow envoie alors les deux exe à SignPath (artefact `lanceur-unsigned`), attend la signature, remplace les fichiers du dossier `release` par les versions signées et publie celles-ci. La configuration d'artefact SignPath doit signer `Lanceur-Setup.exe` et `Lanceur-Update.exe`.
