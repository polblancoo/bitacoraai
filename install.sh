#!/bin/sh
# Instalador de CrewHelm (tm-server) para Linux: sin sudo, todo dentro de tu usuario.
#   curl -fsSL https://raw.githubusercontent.com/polblancoo/crewhelm/main/install.sh | sh
# Opciones (después de «sh -s --»):  --port 7878   --no-service   --no-shell-tools   --shell-tools-only   --with-agent   --with-opencode   --with-claude   --with-gemini   --with-codex   --with-all-ais   --version v0.7.1
set -eu

REPO="polblancoo/crewhelm"
PORT=7878; SERVICE=1; SHELLTOOLS=1; ONLYTOOLS=0; AGENT=0; OPENCODE=0; CLAUDE=0; GEMINI=0; CODEX=0; VERSION="latest"
while [ $# -gt 0 ]; do
  case "$1" in
    --port) PORT="$2"; shift 2 ;;
    --no-service) SERVICE=0; shift ;;
    --no-shell-tools) SHELLTOOLS=0; shift ;;  # no instalar zsh, eza, starship, fzf… para las terminales
    --shell-tools-only) ONLYTOOLS=1; shift ;;  # solo las herramientas de terminal (no toca tm-server ni el servicio)
    --with-agent) AGENT=1; shift ;;
    --with-opencode) OPENCODE=1; AGENT=1; shift ;;  # agente para usar OpenRouter (modelos gratis y de pago)
    --with-claude) CLAUDE=1; AGENT=1; shift ;;
    --with-gemini) GEMINI=1; AGENT=1; shift ;;
    --with-codex) CODEX=1; AGENT=1; shift ;;
    --with-all-ais) OPENCODE=1; CLAUDE=1; GEMINI=1; CODEX=1; AGENT=1; shift ;;
    --version) VERSION="$2"; shift 2 ;;
    -h|--help) sed -n '2,5p' "$0"; exit 0 ;;
    *) echo "opción desconocida: $1" >&2; exit 1 ;;
  esac
done

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = "Linux" ] || die "este instalador es para Linux (Windows y macOS: próximamente)"
case "$(uname -m)" in x86_64|amd64) ARCH=x86_64 ;; *) die "arquitectura $(uname -m) no soportada todavía (probá: cargo install crewhelm-server)" ;; esac
command -v curl >/dev/null 2>&1 || die "falta curl"

# Herramientas de terminal para las personas del equipo (zsh con colores, eza, fzf, starship…): se instalan solo las que faltan.
# Los paquetes del sistema necesitan root (si hay sudo sin contraseña se usan; si no, se muestra el comando); el resto va a ~/.local/bin sin sudo.
shell_tools() {
  BIN="$HOME/.local/bin"; mkdir -p "$BIN"
  missing=""; for t in zsh eza fzf starship rg tree; do command -v "$t" >/dev/null 2>&1 || [ -x "$BIN/$t" ] || missing="$missing $t"; done
  for t in bat batcat; do command -v "$t" >/dev/null 2>&1 && have_bat=1; done; [ "${have_bat:-0}" = 1 ] || missing="$missing bat"
  [ -n "$missing" ] || { say "Herramientas de terminal: ya están todas"; return 0; }
  say "Herramientas de terminal que faltan:$missing"
  SUDO=""; [ "$(id -u)" = 0 ] || { command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1 && SUDO="sudo -n"; }
  if command -v apt-get >/dev/null 2>&1; then PM="apt-get install -y zsh git ripgrep fd-find bat fzf tree nano vim htop zsh-autosuggestions zsh-syntax-highlighting eza"
  elif command -v dnf >/dev/null 2>&1; then PM="dnf install -y zsh git ripgrep fd-find bat fzf tree nano vim htop zsh-autosuggestions zsh-syntax-highlighting eza"
  elif command -v pacman >/dev/null 2>&1; then PM="pacman -S --noconfirm --needed zsh git ripgrep fd bat fzf tree nano vim htop zsh-autosuggestions zsh-syntax-highlighting eza"
  else PM=""; fi
  if [ -n "$PM" ] && { [ "$(id -u)" = 0 ] || [ -n "$SUDO" ]; }; then
    say "Instalando paquetes del sistema ($PM)"
    # shellcheck disable=SC2086
    $SUDO $PM >/dev/null 2>&1 || echo "   (algún paquete no se pudo instalar; seguimos con lo demás)"
  elif [ -n "$PM" ]; then
    echo "   (no tengo permisos de administrador: para el resto ejecutá una vez  sudo $PM)"
  fi
  # lo que sigue faltando se instala en ~/.local/bin, sin sudo
  if ! command -v starship >/dev/null 2>&1 && [ ! -x "$BIN/starship" ]; then
    say "Instalando starship en $BIN"
    curl -fsSL https://starship.rs/install.sh | sh -s -- -b "$BIN" -y >/dev/null 2>&1 || echo "   (no pude instalar starship; probá: curl -sS https://starship.rs/install.sh | sh -s -- -b \$HOME/.local/bin)"
  fi
  if command -v cargo >/dev/null 2>&1 && ! command -v eza >/dev/null 2>&1 && [ ! -x "$BIN/eza" ]; then
    say "Instalando eza con cargo (tarda unos minutos)"
    cargo install eza --locked --root "$HOME/.local" >/dev/null 2>&1 || echo "   (no pude instalar eza)"
  fi
  say "Herramientas de terminal: listo (las que no se pudieron instalar se pueden pedir después con  sh install.sh --shell-tools-only)"
}
if [ "$ONLYTOOLS" = 1 ]; then shell_tools; exit 0; fi
command -v sha256sum >/dev/null 2>&1 || die "falta sha256sum"

BIN="$HOME/.local/bin"; DATA="$HOME/.local/share/crewhelm"
mkdir -p "$BIN" "$DATA"
if [ -n "${CREWHELM_BASE_URL:-}" ]; then BASE="$CREWHELM_BASE_URL"  # espejo interno o pruebas
elif [ "$VERSION" = "latest" ]; then BASE="https://github.com/$REPO/releases/latest/download"; else BASE="https://github.com/$REPO/releases/download/$VERSION"; fi

TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
fetch() { # fetch <archivo>: baja y verifica el SHA-256 publicado en el mismo release
  say "Descargando $1"
  curl -fsSL "$BASE/$1" -o "$TMP/$1" || return 1
  curl -fsSL "$BASE/SHA256SUMS" -o "$TMP/SHA256SUMS" || return 1
  want="$(grep " $1\$" "$TMP/SHA256SUMS" | cut -d' ' -f1)"; got="$(sha256sum "$TMP/$1" | cut -d' ' -f1)"
  [ -n "$want" ] && [ "$want" = "$got" ] || die "la suma de control de $1 no coincide: no lo instalo"
  tar xzf "$TMP/$1" -C "$TMP"
}
install_one() { # install_one <binario>
  if fetch "$1-linux-$ARCH.tar.gz"; then install -m 755 "$TMP/$1" "$BIN/$1"
  elif command -v cargo >/dev/null 2>&1; then
    say "No pude bajar el binario; lo compilo con cargo (tarda unos minutos)"
    case "$1" in tm-server) crate=crewhelm-server ;; *) crate=crewhelm-agent ;; esac
    cargo install "$crate" --root "$HOME/.local"
  else die "no pude descargar $1 y cargo no está instalado"; fi
}
install_one tm-server
[ "$AGENT" = 1 ] && install_one tm-agent
npm_tool() { # npm_tool <programa> <paquete>: programas de IA que el agente lanza (npm, en ~/.local, sin sudo)
  if command -v "$1" >/dev/null 2>&1 || [ -x "$BIN/$1" ]; then say "$1 ya está instalado"; return; fi
  if command -v npm >/dev/null 2>&1; then
    say "Instalando $1 (npm, paquete $2) en $HOME/.local"
    npm install -g --prefix "$HOME/.local" "$2" >/dev/null 2>&1 || echo "   (no pude instalar $1; probá a mano: npm install -g --prefix \$HOME/.local $2)"
  else echo "   ($1 necesita Node.js/npm, que no está instalado: https://nodejs.org)"; fi
}
[ "$CLAUDE" = 1 ] && npm_tool claude @anthropic-ai/claude-code
[ "$GEMINI" = 1 ] && npm_tool gemini @google/gemini-cli
[ "$CODEX" = 1 ] && npm_tool codex @openai/codex
[ "$OPENCODE" = 1 ] && npm_tool opencode opencode-ai
say "Instalado en $BIN"
case ":$PATH:" in *":$BIN:"*) ;; *) echo "   (agregá $BIN a tu PATH para usar tm-server desde cualquier carpeta)" ;; esac

if [ "$SERVICE" = 1 ] && command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
  UNIT="$HOME/.config/systemd/user"; mkdir -p "$UNIT"
  cat > "$UNIT/tm-server.service" <<UNITEOF
[Unit]
Description=CrewHelm (tm-server): tablero de equipos humanos y de IA
After=network.target

[Service]
Environment=TM_DATABASE=sqlite://$DATA/tm-server.db?mode=rwc
Environment=TM_ADDR=0.0.0.0:$PORT
ExecStart=$BIN/tm-server
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
UNITEOF
  systemctl --user daemon-reload
  systemctl --user enable --now tm-server >/dev/null 2>&1 || die "no pude arrancar el servicio (mirá: journalctl --user -u tm-server)"
  loginctl enable-linger "$USER" >/dev/null 2>&1 || echo "   (para que siga andando al cerrar sesión: sudo loginctl enable-linger $USER)"
  say "Servicio activo: systemctl --user status tm-server"
else
  say "Sin servicio: para arrancarlo a mano ejecutá  TM_DATABASE=sqlite://$DATA/tm-server.db?mode=rwc TM_ADDR=0.0.0.0:$PORT $BIN/tm-server"
fi

[ "$SHELLTOOLS" = 1 ] && shell_tools
printf '\n\033[1;32m¡Listo!\033[0m Abrí  \033[1mhttp://localhost:%s\033[0m  en el navegador: la guía de inicio te lleva paso a paso.\n' "$PORT"
