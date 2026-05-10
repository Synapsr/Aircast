<div align="right">

🌐 **Langue** : **Français** | [English](README.md)

</div>

<div align="center">

# 🎙️ Aircast

### L'app desktop open-source pour le streaming radio

**Stop aux logiciels de streaming propriétaires. Reprenez votre antenne en main.**

[![Dernière release](https://img.shields.io/github/v/release/Synapsr/Aircast?style=for-the-badge&logo=github&label=Release)](https://github.com/Synapsr/Aircast/releases/latest)
[![GitHub Stars](https://img.shields.io/github/stars/Synapsr/Aircast?style=for-the-badge&logo=github)](https://github.com/Synapsr/Aircast)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/Tauri-2-FFC131?style=for-the-badge&logo=tauri&logoColor=black)](https://tauri.app/)

[⬇️ Télécharger](#%EF%B8%8F-démarrage-rapide) • [✨ Fonctionnalités](#-fonctionnalités) • [🏗️ Architecture](docs/architecture.md) • [🤝 Contribuer](docs/contributing.md)

<br>

<img src="src-tauri/icons/icon.png" width="120" alt="Logo Aircast" />

</div>

---

## 💡 C'est quoi Aircast ?

Aircast est une **application desktop native et portable** qui capture n'importe quelle entrée audio et la diffuse vers un serveur Icecast. Vous choisissez votre micro, vous enregistrez vos serveurs en preset, vous cliquez **Go Live**. Une **mode Studio** ajoute la file d'attente musicale, les cartouches de jingles avec déclenchement instantané, le ducking automatique au micro et le crossfade.

Conçu pour les radios qui veulent **garder la main sur leurs outils** — sans abonnement, sans cloud verrouillé, sans frais cachés.

```
                                                     ┌────────────────┐
   🎙️  Micro ─┐                                       │  🌐 Serveur    │
              │                                       │     Icecast    │
   🎵 Musique─┼─►  Mixeur ─►  ffmpeg (PUT) ─►  ─────► │  (votre radio) │
              │                                       └────────────────┘
   🎚️ Cartos─┘
              │
              └────────► 🔊 Monitor local (toujours actif, jamais coupé)
```

---

## 🎯 Pourquoi Aircast ?

|        💸 **Gratuit & open-source**        |        🎙️ **Audio toujours actif**         |        🔁 **Mode Studio**         |
| :----------------------------------------: | :-----------------------------------------: | :-------------------------------: |
| Licence MIT. Aucun abonnement, jamais.     | Le monitor local ne coupe pas au Go Live.   | Musique, cartos, ducking, crossfade. |

|       📦 **Auto-suffisant**       |        🖥️ **Vraiment cross-platform**        |       🌍 **i18n FR / EN**        |
| :-------------------------------: | :------------------------------------------: | :------------------------------: |
| ffmpeg embarqué. Aucune install.  | macOS, Windows, Linux natifs. Pas d'Electron. | Français & anglais. Extensible.  |

---

## ⬇️ Démarrage rapide

Téléchargez la dernière version pour votre plateforme depuis la page [Releases](https://github.com/Synapsr/Aircast/releases/latest) :

| Plateforme                   | Fichier                                    | Notes                                                                          |
| ---------------------------- | ------------------------------------------ | ------------------------------------------------------------------------------ |
| 🍎 **macOS** (Apple Silicon) | `Aircast_<version>_aarch64.dmg`            | Glisser dans Applications. Premier lancement : clic-droit → Ouvrir.            |
| 🍎 **macOS** (Intel)         | `Aircast_<version>_x64.dmg`                | Idem.                                                                          |
| 🪟 **Windows** (portable)     | `Aircast-portable-windows-x64.zip`         | **Pas d'install, pas d'admin.** Décompresser, double-clic sur `Aircast.exe`.   |
| 🪟 **Windows** (installeur)   | `Aircast_<version>_x64-setup.exe`          | Installeur NSIS en mode utilisateur — aucune demande de droits admin.          |
| 🐧 **Linux**                  | `aircast_<version>_amd64.deb` / `.AppImage` | Paquet Debian standard ou AppImage auto-suffisant.                             |

> 🪟 **Note Windows** : nécessite le runtime WebView2, présent par défaut sur Windows 10 1803+ et Windows 11. Sur les versions plus anciennes, télécharger chez [Microsoft](https://developer.microsoft.com/microsoft-edge/webview2/).

C'est tout. Sélectionnez votre micro, renseignez votre serveur Icecast dans **Configuration**, cliquez **Go Live**.

---

## ✨ Fonctionnalités

### 📡 Streaming

|     | Fonctionnalité                | Description                                                                                |
| :-: | ----------------------------- | ------------------------------------------------------------------------------------------ |
| ⚙️  | **Presets de serveur**        | Sauvegardez autant de serveurs Icecast que nécessaire (host, port, mount, codec, bitrate). |
| 🔌  | **Toute entrée audio**        | Micro intégré, interface USB, câble virtuel — tout ce que l'OS expose.                     |
| 🎚️  | **VU-mètre temps réel**       | Monitoring RMS + peak à 20 Hz.                                                             |
| 🔁  | **Auto-reconnect**            | Intervalle configurable, fail-over instantané sur micro-coupure réseau.                    |
| 🚨  | **Erreurs claires**           | Auth, mount, réseau, timeout — chaque cas est classé en message actionnable.               |
| 🎧  | **Codecs**                    | MP3 (libmp3lame) et AAC (natif), 64–320 kbps.                                              |
| 🌐  | **Icecast 2.4+**              | Protocole HTTP PUT — supporte le mount racine `/`, contrairement à `icecast://`.           |

### 🎛️ Mode Studio

|     | Fonctionnalité            | Description                                                                                  |
| :-: | ------------------------- | -------------------------------------------------------------------------------------------- |
| 🎵  | **File musicale**          | Glissez-déposez MP3/WAV/FLAC/OGG. Play, pause, réorganiser, retirer. Décodage en flux.       |
| 🎚️  | **Cartoucheur**            | 12 slots pour vos jingles, déclenchement one-shot, pré-décodés pour latence nulle.           |
| 🎙️  | **Ducking micro**          | La musique baisse automatiquement quand le micro s'ouvre. Niveau et durée configurables.    |
| 🔄  | **Crossfade**              | Transitions douces entre titres, durée configurable.                                         |
| 🎼  | **Format-agnostique**      | `FrameResampler` maison gère n'importe quel taux (22 / 44,1 / 48 / 96 kHz, mono ou stéréo).   |

### 🛠️ Fiabilité

|     | Fonctionnalité          | Description                                                                                |
| :-: | ----------------------- | ------------------------------------------------------------------------------------------ |
| 🧪  | **118 tests unitaires** | 81 Rust + 37 TypeScript couvrant audio, réseau, presets, validation et parité i18n.        |
| 🔒  | **Écritures atomiques** | Si le JSON des presets est corrompu, l'app revient aux defaults — jamais bloquée.         |
| 🪶  | **Callbacks lock-free** | Le chemin temps-réel cpal n'utilise que des atomics et ring buffers. Aucun lock.            |
| 📊  | **Logs structurés**     | Chaque module log via `log`, niveau ajustable via `RUST_LOG`.                              |
| 🌐  | **Deep links**          | Schéma `aircast://` pour partager une configuration de serveur.                             |

---

## 🏗️ Architecture

```
┌───────────────────────────┐                       ┌──────────────────────────────┐
│  UI React + TypeScript    │ ── tauri::invoke ──►  │  Backend Rust (Tauri 2)       │
│  Tailwind v4, i18n FR/EN  │ ◄── tauri events ───  │   audio::capture (cpal)       │
└───────────────────────────┘                       │   studio::mixer + resampler   │
                                                    │   stream::pipeline            │
                                                    │   presets::store              │
                                                    └────────────────┬──────────────┘
                                                                     │ stdin (PCM s16le)
                                                                     ▼
                                                    ┌──────────────────────────────┐
                                                    │  Sidecar ffmpeg (subprocess) │
                                                    │   HTTP PUT → Icecast 2.4+    │
                                                    └──────────────────────────────┘
```

La capture est **toujours active** dès qu'un device est sélectionné. Le pipeline de streaming se branche sur le même flux audio sans le redémarrer — passer de live → idle ne coupe jamais le monitor local.

Notes de design complètes dans [`docs/architecture.md`](docs/architecture.md).

---

## 🛠️ Build depuis les sources

Envie de bricoler ?

```bash
# Prérequis : Rust (stable), Node 20+, pnpm 9+
git clone https://github.com/Synapsr/Aircast.git
cd Aircast
pnpm install
pnpm fetch-ffmpeg          # télécharge le sidecar ffmpeg pour votre OS
pnpm tauri dev             # lance l'app en mode dev
```

Pour produire des bundles installables pour votre plateforme hôte :

```bash
pnpm build:bundle          # → src-tauri/target/release/bundle/...
```

Les builds CI pour **macOS (arm64 + x64)**, **Windows** et **Linux** sont produits automatiquement à chaque tag git — voir [`.github/workflows/release.yml`](.github/workflows/release.yml).

---

## 🤝 Contribuer

Les PR sont les bienvenues. Lisez [`docs/contributing.md`](docs/contributing.md) d'abord — il couvre les vérifs locales, le style et les invariants d'architecture issus de bugs réels. Ne les cassez pas sans expliquer pourquoi.

La suite de checks que la CI passe à chaque PR :

```bash
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
cd ..
pnpm typecheck
pnpm test
pnpm build
```

---

## 📜 Licence

- **Source d'Aircast** — MIT, voir [`LICENSE`](LICENSE).
- **ffmpeg embarqué** — build statique LGPL. Aircast lance ffmpeg comme sous-processus séparé : selon l'interprétation FSF de la "simple agrégation", la licence de ffmpeg ne se propage pas à la source d'Aircast.

---

<div align="center">

Fait avec ♥ pour les radios, par [Synapsr](https://github.com/Synapsr).

</div>

---

<div align="center">

### Avec le soutien de

<a href="https://culturesnumeriques.ac-rennes.fr/spip.php?rubrique80">
  <img src="https://podeduc.apps.education.fr/media/files/be7df5511bc2365fa61ea304696e5c777a2153718e3a30007267ed7d9e4c8f42/banniereinstitdynamique_N7Kkcgu.png" alt="DRANE Bretagne" width="640" />
</a>

<sub>Compagnon du projet DRANE [Porte-Voix.app](https://porte-voix.app/)</sub>

</div>
