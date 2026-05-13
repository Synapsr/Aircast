<div align="right">

🌐 **Langue** : **Français** | [English](README.md)

</div>

<div align="center">

# 🎙️ Aircast

### L'app desktop open-source pour le streaming radio

**Stop aux logiciels de streaming propriétaires. Reprenez votre antenne en main.**

[![Dernière release](https://img.shields.io/github/v/release/Synapsr/Aircast?style=for-the-badge&logo=github&label=Release)](https://github.com/Synapsr/Aircast/releases/latest)
[![GitHub Stars](https://img.shields.io/github/stars/Synapsr/Aircast?style=for-the-badge&logo=github)](https://github.com/Synapsr/Aircast)
[![License](https://img.shields.io/badge/License-GPL_3.0-blue?style=for-the-badge)](LICENSE)
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
| Licence GPL-3.0. Aucun abonnement, jamais.     | Le monitor local ne coupe pas au Go Live.   | Musique, cartos, ducking, crossfade. |

|       📦 **Auto-suffisant**       |        🖥️ **Vraiment cross-platform**        |       🌍 **i18n FR / EN**        |
| :-------------------------------: | :------------------------------------------: | :------------------------------: |
| ffmpeg embarqué. Aucune install.  | macOS, Windows, Linux natifs. Pas d'Electron. | Français & anglais. Extensible.  |

---

## ⬇️ Démarrage rapide

Téléchargez la dernière version pour votre plateforme depuis la page [Releases](https://github.com/Synapsr/Aircast/releases/latest) :

| Plateforme                   | Fichier                                    |
| ---------------------------- | ------------------------------------------ |
| 🍎 **macOS** (Apple Silicon) | `Aircast_<version>_aarch64.dmg`            |
| 🪟 **Windows** (portable)     | `Aircast-portable-windows-x64.zip`         |
| 🪟 **Windows** (installeur)   | `Aircast_<version>_x64-setup.exe`          |
| 🐧 **Linux**                  | `aircast_<version>_amd64.deb` / `.AppImage` |

> Aircast n'est pas encore signé (Apple Developer ID / Windows EV), donc
> l'OS affiche un avertissement au premier lancement. Voir le
> **[guide d'installation complet](docs/installation.fr.md)** pour la
> procédure exacte par plateforme (une seule commande Terminal pour macOS,
> « Exécuter quand même » pour Windows, rien pour Linux).

Une fois installé : sélectionnez votre micro, ajoutez votre serveur Icecast dans **Réglages**, cliquez **Passer à l'antenne**.

---

## ✨ Fonctionnalités

### 🎚️ Trois modes, une seule app

|     | Mode      | Ce qu'il fait                                                                            |
| :-: | --------- | ---------------------------------------------------------------------------------------- |
| 🎙️  | **Simple** | Choisir un micro → choisir un serveur → Passer à l'antenne. Le chemin le plus rapide, source→destination visible en un coup d'œil. |
| 🎛️  | **Studio** | File musicale, cartoucheur 9 slots, ducking micro, crossfade, chip "À l'antenne" en direct. Panneau radio complet. |
| 📡  | **Relais** | Rediffuse une URL de flux existante (HTTP/HTTPS/HLS/Icecast). Sources amont nommées, transcodées à la volée. |

Chaque mode peut être masqué dans Réglages → Avancé si une station n'en utilise qu'un.

### 📡 Streaming

|     | Fonctionnalité                | Description                                                                                |
| :-: | ----------------------------- | ------------------------------------------------------------------------------------------ |
| ⚙️  | **Presets de serveur**        | Sauvegardez autant de serveurs Icecast que nécessaire (host, port, mount, codec, bitrate). |
| 🔌  | **Toute entrée audio**        | Micro intégré, interface USB, câble virtuel — tout ce que l'OS expose.                     |
| 🎚️  | **VU-mètre temps réel**       | Monitoring RMS + peak à 20 Hz, avec échelle colorée correcte (vert/jaune/rouge à positions fixes, pas écrasées). |
| 🔁  | **Auto-reconnect**            | Intervalle configurable, fail-over instantané sur micro-coupure réseau.                    |
| 🚨  | **Dialogue d'erreur riche**   | Les erreurs ffmpeg/Icecast sont classées en messages actionnables ; le détail brut reste disponible pour debug. |
| 🎧  | **Codecs**                    | MP3 (libmp3lame) et AAC (natif), 64–320 kbps.                                              |
| 🌐  | **Icecast 2.4+**              | Protocole HTTP PUT — supporte le mount racine `/`, contrairement à `icecast://`.           |
| ⚠️  | **Garde-fou changement mode** | Changer de mode en direct affiche une modale listant les conséquences concrètes (musique stop, micro s'ouvre, etc.) pour ne pas mettre du silence à l'antenne par accident. |

### 🎵 Mode Studio

|     | Fonctionnalité            | Description                                                                                  |
| :-: | ------------------------- | -------------------------------------------------------------------------------------------- |
| 🎵  | **File musicale**          | Glissez-déposez MP3/WAV/FLAC/OGG. Play, pause, réorganiser, retirer. Décodage en flux.       |
| 🎚️  | **Cartoucheur**            | 9 slots pour vos jingles, déclenchement one-shot, pré-décodés pour latence nulle.            |
| 🎙️  | **Ducking micro**          | La musique baisse automatiquement quand le micro s'ouvre. Niveau configurable.              |
| 🔄  | **Crossfade**              | Transitions douces entre titres, durée configurable.                                         |
| 🎼  | **Format-agnostique**      | `FrameResampler` maison gère n'importe quel taux (22 / 44,1 / 48 / 96 kHz, mono ou stéréo).   |
| 🏷️  | **Chip à l'antenne**       | Le titre réellement diffusé aux auditeurs est affiché en direct dans la card Now Playing — un clic pour éditer les réglages de diffusion. |

### 📡 Mode Relais

|     | Fonctionnalité           | Description                                                                              |
| :-: | ------------------------ | ---------------------------------------------------------------------------------------- |
| 🔗  | **Sources nommées**      | Sauvegardez autant d'URLs amont que vous voulez (flux audio HTTP/HTTPS, HLS .m3u8, Icecast, fichiers locaux). |
| 🔁  | **Reconnect amont**      | Si le flux amont coupe, Aircast réessaye toutes les 5 s avec retour visuel en direct (connexion / flux reçu / reconnexion). |
| 🎚️  | **Même UX destination**  | Source → flèche → serveur : on comprend instantanément "ce qui sort de quoi". |

### 🏷️ Diffusion du titre

Pousse le titre vers l'endpoint `/admin/metadata` d'Icecast avec **uniquement les credentials source** (pas besoin de mot de passe admin — fonctionne avec l'auth mount-level que libshout et butt utilisent depuis des années).

|     | Mode      | Ce qu'il pousse                                                                            |
| :-: | --------- | ----------------------------------------------------------------------------------------- |
| 🎼  | **Auto**   | Rend un modèle depuis les tags ID3 (`{title}` `{artist}` `{album}` `{next_title}` `{station}` `{show}` …). Configurable par preset. |
| 📝  | **Statique** | Texte fixe — utile pour les talk-shows ou les pauses (ex. *« Vous écoutez Radio XYZ »*). |
| 📂  | **Fichier** | Lit un fichier texte externe à intervalle configurable (UTF-8 / UTF-16 BOM-aware). Parfait pour synchroniser avec Mixxx, RadioDJ ou un autre broadcaster. |

Plus un override micro (titre différent quand le micro est ouvert) et un bouton "diffuser maintenant" pour tester. Le titre actuellement diffusé est affiché en direct dans une strip (mode Simple) ou une chip (mode Studio).

### 🛠️ Fiabilité & support

|     | Fonctionnalité                | Description                                                                              |
| :-: | ----------------------------- | ---------------------------------------------------------------------------------------- |
| 🧪  | **100+ tests unitaires**      | 100 Rust + 37 TypeScript couvrant le resampling audio, le framing URL, la détection BOM, les presets, la validation et la parité i18n. |
| 🔒  | **Écritures atomiques**       | Si le JSON des presets est corrompu, l'app revient aux defaults — jamais bloquée.        |
| 🪶  | **Callbacks lock-free**       | Le chemin temps-réel cpal n'utilise que des atomics et ring buffers. Aucun lock.         |
| 📊  | **Log rotatif persistant**    | Logger fichier toujours actif avec rotation. Chaque transition stream/mode/erreur horodatée. |
| 🩺  | **Bundle de diagnostic**      | Un clic dans Réglages → Avancé copie un rapport caviardé (version, OS, config active, 300 dernières lignes de log) prêt à coller dans un bug report. |
| 🌐  | **Deep links**                | Schéma `aircast://` pour partager une configuration de serveur.                          |
| 🛡️  | **Isolation dev / prod**      | Les builds dev (`pnpm tauri:dev`) utilisent un identifier séparé, on peut itérer localement sans toucher au fichier de presets prod. |

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

- **Source d'Aircast** — GPL-3.0-or-later, voir [`LICENSE`](LICENSE).
- **ffmpeg embarqué** — build statique LGPL. Aircast lance ffmpeg comme sous-processus séparé : selon l'interprétation FSF de la "simple agrégation", la licence de ffmpeg ne se propage pas à la source d'Aircast.

---

<div align="center">

Fait avec ♥ pour les radios, par [Synapsr](https://github.com/Synapsr).

</div>

---

<div align="center">

<img src="public/france2030.svg" alt="France 2030" width="120" />

<sub>Opération soutenue par l'État dans le cadre de l'action *Territoires Numériques Éducatifs* du *Programme d'investissements d'avenir*, opérée par la Caisse des Dépôts.</sub>

<sub>[Découvrez la Suite.Studio](https://suite.studio/) · [Porte-Voix.app](https://porte-voix.app/)</sub>

</div>
