#!/usr/bin/env bash
set -euo pipefail

# Installa Server Monitor Tray per l'utente corrente:
# - build in release
# - binario in ~/.local/bin
# - icona nel tema icone utente
# - config di default in ~/.config/server-monitor-tray (se non già presente)
# - voce nel menu applicazioni e avvio automatico al login

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_NAME="server-monitor-tray"

BIN_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"
CONFIG_DIR="$HOME/.config/$APP_NAME"
APPLICATIONS_DIR="$HOME/.local/share/applications"
AUTOSTART_DIR="$HOME/.config/autostart"

echo "==> Compilazione in modalità release..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

echo "==> Creazione directory..."
mkdir -p "$BIN_DIR" "$ICON_DIR" "$CONFIG_DIR" "$APPLICATIONS_DIR" "$AUTOSTART_DIR"

echo "==> Installazione binario e icona..."
# Se l'applet è già in esecuzione il binario è "occupato": si copia su un file
# temporaneo e si rinomina, operazione atomica che non richiede di chiudere il processo attivo.
cp "$SCRIPT_DIR/target/release/watchdog" "$BIN_DIR/$APP_NAME.new"
mv -f "$BIN_DIR/$APP_NAME.new" "$BIN_DIR/$APP_NAME"
cp "$SCRIPT_DIR/icon_app.png" "$ICON_DIR/$APP_NAME.png"

if [ ! -f "$CONFIG_DIR/watchdog.conf" ]; then
    echo "==> Creazione config di default da watchdog.conf.example..."
    cp "$SCRIPT_DIR/watchdog.conf.example" "$CONFIG_DIR/watchdog.conf"
else
    echo "==> Config esistente trovato in $CONFIG_DIR/watchdog.conf, non sovrascritto."
fi

echo "==> Creazione file .desktop (menu applicazioni + autostart)..."
DESKTOP_FILE="$APPLICATIONS_DIR/$APP_NAME.desktop"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=Server Monitor Tray
Comment=Monitora siti/host via HTTP o ping con icona nella system tray
Exec=$BIN_DIR/$APP_NAME $CONFIG_DIR/watchdog.conf
Icon=$APP_NAME
Terminal=false
Categories=Network;
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF

cp "$DESKTOP_FILE" "$AUTOSTART_DIR/$APP_NAME.desktop"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPLICATIONS_DIR" || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true

echo ""
echo "Installazione completata."
echo "  - Binario:     $BIN_DIR/$APP_NAME"
echo "  - Config:      $CONFIG_DIR/watchdog.conf"
echo "  - Menu/avvio:  $DESKTOP_FILE"
echo ""
echo "L'applet partirà automaticamente al prossimo login, oppure lanciala ora con:"
echo "  $BIN_DIR/$APP_NAME $CONFIG_DIR/watchdog.conf"
