#!/bin/sh
# Opens Hermes' API to FlowFlow on this machine, reachable only on the
# Tailscale network, then prints the QR code the iPhone scans to link.
# Run it in a terminal on the Hermes machine, never through a Hermes chat:
# the QR code carries the key.
set -eu

HERMES_HOME="${HERMES_HOME:-$HOME/.hermes}"
ENV_FILE="$HERMES_HOME/.env"
PORT="${API_SERVER_PORT:-8642}"

say() { printf '%s\n' "$*"; }
fail() { printf 'erreur : %s\n' "$*" >&2; exit 1; }

command -v hermes >/dev/null 2>&1 || fail "hermes est introuvable sur cette machine."
command -v tailscale >/dev/null 2>&1 || fail "Tailscale n'est pas installé : https://tailscale.com/download"
command -v curl >/dev/null 2>&1 || fail "curl est introuvable."
[ -f "$ENV_FILE" ] || fail "$ENV_FILE est introuvable."

cp -p "$ENV_FILE" "$ENV_FILE.bak-flowflow-$(date +%Y%m%d%H%M%S)"
say "ok  $ENV_FILE sauvegardé"

key=$(grep '^API_SERVER_KEY=' "$ENV_FILE" | tail -n 1 | cut -d= -f2-)
if [ -z "$key" ]; then
  key=$(openssl rand -hex 32)
  printf '\n# FlowFlow chat\nAPI_SERVER_ENABLED=true\nAPI_SERVER_KEY=%s\n' "$key" >>"$ENV_FILE"
  say "ok  accès activé, clé créée"
else
  grep -q '^API_SERVER_ENABLED=true' "$ENV_FILE" || printf 'API_SERVER_ENABLED=true\n' >>"$ENV_FILE"
  say "ok  accès activé, clé existante gardée"
fi
chmod 600 "$ENV_FILE"

hermes gateway restart >/dev/null
tries=0
until curl -s -o /dev/null "http://127.0.0.1:$PORT/health"; do
  tries=$((tries + 1))
  [ "$tries" -lt 30 ] || fail "Hermes n'écoute pas sur le port $PORT."
  sleep 1
done
say "ok  Hermes redémarré"

tailscale serve --bg --https="$PORT" "http://127.0.0.1:$PORT" >/dev/null
url=$(tailscale serve status 2>/dev/null | grep -o "https://[^ ]*:$PORT" | head -n 1)
[ -n "$url" ] || fail "adresse Tailscale introuvable (tailscale serve status)."
say "ok  joignable sur ton réseau Tailscale, et seulement là"

encoded=$(printf '%s' "$url" | sed 's/:/%3A/g; s#/#%2F#g')
link="flowflow://hermes?url=$encoded&key=$key"

say ""
say "Adresse  $url"
if command -v qrencode >/dev/null 2>&1; then
  say "Scanne ce code avec l'appareil photo de ton iPhone :"
  qrencode -t ANSIUTF8 -m 2 "$link"
else
  say "Pour afficher le QR code : installe qrencode (apt install qrencode, ou"
  say "brew install qrencode), puis relance ce script."
fi
say "Ne partage pas ce code : il contient ta clé."
