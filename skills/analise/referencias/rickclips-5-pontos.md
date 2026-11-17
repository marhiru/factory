# Estudo detalhado pelos 5 pontos de qualidade (2026-11-17)

Três shorts do mesmo canal de `rickclips.md`, medidos com `vf analyze` + `scripts/pontos.py` e lidos quadro a
quadro (folhas densas de 2-6 fps e quadro a quadro nos dissolves). Cada achado é comparado com as regras da
fábrica (QA visual etapas 3, 3b, 5 e 6) e vira regra ou ferramenta. Só a **gramática** serve; nunca copie
roteiro, personagens, cenários ou planos.

| | A: jarro (35M) | B: trilhos (30M) | C: emprego no Japão (19M) |
|---|---|---|---|
| Duração / planos | 64,8 s / 20 | 65,2 s / 13 | 60,5 s / 27 |
| Transições suaves (dissolve) | **15 de 19** | **12 de 12** | **20 de 26** |
| Dissolve mediano | 7 f (0,23 s) | 9,5 f (0,32 s) | 7 f (0,23 s) |
| Frames parados | 0,05 % | 0,05 % | 0,06 % |
| Arco de luz (Q quente, F frio) | `·Qqq··QQQQQFffFFFqQQ` | `QFq·QqQffQQqq` | `··FQF···Fff·fFFQqQFqFFFF·FF` |
| Luma (mín-máx por plano) | 0,30-0,61 | 0,26-0,68 | 0,27-0,70 |
| Faixa da legenda (altura) | 0,70-0,72 | 0,69-0,72 | 0,70-0,72 |

## 1. Posicionamento coeso (QA 3)

O que se vê:
- **Todo contato é físico:** quatro homens sentados numa viga sobre o cânion (B), dois personagens com os pés
  sobre a mesa segurando copos (C), o chefe com a mão no ombro do funcionário enquanto anda (C), o inventor
  puxando a carroça com corda tensionada (A). Ninguém flutua; pés no chão, quadril no assento.
- **O cenário é persistente:** na sala de C, calendário ao lado da porta, relógio acima da mesa, caixas no canto
  e a mancha de sol na parede estão **no mesmo lugar** em todos os ângulos. Em B, a caixa verde com o calendário
  e a bandeira azul ficam no mesmo ponto do mundo em close, plano médio e plano conjunto.
- **Primeiro plano emoldura sem tapar:** nuca do protagonista, colegas de costas, botão vermelho desfocado
  ficam nas **bordas** do quadro, desfocados, nunca sobre o rosto do assunto.
- **Rostos nunca na faixa da legenda:** a legenda mora em 0,69-0,72 da altura; rostos e a ação principal ficam
  entre ~0,25 e 0,60.
- **O objeto da história está em todo plano do seu ato:** o jarro aparece no salão, na colina, na noite e no
  final; o botão "FIRE" fica no primeiro plano enquanto a câmera troca de personagem.

Comparação com a fábrica: a etapa 3 já exigia suporte e não tapar a cena; faltava **cenário persistente entre
ângulos**, **faixa proibida da legenda** e **moldura de primeiro plano só nas bordas**. Entraram no QA 3.

## 2. Identidade estável (QA 3b)

- Cada personagem tem **um modelo** que nunca muda: o chefe de C (cabelo grisalho, gravata vermelha, olhos
  fechados sorrindo) é idêntico em 6 planos ao longo de 50 s; o engenheiro de B (boné listrado, colete verde,
  gravata-borboleta) em 8 planos.
- **Emoção é estado do mesmo modelo, não outro desenho:** o chefe de B fica roxo com nuvens de vapor ("his face
  goes purple") e volta ao normal; a forma, o chapéu, o colete e a bengala continuam os mesmos.
- **Props também têm identidade e estado:** o calendário passa de MON 7 para MON 28 e NOV FRI 18 (o tempo anda
  pelo prop); a mesa do colega some e fica com as marcas de copo; o jarro perde as bolas uma a uma.

Comparação: a regra 3b proibia trocar o asset; agora há a ferramenta `animacao/scripts/elenco-lock.mjs`
(trava por hash dos packs aprovados) e a regra de que **mudança de emoção/estado = estado novo do mesmo pack**.
Props que contam o tempo (calendário, relógio, termômetro) entram como estados de um mesmo prop.

## 3. Câmera entre trechos (QA 5)

O que se mede: **75-100 % das transições são dissolves de 0,2-0,3 s**, e o corte seco é minoria. O `vf frames`
chama de "plano" de 12-15 s o que na verdade são vários enquadramentos costurados por movimento:

- **Zoom-in no detalhe → zoom-out para os personagens, sem corte** (B, 36-48 s): calendário "MAIO" →
  recua para o engenheiro → aproxima até o vão do trilho com o paquímetro (maio, julho) → recua para o
  engenheiro e o chefe com o trilho quente no primeiro plano. É exatamente a regra da fábrica para trechos
  contínuos no mesmo contexto.
- **Dissolve com a câmera andando** (C, 7,8 s): a câmera atravessa a porta da sala e, durante um dissolve de
  ~6 frames, continua avançando até a mesa, **no mesmo sentido**. A mancha de sol na parede está no mesmo lugar
  nos dois ângulos, então o olho lê como um movimento só.
- **Chicote (whip pan) com motion blur** para outro personagem do mesmo lugar (B, 7,5 s e 10,5 s).
- **Atravessar janela/porta** para entrar num interior (B, 12,5-13,5 s: do trem para dentro do vagão).
- **FX como cortina:** a poeira do desabamento enche o quadro e, quando abre, já estamos dentro do jarro
  (A, 47-49 s).
- **Foco que muda de plano** (rack focus) entre o botão em primeiro plano e o personagem atrás (C, 0-1 s).
- **Câmera nunca parada:** 0,05 % de frames parados; mesmo o top-down do pêndulo gira devagar (A).

Comparação: a regra zoom-out → zoom-in está confirmada. Faltavam: a **duração** do dissolve (0,2-0,3 s, não
2-3 f), a regra de **manter o sentido do movimento através do dissolve**, e as alternativas para quando o
assunto muda dentro do mesmo lugar (chicote, atravessar porta, cortina de FX, rack focus). Os cartões agora
sugerem a junção sozinhos (`continuo_sugerido` no `planos.mjs`).

## 4. Luz de cenário (QA 6)

- **Arco de temperatura acompanha a história:** em A, salão quente → pôr do sol no exílio (o plano mais quente,
  R-B +87) → noite fria do alarme (−83) → manhã quente da revelação. Em C, escritório frio-neutro → noite azul
  (−124) com uma única janela acesa quente → final azul com luminária quente.
- **Ênfase = acento quente num ambiente frio:** em C, os planos quentes são justamente os do chefe e do
  "colega feliz"; o resto é frio. A mancha de sol da janela (gobo retangular) cai **sempre** sobre o colega
  feliz. Na noite final, uma arandela quente faz uma poça de luz só na cabeça do chefe.
- **Contraluz na emoção:** o inventor arrastando o jarro morro acima fica contra o sol com god rays e borda
  iluminada (A, 29,7 s).
- **Práticos justificam a luz:** tochas, arandelas, luminária, janela, botão vermelho aceso. Nenhuma cena é
  "estúdio chapado".
- **Noite legível:** a luma nunca cai abaixo de ~0,26; a noite é azul e clara, não preta.
- **Luz é contínua entre ângulos:** a mancha de sol fica no mesmo ponto da parede em todos os planos da sala.

Comparação: o rig por cenário e os cues de ênfase estavam certos; entraram as receitas medidas (arco de
temperatura por ato, gobo de janela no assunto, poça quente na noite fria, contraluz na emoção, piso de luma
0,26 à noite) e a **continuidade de luz entre ângulos**.

## 5. FX (QA 6)

| FX visto | Onde | Leitura | Receita na fábrica |
|---|---|---|---|
| **Trilho em brasa:** o material do trilho vai de roxo → magenta → amarelo-branco com bloom | B, 37-60 s | o calor é visto no próprio objeto, no ritmo da fala ("maio… julho") | `fx/heat.ts` (`heatOn` no material, `cueHeat` por frames) |
| Faíscas sob as rodas | B, 12 s | o trem "raspa" no trilho perfeito | `ShaderFx("sparkle")` laranja ou pixelfx `impacts` recolorido |
| Rosto roxo + nuvens de vapor | B, 16-19 s | raiva como estado do personagem | estado `raiva` do pack + pack `fx-vapor` |
| Martelo + estrela de impacto + brilho na chave | C, 1-4 s | o objeto-piada ganha um "ding" na palavra | `ring` curto + `sparkle` no `at_word` |
| Desabamento: poeira iluminada + destroços | A, 46-48 s | hipérbole no clímax e cortina de transição | `ShaderFx("dust")` (lado da luz quente, sombra fria, destroços em parábola) |
| Cogumelo de explosão | B, 64 s | o payoff do absurdo | `dust` grande + `glow` laranja no núcleo + cue `flash` |
| Raios de sol, bloom de lâmpadas e metal | A, B, C | o mundo "brilha" | `beam`, `LightCueEffect.rays`, emissivos > 1 |

Comparação: a régua de FX (antecipação → pico → dissipação, iluminar o entorno) está confirmada. Faltavam
**FX no material do próprio asset** (calor, brilho de alerta) e **fumaça/poeira que ocupa espaço** (não
aditiva), agora no motor (`heat.ts` e o tipo `dust`).

## Regras novas (resumo do que entrou nas skills)

1. Legenda em 0,66-0,76 da altura é zona proibida para rosto e ação principal; primeiro plano só nas bordas.
2. Cenário persistente: props e luz fixos no mundo valem para todos os ângulos do mesmo cenário.
3. Emoção e passagem de tempo são **estados** do mesmo pack/prop; `elenco-lock.mjs check` antes de aceitar correção.
4. Mesmo contexto contínuo: zoom-out → zoom-in sem corte; mudando o ângulo, dissolve de 0,2-0,3 s com a câmera
   andando no mesmo sentido; corte seco ≤ 25 % das transições.
5. Arco de temperatura por ato; ênfase quente sobre ambiente frio; noite com luma ≥ 0,26.
6. FX no material (calor) e poeira que ocupa espaço; cada FX ilumina ou sombreia o entorno.
