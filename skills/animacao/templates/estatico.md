# Motor `estatico`: vídeo longo com tema (10-15 min)

Imagens caprichadas (Google Flow / Nano Banana), movimento leve e edição que prende. É o **único** motor em que zoom/pan é o movimento principal, e só para `yt-long`. Shorts e TikTok continuam com animação obrigatória.

## Pipeline
1. `roteiro/beats.json` aprovado no `roteiro-check.mjs` e alinhado (`vf align`). Cada beat tem `cenario` e `visual` com um plano por item.
2. Imagens curadas em `public/img/planos/b<beat>-<n>.jpg` (skill `assets`: lotes do Flow → `vf sheet` → `curadoria.mjs aplicar`).
3. `python3 roteiro/cenas.py` (cópia de `templates/cenas/estatico.py`) gera o `data/edl.json` com `perfil: estatico`.
4. `node $CHK data/edl.json` (lê o perfil do EDL) precisa sair 0.
5. Render com HyperFrames: `skills/edicao/scripts/render-hf.sh yt-long out/video_yt-long.mp4`.

## O que o `estatico.py` aplica (e o `edl-check --perfil estatico` cobra)
| Regra | Valor |
|---|---|
| Corte | sempre no início de uma palavra (2 frames antes); o beat é dividido pelos seus planos em partes de fala iguais |
| Movimento | todo still com zoom-in de ~1,3%/s (plano de 6 s = 1,00 → 1,08); 1 em cada 4 planos com pan lento; entre 0,6%/s e 3,5%/s |
| Duração do plano | 1,5-9 s; média ≤ 3,2 s nos primeiros 30 s |
| Interrupção de padrão | card de capítulo, card (`dado`, `citacao`, `mapa`, `comparacao`), título ou punch a cada ≤ 40 s |
| Revelação/virada | `punch` (zoom de 4-6% em 0,18 s) no primeiro plano do beat |
| Capítulo | card com número + título (2,6 s, entrada em blur) e light leak; fade no plano seguinte |
| Repetição | a mesma imagem não volta antes de 120 s |
| Acabamento | grão de película (troca a cada 2 frames), vinheta, graduação única do tema (`beats.json > grade`) |

## Cards (HTML no HyperFrames, sempre nítidos)
```jsonc
{ "type": "card", "kind": "capitulo",   "data": { "numero": "Capítulo 2", "titulo": "O boato", "bg": "img/planos/b9-1.jpg" } }
{ "type": "card", "kind": "dado",       "data": { "valor": "4.200", "legenda": "pessoas na praça", "bg": "..." } }   // conta de 0 até o valor
{ "type": "card", "kind": "citacao",    "data": { "texto": "...", "autor": "Jornal do Commercio, 1889" } }             // palavra a palavra
{ "type": "card", "kind": "mapa",       "data": { "img": "img/mapa.jpg", "ponto": [0.62, 0.41], "rotulo": "Rio de Janeiro", "zoom": 1.3 } }
{ "type": "card", "kind": "comparacao", "data": { "lados": [ { "img": "...", "rotulo": "1888" }, { "img": "...", "rotulo": "1889" } ] } }
```
Card sem `bg` ganha um gradiente que deriva. Fundo chapado parado reprova no QA (congelado/preto).

## Edição leve que prende (o que olhar no corte)
- **Corte na palavra:** o plano muda junto da palavra que o nomeia ("…e o **navio** afundou" → corta no "navio").
- **J/L cut:** nas viradas, o som do próximo trecho (whoosh/riser) entra 0,25 s antes do corte (`sfx` no beat).
- **Ritmo:** gancho rápido (2-3 s/plano), corpo 4-7 s, respiro de 1 plano mais longo depois de cada payoff.
- **Música:** troca ou muda de intensidade por capítulo (`audio` com `start` por capítulo + `vf mix` com ducking). Silêncio de 0,5 s antes de uma revelação, coberto por ambiente (o QA reprova ar morto > 1 s).
- **Nada de legenda queimada** no longo: suba o SRT (`data/transcript.srt`) no YouTube.

## Render
- HyperFrames (`renderer: hyperframes`). Medido no M4: 60 s de vídeo em 39 s (~8 min para 12 min), paridade com o mygo SSIM ≥ 0,99.
- Vídeos acima de 2 min usam captura segmentada (`HF_SEGMENTED_CAPTURE`). Se cair no meio, rodar de novo retoma (`--resume`).
- `--draft` para revisão rápida e `--from/--to` para renderizar um trecho.
