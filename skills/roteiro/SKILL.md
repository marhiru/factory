---
name: roteiro
description: "Templates e regras para escrever roteiros da fábrica de vídeo: YouTube longo (capítulos, retenção), YouTube Shorts e TikTok (gancho em 1-2 s, loop), com fatos e fontes, contagem de duração por WPM e saída em beats.json com intenção visual por beat. Use na etapa de roteiro de um projeto video-factory ou quando pedirem um roteiro para vídeo de YouTube/TikTok."
---

# Roteiro (etapa 1 da fábrica)

Entrada: tema, formato principal (`factory.yaml`) e, se houver, a análise de referência (`ref/*/analysis.md`).
Saídas, sempre as três:
1. `SCRIPT.md`: fatos com fonte, promessa, roteiro e contagem.
2. `roteiro/roteiro.txt`: só o texto falado, uma frase por linha, exatamente como vai para a voz.
3. `roteiro/beats.json`: um beat por frase ou ideia, com `texto`, `visual`, `camera`, `assets`, `sfx` e `texto_tela`.

## Escolha o template
| Formato | Template | Base |
|---|---|---|
| `yt-short` explicativo (história real + causa) | use a skill **`roteiro-short-explicativo`** (fórmula de 9 beats, já validada) | 30-70 s |
| `yt-short` / `tiktok` história absurda + mecanismo (física, psicologia, história) | `templates/short-historia-mecanismo.md` (fórmula estudada em shorts de 6-34M views) | 45-70 s |
| `yt-short` / `tiktok` geral (lista, curiosidade, tutorial, opinião) | `templates/short.md` | 15-60 s |
| `tiktok` nativo (tendência, POV, loop) | `templates/tiktok.md` | 7-45 s |
| `yt-long` estático com tema (motor `estatico`) | `templates/yt-long-estatico.md`: esquemática de hook rate | 10-15 min |
| `yt-long` (outros motores) | `templates/yt-long.md` | 8-20 min |
| qualquer um | `templates/ganchos.md` | biblioteca de aberturas |

## Processo
1. **Pesquisa antes de escrever.** Liste os fatos com fonte no topo do `SCRIPT.md`. Só entra no roteiro fato com fonte. Número sem fonte sai do texto.
2. **Promessa em uma frase**: o que o espectador ganha se ficar até o fim. O gancho cobra essa promessa.
3. Escreva pelo template. Linguagem falada, frases de 8-16 palavras, nada de jargão sem explicação na frase seguinte.
4. **Cada frase precisa ser ilustrável.** Preencha o `visual` de cada beat com algo concreto que se possa mostrar. A narração nunca descreve o que a tela já mostra: a imagem carrega a piada ou o dado, e a fala acrescenta.
5. **Conte as palavras** e divida pelo `wpm_alvo` do `factory.yaml` (Shorts/TikTok: 3,7 palavras/s; longo: ~160 wpm). Ajuste até caber na faixa do formato (`$VF formats`).
6. Escreva números por extenso **só** no `roteiro.txt` que vai para a voz ("mil oitocentos e oitenta e nove"). No `SCRIPT.md` e em `texto_tela`, use algarismos.
7. **Trava do longo:** `node ${CLAUDE_PLUGIN_ROOT}/skills/roteiro/scripts/roteiro-check.mjs roteiro/beats.json` precisa sair 0 (gancho sem preparação, promessa e loop até 0:40, re-hooks, capítulos fechando com loop, payoff, fecho, densidade de visuais). Para um piloto de 2-4 min, use `--piloto`. Se reprovar, reescreva: o usuário só vê roteiro aprovado na trava.
8. Mostre ao usuário: promessa, roteiro e duração estimada. **Pare para aprovação.** Ao aprovar, marque `etapas.roteiro: feito` e `aprovacao_roteiro: feito`.

## Depois da voz
Com `data/words.json` pronto (skill `assets`), rode `$VF align roteiro/beats.json data/words.json`. Ele grava `w0`, `w1`, `start` e `end` em cada beat. Beats marcados `CONFERIR` indicam que a voz mudou o texto: corrija o `beats.json` ou regrave a frase.

## Regras fixas
- História, personagens e voz sempre **originais**. Referências servem só pela fórmula, nunca citadas no vídeo.
- Sem CTA no meio de shorts. No longo, no máximo um CTA leve depois do primeiro pico de valor e outro no fim.
- O vídeo termina na última palavra útil: sem outro, sem "é isso, pessoal".
