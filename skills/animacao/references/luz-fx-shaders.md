# Luz de cenário, FX e shaders

Leia ao montar cenários, iluminar planos ou criar qualquer FX. Vale para todos os motores animados
(`cutout-2d`/mygo, `remotion-2d`, `remotion-three`). O revisor confere pela [etapa 6 do QA visual](../../edicao/references/qa-cinco-etapas.md).

## 1. Luz é direção, não só exposição

A luz tem três funções ao mesmo tempo:

1. **Harmonia do cenário.** Uma luz-chave por cenário com direção, cor e dureza fixas (`rig` no cartão do
   cenário). Todo asset posto no cenário recebe a mesma luz: lado iluminado, sombra de contato e cor da
   sombra coerentes com o rig. Asset que parece "colado" (luz própria, sombra no lado errado, sem contato)
   quebra a harmonia e reprova, igual a um asset mal posicionado.
2. **Leitura.** O assunto do plano é a área de maior contraste e mais quente; o fundo fica um degrau abaixo.
   O olho vai para onde a luz manda: nunca deixe um prop de fundo mais claro do que quem fala.
3. **Emoção e ênfase (dinâmica).** A luz muda com a história: destacar um personagem, abrir a cena numa
   revelação, fechar na tensão. É uma ação do plano, cai numa palavra como qualquer ação.

### Medido nas referências (`analise/referencias/rickclips-5-pontos.md`)

| Receita | Como aparece | Na fábrica |
|---|---|---|
| Arco de temperatura por ato | quente → pôr do sol (+87) → noite (−83) → manhã quente | `temp` por ato nos cues; plates dia/golden/noite no 2D |
| Ênfase quente em ambiente frio | só o chefe e o "colega feliz" ficam quentes | `spot` + `temp` positivo no assunto; resto frio |
| Gobo de janela no assunto | retângulo de sol na parede cai sempre sobre quem importa | `beam` curto ou mancha emissiva na parede, fixa no mundo |
| Poça quente na noite fria | arandela ilumina só a cabeça do chefe | prático emissivo > 1 + `spot` r 0,2 |
| Contraluz na emoção | personagem contra o sol, god rays, borda acesa | `rays` na fonte + rim interno do card |
| Noite legível | luma ≥ 0,26, azul claro | `exposure` ≥ −0,3; preenchimento azul forte |
| Luz contínua entre ângulos | a mancha de sol não muda de lugar | luz presa ao mundo, nunca à câmera, dentro do cenário |

### Rig por cenário (interno e global)

| Camada | Externo (global) | Interno |
|---|---|---|
| Chave | sol: direção fixa por plano, relativa à câmera, congelada no plano | fonte motivada visível ou implícita: janela, luminária, tela, fogo |
| Preenchimento | céu (hemisférica fria) | rebatida da parede/chão na cor do ambiente, 25-40 % da chave |
| Recorte (rim) | contraluz quente no golden | prático atrás do assunto; só por dentro da silhueta |
| Práticos | postes, letreiros, janelas acesas | luminárias, abajur, monitor: emissivos > 1 (florescem no bloom) |
| Ar | névoa de distância, raios de sol | poeira no feixe, god rays da janela, volume em fumaça |

Regras: uma única chave dominante; cor da sombra complementar à chave (chave quente → sombra azulada);
práticos sempre justificam a luz que projetam; interno nunca é "luz de estúdio" chapada sem fonte.

### Cues de luz (ênfase dinâmica)

Registre no cartão do plano (`luz:`), em palavras do roteiro:

| Cue | Quando usar | Receita |
|---|---|---|
| `spot` | destacar quem fala/reage, o prop-chave, a revelação | periferia −30-45 % e menos saturada, assunto +10 % e mais quente; entra em 6-10 frames |
| `rays` | revelação, "a resposta", janela/sol entrando | god rays da fonte HDR do cenário, só dela; 0,5-0,9 |
| `exposure` | abrir (+0,2-0,3) na solução; fechar (−0,3-0,5) na ameaça | sempre gradual, nunca pisca |
| `temp` | troca de clima/ato sem trocar o rig | +0,3-0,5 quente (alívio, nostalgia), −0,3-0,5 frio (perigo, noite) |
| `flash` | impacto | 2-4 frames, decai; só com FX de impacto junto |

Proibido: ênfase que tapa o rosto, cue que muda a direção da chave no meio do plano, piscar a exposição
sem motivo, spot em quem não é o assunto daquela fala.

### Implementação por motor

- **`remotion-three`**: `src/fx/lightcue.ts` (`LightCueEffect` + `cueAt` + `applyCues`). Coloque o efeito
  depois do DOF e **antes** do bloom e do tone mapping no `EffectComposer` do `Bridge.tsx`. O alvo pode ser
  posição no mundo (o spot segue o personagem com a câmera) ou um ponto de tela. Rig do mundo em `lights.ts`.
- **`cutout-2d`/mygo**: a luz é pintada no plate (chave, sombras, práticos) e reforçada por camadas de
  ator: um pack `luz-spot` (PNG grande com vinheta radial escura e centro transparente, `opacity` em keys
  na palavra), `luz-raios` (feixe em PNG com alpha, render da `FxSheet` tipo `beam`) e sombras de contato
  sob cada ator. Mudança de clima = segundo plate do mesmo cenário (dia/golden/noite) cruzado em 8-12 frames.
- **`estatico`** (yt-long): a luz já vem na imagem gerada (style bible fixa a luz por cenário); no
  HyperFrames use os overlays (light leak, graduação) só para ênfase pontual.

## 2. FX: régua de qualidade

Um FX bom tem **antecipação → pico → dissipação**, forma legível em silhueta, núcleo quente que vai ao
branco, borda na cor do efeito, partículas secundárias e luz que afeta o cenário (o brilho ilumina o chão e
o personagem por perto). FX ruim: elipse borrada, cor chapada, aparece e some sem curva, não toca o mundo.

Checklist por FX (o agente de FX entrega com evidência; o revisor confere na folha densa):
- anima em pelo menos 3 fases e cai na palavra/batida certa (0-3 frames antes);
- tem núcleo + borda + secundário (faísca, fumaça, anel, poeira);
- ilumina o entorno (cue de luz, emissivo > 1 no 3D, ou camada de luz no 2D);
- casa com o estilo: contorno, paleta e resolução do resto do quadro (pixel art só em vídeo pixel art
  ou como piada deliberada);
- some antes da borda do card (nenhum quadrado visível) e não esconde rosto, legenda ou safe zone.

## 3. Motor de shaders (GLSL)

`remotion-three` traz o motor em `src/fx/`:

| Arquivo | O quê |
|---|---|
| `shaderfx.ts` | `ShaderFx(kind, opts)`: card voltado para a câmera, aditivo, HDR (floresce no bloom). Tipos: `glow`, `energy` (chama/plasma fbm), `ring` (onda de choque), `sparkle`, `beam` (feixe/god ray com poeira), `dust` (poeira/fumaça iluminada que cobre o fundo + destroços; serve de cortina de transição; `uLight` = direção da chave) |
| `lightcue.ts` | luz dinâmica de pós: `spot`, `rays`, `exposure`, `temp` por keyframe |
| `heat.ts` | calor no material do asset: `heatOn(material, cueHeat(f, de, até))`, rampa roxo → magenta → laranja → amarelo-branco com emissivo > 1 no pico |
| `FxSheet.tsx` | composições `FxSheet` (um FX em fundo transparente → sprites) e `FxLab` (folha de conferência) |

- `intensity` 1,2-2 para FX perto de personagem; 2,5-4 só para fontes (lâmpada, explosão). Acima disso o
  núcleo vira branco chapado.
- FX novo = nova função no `FX_GLSL` com o mesmo contrato (`vec4` pré-multiplicado, envelope pelo `env`),
  determinístico em `(uTime, uSeed)`. Confira no `FxLab` com stills em 3 frames antes de usar.
- **Para o mygo/cutout**: renderize a `FxSheet` em PNG com alpha e converta:
  ```
  bunx remotion render src/index.ts FxSheet out/fx/ring --sequence --image-format=png \
    --props '{"kind":"ring","color":"#ffd27a","duration":0.9}'
  bun ${PLUGIN}/skills/animacao/scripts/fx-pack.mjs out/fx/ring public/sprites/fx-ring
  ```
  O modo sheet converte luz em cobertura (nunca escurece o fundo) e grava em sRGB.

## 4. Bibliotecas de referência (`vf init` liga em `ref/fx/`)

- **pixelfx** (MIT, patch-gg/pixelfx): 244 FX em código (fogo, explosões, gelo, raio, magia, impactos,
  transições, críticos) com atuação do personagem (antecipação, congelamento no contato, recuperação).
  Use como **catálogo de timing e composição** de FX e, em vídeo pixel art, direto:
  ```
  node ref/fx/pixelfx/tools/list.mjs explosão impacto          # procurar
  bun ${PLUGIN}/skills/animacao/scripts/fx-pack.mjs explosions/cartoon-pow public/sprites/fx-pow --scale 6 \
    --opts '{"palette":["#102","#215","#43a","#8cf","#fff"]}'  # recolorir para a paleta do vídeo
  ```
  A skill do pixelfx (`ref/fx/pixelfx/skills/pixelfx/SKILL.md`) ensina o laço "rascunho, render, olhar,
  refinar": siga o mesmo laço nos shaders.
- **pixel-splats** (Apache-2.0, patch-gg/pixel-splats): dioramas de splats gaussianos em WebGL2 com luz por
  splat (chave com shadow map, céu/chão, até 4 pontuais, rim, especular) e passe de pixel. Use como
  referência de **rig de luz em miniatura** e de luz que conta a cena (fogueira iluminando a raposa, farol
  varrendo o mar); `tools/film.mjs` filma uma cena em MP4 para estudo.
- Registre todo FX importado em `assets/manifest.yaml` com fonte e licença (`ORIGEM.json` do pack).
