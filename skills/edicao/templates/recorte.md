# Recorte: vídeo longo → Shorts/TikTok

## 1. Achar os trechos
```
$VF analyze out/final_yt-long.mp4 -o out/recorte_analise --lang pt
```
Leia `analysis.md` (planos + fala por plano) e o `roteiro/beats.json` (beats com `recorte: true`). Um bom corte:
- tem 20-55 s e é **autocontido** (não depende do que veio antes);
- começa numa frase que funciona como gancho (ou permite regravar uma frase de gancho por cima);
- termina num payoff ou num fato forte, nunca no meio de um raciocínio;
- tem palavras/s ≥ 2,8. Trechos lentos do longo não funcionam em short.

Liste os candidatos com início/fim **na fronteira de palavra** (`data/words.json`): comece 0,05 s antes da primeira palavra e termine 0,3 s depois da última.

## 2. Reenquadrar 16:9 → 9:16
Prefira, nesta ordem:
1. **Re-render vertical** a partir do mesmo `edl.json` (melhor qualidade): copie o EDL, filtre os clipes do intervalo, subtraia o início dos tempos e renderize a composição `yt-short`.
2. **Crop guiado**: `$VF export trecho.mp4 -f yt-short -o out/short_N.mp4 --fit crop`. Só serve quando o assunto está no centro.
3. **Blur fill**: `--fit blur` (vídeo inteiro no meio, fundo desfocado). Use para gráficos ou mapas largos.

Cortar o trecho sem recodificar duas vezes:
```
ffmpeg -ss <ini> -to <fim> -i out/final_yt-long.mp4 -c:v libx264 -crf 16 -c:a copy out/trecho_N.mp4
```

## 3. Legendas e gancho
- Shorts/TikTok derivados ganham legenda karaokê (re-render com `captions.enabled: true`, `words_per_page: 3`).
- Se o trecho não abre com gancho, escreva uma frase nova, gere com a mesma voz e coloque no início (≤ 3 s).

## 4. Registrar
Em `factory.yaml > derivados`: `{formato, inicio, fim, arquivo, gancho}`. Cada derivado passa por `vf qa -f <formato>`.
