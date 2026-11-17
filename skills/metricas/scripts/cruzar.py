#!/usr/bin/env python3
"""Cruza as quedas de retenção do `vf analytics` com o roteiro do projeto (beats.json alinhado e words.json).

uso: python3 -I cruzar.py <out/analytics/yt-<conta>-<data>.json> <ID do vídeo> [roteiro/beats.json] [data/words.json]

Imprime, para cada queda (e para os marcos 0-3 s, 10 %, 25 %, 50 %), o beat e as palavras ditas naquele
instante. Sem beats/words, imprime só os tempos. Saída em Markdown, pronta para o relatório.
"""
import json
import sys


def load(p):
    try:
        with open(p, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(2)
    an = load(sys.argv[1]) or {}
    vid = sys.argv[2]
    beats = (load(sys.argv[3]) if len(sys.argv) > 3 else load("roteiro/beats.json")) or {}
    words = (load(sys.argv[4]) if len(sys.argv) > 4 else load("data/words.json")) or []
    beats = beats.get("beats", []) if isinstance(beats, dict) else beats

    v = next((x for x in an.get("videos", []) if x.get("id") == vid), None)
    if not v:
        sys.exit(f"vídeo {vid} não está em {sys.argv[1]} (rode vf analytics platform=yt video={vid})")
    ret = v.get("retencao") or {}
    curva = ret.get("curva") or []
    if not curva:
        sys.exit(f"{vid} sem curva de retenção (vídeo sem views no período, ou Analytics ainda sem dados: espere 48-72 h)")

    def beat_at(s):
        for b in beats:
            if b.get("start") is not None and b.get("end") is not None and b["start"] <= s <= b["end"] + 0.25:
                return b
        return None

    def fala(s, janela=2.0):
        ws = [w["word"] for w in words if s - janela <= w.get("start", -1) <= s + janela]
        return " ".join(ws)

    def ponto(s):
        b = beat_at(s)
        tb = f"beat {b.get('id')} ({b.get('tipo', '')}): {(b.get('texto') or b.get('text') or '')[:90]}" if b else "-"
        return tb, fala(s)

    dur = v.get("duracao_s") or 0
    print(f"## Retenção × roteiro — {v.get('titulo', vid)} ({vid}, {dur:.0f} s, {v.get('tipo')})\n")
    m = ret.get("marcos", {})
    print("Marcos (fração da audiência ainda assistindo; >1 = replays): " + " · ".join(f"{k} {val:.0%}" if isinstance(val, (int, float)) else f"{k} -" for k, val in m.items()) + "\n")
    print("| quando | assistindo | relativa | beat | fala (±2 s) |")
    print("|---|---|---|---|---|")
    alvo = [0.0, 1.0, 3.0] + [dur * f for f in (0.1, 0.25, 0.5)]
    alvo += [q["s"] for q in ret.get("maiores_quedas", [])]
    vistos = set()
    for s in sorted(alvo):
        p = min(curva, key=lambda r: abs(r["s"] - s))
        if p["s"] in vistos:
            continue
        vistos.add(p["s"])
        queda = next((q for q in ret.get("maiores_quedas", []) if abs(q["s"] - p["s"]) < 0.01), None)
        tb, fl = ponto(p["s"])
        marca = f" **queda {queda['queda']:.0%}**" if queda else ""
        print(f"| {p['s']:.1f} s{marca} | {p['assistindo']:.0%} | {p['relativa']:.2f} | {tb} | {fl} |")
    print("\n`relativa`: 0,5 = igual a vídeos de mesma duração no YouTube; acima é melhor que a média, abaixo pior.")


if __name__ == "__main__":
    main()
