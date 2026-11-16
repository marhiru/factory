# Template: YouTube longo estático com tema (10-15 min)

Imagens caprichadas com zoom-in leve, narração forte e edição leve que prende. Como quase não há "ação" na tela, **a retenção depende do roteiro**: cada trecho precisa deixar uma pergunta aberta. A trava `scripts/roteiro-check.mjs` cobra tudo o que está abaixo antes de o roteiro ir para o usuário.

Ritmo de fala: 150-170 wpm (~1.700-2.400 palavras). Uma imagem a cada 4-7 s (2-3 s no gancho): ~110-150 imagens.

## Esquemática (hook rate alto)
| Bloco | Tempo | `tipo` no beats.json | O que acontece |
|---|---|---|---|
| **Frame 0 + 1ª frase** | 0-5 s | `gancho` | A imagem mais forte do vídeo e a afirmação mais improvável que for verdade. Sem "olá", sem logo, sem contexto. Até 14 palavras. **Precisa ser a mesma promessa da thumbnail e do título**: quem clicou recebe na hora o que a capa prometeu. |
| **Stakes + prova** | 5-20 s | `gancho` / `contexto` | Por que importa para quem assiste, com um dado concreto ou uma imagem que prova (documento, lugar, número). |
| **Promessa** | 20-35 s | `promessa` | "Até o fim deste vídeo você vai entender X." Concreta e cobrável. |
| **Loop aberto** | junto da promessa | `loop` | "...e a parte mais estranha não é essa." Uma pergunta que só se fecha lá na frente. |
| **Re-hook 1** | 35-75 s | `reganho` / `virada` | A primeira virada, antes da queda natural do primeiro minuto. |
| **Capítulos** | 2-3 min cada, ≥ 3 | `capitulo` (com `titulo`) | Pergunta → desenvolvimento → mini-payoff → `loop` para o próximo nos últimos 60 s do capítulo. |
| **Re-hook do meio** | 40-60% | `reganho` / `revelacao` | Revelação parcial que muda o sentido do que veio antes. |
| **Payoff** | 65-92% | `payoff` | Entrega exatamente o que o gancho prometeu. Sem enrolar. |
| **Fecho** | < 20 s | `fecho` | Callback ao frame 0 e teaser do próximo vídeo (tela final). Sem "se inscreva". |

Tensão contínua: nenhum trecho de 90 s sem `pergunta`, `virada`, `loop`, `reganho` ou `revelacao` (ideal ≤ 60 s).

## Técnicas de narrativa que prendem
- **Escada de tensão:** cada capítulo sobe a aposta (mais gente, mais dinheiro, mais risco, mais estranho).
- **Bucket brigades** para atravessar transições: "Mas tem um detalhe.", "E é aqui que fica estranho.", "Só que ninguém contou para ele."
- **Concreto sempre:** nome, lugar, data, número exato. "Muita gente" vira "4.200 pessoas".
- **Pergunta na cabeça do espectador antes da resposta:** apresente o mistério, segure 20-60 s, responda e abra o próximo.
- **Ritmo de frase:** curta no gancho e nas viradas (≤ 12 palavras), mais longa só para explicar. Uma ideia por frase.
- **Contraste:** antes/depois, esperado/real, o que diziam/o que aconteceu.
- **Personagem:** alguém com um objetivo e um obstáculo, mesmo em temas sem pessoas (o lugar, o objeto, a ideia vira personagem).
- **Nada de recapitulação longa:** "como vimos" só em uma frase, e só para preparar uma virada.

## beats.json do longo estático
- Todo beat tem `cenario`: a chave do lugar ou da situação (ex.: `porto-noite`, `palacio-salao`). As imagens são geradas **por cenário** em lotes de 24 no Google Flow.
- `visual`: um plano por item, separados por `;`, cada um descrevendo a imagem (sujeito, ação, enquadramento). Gancho: 1 plano a cada ~2-3 s. Corpo: 1 a cada 4-7 s.
- Marque `card` para as interrupções de padrão (a cada 20-40 s): `{"kind": "dado", "valor": "4.200", "legenda": "..."}`, `{"kind": "citacao", ...}`, `{"kind": "mapa", ...}`, `{"kind": "comparacao", ...}`. O capítulo ganha card automaticamente.
- `sfx`: `whoosh` nas viradas, `riser` antes de revelação, `impacto` no punch.

## Checklist antes de mostrar ao usuário
- [ ] `node ${CLAUDE_PLUGIN_ROOT}/skills/roteiro/scripts/roteiro-check.mjs` sai 0.
- [ ] Título, thumbnail e frame 0 contam a mesma promessa (rascunho em `SCRIPT.md > publicação`).
- [ ] Toda afirmação factual tem fonte no topo do `SCRIPT.md`.
- [ ] Lendo só as frases de `tipo: loop`, dá vontade de saber a resposta.
