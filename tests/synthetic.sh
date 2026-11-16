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
"$VF" qa "$T/synth.mp4" -f yt-long -o "$T/qa" --lenient >/dev/null || true
echo "OK: qa gerou $(ls "$T/qa/sheets" | wc -l | tr -d ' ') sheets"

# ---- QA estrito: um vídeo bom passa; cada defeito reprova na checagem certa ----
cat > "$T/fmt.yaml" <<'YAML'
teste:
  platform: teste
  label: "teste"
  aspect: "16:9"
  width: 1280
  height: 720
  fps: [30]
  duration: { min: 1, max: 600, target: 8 }
  wpm: 160
  lufs: -14
  true_peak: -1.0
  lufs_tolerance: 40
  video: { codec: h264, crf: 20, preset: fast, pix_fmt: yuv420p }
  audio: { codec: aac, bitrate: 192k, sample_rate: 48000 }
  safe_zone: { top: 0, bottom: 0, left: 0, right: 0 }
  qa: { freeze_frames: 8, duration_strict: [4, 15], min_life: 0.004, max_shot_s: 9, max_bitrate_mbps: 20,
        audio_start_s: 0.3, hook_s: 2, max_silence_s: 1.0, max_voice_gap_s: 1.5, repeat_window_s: 0 }
YAML
mk() { # mk <nome> <filtro de vídeo lavfi> <filtro de áudio lavfi> <dur>  (com um corte em 1 s: gancho visual)
  ffmpeg -hide_banner -loglevel error -y -f lavfi -i "$2" -f lavfi -i "$3" \
    -filter_complex "[0:v]split[x][y];[x]trim=end=1,hue=h=180,setpts=PTS-STARTPTS[p];[y]trim=start=1,setpts=PTS-STARTPTS[q];[p][q]concat=n=2:v=1[v]" \
    -t "$4" -map "[v]" -map 1:a -c:v libx264 -crf 20 -pix_fmt yuv420p -c:a aac -ar 48000 "$T/$1.mp4"
}
TONE="sine=f=330:r=48000,volume=0.3"
mk bom "testsrc2=s=1280x720:r=30" "$TONE" 8
ffmpeg -hide_banner -loglevel error -y -f lavfi -i "testsrc2=s=1280x720:r=30:d=2.5" -f lavfi -i "testsrc2=s=1280x720:r=30:d=3" \
  -filter_complex "[1:v]trim=end_frame=1,loop=loop=89:size=1,setpts=N/30/TB[f];[0:v][f]concat=n=2:v=1[v0];[v0]fps=30[v]" -map "[v]" \
  -c:v libx264 -crf 20 -pix_fmt yuv420p "$T/cg.mp4"
ffmpeg -hide_banner -loglevel error -y -i "$T/bom.mp4" -i "$T/cg.mp4" -filter_complex "[0:v]trim=end=2,setpts=PTS-STARTPTS[a];[a][1:v]concat=n=2:v=1[v]" -map "[v]" -map 0:a -shortest \
  -c:v libx264 -crf 20 -pix_fmt yuv420p -c:a aac "$T/congelado.mp4"
mk silencio "testsrc2=s=1280x720:r=30" "$TONE,volume='if(between(t,3,5.5),0,1)':eval=frame" 8
mk barras "testsrc2=s=1280x720:r=30,scale=1280:540,pad=1280:720:0:90:black" "$TONE" 8
mk longo "testsrc2=s=1280x720:r=30" "$TONE" 20
for v in bom congelado silencio barras longo; do
  VF_FORMATS="$T/fmt.yaml" "$VF" qa "$T/$v.mp4" -f teste -o "$T/qa_$v" >/dev/null 2>&1 && echo "$v: 0" > "$T/rc_$v" || echo "$v: 1" > "$T/rc_$v"
done
python3 - "$T" <<'PY'
import json, sys
t = sys.argv[1]
def fails(v): return {c["name"] for c in json.load(open(f"{t}/qa_{v}/qa.json"))["checks"] if c["level"] == "FAIL"}
rc = lambda v: open(f"{t}/rc_{v}").read().split(": ")[1].strip()
assert rc("bom") == "0", ("vídeo bom reprovou", fails("bom"))
for v, need in [("congelado", "congelados"), ("silencio", "ar morto"), ("barras", "barras pretas"), ("longo", "duração × alvo")]:
    f = fails(v)
    assert rc(v) == "1" and need in f, (v, f)
    print(f"OK: {v} reprovado em '{need}'" + (f" (+ {sorted(f - {need})})" if f - {need} else ""))
print("OK: vídeo bom aprovado no QA estrito")
PY
"$VF" export "$T/synth.mp4" -f yt-short -o "$T/short.mp4" --fit blur --fast && "$VF" probe "$T/short.mp4" | grep -E '"(width|height)"'
