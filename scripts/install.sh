#!/usr/bin/env bash
# Instala as ferramentas da fábrica no PATH (links simbólicos: recompilar com build.sh já atualiza).
# uso: scripts/install.sh [destino]   (padrão: ~/.local/bin)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${1:-$HOME/.local/bin}"
mkdir -p "$DEST"
[ -x "$ROOT/bin/vf" ] || "$ROOT/scripts/build.sh"
link() { ln -sfn "$1" "$DEST/$2"; echo "  $DEST/$2 -> $1"; }
echo "instalando em $DEST:"
link "$ROOT/bin/vf" vf
[ -x "$ROOT/bin/vf-mygo" ] && link "$ROOT/bin/vf-mygo" vf-mygo
link "$ROOT/scripts/novo.sh" vf-novo
link "$ROOT/skills/edicao/scripts/render-hf.sh" vf-render-hf
case ":$PATH:" in *":$DEST:"*) ;; *) echo "atenção: $DEST não está no PATH (fish: fish_add_path $DEST · zsh: export PATH=\"$DEST:\$PATH\")";; esac
"$DEST/vf" doctor || true
