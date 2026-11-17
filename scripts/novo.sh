#!/usr/bin/env bash
# Cria um projeto de vídeo da fábrica.
# uso: novo.sh <pasta> --formato yt-short[,tiktok] --motor cutout-2d [--idioma pt] [--duracao 60] [--canal ~/Projects/canais/<c>] [--install]
# motores: cutout-2d (padrão: animação de personagens/props) | estatico (yt-long com tema: imagens caprichadas + zoom leve)
#          | remotion-2d | ia-video | kenburns (só fotos, a pedido) | remotion-three | mapas-3d
set -euo pipefail
ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"  # readlink: funciona também via link em ~/.local/bin
[ $# -ge 1 ] || { sed -n 2,3p "$0"; exit 2; }
DEST="$1"; shift
FORMATOS="yt-short"; MOTOR="cutout-2d"; IDIOMA="pt"; DURACAO=""; INSTALL=0; CANAL=""
while [ $# -gt 0 ]; do
  case "$1" in
    --formato|--formatos) FORMATOS="$2"; shift 2;;
    --motor) MOTOR="$2"; shift 2;;
    --idioma) IDIOMA="$2"; shift 2;;
    --duracao) DURACAO="$2"; shift 2;;
    --install) INSTALL=1; shift;;
    --canal) CANAL="$(cd "$2" && pwd)"; shift 2;;
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
case "$MOTOR" in cutout-2d|estatico|blender-3d|remotion-2d|kenburns|ia-video|remotion-three|mapas-3d) ;; *) echo "motor inválido: $MOTOR" >&2; exit 2;; esac
[ "$MOTOR" = estatico ] && [ "$PRINCIPAL" != yt-long ] && { echo "motor estatico é só para yt-long (shorts são animados)" >&2; exit 2; }
case "$MOTOR" in blender-3d) echo "motor blender-3d ainda não implementado (especificação: skills/animacao/templates/mundo-3d-blender.md); use cutout-2d" >&2; exit 2;; esac
for f in "${FS[@]}"; do [ "$f" = yt-long ] && [ "$MOTOR" = blender-3d ] && { echo "blender-3d é só para yt-short/tiktok" >&2; exit 2; }; done
[ "$MOTOR" = kenburns ] && echo "aviso: kenburns é vídeo de fotos, sem animação; use só se o usuário pediu isso (a trava edl-check reprova)" >&2
DURACAO="${DURACAO:-$DUR_DEF}"
# renderer: mygo (GPU nativa) para motores 2D quando o binário existe; senão Remotion
# hyperframes (HTML + GSAP) para vídeo longo sem clipes `scene`: cartelas, acabamento de filme, tipografia web
RENDERER="remotion"
case "$MOTOR" in cutout-2d|remotion-2d|kenburns|ia-video) [ -x "$ROOT/bin/vf-mygo" ] && RENDERER="mygo";; esac
case "$MOTOR" in estatico) RENDERER="hyperframes";; kenburns|ia-video) [ "$PRINCIPAL" = yt-long ] && RENDERER="hyperframes";; esac
if [ -e "$DEST" ] && [ -n "$(ls -A "$DEST" 2>/dev/null)" ]; then echo "$DEST já existe e não está vazia" >&2; exit 1; fi
mkdir -p "$DEST"
NOME="$(basename "$DEST")"

cp -R "$ROOT/templates/projeto/." "$DEST/"
case "$MOTOR" in
  cutout-2d|remotion-2d|kenburns|ia-video)
    # todos renderizam o mesmo data/edl.json (mygo ou o template Remotion)
    cp -R "$ROOT/skills/animacao/templates/remotion-2d/." "$DEST/"
    mkdir -p "$DEST/sprites" "$DEST/public/sprites" "$DEST/public/img/plates"
    cp "$ROOT/skills/animacao/templates/sprites/"*.ts* "$ROOT/skills/animacao/templates/sprites/export-sprites.mjs" "$ROOT/skills/animacao/templates/sprites/README.md" "$DEST/sprites/"
    cp "$ROOT/skills/animacao/templates/cenas/vfscene.py" "$DEST/roteiro/"
    cp "$ROOT/skills/animacao/templates/cenas/cenas.exemplo.py" "$DEST/roteiro/cenas.py"
    ;;
  estatico)
    cp "$ROOT/skills/animacao/templates/hyperframes/package.json" "$DEST/package.json"
    mkdir -p "$DEST/public/img/planos" "$DEST/assets/flow" "$DEST/assets/ref"
    cp "$ROOT/skills/animacao/templates/cenas/vfscene.py" "$DEST/roteiro/"
    cp "$ROOT/skills/animacao/templates/cenas/estatico.py" "$DEST/roteiro/cenas.py"
    cp "$ROOT/skills/assets/templates/flow-estilo.json" "$DEST/assets/flow/estilo.json"
    ;;
  remotion-three)
    cp -R "$ROOT/skills/animacao/templates/remotion-three/." "$DEST/"
    mkdir -p "$DEST/scripts" "$DEST/public/clips" "$DEST/public/bridge/kenney"
    cp "$ROOT/skills/edicao/templates/cartoon/cartoon-stills.mjs" "$ROOT/skills/edicao/templates/cartoon/cartoon-plan.mjs" "$ROOT/skills/edicao/templates/cartoon/sbs_sheet.py" "$DEST/scripts/"
    cp "$ROOT/skills/edicao/templates/cartoon/LICENSE" "$DEST/scripts/LICENSE-cartoon"
    ;;
  mapas-3d)
    echo "motor mapas-3d: siga a skill mapas-3d-videos-virais (kit-mapas-3d)" ;;
esac
if [ "$MOTOR" = estatico ]; then
  cp "$ROOT/skills/assets/templates/style-bible-tema.md" "$DEST/assets/style-bible.md"
else
  cp "$ROOT/skills/assets/templates/style-bible.md" "$DEST/assets/style-bible.md"
fi

# canal: a style bible e o kit do canal valem para todo vídeo dele
if [ -n "$CANAL" ]; then
  [ -f "$CANAL/canal.yaml" ] || { echo "$CANAL/canal.yaml não existe (skill canal: novo)" >&2; exit 1; }
  [ -f "$CANAL/style-bible.md" ] && cp "$CANAL/style-bible.md" "$DEST/assets/style-bible.md"
  [ -d "$CANAL/kit/sprites" ] && mkdir -p "$DEST/public/sprites" && cp -R "$CANAL/kit/sprites/." "$DEST/public/sprites/"
fi

LIST="$(printf '%s' "$FORMATOS" | sed 's/,/, /g')"
find "$DEST" -type f \( -name '*.yaml' -o -name '*.md' -o -name '*.json' \) -not -path '*/node_modules/*' -print0 |
  xargs -0 sed -i '' -e "s|__NOME__|$NOME|g" -e "s|__DATA__|$(date +%F)|g" -e "s|__FORMATOS__|$LIST|g" \
    -e "s|__PRINCIPAL__|$PRINCIPAL|g" -e "s|__MOTOR__|$MOTOR|g" -e "s|__IDIOMA__|$IDIOMA|g" \
    -e "s|__DURACAO__|$DURACAO|g" -e "s|__WPM__|$WPM|g" -e "s|__RENDERER__|$RENDERER|g" -e "s|__CANAL__|$CANAL|g"

(cd "$DEST" && git init -q 2>/dev/null || true)
if [ "$INSTALL" = 1 ] && [ -f "$DEST/package.json" ]; then (cd "$DEST" && bun install --silent); fi
echo "projeto criado: $DEST  (formatos: $LIST · motor: $MOTOR · duração alvo: ${DURACAO}s · ${WPM} wpm · renderer: $RENDERER)"
