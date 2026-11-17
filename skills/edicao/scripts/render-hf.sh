#!/usr/bin/env bash
# Render HyperFrames de um projeto da fábrica: edl.json -> hf/index.html -> lint -> MP4 mudo.
# uso: render-hf.sh <formato> <saida.mp4> [--draft] [--scale 1.3333] [--from s --to s] [--workers N]
# O som entra depois no `vf export --audio public/audio/mix.wav` (igual ao mygo).
set -euo pipefail
ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../../.." && pwd)"
[ $# -ge 2 ] || { sed -n 3,4p "$0"; exit 2; }
FMT="$1"; OUT="$2"; shift 2
Q=high; SCALE=1; EXTRA=()
while [ $# -gt 0 ]; do
  case "$1" in
    --draft) Q=draft; shift;;
    --scale) SCALE="$2"; shift 2;;
    --from) EXTRA+=(--start "$2"); shift 2;;
    --to) EXTRA+=(--end "$2"); shift 2;;
    --workers) EXTRA+=(--workers "$2"); shift 2;;
    *) echo "opção desconhecida: $1" >&2; exit 2;;
  esac
done
export HYPERFRAMES_NO_TELEMETRY=1 DO_NOT_TRACK=1
bun "$ROOT/skills/animacao/scripts/edl2hf.mjs" data/edl.json --formato "$FMT" --scale "$SCALE"
DUR=$(bun -e 'console.log(JSON.parse(require("fs").readFileSync("data/edl.json","utf8")).duration)')
# vídeos longos: captura em segmentos, retomável com --resume se cair no meio
if bun -e "process.exit(Number($DUR) > 120 ? 0 : 1)"; then export HF_SEGMENTED_CAPTURE=true; EXTRA+=(--resume); fi
bun run --bun hyperframes lint hf >/dev/null || { bun run --bun hyperframes lint hf; exit 1; }
mkdir -p "$(dirname "$OUT")"
T0=$(date +%s)
bun run --bun hyperframes render hf -o "$OUT" --quality "$Q" --quiet ${EXTRA[@]+"${EXTRA[@]}"}
echo "hyperframes: $OUT (${DUR}s de vídeo em $(( $(date +%s) - T0 ))s)"
