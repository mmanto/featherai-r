# Build del AppImage de featherai.
#
#   make appimage   # bundle release + fix del WebView + copia a ~/Programas
#   make clean      # borra la cache de build de esta app (no toca otros proyectos)
#
# `make` sin argumentos equivale a `make appimage`.
#
# El AppImage se genera con el WebView del sistema (no empaqueta WebKitGTK/GTK):
# después del bundle se corre scripts/fix-appimage-system-webview.sh, igual que
# el workflow de CI. El resultado queda copiado en $(PROGRAMAS_DIR), que por
# defecto es ~/Programas.

SHELL := /bin/bash
.DEFAULT_GOAL := appimage

APP_NAME := featherai

# Misma resolución que usa scripts/fix-appimage-system-webview.sh y dx: el
# target puede venir de CARGO_TARGET_DIR (en esta máquina apunta a
# ~/rust_shared_target, compartido con otros proyectos).
TARGET_DIR := $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)
DX_APP_DIR := $(TARGET_DIR)/dx/$(APP_NAME)
BUNDLE_DIR := $(DX_APP_DIR)/bundle/linux/appimage

PROGRAMAS_DIR ?= $(HOME)/Programas

.PHONY: all appimage clean

all: appimage

appimage:
	@command -v dx >/dev/null || { echo "ERROR: falta dioxus-cli (dx) en PATH" >&2; exit 1; }
	dx bundle --platform desktop --package-types appimage --release
	@set -euo pipefail; \
	base='$(BUNDLE_DIR)'; \
	appimage="$$(find "$$base" -maxdepth 1 -name '*.AppImage' -printf '%T@ %p\n' 2>/dev/null \
		| sort -nr | head -1 | cut -d' ' -f2-)"; \
	if [[ -z "$$appimage" ]]; then \
		echo "ERROR: dx no generó ningún AppImage en $$base" >&2; \
		exit 1; \
	fi; \
	./scripts/fix-appimage-system-webview.sh "$$appimage"; \
	mkdir -p '$(PROGRAMAS_DIR)'; \
	dest='$(PROGRAMAS_DIR)'/"$$(basename "$$appimage")"; \
	install -m 755 "$$appimage" "$$dest"; \
	echo "AppImage copiado en $$dest"

# Cache de build de esta app únicamente. `cargo clean` sin `-p` borraría todo
# $(TARGET_DIR), que está compartido con otros proyectos de la máquina.
clean:
	cargo clean -p $(APP_NAME)
	rm -rf '$(DX_APP_DIR)'
	@echo "Cache de $(APP_NAME) eliminada"
