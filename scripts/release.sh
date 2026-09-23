#!/usr/bin/env bash
# Crea un release de featherai:
#   1. Sube la versión en Cargo.toml, Dioxus.toml y Cargo.lock (quedan iguales).
#   2. Commitea el bump (`chore: versión X.Y.Z`).
#   3. Crea el tag anotado `vX.Y.Z`, que dispara el workflow de bundles
#      (.github/workflows/bundle.yml).
#
# Uso:
#   ./scripts/release.sh <version|patch|minor|major> [--push] [--dry-run]
#
# Ejemplos:
#   ./scripts/release.sh patch           # 0.1.4 -> 0.1.5
#   ./scripts/release.sh minor           # 0.1.4 -> 0.2.0
#   ./scripts/release.sh 0.2.0 --push    # versión explícita + push a origin
#
# `Cargo.toml` es la fuente de verdad de la versión; `Dioxus.toml` y `Cargo.lock`
# deben coincidir (el script los actualiza y verifica que partan iguales).
# Requiere working tree limpio: el bump es un commit nuevo sobre HEAD.

set -euo pipefail

cd "$(dirname "$0")/.."

usage() {
  cat <<'EOF'
Uso: ./scripts/release.sh <version|patch|minor|major> [--push] [--dry-run]

  version    versión semver explícita (ej. 0.2.0)
  patch      sube el patch (0.1.4 -> 0.1.5)
  minor      sube el minor y resetea patch (0.1.4 -> 0.2.0)
  major      sube el major y resetea el resto (0.1.4 -> 1.0.0)
  --push     pushea el commit y el tag a origin
  --dry-run  edita los archivos, muestra el diff y los revierte (no commitea)
EOF
  exit 2
}

[[ $# -ge 1 ]] || usage

NEW_VERSION=""
PUSH=0
DRY_RUN=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --push) PUSH=1 ;;
    --dry-run|-n) DRY_RUN=1 ;;
    -h|--help) usage ;;
    -*) echo "flag desconocido: $1" >&2; usage ;;
    *) NEW_VERSION="$1" ;;
  esac
  shift
done
[[ -n "$NEW_VERSION" ]] || usage

# --- estado del repo ---
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || { echo "no es un repo git" >&2; exit 1; }
branch=$(git rev-parse --abbrev-ref HEAD)

if [[ -n "$(git status --porcelain)" ]]; then
  echo "working tree sucio; commiteá o descartá los cambios antes del release:" >&2
  git status --short >&2
  exit 1
fi

get_cargo_version()  { awk -F'"' '/^version = "/{print $2; exit}' Cargo.toml; }
get_dioxus_version() { awk -F'"' '/^version = "/{print $2; exit}' Dioxus.toml; }
get_lock_version()   { awk -F'"' '/^name = "featherai"$/{f=1} f && /^version = "/{print $2; exit}' Cargo.lock; }

current=$(get_cargo_version)
dioxus_cur=$(get_dioxus_version)
lock_cur=$(get_lock_version)

[[ "$dioxus_cur" == "$current" ]] || {
  echo "Cargo.toml ($current) y Dioxus.toml ($dioxus_cur) no coinciden; alinealos primero" >&2
  exit 1
}
if [[ "$lock_cur" != "$current" ]]; then
  echo "AVISO: Cargo.lock dice $lock_cur (Cargo.toml $current); el bump lo corrige" >&2
fi

# --- nueva versión ---
case "$NEW_VERSION" in
  patch|minor|major)
    IFS='.' read -r maj min pat <<< "$current"
    case "$NEW_VERSION" in
      patch) pat=$((pat + 1)) ;;
      minor) min=$((min + 1)); pat=0 ;;
      major) maj=$((maj + 1)); min=0; pat=0 ;;
    esac
    NEW_VERSION=$(printf '%d.%d.%d' "$maj" "$min" "$pat")
    ;;
esac

[[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "versión inválida: $NEW_VERSION (esperado X.Y.Z)" >&2
  exit 1
}

if [[ "$(printf '%s\n' "$current" "$NEW_VERSION" | sort -V | tail -1)" != "$NEW_VERSION" || "$NEW_VERSION" == "$current" ]]; then
  echo "la nueva versión ($NEW_VERSION) debe ser mayor que la actual ($current)" >&2
  exit 1
fi

if git rev-parse -q --verify "refs/tags/v${NEW_VERSION}" >/dev/null 2>&1; then
  echo "el tag v${NEW_VERSION} ya existe" >&2
  exit 1
fi

# --- aplicar el bump ---
sed -i -E 's/^version = ".*"/version = "'"${NEW_VERSION}"'"/' Cargo.toml
sed -i -E 's/^version = ".*"/version = "'"${NEW_VERSION}"'"/' Dioxus.toml
awk -v v="$NEW_VERSION" -F'"' '
  /^name = "featherai"$/ { f = 1 }
  f && /^version = "/ { print "version = \"" v "\""; f = 0; next }
  { print }
' Cargo.lock > Cargo.lock.tmp && mv Cargo.lock.tmp Cargo.lock

# --- los tres deben quedar iguales ---
c=$(get_cargo_version); d=$(get_dioxus_version); l=$(get_lock_version)
if [[ "$c" != "$NEW_VERSION" || "$d" != "$NEW_VERSION" || "$l" != "$NEW_VERSION" ]]; then
  echo "no coinciden tras editar: cargo=$c dioxus=$d lock=$l" >&2
  exit 1
fi

echo "==> $current -> $NEW_VERSION (branch $branch)"

if [[ "$DRY_RUN" == 1 ]]; then
  git --no-pager diff -- Cargo.toml Dioxus.toml Cargo.lock
  git checkout -- Cargo.toml Dioxus.toml Cargo.lock
  echo
  echo "--dry-run: nada commiteado. Confirmá con: ./scripts/release.sh $NEW_VERSION"
  exit 0
fi

git add Cargo.toml Dioxus.toml Cargo.lock
git commit -m "chore: versión ${NEW_VERSION}"
git tag -a "v${NEW_VERSION}" -m "v${NEW_VERSION}"

echo "OK: commit + tag v${NEW_VERSION} creados."

if [[ "$PUSH" == 1 ]]; then
  git push origin "$branch"
  git push origin "v${NEW_VERSION}"
  echo "OK: pusheado. El workflow de bundles arranca con el tag v${NEW_VERSION}."
else
  echo "Para publicar: git push origin $branch && git push origin v${NEW_VERSION}"
fi
