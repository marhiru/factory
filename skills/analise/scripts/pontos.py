#!/usr/bin/env python3
"""Mede uma referência (saída do `vf analyze`) pelos pontos de qualidade da fábrica.

uso: python3 -I pontos.py ref/<slug> [ref/<slug2> ...]  ->  imprime e grava ref/<slug>/pontos.json

Por vídeo:
  transições   dissolves x cortes secos, duração dos dissolves (frames e s)       -> câmera (QA 5)
  vida         % de frames parados e "vida" mediana (diff em 0,5 s)               -> câmera/animação
  luz          temperatura (R-B do quadro médio) e luma por plano, arco quente/frio -> luz (QA 6)
  legenda      faixa vertical onde as legendas aparecem (para a regra de posicionamento, QA 3)
Precisa de ffmpeg. Não depende de nada além da biblioteca padrão.
"""
import json
import statistics as st
import subprocess
import sys
from pathlib import Path


def rgb_at(src: Path, t: float, vf: str = "scale=1:1:flags=area"):
    raw = subprocess.run(["ffmpeg", "-v", "error", "-ss", f"{t:.3f}", "-i", str(src), "-frames:v", "1", "-vf", vf,
                          "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], capture_output=True).stdout
    return raw


def caption_band(src: Path, dur: float, w: int = 180, h: int = 320):
    """linhas com texto branco puro em 16 quadros: onde mora a legenda (fração da altura, de cima)"""
    hits = [0] * h
    n = 16
    for k in range(n):
        raw = rgb_at(src, dur * (k + 0.5) / n, f"scale={w}:{h}:flags=bilinear")
        if len(raw) < w * h * 3:
            continue
        for y in range(h):
            row = raw[y * w * 3:(y + 1) * w * 3]
            white = sum(1 for x in range(w) if min(row[x * 3:x * 3 + 3]) > 225)
            if 4 <= white <= w * 0.35:  # texto, não céu/parede branca inteira
                hits[y] += 1
    ys = [y for y, c in enumerate(hits) if c >= n * 0.4]
    return (round(min(ys) / h, 2), round((max(ys) + 1) / h, 2)) if ys else None


def measure(d: Path):
    shots = json.loads((d / "shots.json").read_text())
    frames = [json.loads(l) for l in (d / "frames.jsonl").read_text().splitlines() if l.strip()]
    src = next(d.glob("source.*"))
    fps = shots["fps"]
    xf = shots.get("xfade", [])
    n_tr = max(0, len(shots["shots"]) - 1)
    # um dissolve que cobre a fronteira entre planos conta como transição suave
    bounds = [s["a"] for s in shots["shots"][1:]]
    soft = sum(1 for b in bounds if any(a - 2 <= b <= c + 2 for a, c in xf))
    slow = [f["slow"] for f in frames]
    per = []
    for s in shots["shots"]:
        raw = rgb_at(src, (s["ta"] + s["tb"]) / 2)
        r, g, b = (raw[0], raw[1], raw[2]) if len(raw) >= 3 else (0, 0, 0)
        per.append({"id": s["id"], "t": round(s["ta"], 2), "dur": round(s["dur"], 2), "temp": r - b, "luma": round(s["luma"], 3)})
    temps = [p["temp"] for p in per]
    out = {
        "video": d.name,
        "transicoes": {"total": n_tr, "suaves": soft, "secas": n_tr - soft,
                       "dissolve_frames_mediana": st.median([c - a for a, c in xf]) if xf else 0,
                       "dissolve_s_mediana": round(st.median([c - a for a, c in xf]) / fps, 3) if xf else 0},
        "vida": {"parados_pct": round(100 * sum(1 for v in slow if v < 0.01) / len(slow), 2), "mediana": round(st.median(slow), 3)},
        "luz": {"quentes": sum(1 for t in temps if t > 20), "frios": sum(1 for t in temps if t < -20),
                "luma_min": min(p["luma"] for p in per), "luma_max": max(p["luma"] for p in per), "planos": per},
        "legenda_faixa": caption_band(src, shots["duration"]),
    }
    (d / "pontos.json").write_text(json.dumps(out, indent=1, ensure_ascii=False))
    return out


def arc(per):
    sym = lambda t: "Q" if t > 40 else "q" if t > 20 else "F" if t < -40 else "f" if t < -20 else "·"
    return "".join(sym(p["temp"]) for p in per)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    for a in sys.argv[1:]:
        o = measure(Path(a))
        t = o["transicoes"]
        print(f"== {o['video']}")
        print(f"  transições: {t['suaves']}/{t['total']} suaves ({t['secas']} secas) · dissolve mediano {t['dissolve_frames_mediana']} f ({t['dissolve_s_mediana']} s)")
        print(f"  vida: {o['vida']['parados_pct']}% parados · mediana {o['vida']['mediana']}")
        print(f"  luz: arco {arc(o['luz']['planos'])}  (Q/q quente, F/f frio) · luma {o['luz']['luma_min']}-{o['luz']['luma_max']}")
        print(f"  legenda: faixa vertical {o['legenda_faixa']}")
