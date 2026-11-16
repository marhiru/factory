#!/usr/bin/env bash
# Teste de integração: vídeo sintético com cortes em frames conhecidos (60, 150, 240 a 30 fps).
set -euo pipefail
VF="${VF:-$(cd "$(dirname "$0")/.." && pwd)/bin/vf}"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "testsrc2=s=1280x720:r=30:d=2" \
  -f lavfi -i "mandelbrot=s=1280x720:r=30,trim=duration=3" \
  -f lavfi -i "smptehdbars=s=1280x720:r=30:d=3" \
  -f lavfi -i "life=s=1280x720:r=30:mold=10:ratio=0.1:death_color=#C83232:life_color=#00ff00,trim=duration=2" \
  -f lavfi -i "sine=f=440:d=10:r=48000" \
  -filter_complex "[0:v]format=yuv420p,setpts=PTS-STARTPTS[a];[1:v]format=yuv420p,setpts=PTS-STARTPTS[b];[2:v]format=yuv420p,setpts=PTS-STARTPTS[c];[3:v]format=yuv420p,setpts=PTS-STARTPTS[d];[a][b][c][d]concat=n=4:v=1:a=0[v]" \
  -map "[v]" -map 4:a -c:v libx264 -g 250 -crf 20 -c:a aac "$T/synth.mp4"
for J in 1 8; do
  "$VF" frames "$T/synth.mp4" -o "$T/f$J" -j $J 2>&1 | sed "s/^/[jobs=$J] /"
done
python3 - "$T" <<'PY'
import json, sys
t = sys.argv[1]
a = [s["a"] for s in json.load(open(f"{t}/f1/shots.json"))["shots"]]
b = [s["a"] for s in json.load(open(f"{t}/f8/shots.json"))["shots"]]
fa = [json.loads(l) for l in open(f"{t}/f1/frames.jsonl")]
fb = [json.loads(l) for l in open(f"{t}/f8/frames.jsonl")]
print("cortes 1 job :", a)
print("cortes 8 jobs:", b)
assert a == [0, 60, 150, 240], a
assert a == b, "segmentação paralela mudou os cortes"
assert len(fa) == len(fb) == 300, (len(fa), len(fb))
d = max(abs(x["motion"] - y["motion"]) for x, y in zip(fa, fb))
assert d < 0.02, f"motion diverge entre 1 e 8 jobs: {d}"
print("OK: cortes exatos e paralelo == serial (max dif. motion %.4f)" % d)
PY
"$VF" qa "$T/synth.mp4" -f yt-long -o "$T/qa" >/dev/null && echo "OK: qa gerou $(ls "$T/qa/sheets" | wc -l | tr -d ' ') sheets"
"$VF" export "$T/synth.mp4" -f yt-short -o "$T/short.mp4" --fit blur --fast && "$VF" probe "$T/short.mp4" | grep -E '"(width|height)"'
