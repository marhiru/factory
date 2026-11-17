# /// script
# requires-python = ">=3.11,<3.12"
# dependencies = ["chatterbox-tts==0.1.7", "huggingface_hub", "setuptools<81"]  # setuptools: a marca d'água perth usa pkg_resources
# ///
"""Narração local com Chatterbox (Resemble AI, MIT), sem API e sem custo por caractere.

uso (o uv cria o ambiente isolado na primeira vez; os modelos vão para ~/.cache/huggingface):
  uv run --script voz-chatterbox.py voice/texto.txt -o public/voice/voice.wav [--ref voz/ref.wav]
         [--modelo pt-br|multilingual|en] [--lang pt] [--exaggeration 0.5] [--cfg 0.5] [--temperature 0.8]
         [--seed 42] [--tempo 1.0] [--pausa 0.25] [--pausa-paragrafo 0.6] [--refazer 3,7] [--device auto|mps|cpu]
  (ou pelo terminal: vf voz voice/texto.txt -o public/voice/voice.wav ...)

Texto: uma frase por linha; linha em branco = pausa de parágrafo. Frases longas são quebradas em pedaços de
até ~250 caracteres (o modelo gera no máximo ~40 s por chamada). Cada pedaço fica em cache
(voice/chatterbox/NNN-<hash>.wav): editar uma linha regera só aquele pedaço. `--refazer 3,7` regera os
pedaços 3 e 7 com outra semente (tomada nova) sem mexer no resto.

Saída: WAV 48 kHz mono, silêncio inicial cortado, e voice/chatterbox/pedacos.json com o início/fim de cada
pedaço (ajuda a achar a frase errada para refazer). Todo áudio do Chatterbox leva a marca d'água
imperceptível PerTh da Resemble AI.
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

BASE_REPO = "ResembleAI/chatterbox"
PTBR_REPO = "ResembleAI/Chatterbox-Multilingual-pt-br"


def pedacos(texto: str, limite: int = 250):
    """[(texto, pausa_depois: 'frase'|'paragrafo')]"""
    out = []
    blocos = re.split(r"\n\s*\n", texto.strip())
    for bi, bloco in enumerate(blocos):
        linhas = [l.strip() for l in bloco.splitlines() if l.strip() and not l.strip().startswith("#")]
        frases = []
        for l in linhas:
            if len(l) <= limite:
                frases.append(l)
                continue
            # quebra por frase; se ainda passar, por vírgula/ponto e vírgula; por fim por espaço
            for f in re.split(r"(?<=[.!?…])\s+", l):
                while len(f) > limite:
                    corte = max(f.rfind(", ", 0, limite), f.rfind("; ", 0, limite), f.rfind(": ", 0, limite))
                    if corte < limite // 3:
                        corte = f.rfind(" ", 0, limite)
                    if corte <= 0:
                        corte = limite
                    frases.append(f[: corte + 1].strip())
                    f = f[corte + 1 :].strip()
                if f:
                    frases.append(f)
        for fi, f in enumerate(frases):
            fim_bloco = fi == len(frases) - 1 and bi < len(blocos) - 1
            out.append((f, "paragrafo" if fim_bloco else "frase"))
    return out


def ckpt_ptbr():
    """monta a pasta do ajuste fino pt-BR como a demo oficial: base (ve, tokenizer, conds) + T3 pt-br + S3Gen v3"""
    from huggingface_hub import hf_hub_download, snapshot_download

    base = Path(snapshot_download(repo_id=BASE_REPO, allow_patterns=["ve.pt", "grapheme_mtl_merged_expanded_v1.json", "conds.pt", "Cangjie5_TC.json"]))
    dest = Path.home() / ".cache" / "vf" / "chatterbox-pt-br"
    dest.mkdir(parents=True, exist_ok=True)
    for f in ["ve.pt", "grapheme_mtl_merged_expanded_v1.json", "conds.pt", "Cangjie5_TC.json"]:
        link = dest / f
        if not link.exists() and (base / f).exists():
            link.symlink_to(base / f)
    # chatterbox-tts 0.1.7 (PyPI) carrega nomes fixos: o T3 pt-BR entra como t3_mtl23ls_v2 e o S3Gen v3 como s3gen.pt
    for f, nome in [("t3_pt_br.safetensors", "t3_mtl23ls_v2.safetensors"), ("s3gen_v3.pt", "s3gen.pt")]:
        link = dest / nome
        if not link.exists():
            link.symlink_to(Path(hf_hub_download(repo_id=PTBR_REPO, filename=f)))
    return dest


def carregar_ptbr(device: str):
    """igual à demo oficial pt-BR: T3 pt-BR + S3Gen v3 (carregado com strict=False: o v3 não traz os
    buffers fixos do tokenizador, que o S3Gen recria sozinho)"""
    import torch
    from chatterbox import mtl_tts as m

    d = ckpt_ptbr()
    ve = m.VoiceEncoder()
    ve.load_state_dict(torch.load(d / "ve.pt", map_location="cpu", weights_only=True))
    ve.to(device).eval()
    t3 = m.T3(m.T3Config.multilingual())
    st = m.load_safetensors(d / "t3_mtl23ls_v2.safetensors")
    if "model" in st.keys():
        st = st["model"][0]
    t3.load_state_dict(st)
    t3.to(device).eval()
    s3 = m.S3Gen()
    s3.load_state_dict(torch.load(d / "s3gen.pt", map_location="cpu", weights_only=True), strict=False)
    s3.to(device).eval()
    tok = m.MTLTokenizer(str(d / "grapheme_mtl_merged_expanded_v1.json"))
    conds = m.Conditionals.load(d / "conds.pt", map_location="cpu").to(device) if (d / "conds.pt").exists() else None
    return m.ChatterboxMultilingualTTS(t3, s3, ve, tok, device, conds=conds)


def carregar(modelo: str, device: str):
    import torch

    if device == "auto":
        device = "mps" if torch.backends.mps.is_available() else "cpu"

    def load(dev):
        if modelo == "en":
            from chatterbox.tts import ChatterboxTTS

            return ChatterboxTTS.from_pretrained(device=dev)
        from chatterbox.mtl_tts import ChatterboxMultilingualTTS

        if modelo == "pt-br":
            return carregar_ptbr(dev)
        return ChatterboxMultilingualTTS.from_pretrained(device=dev)

    try:
        return load(device), device
    except Exception as e:  # MPS ainda falha em algumas versões do torch: cai para CPU
        if device != "cpu":
            print(f"aviso: {device} falhou ({e.__class__.__name__}: {e}); usando CPU", file=sys.stderr)
            return load("cpu"), "cpu"
        raise


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("texto", help="arquivo de texto (uma frase por linha) ou - para stdin")
    ap.add_argument("-o", "--out", default="public/voice/voice.wav")
    ap.add_argument("--ref", help="WAV de referência da voz (10-20 s, limpo, mesmo idioma): clona a voz")
    ap.add_argument("--modelo", default=None, choices=["pt-br", "multilingual", "en"], help="padrão: pt-br com --lang pt, multilingual nos outros")
    ap.add_argument("--lang", default="pt")
    ap.add_argument("--exaggeration", type=float, default=0.5, help="expressividade (0.25-2.0); alto acelera a fala")
    ap.add_argument("--cfg", type=float, default=0.5, help="cfg_weight: baixo = fala mais lenta; 0 evita sotaque da referência")
    ap.add_argument("--temperature", type=float, default=0.8)
    ap.add_argument("--seed", type=int, default=42, help="semente fixa = mesma voz/entonação a cada geração")
    ap.add_argument("--tempo", type=float, default=1.0, help="velocidade final (atempo do ffmpeg, mantém o tom): 1.05 = 5%% mais rápido")
    ap.add_argument("--pausa", type=float, default=0.25, help="silêncio entre frases (s)")
    ap.add_argument("--pausa-paragrafo", type=float, default=0.6)
    ap.add_argument("--refazer", default="", help="números dos pedaços para gerar de novo com outra semente (ex.: 3,7)")
    ap.add_argument("--device", default="auto", choices=["auto", "mps", "cpu", "cuda"])
    ap.add_argument("--cache", default="voice/chatterbox", help="pasta dos pedaços")
    a = ap.parse_args()

    texto = sys.stdin.read() if a.texto == "-" else Path(a.texto).read_text(encoding="utf-8")
    partes = pedacos(texto)
    if not partes:
        sys.exit("texto vazio")
    modelo = a.modelo or ("pt-br" if a.lang.lower().startswith("pt") else "multilingual")
    if a.ref and not Path(a.ref).is_file():
        sys.exit(f"--ref {a.ref} não existe")
    if not a.ref:
        print("aviso: sem --ref, usa a voz embutida do modelo (inglesa; o sotaque pode vazar). Para a voz do canal, grave/gere 10-20 s limpos em pt-BR e passe --ref.", file=sys.stderr)
    cache = Path(a.cache)
    cache.mkdir(parents=True, exist_ok=True)
    refazer = {int(x) for x in a.refazer.split(",") if x.strip()}
    ref_hash = hashlib.sha1(Path(a.ref).read_bytes()).hexdigest()[:8] if a.ref else "builtin"

    import torch
    import torchaudio as ta

    model = None
    device = a.device
    arquivos = []
    for i, (t, _) in enumerate(partes, 1):
        chave = json.dumps([t, modelo, a.lang, ref_hash, a.exaggeration, a.cfg, a.temperature, a.seed], ensure_ascii=False)
        h = hashlib.sha1(chave.encode()).hexdigest()[:10]
        f = cache / f"{i:03d}-{h}.wav"
        if f.exists() and i not in refazer:
            arquivos.append(f)
            continue
        if model is None:
            print(f"carregando Chatterbox ({modelo}, {device})… a primeira vez baixa ~3-4 GB", file=sys.stderr)
            model, device = carregar(modelo, device)
            print(f"modelo pronto em {device}", file=sys.stderr)
        seed = a.seed + (1000 * i if i in refazer else 0) + (int.from_bytes(os.urandom(2), "big") if i in refazer else 0)
        torch.manual_seed(seed)
        kw = dict(audio_prompt_path=a.ref, exaggeration=a.exaggeration, cfg_weight=a.cfg, temperature=a.temperature)
        if modelo != "en":
            kw["language_id"] = a.lang.split("-")[0].lower()
        print(f"[{i}/{len(partes)}] {t[:70]}{'…' if len(t) > 70 else ''}", file=sys.stderr)
        wav = model.generate(t, **kw)
        ta.save(str(f), wav.cpu(), model.sr)
        # tomada nova substitui as anteriores do mesmo número
        for velho in cache.glob(f"{i:03d}-*.wav"):
            if velho != f:
                velho.unlink()
        arquivos.append(f)

    # junta: pedaço + silêncio (frase/parágrafo), 48 kHz mono, corta silêncio inicial, aplica tempo
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    lista = cache / "concat.txt"
    sil = {}
    linhas, tempos, t0 = [], [], 0.0
    for (txt, pausa), f in zip(partes, arquivos):
        dur = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(f)]).decode().strip() or 0)
        linhas.append(f"file '{f.resolve()}'")
        tempos.append({"n": len(tempos) + 1, "texto": txt, "arquivo": f.name, "inicio": round(t0, 3), "fim": round(t0 + dur, 3)})
        t0 += dur
        p = a.pausa_paragrafo if pausa == "paragrafo" else a.pausa
        if p > 0:
            if p not in sil:
                sil[p] = cache / f"silencio-{int(p * 1000)}.wav"
                # mesmo formato dos pedaços (torchaudio, taxa do modelo), senão o concat do ffmpeg quebra
                sr = model.sr if model is not None else ta.info(str(arquivos[0])).sample_rate
                ta.save(str(sil[p]), torch.zeros(1, int(sr * p)), sr)
            linhas.append(f"file '{sil[p].resolve()}'")
            t0 += p
    lista.write_text("\n".join(linhas) + "\n")
    filtros = ["silenceremove=start_periods=1:start_threshold=-45dB"]
    if abs(a.tempo - 1.0) > 1e-3:
        filtros.append(f"atempo={a.tempo}")
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-f", "concat", "-safe", "0", "-i", str(lista), "-af", ",".join(filtros), "-ar", "48000", "-ac", "1", "-c:a", "pcm_s16le", str(out)], check=True)
    meta = {
        "provedor": "chatterbox", "modelo": modelo, "lang": a.lang, "ref": a.ref, "exaggeration": a.exaggeration, "cfg_weight": a.cfg,
        "temperature": a.temperature, "seed": a.seed, "tempo": a.tempo, "device": device, "saida": str(out),
        "pedacos": tempos, "obs": "tempos antes do corte de silêncio inicial e do atempo; o alinhamento final vem do vf transcribe",
    }
    (cache / "pedacos.json").write_text(json.dumps(meta, ensure_ascii=False, indent=2))
    total = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(out)]).decode().strip() or 0)
    print(f"{len(partes)} pedaço(s), {total:.1f} s -> {out}  (pedaços: {cache}/pedacos.json)")


if __name__ == "__main__":
    main()
