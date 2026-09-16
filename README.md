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

## Sincronización entre nodos (GuardianDB / Iroh)

Cada instalación es local-first: escribe en su data dir (`~/.local/share/featherai`,
`%LOCALAPPDATA%\featherai` o `$FEATHRAI_DATA_DIR`) y replica peer-to-peer contra los
nodos que conoce. El **espacio de datos** de cada store (namespace de iroh-docs) se
resuelve al abrirlo, contra los pares conectados en ese momento; por eso la app conecta
los pares *antes* de abrir la base (`src/net.rs`).

### Cómo se encuentran los dos nodos

- **Misma red interna (sin configurar nada):** descubrimiento mDNS propio (servicio
  `featherai`). Necesita multicast entre los equipos: misma VLAN/SSID, sin aislamiento de
  clientes y con el puerto 5353/UDP permitido.
- **Otra red (o mDNS bloqueado):** en *Ajustes → Nodos pares* se pega el id del otro nodo
  (el que muestra su Ajustes). Queda guardado en `<data dir>/peers.txt` y se reconecta en
  cada arranque. Equivalente por entorno: `FEATHRAI_PEERS=<id>[,<id>…]`.
- `FEATHRAI_LAN_WAIT_MS` (por defecto `2500`): espera del descubrimiento LAN antes de
  abrir la base. `0` no espera (sólo conecta lo que ya esté en `peers.txt`/`FEATHRAI_PEERS`).

### Qué esperar al conectar dos instalaciones

- Cuando un nodo ve al otro **al arrancar**, importa su espacio: a partir de ahí los dos
  listan los mismos proyectos/tareas/usuarios y escriben en los dos sentidos (LWW por
  registro). Los datos que el nodo importador tenía en su espacio propio quedan en disco
  pero dejan de listarse — si hay datos propios en ambos lados, hacer backup del data dir
  antes de conectarlos.
- Un par que aparece **después** del arranque queda conectado, pero se une al espacio
  compartido recién en el próximo arranque (Ajustes lo marca: "se une a este espacio en
  el próximo arranque").
- Con más de dos nodos, todos los que vean a alguno del grupo terminan en el mismo espacio.

### Diagnóstico

`FEATHRAI_LOG=info|debug|trace` (o `1`) agrega las trazas de iroh/guardian-db a
`featherai.log`; sin la variable no hay logs de red. Qué mirar:

- `main: pares: N configurados (M con conexión), K por red interna; mDNS: sí|no` — cuántos
  pares ve este nodo y cuántos quedaron conectados.
- `Imported shared iroh-docs document via ticket` — se unió al espacio de un par;
  `Created new iroh-docs document` — creó un espacio propio (no había pares al abrir);
  `possible split-brain` — había un par que no respondió a tiempo (se repara reiniciando).
- `sin conexión con el par …` / `mDNS no disponible` (líneas de `src/net.rs`).

`FEATHRAI_SENTINEL_PORT=<puerto>` expone el Admin RPC de sentinel para inspeccionar la
base en vivo: `guardian-sentinel --connect 127.0.0.1:<puerto>` (las trazas de la app dicen
`sentinel RPC activo en 127.0.0.1:<puerto>`). El proceso que tiene abierta la base sigue
siendo la app, así que este modo adjunto es el único que lee **esta** base: el modo
`guardian-sentinel --data-dir <dir>` abre `<dir>/db`, mientras la app usa
`<dir>/guardian` + `<dir>/iroh` (además chocaría con el lock redb del `iroh/`).

Los tests de convergencia entre dos nodos (red real en loopback) están en
`src/persistence/peers_tests.rs`; correr con
`cargo test --bin featherai peers_tests -- --test-threads=1` (el de mDNS requiere
multicast: agregar `--ignored`).

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
<target>/dx/featherai/bundle/linux/deb/featherai_0.1.2_amd64.deb
<target>/dx/featherai/bundle/linux/rpm/featherai-0.1.2-1.x86_64.rpm
<target>/dx/featherai/bundle/linux/appimage/featherai_0.1.2_x86_64.AppImage
```

Install:

```bash
sudo apt install ./featherai_0.1.2_amd64.deb      # Debian / Ubuntu
sudo dnf install ./featherai-0.1.2-1.x86_64.rpm   # Fedora / RHEL
./featherai_0.1.2_x86_64.AppImage                 # portable, no install
APPIMAGE_EXTRACT_AND_RUN=1 ./featherai_0.1.2_x86_64.AppImage   # sin FUSE2
```

`dx` writes the `.deb`/`.rpm` itself (no `dpkg-deb`/`rpmbuild` needed); the
AppImage step downloads `linuxdeploy`. `[bundle.deb].depends` declares the
WebView libraries (`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libxdo3`, `libssl3`,
with `t64` alternatives for newer Debian/Ubuntu) because `dx` cannot infer them
from the binary. The data directory — `~/.local/share/featherai` (or `$XDG_DATA_HOME/featherai`
if set), `%LOCALAPPDATA%\featherai` on Windows — is **not** removed by
uninstalling. Versions up to `v0.0.2` used `%APPDATA%\featherai` on Windows and
`$HOME/.local/share/featherai` on any OS; on first run the app moves that legacy
directory to the OS location above and logs the migration in `featherai.log`.

Con el driver propietario de NVIDIA, WebKitGTK deja la ventana en gris (X11) o
rompe el sync explícito (Wayland). La app aplica el workaround en runtime, antes
de crear la ventana (`webkit2gtk-nvidia-quirk`): `WEBKIT_DISABLE_DMABUF_RENDERER=1`
en X11 y `__NV_DISABLE_EXPLICIT_SYNC=1` en Wayland sin `egl-wayland2`. Si alguna
de esas dos variables ya está definida en el entorno, la app **no** la toca: un
valor explícito (incluido `WEBKIT_DISABLE_DMABUF_RENDERER=0`) manda. Por eso
antes fallaba solo al lanzar desde el menú/AppImage —esas vías no heredan lo que
el shell exporta— y andaba desde la terminal. Qué quedó aplicado se ve en
`featherai.log`.

### Windows

Prerequisites: the MSVC toolchain (`rustup default stable-msvc`), Visual Studio
Build Tools with "Desktop development with C++", and WebView2 (already present
on Windows 10/11 via Edge; the NSIS installer ships the bootstrapper).

```powershell
dx bundle --platform desktop --package-types nsis --package-types msi --release
```

Output in `target\dx\featherai\bundle\windows\`:
`Featherai_0.1.2_x64-setup.exe` (NSIS) and `Featherai_0.1.2_x64.msi` (WiX).
`dx` downloads NSIS 3.11 and WiX 3.14 on first use. The `.exe`/`.msi` icon, name
and version come from `[bundle]`/`[bundle.windows]` in `Dioxus.toml`
(`identifier`, `icon_path`, `version`); they must stay stable across releases so
Windows treats an update as the same app.

`src/main.rs` carries
`#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]`,
so the release `.exe` is a pure GUI app (no console window behind it) while
debug builds keep the logs.

