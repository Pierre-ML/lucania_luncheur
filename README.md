<div align="center"><img src="public/favicon.svg" alt="Lucania Luncheur" width="96" /><h1>Lucania Luncheur</h1><p>Installez, mettez à jour et lancez vos applications Windows depuis un seul endroit.<br/>Install, update and launch your Windows apps from one place.</p>
<a href="https://github.com/Pierre-ML/lucania_luncheur/releases/latest/download/Lucania-Luncheur-Setup.exe"><b>⬇ Télécharger pour Windows · Download for Windows</b></a><br/><sub>setup pour installer · update pour mettre à jour / setup to install · update to update · <a href="https://github.com/Pierre-ML/lucania_luncheur/releases/latest/download/Lucania-Luncheur-Update.exe">Lucania-Luncheur-Update.exe</a> · <a href="https://github.com/Pierre-ML/lucania_luncheur/releases/latest">releases</a></sub></div>

<p align="center">
<a href="https://github.com/Pierre-ML/lucania_luncheur/releases/latest"><img src="https://img.shields.io/github/v/release/Pierre-ML/lucania_luncheur" alt="Dernière release" /></a>
<a href="LICENSE"><img src="https://img.shields.io/badge/licence-MIT-green" alt="MIT" /></a>
<img src="https://img.shields.io/badge/plateforme-Windows-blue" alt="Windows" />
<a href="https://github.com/Pierre-ML/lucania_luncheur/releases"><img src="https://img.shields.io/github/downloads/Pierre-ML/lucania_luncheur/total" alt="Téléchargements" /></a>
</p>

<p align="center"><a href="#français">Français</a> · <a href="#english">English</a></p>

<!-- TODO: capture d'écran -->

---

# Français

## Présentation

Lucania Luncheur est une application Windows (Tauri v2, interface Astro) qui liste des applications publiées sur GitHub Releases. Pour chaque application, il affiche si elle est installée et quelle est la dernière version disponible. Il permet de l'installer, de la mettre à jour, de la lancer ou de la désinstaller.

Le loup est l'emblème de Lucania Luncheur.

Lucania Luncheur s'installe pour votre compte Windows, avec un installateur NSIS, sans droits administrateur. Il se met à jour avec `Lucania-Luncheur-Update.exe`.

## Applications disponibles

Le plus simple est d'installer Lucania Luncheur : il installe et met à jour toutes les applications ci-dessous. Chaque application reste aussi téléchargeable seule, avec son lien direct.

| | Application | Description | Téléchargement |
| --- | --- | --- | --- |
| <img src="public/app-icons/lucaia.svg" width="40" /> | [Luca-ia](https://github.com/Pierre-ML/Luca-ia) | Interface de chat locale pour modèles GGUF, moteur llama.cpp intégré. | [Luca-ia-Setup.exe](https://github.com/Pierre-ML/Luca-ia/releases/latest/download/Luca-ia-Setup.exe) |
| <img src="public/app-icons/lucascrap.svg" width="40" /> | [Lucascrap](https://github.com/Pierre-ML/lucascrap) | Téléchargeur d'images et de vidéos (Google Images, X, Reddit) avec conversion ffmpeg. | [Lucascrap-Setup.exe](https://github.com/Pierre-ML/lucascrap/releases/latest/download/Lucascrap-Setup.exe) |
| <img src="public/app-icons/lucamedia.svg" width="40" /> | [Lucamedia](https://github.com/Pierre-ML/lucamedia) | Téléchargeur de vidéos HLS/m3u8 avec vérification d'intégrité. | [Lucamedia-Setup.exe](https://github.com/Pierre-ML/lucamedia/releases/latest/download/Lucamedia-Setup.exe) |
| <img src="public/app-icons/lucaaudit.svg" width="40" /> | [Luca-Audit](https://github.com/Pierre-ML/luca-audit) | Audit local de performance, d'accessibilité et d'éco-conception. | [Luca-Audit-Setup.exe](https://github.com/Pierre-ML/luca-audit/releases/latest/download/Luca-Audit-Setup.exe) |

## Fonctionnalités

- 📦 Installation d'une application en un clic, avec barre de progression et annulation possible
- 🔄 Mise à jour d'une application, avec la version installée et la dernière version affichées
- ⏫ « Tout mettre à jour » : toutes les applications à mettre à jour en une fois
- ▶️ Lancement d'une application installée (« Déjà lancée » si elle tourne)
- 🗑️ Désinstallation, avec ou sans suppression des données de l'application
- 🔎 Filtres (toutes, installées, mises à jour, non installées) et champ de recherche
- 🔁 Mise à jour de Lucania Luncheur depuis l'application, après confirmation
- 🛡️ Installation pour l'utilisateur courant, sans droits administrateur
- 🆕 Ajout d'une application sans nouvelle version de Lucania Luncheur (voir « Ajouter une application »)
- 🎨 13 thèmes (dont « Système »)
- 🌍 Interface en français et en anglais
- 📶 Avertissement hors ligne ou en cas de limite de requêtes GitHub

## Installation

1. Téléchargez [`Lucania-Luncheur-Setup.exe`](https://github.com/Pierre-ML/lucania_luncheur/releases/latest/download/Lucania-Luncheur-Setup.exe) (lien direct, dernière release).
2. Lancez l'installateur. Lucania Luncheur s'installe pour votre compte Windows. Aucune autorisation administrateur n'est demandée.
3. Ouvrez Lucania Luncheur, puis installez les applications de votre choix.

Prérequis : Windows 10 ou 11 (64 bits), avec WebView2 (présent sur Windows 11).

> **Antivirus.** L'exécutable n'est pas signé (la signature SignPath est facultative, voir Release). Windows SmartScreen ou certains antivirus peuvent donc l'analyser ou afficher un avertissement.

Le nom du fichier ne change jamais : une seule exception dans votre antivirus suffit pour toutes les versions.

## Mise à jour

Depuis l'application : Lucania Luncheur vérifie la dernière release GitHub et propose la mise à jour. Après confirmation, il télécharge la nouvelle version, se ferme puis redémarre.

À la main : téléchargez [`Lucania-Luncheur-Update.exe`](https://github.com/Pierre-ML/lucania_luncheur/releases/latest/download/Lucania-Luncheur-Update.exe) (lien direct, dernière release) et lancez-le.

Données de Lucania Luncheur : `%LOCALAPPDATA%\com.lanceur.desktop` (config.json, cache WebView2) et `%APPDATA%\com.lanceur.desktop`. Elles ne sont supprimées que par une vraie désinstallation.

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

Le code (`src-tauri/src/registry.rs`) lit aussi `https://raw.githubusercontent.com/Pierre-ML/lucania_luncheur/main/apps.json` (branche `main`). Si ce fichier est valide, il surcharge le `apps.json` embarqué dans l'app : ajouter une application ne demande alors pas de nouvelle version de Lucania Luncheur. Seuls les dépôts des mêmes propriétaires que ceux déjà acceptés sont pris en compte ; les autres entrées sont ignorées. Sinon, ou en cas d'échec, l'app garde le dernier registre valide, à défaut celui embarqué.

### Ce que chaque application doit publier

- Des fichiers à NOM FIXE (sans numéro de version) dans les GitHub Releases du dépôt, correspondant à `setup` et `updater` (par exemple `MonApp-Setup.exe` et `MonApp-Update.exe`), pour que les liens `releases/latest/download/<fichier>` restent valides.
- Un installateur NSIS en mode currentUser (installation dans `%LOCALAPPDATA%\Programs\<productName>`, clé de désinstallation dans `HKCU`), avec `DisplayVersion` renseigné : Lucania Luncheur en déduit l'état installé et la version.
- Un tag `vX.Y.Z` par release.

## Développement

Prérequis : Node.js 22, Rust (toolchain stable), WebView2 (présent sur Windows 11).

| Commande | Rôle |
| --- | --- |
| `npm install` | Installer les dépendances |
| `npm run desktop:dev` | Lancer l'app en développement |
| `npm run check` | Vérifications du frontend |
| `npm run desktop:build` | `tauri build` + installateurs à nom fixe |
| `npm run version:set X.Y.Z` | Changer la version dans les 4 fichiers qui la portent |

`npm run desktop:build` produit dans `src-tauri/target/release/bundle/release/` : `Lucania-Luncheur-Setup.exe` (première installation) et `Lucania-Luncheur-Update.exe` (mise à jour).

Version : la source est `src-tauri/tauri.conf.json` ; `package.json`, `package-lock.json` et `src-tauri/Cargo.toml` doivent porter la même (le workflow le vérifie, `version:set` les met à jour ensemble).

## Release

Le workflow `.github/workflows/build.yml` se lance à chaque push sur `main` : il vérifie que les versions concordent, construit sous Windows, vérifie la présence des deux installateurs, puis publie une release `Lucania Luncheur vX.Y.Z` (tag `vX.Y.Z`) avec `Lucania-Luncheur-Setup.exe` et `Lucania-Luncheur-Update.exe`, sauf si ce tag existe déjà. Pour sortir une version : `npm run version:set X.Y.Z`, commit, push.

### Signature SignPath (facultative)

Tant que rien n'est configuré, les installateurs sont publiés non signés. Pour activer la signature, dans Settings > Secrets and variables > Actions du dépôt :

- secret `SIGNPATH_API_TOKEN` ;
- variables `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG`, `SIGNPATH_SIGNING_POLICY_SLUG`, `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`.

Le workflow envoie alors les deux exe à SignPath (artefact `lanceur-unsigned`), attend la signature, remplace les fichiers du dossier `release` par les versions signées et publie celles-ci. La configuration d'artefact SignPath doit signer `Lucania-Luncheur-Setup.exe` et `Lucania-Luncheur-Update.exe`.

## Stack

Astro 7 · JavaScript vanilla · Tailwind CSS v4 · daisyUI 5 · Tauri v2 · Rust · installateur NSIS.

## Structure

- `src/pages` : page d'accueil. `src/components` : composants Astro. `src/layouts` : mise en page.
- `src/lib/client` : JavaScript de l'interface (applications, actions, thèmes, mise à jour).
- `src/i18n` : textes FR et EN. `src/styles` : styles et thèmes.
- `public/app-icons` : icônes des applications du catalogue.
- `apps.json` : catalogue des applications.
- `src-tauri` : application Tauri (registre, détection, téléchargement, processus, mise à jour de Lucania Luncheur, installateur NSIS dans `src-tauri/windows`).
- `scripts` : installateurs à nom fixe, updater, gestion de la version.
- `.github/workflows/build.yml` : build et release.

## Confidentialité

Lucania Luncheur ne contacte que GitHub : pour lire la liste des applications (`raw.githubusercontent.com`), vérifier les dernières versions et télécharger les installateurs. Il n'envoie aucune donnée personnelle.

## Licence

[MIT](LICENSE). Dépôt : https://github.com/Pierre-ML/lucania_luncheur

---

# English

## Overview

Lucania Luncheur is a Windows app (Tauri v2, Astro interface) that lists applications published on GitHub Releases. For each application, it shows whether it is installed and what the latest version is. It can install, update, launch or uninstall it.

The wolf is the emblem of Lucania Luncheur.

Lucania Luncheur installs for your Windows account, with an NSIS installer, without administrator rights. It updates itself with `Lucania-Luncheur-Update.exe`.

## Available apps

The simplest way is to install Lucania Luncheur: it installs and updates all the apps below. Each app can also be downloaded on its own, with its direct link.

| | App | Description | Download |
| --- | --- | --- | --- |
| <img src="public/app-icons/lucaia.svg" width="40" /> | [Luca-ia](https://github.com/Pierre-ML/Luca-ia) | Local chat interface for GGUF models, with a built-in llama.cpp engine. | [Luca-ia-Setup.exe](https://github.com/Pierre-ML/Luca-ia/releases/latest/download/Luca-ia-Setup.exe) |
| <img src="public/app-icons/lucascrap.svg" width="40" /> | [Lucascrap](https://github.com/Pierre-ML/lucascrap) | Image and video downloader (Google Images, X, Reddit) with ffmpeg conversion. | [Lucascrap-Setup.exe](https://github.com/Pierre-ML/lucascrap/releases/latest/download/Lucascrap-Setup.exe) |
| <img src="public/app-icons/lucamedia.svg" width="40" /> | [Lucamedia](https://github.com/Pierre-ML/lucamedia) | HLS/m3u8 video downloader with integrity checking. | [Lucamedia-Setup.exe](https://github.com/Pierre-ML/lucamedia/releases/latest/download/Lucamedia-Setup.exe) |
| <img src="public/app-icons/lucaaudit.svg" width="40" /> | [Luca-Audit](https://github.com/Pierre-ML/luca-audit) | Local audit of performance, accessibility and eco-design. | [Luca-Audit-Setup.exe](https://github.com/Pierre-ML/luca-audit/releases/latest/download/Luca-Audit-Setup.exe) |

## Features

- 📦 One-click app install, with a progress bar and cancellation
- 🔄 App update, with the installed version and the latest version shown
- ⏫ "Update all": every app that can be updated, at once
- ▶️ Launch an installed app ("Already running" if it is running)
- 🗑️ Uninstall, with or without deleting the app's data
- 🔎 Filters (all, installed, updates, not installed) and a search field
- 🔁 Lucania Luncheur update from within the app, after confirmation
- 🛡️ Per-user install, no administrator rights
- 🆕 Add an app without a new Lucania Luncheur release (see "Add an app")
- 🎨 13 themes (including "System")
- 🌍 French and English interface
- 📶 Warning when offline or when the GitHub rate limit is reached

## Installation

1. Download [`Lucania-Luncheur-Setup.exe`](https://github.com/Pierre-ML/lucania_luncheur/releases/latest/download/Lucania-Luncheur-Setup.exe) (direct link, latest release).
2. Run the installer. Lucania Luncheur installs for your Windows account. No administrator permission is asked.
3. Open Lucania Luncheur, then install the apps you want.

Requirements: Windows 10 or 11 (64-bit), with WebView2 (included in Windows 11).

> **Antivirus.** The executable is not signed (SignPath signing is optional, see Release). Windows SmartScreen or some antivirus tools may scan it or show a warning.

The file name never changes: a single exception in your antivirus is enough for all versions.

## Update

From the app: Lucania Luncheur checks the latest GitHub release and offers the update. After confirmation, it downloads the new version, closes and restarts.

By hand: download [`Lucania-Luncheur-Update.exe`](https://github.com/Pierre-ML/lucania_luncheur/releases/latest/download/Lucania-Luncheur-Update.exe) (direct link, latest release) and run it.

Lucania Luncheur data: `%LOCALAPPDATA%\com.lanceur.desktop` (config.json, WebView2 cache) and `%APPDATA%\com.lanceur.desktop`. They are only deleted by a real uninstall.

## Add an app

Edit only `apps.json` at the root. One object per app:

| Field | Purpose |
| --- | --- |
| `id` | unique, stable identifier |
| `enabled` | `true` to display it |
| `name` | displayed name |
| `description.fr` / `description.en` | short description |
| `icon` | name of a file in `public/app-icons/`, or a `raw.githubusercontent.com` URL |
| `repo` | GitHub repository `owner/repo` |
| `setup` | name of the install file in the release |
| `updater` | name of the update file in the release |
| `identifier` | the app's Tauri identifier (data folder `%LOCALAPPDATA%\<identifier>`) |
| `productName` | name of the `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\<productName>` key written by the installer |
| `exe` | name of the installed main executable |
| `waitPidArg` (optional) | argument that receives the PID to wait for when the updater starts (e.g. `/LANCEUR_WAITPID=`) |
| `keepDataArg` (optional) | argument passed to the uninstaller to keep data |
| `installArgs` (optional) | arguments passed to the installer |
| `updateArgs` (optional) | arguments passed to the update exe |
| `maxSizeMb` (optional) | maximum accepted download size |

The code (`src-tauri/src/registry.rs`) also reads `https://raw.githubusercontent.com/Pierre-ML/lucania_luncheur/main/apps.json` (`main` branch). If that file is valid, it overrides the `apps.json` embedded in the app: adding an app then needs no new Lucania Luncheur release. Only repositories owned by the same owners as those already accepted are taken into account; other entries are ignored. Otherwise, or on failure, the app keeps the last valid registry, or the embedded one.

### What each app must publish

- FIXED-NAME files (no version number) in the repository's GitHub Releases, matching `setup` and `updater` (for example `MyApp-Setup.exe` and `MyApp-Update.exe`), so that `releases/latest/download/<file>` links stay valid.
- An NSIS installer in currentUser mode (install into `%LOCALAPPDATA%\Programs\<productName>`, uninstall key in `HKCU`), with `DisplayVersion` set: Lucania Luncheur uses it to find the installed state and version.
- A `vX.Y.Z` tag per release.

## Development

Requirements: Node.js 22, Rust (stable toolchain), WebView2 (included in Windows 11).

| Command | Purpose |
| --- | --- |
| `npm install` | Install dependencies |
| `npm run desktop:dev` | Run the app in development |
| `npm run check` | Frontend checks |
| `npm run desktop:build` | `tauri build` + fixed-name installers |
| `npm run version:set X.Y.Z` | Change the version in the 4 files that carry it |

`npm run desktop:build` produces, in `src-tauri/target/release/bundle/release/`: `Lucania-Luncheur-Setup.exe` (first install) and `Lucania-Luncheur-Update.exe` (update).

Version: the source is `src-tauri/tauri.conf.json`; `package.json`, `package-lock.json` and `src-tauri/Cargo.toml` must carry the same one (the workflow checks it, `version:set` updates them together).

## Release

The `.github/workflows/build.yml` workflow runs on every push to `main`: it checks that versions match, builds on Windows, checks that both installers exist, then publishes a `Lucania Luncheur vX.Y.Z` release (tag `vX.Y.Z`) with `Lucania-Luncheur-Setup.exe` and `Lucania-Luncheur-Update.exe`, unless that tag already exists. To ship a version: `npm run version:set X.Y.Z`, commit, push.

### SignPath signing (optional)

As long as nothing is configured, installers are published unsigned. To enable signing, in the repository's Settings > Secrets and variables > Actions:

- secret `SIGNPATH_API_TOKEN`;
- variables `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG`, `SIGNPATH_SIGNING_POLICY_SLUG`, `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`.

The workflow then sends both exe files to SignPath (artifact `lanceur-unsigned`), waits for the signature, replaces the files in the `release` folder with the signed versions and publishes those. The SignPath artifact configuration must sign `Lucania-Luncheur-Setup.exe` and `Lucania-Luncheur-Update.exe`.

## Stack

Astro 7 · vanilla JavaScript · Tailwind CSS v4 · daisyUI 5 · Tauri v2 · Rust · NSIS installer.

## Structure

- `src/pages`: home page. `src/components`: Astro components. `src/layouts`: layout.
- `src/lib/client`: interface JavaScript (apps, actions, themes, update).
- `src/i18n`: FR and EN texts. `src/styles`: styles and themes.
- `public/app-icons`: icons of the catalog apps.
- `apps.json`: app catalog.
- `src-tauri`: Tauri app (registry, detection, download, processes, Lucania Luncheur self-update, NSIS installer in `src-tauri/windows`).
- `scripts`: fixed-name installers, updater, version management.
- `.github/workflows/build.yml`: build and release.

## Privacy

Lucania Luncheur only contacts GitHub: to read the app list (`raw.githubusercontent.com`), check the latest versions and download installers. It sends no personal data.

## License

[MIT](LICENSE). Repository: https://github.com/Pierre-ML/lucania_luncheur
