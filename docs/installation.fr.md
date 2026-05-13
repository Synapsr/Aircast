# Installer Aircast

🌐 **[This page in English](installing.md)**

Aircast distribue des installateurs prêts à l'emploi pour macOS, Windows et
Linux sur [GitHub Releases](https://github.com/Synapsr/Aircast/releases/latest).

> **À propos de la signature** — Aircast n'est pas encore signé avec un
> certificat Apple Developer ID ni Windows EV. Le système d'exploitation
> affiche un avertissement la première fois que vous lancez l'app. La
> procédure ci-dessous est la façon standard de confirmer que vous faites
> confiance au binaire ; elle ne contourne aucune sécurité réelle.

---

## 🍎 macOS (Apple Silicon)

1. Téléchargez **`Aircast_x.x.x_aarch64.dmg`** depuis la dernière release.
2. Ouvrez le DMG et glissez **Aircast** dans **Applications**.
3. **Ouvrez le Terminal** (Spotlight `⌘ + espace` → tapez `Terminal`).
4. Collez cette commande et appuyez sur Entrée :

   ```bash
   xattr -d com.apple.quarantine /Applications/Aircast.app
   ```

5. Double-cliquez maintenant sur Aircast dans Applications — l'app s'ouvre
   normalement.

### Pourquoi cette étape ?

macOS marque toutes les apps téléchargées depuis Internet d'un attribut de
"quarantaine". Comme Aircast n'est pas encore signé avec un certificat
Apple Developer ID payant, Gatekeeper refuse de l'ouvrir ou affiche
*« Aircast est endommagé et ne peut pas être ouvert »*. La commande
`xattr` retire ce drapeau de quarantaine pour que l'OS considère Aircast
comme n'importe quelle autre app installée localement.

> Une seule fois suffit. Après le premier lancement, Aircast s'ouvre
> normalement au double-clic.

---

## 🪟 Windows

1. Téléchargez **`Aircast_x.x.x_x64-setup.exe`** (installateur) ou
   **`Aircast-portable-windows-x64.zip`** (portable, juste à décompresser).
2. Double-cliquez sur le fichier téléchargé.
3. **Windows SmartScreen** affiche *« Windows a protégé votre PC »*.
   Cliquez **Informations complémentaires** (lien en bas à gauche), puis
   **Exécuter quand même**.
4. L'app se lance ; SmartScreen ne reviendra plus pour ce fichier.

### Pré-requis

Windows 10 1803+ ou Windows 11. Sur des Windows plus anciens, installer le
[runtime WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
(gratuit, ~1 min).

---

## 🐧 Linux

Selon votre distribution :

- **Debian / Ubuntu** :
  ```bash
  sudo dpkg -i Aircast_x.x.x_amd64.deb
  ```
- **Fedora / RHEL** :
  ```bash
  sudo dnf install Aircast-x.x.x-1.x86_64.rpm
  ```
- **AppImage (toutes distros)** :
  ```bash
  chmod +x Aircast_x.x.x_amd64.AppImage
  ./Aircast_x.x.x_amd64.AppImage
  ```

Aucun avertissement de sécurité sous Linux.

---

## Premier lancement

La première fois qu'Aircast s'ouvre, vous aurez besoin de :

1. **Un microphone** — sélectionnez-en un dans le sélecteur en haut.
2. **Un serveur Icecast** — ouvrez **Réglages** (en haut à droite) →
   onglet **Serveurs** → renseignez l'hôte, le port, le mount, le nom
   d'utilisateur et le mot de passe fournis par votre station.
3. Cliquez **Passer à l'antenne**. Le VU-mètre doit réagir à votre voix
   et le statut passe à « À l'antenne ».

Pour le mode relais (rediffuser un flux existant), passez en **Relais**
dans le header, ajoutez une URL amont dans **Réglages → Sources relais**,
et lancez le live.

---

## Dépannage

**macOS refuse toujours d'ouvrir Aircast après la commande `xattr`** —
essayez la variante récursive avec `sudo` :

```bash
sudo xattr -dr com.apple.quarantine /Applications/Aircast.app
open /Applications/Aircast.app
```

**Windows SmartScreen revient à chaque lancement** — votre antivirus a
probablement mis le binaire en quarantaine. Ajoutez une exception pour le
dossier d'installation d'Aircast, ou re-téléchargez et réessayez.

**Erreur « Aircast.app est endommagée » après la commande `xattr`** —
votre DMG a peut-être été corrompu au téléchargement (rare).
Re-téléchargez depuis la page de release et réessayez.

**Autres problèmes** — ouvrez l'onglet **Réglages → Avancé** et cliquez
sur **Copier le rapport de diagnostic**. Collez le résultat dans une
[nouvelle issue sur GitHub](https://github.com/Synapsr/Aircast/issues) —
il contient tout ce dont nous avons besoin (version, OS, logs récents,
config caviardée).
