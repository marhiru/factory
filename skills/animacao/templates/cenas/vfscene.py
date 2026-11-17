"""vfscene: escreve data/edl.json a partir de um script Python (cenas cutout 2D, stills com movimento,
cartelas e overlays), com todo tempo amarrado a palavras da narração (data/words.json).

Por que script e não JSON à mão: quando a voz muda, `python3 roteiro/cenas.py` refaz tudo.

    import sys; sys.path.insert(0, "roteiro")      # vfscene.py copiado pelo novo.sh para roteiro/
    from vfscene import *
    init()                                          # lê data/words.json (+ roteiro/beats.json)
    scene(0.0, w(12), [A("heroi", "heroi", 0.5, 0.86, 0.5, "neutro", keys=[st(w(5), "susto")])], cam=push(1, 1.06))
    shot("img/b01-01.jpg", w(12), w(20), cam=zoom(1.0, 1.07))
    save()

Tempos: `T` (absoluto, segundos do vídeo) em keys/enter/exit é convertido para `t`/`at` relativos ao clipe.
`w(i)` = início da palavra i menos LEAD (2 frames): o visual reage um pouco antes da fala.
"""
import json, os, sys

ROOT = os.getcwd()
FPS = 30
LEAD = 2 / FPS
W, BEATS, CLIPS, OVERLAYS, AUDIO = [], [], [], [], []
META = {}


def init(root=None, fps=30, words="data/words.json", beats="roteiro/beats.json"):
    # W/BEATS são atualizados no lugar: `from vfscene import *` continua vendo as mesmas listas
    global ROOT, FPS, LEAD
    ROOT = os.path.abspath(root or os.getcwd())
    FPS, LEAD = fps, 2 / fps
    W[:] = json.load(open(os.path.join(ROOT, words)))
    bp = os.path.join(ROOT, beats)
    BEATS[:] = json.load(open(bp)).get("beats", []) if os.path.exists(bp) else []


def w(i, lead=None):
    """momento da reação à palavra i (índice em words.json)"""
    return round(W[i]["start"] - (LEAD if lead is None else lead), 3)


def wend(i):
    return round(W[i]["end"], 3)


def find(text, after=0.0):
    """índice da primeira palavra (normalizada) igual a `text` a partir do tempo `after`"""
    norm = lambda s: "".join(ch for ch in s.lower() if ch.isalnum())
    t = norm(text)
    for k, x in enumerate(W):
        if x["start"] >= after and norm(x["word"]) == t:
            return k
    raise KeyError(f"palavra '{text}' não encontrada depois de {after}s")


def beat(bid):
    for b in BEATS:
        if str(b.get("id")) == str(bid):
            return b
    raise KeyError(f"beat {bid}")


# ---------- câmera / movimento ----------
def push(s0, s1, x0=0.0, x1=0.0, y0=0.0, y1=0.0, ease="linear"):
    return {"from": {"scale": s0, "x": x0, "y": y0}, "to": {"scale": s1, "x": x1, "y": y1}, "ease": ease}


def zoom(s0=1.0, s1=1.07, ease="linear", x=0.0, y=0.0):
    """zoom-in leve centrado (x/y deslocam o ponto de fuga)"""
    return {"from": {"scale": s0, "x": 0.0, "y": 0.0}, "to": {"scale": s1, "x": x, "y": y}, "ease": ease}


def pan(dx=0.04, scale=None, dy=0.0, ease="linear"):
    """pan lateral com escala suficiente para não mostrar borda: scale >= 1 + 2|x|"""
    s = scale or round(1 + 2 * max(abs(dx), abs(dy)) + 0.02, 3)
    return {"from": {"scale": s, "x": dx, "y": dy}, "to": {"scale": s, "x": -dx, "y": -dy}, "ease": ease}


def junta(s_a, foco_a, s_b, foco_b, recuo=0.12, via_at=0.5, ease="inOut"):
    """junção de trechos contínuos no mesmo cenário (QA 5): zoom-out do enquadramento A e zoom-in no assunto B,
    sem corte. foco_* = (x, y) do deslocamento da câmera; recuo = quanto a câmera abre no meio (escala)."""
    mid = max(1.0, min(s_a, s_b) - recuo)
    vx = round((foco_a[0] + foco_b[0]) / 2 * 0.4, 4)
    vy = round((foco_a[1] + foco_b[1]) / 2 * 0.4, 4)
    return {"from": {"scale": s_a, "x": foco_a[0], "y": foco_a[1]}, "via": {"scale": mid, "x": vx, "y": vy},
            "to": {"scale": s_b, "x": foco_b[0], "y": foco_b[1]}, "via_at": via_at, "ease": ease}


XF = {"kind": "fade", "dur": 0.25}  # dissolve medido nas referências: 0,2-0,3 s, com a câmera seguindo
FLASH = {"kind": "flash", "dur": 0.2}


def _rel(d, a):
    d = dict(d)
    for k in d.get("keys", []):
        if "T" in k:
            k["t"] = round(k.pop("T") - a, 3)
    for mv in ("enter", "exit"):
        if mv in d and "T" in d[mv]:
            d[mv] = dict(d[mv])
            d[mv]["at"] = round(d[mv].pop("T") - a, 3)
    return d


# ---------- clipes ----------
def scene(a, b, actors, bg, cam=None, trans=None, id=None, **kw):
    """cena cutout: plate `bg` (caminho em public/ ou {"color": ...}) + atores"""
    a, b = round(a, 3), round(b, 3)
    bgd = bg if isinstance(bg, dict) else {"src": bg}
    clip = {"id": id or f"c{len(CLIPS) + 1:03d}", "type": "scene", "start": a, "end": b,
            "scene": {"bg": bgd, "camera": cam or push(1.0, 1.06), "actors": [_rel(x, a) for x in actors]}, **kw}
    if trans:
        clip["transition_in"] = trans
    CLIPS.append(clip)
    return clip


def shot(src, a, b, cam=None, trans=None, id=None, **kw):
    """still com movimento (motor estático / respiro): zoom-in leve por padrão"""
    a, b = round(a, 3), round(b, 3)
    clip = {"id": id or f"i{len(CLIPS) + 1:03d}", "type": "image", "src": src, "start": a, "end": b, "fit": "cover",
            "motion": cam or zoom(1.0, 1.0 + min(0.10, max(0.05, 0.012 * (b - a))))}
    if trans:
        clip["transition_in"] = trans
    clip.update(kw)
    CLIPS.append(clip)
    return clip


def text(t, a, b, style="title", layer=2, **kw):
    clip = {"id": kw.pop("id", f"t{len(CLIPS) + 1:03d}"), "type": "text", "style": style, "text": t, "start": round(a, 3), "end": round(b, 3), "layer": layer, **kw}
    CLIPS.append(clip)
    return clip


def card(kind, a, b, data, layer=1, **kw):
    """interrupção de padrão (renderers HTML): kind = dado | citacao | mapa | comparacao | capitulo"""
    clip = {"id": kw.pop("id", f"k{len(CLIPS) + 1:03d}"), "type": "card", "kind": kind, "start": round(a, 3), "end": round(b, 3), "layer": layer, "data": data, **kw}
    CLIPS.append(clip)
    return clip


def A(id, sprite, x, y, h, state=None, **kw):
    """ator (personagem): pés em (x, y), altura h (fração do quadro)"""
    d = {"id": id, "sprite": f"sprites/{sprite}", "x": x, "y": y, "h": h, "shadow": True, "depth": 0.15}
    if state:
        d["state"] = state
    d.update(kw)
    return d


def fx(id, sprite, x, y, h, **kw):
    """efeito/prop: flutua, sem sombra nem piscada"""
    d = {"id": id, "sprite": f"sprites/{sprite}", "x": x, "y": y, "h": h, "idle": "float", "depth": 0.15, "blink": False}
    d.update(kw)
    return d


def st(T, state, **kw):
    return {"T": T, "state": state, **kw}


def sfx(src, T, gain_db=-8):
    AUDIO.append({"src": src, "role": "sfx", "start": round(T, 3), "gain_db": gain_db})


def load_parts(folder="roteiro/cenas"):
    """esteira: cada sub-agente escreve só roteiro/cenas/sNN.py (um por cartão de plano). Este carrega todos em
    ordem, no mesmo namespace (scene/A/fx/st/w/... já disponíveis dentro de cada parte)."""
    import glob, runpy
    g = {k: v for k, v in globals().items() if not k.startswith("__")}
    parts = sorted(glob.glob(os.path.join(ROOT, folder, "s*.py")))
    for p in parts:
        runpy.run_path(p, init_globals=g)
    return len(parts)


def save(path="data/edl.json", tail=0.4, **meta):
    """mescla com o edl existente (mantém audio/font/captions) e grava"""
    p = os.path.join(ROOT, path)
    edl = json.load(open(p)) if os.path.exists(p) else {"fps": FPS, "background": "#000000", "audio": [{"src": "voice/voice.wav", "role": "voice"}]}
    edl["fps"] = FPS
    edl["clips"] = sorted(CLIPS, key=lambda c: (c.get("layer", 0), c["start"]))
    end = max(c["end"] for c in CLIPS)
    edl["duration"] = round(max(end, (W[-1]["end"] + tail) if W else 0), 3)
    # o último plano da base cobre a cauda (senão sobra tela preta depois da última palavra)
    base = [c for c in CLIPS if c.get("layer", 0) == 0]
    if base:
        last = max(base, key=lambda c: c["end"])
        last["end"] = max(last["end"], edl["duration"])
    if AUDIO:
        edl["audio"] = [x for x in edl.get("audio", []) if x.get("role") != "sfx"] + AUDIO
    if OVERLAYS:
        edl["overlays"] = OVERLAYS
    edl.update(meta)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    json.dump(edl, open(p, "w"), indent=1, ensure_ascii=False)
    n_act = sum(len(c["scene"]["actors"]) for c in CLIPS if c["type"] == "scene")
    kinds = {}
    for c in CLIPS:
        kinds[c["type"]] = kinds.get(c["type"], 0) + 1
    print(f"{len(CLIPS)} clipes {kinds}, {n_act} atores, {edl['duration']} s -> {path}", file=sys.stderr)
    return edl
