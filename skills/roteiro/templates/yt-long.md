# Template — YouTube vídeo longo (8-20 min)

Ritmo de fala ~150-170 wpm. Um "re-gancho" (loop aberto) a cada 60-90 s. Mudança visual a cada 3-6 s.

## Estrutura
| Bloco | Tempo | Conteúdo |
|---|---|---|
| **Cold open / gancho** | 0:00-0:30 | Comece pelo momento mais forte ou pela pergunta central. Mostre o resultado ou o conflito. Termine com a promessa: "até o fim você vai entender X". |
| **Contexto** | 0:30-1:30 | O mínimo para acompanhar. Apresente o "personagem" (pessoa, lugar, objeto). |
| **Capítulo 1..N** | 2-4 min cada | Cada capítulo tem pergunta → desenvolvimento → mini-payoff → **loop aberto** para o próximo ("mas isso criou um problema muito maior"). |
| **Clímax** | ~75-85% | O payoff principal da promessa. |
| **Resolução** | 1-2 min | Consequências e o que mudou. Callback ao cold open. |
| **Fecho** | <20 s | Uma frase final forte. Se houver CTA, ele aponta para um vídeo relacionado (tela final), não para "se inscreva". |

## Capítulos do YouTube
Gere no `SCRIPT.md` a lista `00:00 Título` a partir dos `start` do `beats.json` (depois do `vf align`). O primeiro capítulo é sempre `00:00`, são pelo menos 3 e cada um tem 10 s ou mais.

## Beats no longo
- Um beat = um parágrafo falado de 15-40 s com 3-8 visuais. Em `visual`, liste os planos separados por `;` (o motor kenburns/ia-video gera um clipe por item).
- Marque `tipo: capitulo` no primeiro beat de cada capítulo, com `titulo`.
- Marque `tipo: reganho` nos loops abertos: na edição eles ganham respiro sonoro ou um corte mais forte.

## Retenção (verifique antes de entregar)
- [ ] Os primeiros 30 s não têm "olá", logo nem patrocinador.
- [ ] Existe um loop aberto em cada transição de capítulo.
- [ ] Nenhum trecho passa de 90 s sem uma pergunta nova, uma virada ou um dado surpreendente.
- [ ] O clímax entrega exatamente o que o gancho prometeu.
- [ ] Sobram 3-5 momentos naturais para cortar shorts (beats autocontidos de 20-50 s): marque `recorte: true` neles.
