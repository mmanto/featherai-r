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
# Fix: borrar las libs empaquetadas (usr/lib) y re-empaquetar con appimagetool.
# El binario queda con RUNPATH `$ORIGIN/../lib` apuntando a un directorio que ya
# no existe, así que el loader cae a las rutas del sistema y usa el WebKitGTK
# instalado — igual que el .deb/.rpm, que declaran libwebkit2gtk-4.1-0,
# libgtk-3-0, libxdo3 y libssl3 en `depends`.
#
# Requisito en la máquina de destino: el stack del sistema (webkit2gtk-4.1,
# gtk3, libxdo, openssl), el mismo que ya piden .deb/.rpm.
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

echo "Quitando las libs empaquetadas (usr/lib) para usar el WebView del sistema"
rm -rf squashfs-root/usr/lib squashfs-root/usr/lib64

APPIMAGETOOL="$WORK/appimagetool"
curl -fsSL -o "$APPIMAGETOOL" \
    "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
chmod +x "$APPIMAGETOOL"

echo "Re-empaquetando en $APPIMAGE"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" squashfs-root "$APPIMAGE" >/dev/null

# Verificación barata: el resultado no debe traer el stack GTK/WebKit empaquetado.
"$APPIMAGE" --appimage-extract >/dev/null
if [[ -e squashfs-root/usr/lib/libwebkit2gtk-4.1.so.0 ]] || [[ -e squashfs-root/usr/lib ]]; then
    echo "ERROR: el AppImage re-empaquetado sigue trayendo usr/lib" >&2
    exit 1
fi
rm -rf squashfs-root

echo "OK: $APPIMAGE (usa el WebView del sistema)"
