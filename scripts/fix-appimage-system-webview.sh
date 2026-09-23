#!/usr/bin/env bash
# Convierte el AppImage que produce `dx bundle` en uno que usa el WebView del
# sistema en lugar de empaquetar el WebKitGTK/GTK del equipo que lo compiló.
#
# Por qué: `dx bundle --package-types appimage` corre linuxdeploy sin plugin de
# GTK, así que empaqueta el stack GTK/WebKitGTK de la máquina de build. Esa
# libwebkit2gtk trae hardcodeado el directorio de sus procesos helper
# (PKGLIBEXECDIR) en tiempo de compilación: en Ubuntu es
# /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1, que no existe en Arch/Fedora/etc.,
# y WebKit no puede lanzar WebKitNetworkProcess
# ("Unable to spawn a new child process .../WebKitNetworkProcess"). La variable
# WEBKIT_EXEC_PATH no sirve: WebKitGTK solo la respeta en builds con
# ENABLE(DEVELOPER_MODE), no en los paquetes de las distros.
#
# Fix: borrar las `.so` empaquetadas de usr/lib (el stack GTK/WebKit) y
# re-empaquetar con appimagetool. El binario queda con RUNPATH `$ORIGIN/../lib`
# apuntando a un directorio que ya no trae el stack, así que el loader cae a las
# rutas del sistema y usa el WebKitGTK instalado — igual que el .deb/.rpm, que
# declaran libwebkit2gtk-4.1-0, libgtk-3-0, libxdo3 y libssl3 en `depends`.
#
# Dos excepciones que se conservan dentro de usr/lib:
#
# - `libxdo`: no es parte del stack GTK/WebKit y el sistema **no** la garantiza
#   por soname. El runner de build (Ubuntu) linkea `libxdo.so.3`, mientras que
#   Arch ya sólo provee `libxdo.so.4` (xdotool >= 4); sin la copia empaquetada
#   el AppImage muere al arrancar con
#   "libxdo.so.3: cannot open shared object file" (exit 127).
# - `usr/lib/<AppName>/assets`: los assets web de la app (CSS, fuentes, ícono).
#   No son librerías: borrarlos deja la UI **sin estilos** (el login no lo
#   delata porque usa estilos inline; la grilla y el resto sí). Por eso el
#   borrado es selectivo (`*.so*`) y no un `rm -rf usr/lib`.
#
# Requisito en la máquina de destino: el stack del sistema (webkit2gtk-4.1,
# gtk3, openssl), el mismo que ya piden .deb/.rpm.
#
# Uso:
#   ./scripts/fix-appimage-system-webview.sh [path/al/AppImage]
# Sin argumento, busca en ${CARGO_TARGET_DIR:-target}/dx/featherai/bundle/linux/appimage/.

set -euo pipefail

APPIMAGE="${1:-}"
if [[ -z "$APPIMAGE" ]]; then
    base="${CARGO_TARGET_DIR:-target}/dx/featherai/bundle/linux/appimage"
    APPIMAGE="$(find "$base" -maxdepth 1 -name '*.AppImage' -print -quit 2>/dev/null || true)"
fi
if [[ -z "$APPIMAGE" || ! -f "$APPIMAGE" ]]; then
    echo "No se encontró el AppImage (buscado en ${CARGO_TARGET_DIR:-target}/dx/featherai/bundle/linux/appimage)" >&2
    exit 1
fi
APPIMAGE="$(realpath "$APPIMAGE")"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

echo "Extrayendo $APPIMAGE"
"$APPIMAGE" --appimage-extract >/dev/null

# `libxdo` es la única lib de usr/lib que no pertenece al stack GTK/WebKit y que
# el destino tampoco garantiza por soname (ver cabecera). Se toma el soname que
# pide el binario y se rescata antes de vaciar usr/lib.
XDO="$(readelf -d squashfs-root/usr/bin/featherai 2>/dev/null \
    | sed -n 's/.*(NEEDED).*\[\(libxdo\.so\.[0-9]\+\)\].*/\1/p' | head -1)"
XDO="${XDO:-libxdo.so.3}"

echo "Rescatando $XDO de usr/lib"
STASH="$WORK/stash"
mkdir -p "$STASH"
SRC="$(find squashfs-root/usr/lib squashfs-root/usr/lib64 -name "$XDO" -print -quit 2>/dev/null || true)"
if [[ -z "$SRC" ]]; then
    # linuxdeploy no la empaquetó: se toma del sistema de build (mismo soname).
    SRC="$(ldconfig -p 2>/dev/null | sed -n "s#.* => \(.*/$XDO\)\$#\1#p" | head -1)"
fi
if [[ -n "$SRC" && -f "$SRC" ]]; then
    cp -a "$SRC" "$STASH/"
fi

# `usr/lib` no es sólo el stack: linuxdeploy también deja ahí los assets web de
# la app (`usr/lib/<AppName>/assets`). Se relevan para poder verificar después
# que sobrevivieron al borrado.
ASSETS_DIRS=()
while IFS= read -r d; do
    [[ -n "$d" ]] && ASSETS_DIRS+=("$d")
done < <(find squashfs-root/usr/lib squashfs-root/usr/bin -maxdepth 2 -type d -name assets 2>/dev/null)
if [[ ${#ASSETS_DIRS[@]} -eq 0 ]]; then
    echo "ERROR: no se encontraron assets de la app (usr/lib/<App>/assets);" >&2
    echo "       ¿cambió el layout de dx bundle? Sin ellos la UI queda sin estilos." >&2
    exit 1
fi

echo "Quitando las .so empaquetadas para usar el WebView del sistema"
find squashfs-root/usr/lib squashfs-root/usr/lib64 -maxdepth 1 -name '*.so*' \
    -delete 2>/dev/null || true

if [[ -n "$(ls -A "$STASH" 2>/dev/null)" ]]; then
    mkdir -p squashfs-root/usr/lib
    cp -a "$STASH"/. squashfs-root/usr/lib/
fi

APPIMAGETOOL="$WORK/appimagetool"
curl -fsSL -o "$APPIMAGETOOL" \
    "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
chmod +x "$APPIMAGETOOL"

echo "Re-empaquetando en $APPIMAGE"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" squashfs-root "$APPIMAGE" >/dev/null

# Verificación (sobre una extracción limpia del re-empaquetado): sin stack
# GTK/WebKit, con los assets de la app, con `libxdo` (sin ella no arranca en
# distros cuyo libxdo tiene otro soname) y sin NEEDED sin resolver.
rm -rf squashfs-root
"$APPIMAGE" --appimage-extract >/dev/null
if [[ -e squashfs-root/usr/lib/libwebkit2gtk-4.1.so.0 || -e squashfs-root/usr/lib/libgtk-3.so.0 ]]; then
    echo "ERROR: el AppImage re-empaquetado sigue trayendo el stack GTK/WebKit" >&2
    exit 1
fi
if [[ -z "$(find squashfs-root/usr/lib -name "$XDO" -print -quit 2>/dev/null)" ]]; then
    echo "ERROR: el AppImage re-empaquetado no trae $XDO en usr/lib" >&2
    exit 1
fi
for d in "${ASSETS_DIRS[@]}"; do
    if [[ ! -d "$d" ]]; then
        echo "ERROR: el AppImage re-empaquetado perdió los assets de la app ($d)" >&2
        exit 1
    fi
done
FALTAN="$(ldd squashfs-root/usr/bin/featherai 2>/dev/null | sed -n 's/.*not found.*/&/p')"
if [[ -n "$FALTAN" ]]; then
    echo "ERROR: al binario le faltan libs: $FALTAN" >&2
    exit 1
fi
rm -rf squashfs-root

echo "OK: $APPIMAGE (usa el WebView del sistema)"
