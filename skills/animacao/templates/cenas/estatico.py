#!/usr/bin/env python3
"""Motor ESTÁTICO (yt-long com tema): roteiro/beats.json (com tempos do `vf align`) + imagens curadas
(public/img/planos/<plano>.jpg, plano = b<beat>-<n>) -> data/edl.json pronto para o HyperFrames.

O que ele aplica (padrão da fábrica, ajuste nas constantes):
- cortes SEMPRE em início de palavra (2 frames antes), dividindo o beat pelos seus planos em partes de fala iguais;
- zoom-in leve em todo still (1,0 -> 1,05..1,10 conforme a duração), pan lento em ~1 de cada 4 planos;
- card de capítulo nos beats `capitulo`; cards de interrupção (`beat.card`) no início do beat;
- `punch` (zoom de 4% em 0,15 s) em `revelacao`/`virada`; fade/blur só entre capítulos;
- acabamento: grão, vinheta, graduação e light leak nos capítulos; whoosh nas viradas (se existir o SFX).
uso: python3 roteiro/cenas.py
"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from vfscene import *

init()
B = json.load(open("roteiro/beats.json"))
beats = B["beats"]
assert all("w0" in b for b in beats), "rode antes: vf align roteiro/beats.json data/words.json"

CAP_DUR = 2.6          # card de capítulo (s)
CARD_DUR = 3.4         # card de interrupção (s)
GRADE = B.get("grade", {"kind": "grade", "contrast": 1.06, "saturate": 0.92, "sepia": 0.05})
SFX = {"virada": "audio/sfx/whoosh.wav", "revelacao": "audio/sfx/riser.wav", "capitulo": "audio/sfx/whoosh.wav"}

def img(pid):
    return f"img/planos/{pid}.jpg"

def motion_for(k, d):
    """zoom-in dominante; a cada 4º plano, pan lento. Velocidade ~1,2%/s (perceptível, sem enjoar)."""
    z = round(min(0.10, max(0.012, 0.013 * d)), 3)  # ~1,3%/s: plano de 6 s = 1,08; plano curto anda menos, mesma velocidade
    if k % 4 == 3:
        dx = round(min(0.035, 0.006 * d), 3)
        return pan(dx if (k // 4) % 2 == 0 else -dx)
    return zoom(1.0, 1.0 + z, x=0.0, y=0.0)

caps = 0
k = 0
leaks = []
for i, b in enumerate(beats):
    a = 0.0 if i == 0 else w(b["w0"])
    end = w(beats[i + 1]["w0"]) if i + 1 < len(beats) else W[-1]["end"] + 0.4
    t = a
    trans = None
    if b.get("tipo") == "capitulo":
        caps += 1
        card("capitulo", t, t + CAP_DUR, {"numero": b.get("numero", f"Capítulo {caps}"), "titulo": b.get("titulo", ""), "bg": img(f"b{b['id']}-1")}, layer=0,
             transition_in={"kind": "blur", "dur": 0.5})
        leaks.append(t)
        t += CAP_DUR
        trans = XF
    elif b.get("card"):
        c = dict(b["card"]); kind = c.pop("kind")
        if kind in ("dado", "citacao") and "bg" not in c:
            c["bg"] = img(f"b{b['id']}-1")  # fundo desfocado do próprio beat: card nunca fica chapado/parado
        card(kind, t, t + CARD_DUR, c, layer=0)
        t += CARD_DUR
    vis = [v.strip() for v in str(b.get("visual", "")).split(";") if v.strip()]
    n = len(vis)
    if n == 0:
        continue
    # pontos de corte: palavras do beat divididas em n partes iguais; só cortes depois de t
    ws = list(range(b["w0"], b["w1"] + 1))
    cuts = [t] + [max(t, w(ws[min(len(ws) - 1, round(j * len(ws) / n))])) for j in range(1, n)] + [end]
    for j in range(n):
        s, e = cuts[j], cuts[j + 1]
        if e - s < 0.5:
            continue
        pid = f"b{b['id']}-{j + 1}"
        extra = {}
        if j == 0 and b.get("tipo") in ("revelacao", "virada") and s > 0:
            extra["transition_in"] = {"kind": "punch", "dur": 0.18}
        elif j == 0 and trans:
            extra["transition_in"] = trans
        shot(img(pid), s, e, cam=motion_for(k, e - s), id=pid, **extra)
        k += 1
    if b.get("tipo") in SFX and os.path.exists(os.path.join("public", SFX[b["tipo"]])):
        sfx(SFX[b["tipo"]], max(0.0, a - 0.25), gain_db=-12)

OVERLAYS.extend([{"kind": "grain", "opacity": 0.07}, {"kind": "vignette", "strength": 0.42}, GRADE, {"kind": "leak", "at": leaks, "opacity": 0.3}])
save(perfil="estatico", captions={"enabled": False})
