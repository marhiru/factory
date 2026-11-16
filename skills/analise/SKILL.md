---
name: analise
description: "Estuda vídeos de referência (arquivo ou URL do YouTube/TikTok) frame a frame com a CLI Rust vf: detecção de cortes e crossfades, keyframe por plano, contact sheets, transcrição com tempo por palavra e métricas de ritmo (cortes/min, duração de plano, WPM, gancho, pausas), para abstrair a fórmula de um vídeo ou canal. Use quando o usuário mandar um vídeo/link de referência, pedir para analisar, transcrever ou entender por que um vídeo funciona."
---

# Análise de referência (etapa 0 da fábrica)

`VF=${CLAUDE_PLUGIN_ROOT}/bin/vf` (se não existir: `${CLAUDE_PLUGIN_ROOT}/scripts/build.sh`)

## Rodar
```
$VF analyze "<url ou arquivo>" -o ref/<slug> --lang pt      # --lang auto se não souber
```
Faz numa passada: download (yt-dlp, até 1080p) → decode reduzido → cortes e crossfades → keyframe por plano → contact sheets → transcrição (whisper.cpp Metal, em paralelo com o decode) → `analysis.md` e `analysis.json`. Referência de tempo: 110 s de vídeo 2K em ~12 s no M4.

Peças avulsas, quando só uma parte interessa:
| Comando | Saída |
|---|---|
| `$VF probe <arq>` | specs |
| `$VF frames <arq> -o <dir>` | `frames.jsonl` (luma, croma, movimento, diferença de histograma por frame), `shots.json` (planos + `xfade`) |
| `$VF keyframes <arq> -o <dir> [--every 1]` | keyframes + sheets (por plano ou a cada N s) |
| `$VF transcribe <arq> -o <dir> --lang pt` | `words.json`, `transcript.srt`, `transcript.txt` |

Ajustes: `--threshold 0.3` deixa a detecção de corte mais sensível (o padrão é 0.4). `--min-shot` é a duração mínima de plano em frames. `--no-transcribe` pula a fala.

## Ler (nesta ordem)
1. `ref/<slug>/analysis.md`: specs, ritmo, gancho, tabela de planos com a fala de cada um.
2. **Abra as sheets** `ref/<slug>/sheets/shots_*.jpg` com a ferramenta de leitura de imagem (cada célula é um plano com `#id tempo duração`). Para os 3-5 planos mais importantes, abra a keyframe em `keyframes/`.
3. Se precisar de detalhe de movimento num trecho: `$VF keyframes <arq> -o ref/<slug>/zoom --every 0.25` e leia as sheets do trecho.

## Abstrair (o que escrever em `ref/<slug>/notas.md`)
| Dimensão | Pergunta | Métrica |
|---|---|---|
| Gancho | O que acontece nos primeiros 1-3 s (fala E imagem)? Qual padrão de `roteiro/templates/ganchos.md`? | `hook_end`, `speech_start` |
| Ritmo | Cortes/min, duração mediana de plano, variação (planos longos de propósito?) | `cuts_per_min`, `asl_median`, buckets |
| Fala | WPM, pausas, densidade (fala ocupa quantos % do vídeo) | `wpm`, `pauses_total`, `speech_ratio` |
| Estrutura | Beats: contexto → escalada → virada → payoff. Onde estão os loops abertos? | tabela de planos + transcrição |
| Visual | Paleta (luma/croma), tipo de plano dominante, texto na tela, legendas, movimento | sheets, `motion_mean` |
| Som | Música contínua? SFX em cortes? | ouvir o trecho ou `vf qa` (loudness) |
| Final | Loop, CTA ou corte seco? | último plano |

Termine com **"Fórmula para copiar"** (5-8 regras mensuráveis, ex.: "corte a cada 1,8 s", "gancho com número nos primeiros 6 palavras") e **"Não copiar"** (roteiro, personagens, marca, planos específicos). Leve as regras para `factory.yaml > referencias` e para o roteiro.

Comparando vários vídeos de um canal: rode `analyze` em 3-5 vídeos e monte uma tabela com as métricas de `analysis.json > pace` de cada um. A fórmula é o que se repete em todos.

## Notas
- Downloads ficam em `ref/<slug>/source.*` (já no `.gitignore`). São só para estudo, nunca entram no vídeo.
- A primeira transcrição baixa o modelo `large-v3-turbo-q5_0` (~550 MB) para `~/.cache/vf/models`.

## Estudar FX e transições (quadro a quadro)
As sheets por plano mostram composição, mas não os efeitos. Para FX, faça folhas densas nos momentos de ação (picos de `motion` em `frames.jsonl` ou palavras de impacto na transcrição):
```
ffmpeg -ss <início> -t <dur> -i ref/<slug>/source.mp4 -vf "fps=10,scale=240:-1,tile=8x4" -frames:v 1 ref/<slug>/fx_<momento>.jpg
```
10 fps pega dissolves de 2-3 frames, holds de impacto e transições de zoom-through. Anote cada FX com **momento do roteiro, por que funciona e a receita na fábrica**, como em `referencias/rickclips.md`.

## Estudos salvos
- `referencias/rickclips.md`: shorts "história absurda → mecanismo" (6-34M views): fórmula de roteiro, números de ritmo e catálogo de FX com receitas.
