# Development

Your new jumpstart project includes basic organization with an organized `assets` folder and a `components` folder.
If you chose to develop with the router feature, you will also have a `views` folder.

```
project/
├─ assets/ # Any assets that are used by the app should be placed here
├─ src/
│  ├─ main.rs # The entrypoint for the app. It also defines the routes for the app.
│  ├─ components/
│  │  ├─ mod.rs # Defines the components module
│  ├─ views/ # The views each route will render in the app.
│  │  ├─ mod.rs # Defines the module for the views route and re-exports the components for each route
├─ Cargo.toml # The Cargo.toml file defines the dependencies and feature flags for your project
```

### Automatic Tailwind (Dioxus 0.7+)

As of Dioxus 0.7, there no longer is a need to manually install tailwind. Simply `dx serve` and you're good to go!

Automatic tailwind is supported by checking for a file called `tailwind.css` in your app's manifest directory (next to Cargo.toml). To customize the file, use the dioxus.toml:

```toml
[application]
tailwind_input = "my.css"
tailwind_output = "assets/out.css"
```

### Tailwind Manual Install

To use tailwind plugins or manually customize tailwind, you can can install the Tailwind CLI and use it directly.

1. Install npm: https://docs.npmjs.com/downloading-and-installing-node-js-and-npm
2. Install the Tailwind CSS CLI: https://tailwindcss.com/docs/installation/tailwind-cli
3. Run the following command in the root of the project to start the Tailwind CSS compiler:

```bash
npx @tailwindcss/cli -i ./input.css -o ./assets/tailwind.css --watch
```

### Serving Your App

Run the following command in the root of your project to start developing with the default platform:

```bash
dx serve --platform desktop
```

To run for a different platform, use the `--platform platform` flag. E.g.
```bash
dx serve --platform desktop
```

## Building installers (Linux / Windows)

`dx bundle` packages the app **for the operating system that runs the command**.
There is no cross-compilation of installers: the Windows `.msi`/`.exe` are built
with the Windows-only tools `makensis.exe` (NSIS) and WiX
(`candle.exe`/`light.exe`), so they must be produced on Windows or in CI (see
`.github/workflows/bundle.yml`).

Artifacts land in `<target>/dx/featherai/bundle/<platform>/<format>/`, where
`<target>` is `target/` unless `CARGO_TARGET_DIR` is set (this machine:
`/home/mmanto/rust_shared_target`). Always pass `--release`: the debug bundle
embeds debug info (~750 MB installed vs. a fraction of that in release).

Bundle metadata (identifier, icons, descriptions, `.deb` dependencies) lives in
the `[bundle]` section of `Dioxus.toml`.

### Linux

Build prerequisites — the dev headers of the system WebView:

```bash
# Arch
sudo pacman -S --needed base-devel webkit2gtk-4.1 xdotool openssl librsvg
# Debian / Ubuntu
sudo apt install build-essential libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libssl-dev librsvg2-dev
```

```bash
dx bundle --platform desktop --package-types deb --package-types rpm --package-types appimage --release
```

`--package-types` takes **one value per flag** (`deb,rpm` is rejected with
`invalid value`), and each flag may be repeated. Output (release build: ~20 MB
`.deb`, ~64 MB binary):

```
<target>/dx/featherai/bundle/linux/deb/featherai_0.1.0_amd64.deb
<target>/dx/featherai/bundle/linux/rpm/featherai-0.1.0-1.x86_64.rpm
<target>/dx/featherai/bundle/linux/appimage/featherai_0.1.0_x86_64.AppImage
```

Install:

```bash
sudo apt install ./featherai_0.1.0_amd64.deb      # Debian / Ubuntu
sudo dnf install ./featherai-0.1.0-1.x86_64.rpm   # Fedora / RHEL
./featherai_0.1.0_x86_64.AppImage                 # portable, no install
APPIMAGE_EXTRACT_AND_RUN=1 ./featherai_0.1.0_x86_64.AppImage   # sin FUSE2
```

`dx` writes the `.deb`/`.rpm` itself (no `dpkg-deb`/`rpmbuild` needed); the
AppImage step downloads `linuxdeploy`. `[bundle.deb].depends` declares the
WebView libraries (`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libxdo3`, `libssl3`,
with `t64` alternatives for newer Debian/Ubuntu) because `dx` cannot infer them
from the binary. The data directory (`~/.local/share/featherai`) is **not**
removed by uninstalling.

### Windows

Prerequisites: the MSVC toolchain (`rustup default stable-msvc`), Visual Studio
Build Tools with "Desktop development with C++", and WebView2 (already present
on Windows 10/11 via Edge; the NSIS installer ships the bootstrapper).

```powershell
dx bundle --platform desktop --package-types nsis --package-types msi --release
```

Output in `target\dx\featherai\bundle\windows\`:
`featherai_0.1.0_x64-setup.exe` (NSIS) and `featherai_0.1.0_x64_en-US.msi` (WiX).
`dx` downloads NSIS 3.11 and WiX 3.14 on first use. The `.exe`/`.msi` icon, name
and version come from `[bundle]`/`[bundle.windows]` in `Dioxus.toml`
(`identifier`, `icon_path`, `version`); they must stay stable across releases so
Windows treats an update as the same app.

`src/main.rs` carries
`#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]`,
so the release `.exe` is a pure GUI app (no console window behind it) while
debug builds keep the logs.

