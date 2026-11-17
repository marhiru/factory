# Evidências: canal de shorts "história absurda → mecanismo"

Registro das medições que sustentam as regras da fábrica. **As regras moram nas skills de etapa** (mapa no
fim); este arquivo guarda só fontes, números e correções, para reestudar ou contestar uma regra com dados.
Serve pela **gramática**: nunca copie roteiro, personagens, marca, voz, cenários ou planos.

## Vídeos estudados

| Id | Título | Views (na coleta) | Coleta | Método |
|---|---|---|---|---|
| A | The Ancient Jar That Warned an Empire (`VS1Z_Q-Klt8`) | 35,0M | 16 e 17/11/2026 | `vf analyze` + `scripts/pontos.py` + folhas densas |
| B | Why Perfect Railroads Don't Work (`7xS87iActkU`) | 29,9M | 16 e 17/11/2026 | idem |
| C | The Job Nobody Wants in Japan (`0m6G8pDtzNs`) | 18,6M | 17/11/2026 | idem |
| D | short "árvore na estrada" (o de mais FX) | 6,5M | 16/11/2026 | `vf analyze` + folhas densas (id não registrado) |
| — | títulos e views de 152 shorts do canal | — | 16/11/2026 | lista pública |

## Números (medição de 17/11/2026; a de 16/11 entre parênteses quando diferente)

| | A | B | C | D |
|---|---|---|---|---|
| Duração | 64,8 s | 65,2 s | 60,5 s | 59,4 s |
| Palavras / WPM | 235 / 218 | 213 / 197 | 224 / 223 | — / 220 |
| Fala ocupa / maior pausa | 95 % / 0,24 s | 99 % / 0,07 s | 94 % / 0,42 s | 96 % / 0,22 s |
| Fala começa em | 0,02 s | 0,07 s | 0,03 s | 0,04 s |
| Fim do gancho | 5,0 s | ~3,4 s | 4,2 s | — |
| Planos detectados | 20 (18) | 13 (14) | 27 | 20 |
| Cortes/min · mediana | 17,6 · 2,8 s | 11,0 · 3,7 s | 25,8 · 2,0 s | 19 · 2,7 s |
| Transições suaves | 15 de 19 | 12 de 12 | 20 de 26 | — |
| Dissolve mediano | 7 f (0,23 s) | 9,5 f (0,32 s) | 7 f (0,23 s) | — |
| Frames parados · vida mediana | 0,05 % · 0,120 | 0,05 % · 0,149 | 0,06 % · 0,137 | 0 % · 0,125 |
| Luma (média · mín-máx por plano) | 0,43 · 0,30-0,61 | 0,44 · 0,26-0,68 | 0,51 · 0,27-0,70 | 0,43 |
| Arco de temperatura (Q quente, F frio) | `·Qqq··QQQQQFffFFFqQQ` | `QFq·QqQffQQqq` | `··FQF···Fff·fFFQqQFqFFFF·FF` | — |
| Faixa da legenda (altura) | 0,70-0,72 | 0,69-0,72 | 0,70-0,72 | — |

Nosso short cutout de referência, na mesma régua: vida mediana ~0,035 (4x menos movimento).

## Correções feitas em 17/11/2026 (o que o estudo de 16/11 dizia e estava errado)

| Antes | Medido | Onde foi corrigido |
|---|---|---|
| dissolves de 2-3 frames (estimativa visual) | 6-10 frames (0,2-0,3 s), conferido quadro a quadro em C 7,8 s | glossário, `animacao` SKILL, QA etapa 5 |
| 4 / 11 / 13 dissolves por vídeo | 15 de 19, 12 de 12, 20 de 26 transições suaves (`pontos.py`) | este registro |
| legenda a ~62 % da altura | 0,69-0,72 nos três vídeos | `edicao` SKILL (legendas), QA etapa 3 |
| a 2ª frase sempre abre com "Veja," | A e B: sim (3-7 s); C: "Veja" só abre o mecanismo (82 %) | `roteiro/templates/short-historia-mecanismo.md` |
| beat de pergunta antes do payoff | presente em D; A, B e C vão direto | idem (beat opcional) |
| golden hour é a luz padrão | a temperatura muda por ato; golden é um dos climas | glossário (seção 4), `mundo-3d-blender.md` |
| luma ~0,43 em todas as cenas | média 0,43-0,51; por plano 0,26-0,70; noite ≥ 0,26 | `mundo-3d-blender.md`, QA etapa 6 |
| A tinha 18 planos, B 14 | 20 e 13: o detector junta enquadramentos costurados por movimento | este registro (planos ≠ enquadramentos) |

## Mapa: onde cada parte do estudo virou regra

| Tema | Skill / arquivo |
|---|---|
| Fórmula do roteiro, ritmo de fala, variações A/B/C, visual por beat | `roteiro/templates/short-historia-mecanismo.md` |
| Catálogo de FX com receitas (D + A/B/C) e vocabulário | `animacao/templates/glossario-fx.md` (seção 7) |
| Luz: arco de temperatura, ênfase quente, gobo, poça, contraluz, noite legível | `animacao/references/luz-fx-shaders.md` |
| Câmera: junções, dissolve andando, chicote, porta, cortina de FX, rack focus | `animacao/SKILL.md` + QA etapa 5 |
| Direção: câmera nunca para, pose-to-pose com hold, uma ideia por plano | `animacao/SKILL.md` (direção de animação) |
| Como o mundo é feito (cards 2D em PBR, luz de cinema, fratura, ferramenta) | `animacao/templates/mundo-3d-blender.md` |
| Posicionamento, identidade, cenário persistente, faixa da legenda | `edicao/references/qa-cinco-etapas.md` (3, 3b) |
| Legendas e títulos | `edicao/SKILL.md` (legendas) e `edicao/templates/publicacao.md` |
| Operação do canal, voz como marca, pivô de formato, risco de cópia | `canal/SKILL.md` |
| Método de medição (pontos.py, folhas densas, quadro a quadro) | `analise/SKILL.md` |
