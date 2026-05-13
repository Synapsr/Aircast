# Installing Aircast

🇫🇷 **[Cette page en français](installation.fr.md)**

Aircast distributes ready-to-run installers for macOS, Windows and Linux on
[GitHub Releases](https://github.com/Synapsr/Aircast/releases/latest).

> **Note about code signing** — Aircast is not yet signed with an Apple
> Developer ID nor a Windows EV certificate. The OS will show a one-time
> warning the first time you launch the app. The procedure below is the
> standard way to confirm you trust the binary; it does not bypass any
> real security feature.

---

## 🍎 macOS (Apple Silicon)

1. Download **`Aircast_x.x.x_aarch64.dmg`** from the latest release.
2. Open the DMG and drag **Aircast** into **Applications**.
3. **Open a Terminal** (Spotlight `⌘ + Space` → type `Terminal`).
4. Paste this command and press Enter:

   ```bash
   xattr -d com.apple.quarantine /Applications/Aircast.app
   ```

5. Now double-click Aircast in Applications — it opens normally.

### Why this step?

macOS marks any app downloaded from the internet with a "quarantine"
attribute. Since Aircast isn't signed with a paid Apple Developer ID yet,
Gatekeeper either refuses to open it or shows *"Aircast is damaged and
can't be opened"*. The `xattr` command removes that quarantine flag so the
OS treats Aircast like any other locally-installed app.

> You only need to do this once. After the first launch, Aircast opens
> normally with a double-click.

---

## 🪟 Windows

1. Download **`Aircast_x.x.x_x64-setup.exe`** (installer) or
   **`Aircast-portable-windows-x64.zip`** (no install, just extract).
2. Double-click the file you downloaded.
3. **Windows SmartScreen** shows *"Windows protected your PC"*. Click
   **More info** (small link at the bottom-left), then **Run anyway**.
4. The app launches; SmartScreen won't bother you again for that file.

### Requirements

Windows 10 1803+ or Windows 11. Older Windows needs the
[WebView2 runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
(free, ~1 minute install).

---

## 🐧 Linux

Pick whichever format matches your distro:

- **Debian / Ubuntu**:
  ```bash
  sudo dpkg -i Aircast_x.x.x_amd64.deb
  ```
- **Fedora / RHEL**:
  ```bash
  sudo dnf install Aircast-x.x.x-1.x86_64.rpm
  ```
- **AppImage (any distro)**:
  ```bash
  chmod +x Aircast_x.x.x_amd64.AppImage
  ./Aircast_x.x.x_amd64.AppImage
  ```

No security prompts on Linux.

---

## First launch

The first time Aircast opens you will need:

1. **A microphone** — pick one in the device selector at the top.
2. **An Icecast server** — open **Setup** (top-right) → **Servers** tab →
   add the host, port, mount, username and password your station provided.
3. Click **Go Live**. The VU meter should react to your voice, and the
   status badge turns "On air".

For relay mode (rebroadcasting an existing stream), switch to **Relay** in
the header, add an upstream URL in **Setup → Relay sources**, and Go Live.

---

## Troubleshooting

**macOS still refuses to open Aircast after the `xattr` command** — try
the recursive variant with `sudo`:

```bash
sudo xattr -dr com.apple.quarantine /Applications/Aircast.app
open /Applications/Aircast.app
```

**Windows SmartScreen keeps re-appearing** — that usually means your
antivirus quarantined the binary. Add an exception for the Aircast install
folder, or re-download and try again.

**"Aircast.app is damaged" error after the `xattr` command** — your DMG
download may have been corrupted (rare). Re-download from the release page
and try again.

**Other issues** — open the **Setup → Advanced** tab and click **Copy
diagnostic report**. Paste the result into a [new issue on GitHub](
https://github.com/Synapsr/Aircast/issues) — it contains everything we
need (version, OS, recent logs, sanitized config).
