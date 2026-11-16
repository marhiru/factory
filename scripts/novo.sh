#!/usr/bin/env bash
# Cria um projeto de vídeo da fábrica.
# uso: novo.sh <pasta> --formato yt-short[,tiktok] --motor remotion-2d [--idioma pt] [--duracao 60] [--install]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
[ $# -ge 1 ] || { sed -n 2,3p "$0"; exit 2; }
DEST="$1"; shift
FORMATOS="yt-short"; MOTOR="remotion-2d"; IDIOMA="pt"; DURACAO=""; INSTALL=0
while [ $# -gt 0 ]; do
  case "$1" in
    --formato|--formatos) FORMATOS="$2"; shift 2;;
    --motor) MOTOR="$2"; shift 2;;
    --idioma) IDIOMA="$2"; shift 2;;
    --duracao) DURACAO="$2"; shift 2;;
    --install) INSTALL=1; shift;;
    *) echo "opção desconhecida: $1" >&2; exit 2;;
  esac
done
PRINCIPAL="${FORMATOS%%,*}"
case "$PRINCIPAL" in
  yt-long)  WPM=160; DUR_DEF=720;;
  yt-short) WPM=222; DUR_DEF=60;;
  tiktok)   WPM=222; DUR_DEF=35;;
  *) echo "formato inválido: $PRINCIPAL (yt-long | yt-short | tiktok)" >&2; exit 2;;
esac
IFS=',' read -ra FS <<< "$FORMATOS"
for f in "${FS[@]}"; do case "$f" in yt-long|yt-short|tiktok) ;; *) echo "formato inválido: $f" >&2; exit 2;; esac; done
case "$MOTOR" in remotion-2d|kenburns|ia-video|remotion-three|mapas-3d) ;; *) echo "motor inválido: $MOTOR" >&2; exit 2;; esac
DURACAO="${DURACAO:-$DUR_DEF}"
# renderer: mygo (GPU nativa) para motores 2D quando o binário existe; senão Remotion
RENDERER="remotion"
case "$MOTOR" in remotion-2d|kenburns|ia-video) [ -x "$ROOT/bin/vf-mygo" ] && RENDERER="mygo";; esac
if [ -e "$DEST" ] && [ -n "$(ls -A "$DEST" 2>/dev/null)" ]; then echo "$DEST já existe e não está vazia" >&2; exit 1; fi
mkdir -p "$DEST"
NOME="$(basename "$DEST")"

cp -R "$ROOT/templates/projeto/." "$DEST/"
case "$MOTOR" in
  remotion-2d|kenburns|ia-video)
    # os três renderizam pelo mesmo template Remotion orientado a data/edl.json
    cp -R "$ROOT/skills/animacao/templates/remotion-2d/." "$DEST/"
    ;;
  remotion-three)
    echo "motor remotion-three: siga a skill cartoon-3d-short (templates em github.com/mromidev/claude-animated-shorts)" ;;
  mapas-3d)
    echo "motor mapas-3d: siga a skill mapas-3d-videos-virais (kit-mapas-3d)" ;;
esac
cp "$ROOT/skills/assets/templates/style-bible.md" "$DEST/assets/style-bible.md"

LIST="$(printf '%s' "$FORMATOS" | sed 's/,/, /g')"
find "$DEST" -type f \( -name '*.yaml' -o -name '*.md' -o -name '*.json' \) -not -path '*/node_modules/*' -print0 |
  xargs -0 sed -i '' -e "s|__NOME__|$NOME|g" -e "s|__DATA__|$(date +%F)|g" -e "s|__FORMATOS__|$LIST|g" \
    -e "s|__PRINCIPAL__|$PRINCIPAL|g" -e "s|__MOTOR__|$MOTOR|g" -e "s|__IDIOMA__|$IDIOMA|g" \
    -e "s|__DURACAO__|$DURACAO|g" -e "s|__WPM__|$WPM|g" -e "s|__RENDERER__|$RENDERER|g"

(cd "$DEST" && git init -q 2>/dev/null || true)
if [ "$INSTALL" = 1 ] && [ -f "$DEST/package.json" ]; then (cd "$DEST" && npm install --silent); fi
echo "projeto criado: $DEST  (formatos: $LIST · motor: $MOTOR · duração alvo: ${DURACAO}s · ${WPM} wpm · renderer: $RENDERER)"
