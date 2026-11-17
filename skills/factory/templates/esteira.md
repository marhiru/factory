# Esteira de produção com sub-agentes (construção incremental)

A fábrica produz como um estúdio de animação: **passes incrementais** sobre **cartões de plano**, com o trabalho dividido entre sub-agentes que só tocam nos arquivos que recebem. O orquestrador (a skill `factory`, na sessão principal) é o único que mexe no `factory.yaml`, no `edl.json` final e nos commits.

## 1. Postos da esteira
| # | Posto | Quem | Entrada → Saída | Trava |
|---|---|---|---|---|
| 0 | Pauta | orquestrador + `canal` | `canal.yaml` + banco de ideias → tema escolhido | nota ≥ 7 na pauta |
| 1 | Pesquisa | sub-agente **pesquisador** | tema → fatos com fonte no `SCRIPT.md` | toda afirmação com fonte |
| 2 | Roteiro | orquestrador | fatos → `roteiro.txt` + `beats.json` | `roteiro-check` (longo) + **aprovação humana** |
| 3 | Voz | orquestrador | `roteiro.txt` → `voice.wav` → `words.json` → `vf align` | `CONFERIR` = 0 |
| 4 | Cartões | orquestrador | `scripts/planos.mjs` → `roteiro/planos/sNN.json` | todo cartão com janela ≥ 0,5 s |
| 5 | Elenco | sub-agente **elenco** | lista de personagens/props dos cartões → `public/sprites/` | mínimo de estados por personagem |
| 6 | Cenários | sub-agentes **cenógrafos** (1 por grupo de cenários) | `cenario` dos cartões → plates / imagens / sets 3D | still de cada cenário aprovado |
| 7 | Cenas | sub-agentes **animadores** (1 por lote de cartões) | cartões do lote → `roteiro/cenas/sNN.py` + stills | critérios do cartão + `edl-check` |
| 7a | Câmera | sub-agente **diretor de câmera** | cartões do vídeo inteiro → `camera` de cada cena + `notas` de continuidade | trechos contínuos sem corte (zoom-out → zoom-in); margens (QA etapa 5) |
| 7b | Iluminação | sub-agente **iluminador** | cenários + cartões → rig por cenário + cues `luz:` por cartão | harmonia e ênfase (QA etapa 6) |
| 7c | FX | sub-agente **artista de FX** | cartões com `fx` → packs `public/sprites/fx-*` ou `ShaderFx` no 3D | régua de FX (QA etapa 6) |
| 8 | Som | sub-agente **sonoplasta** | cartões (`sfx`, viradas) → `public/audio/` + entradas de áudio | sem ar morto, J-cuts nas viradas |
| 8b | Continuidade | sub-agente **continuísta** | todo pedido de correção + packs do elenco → veredito antes × depois | mesmo asset, só o atributo pedido mudou (QA etapa 3b) |
| 9 | Revisão | sub-agente **revisor** (nunca quem fez) | stills + folha densa + cartões → lista de defeitos por cartão | 0 defeitos bloqueantes |
| 10 | Render/QA/entrega | orquestrador | `edl.json` → MP4 → `vf qa` | QA estrito código 0 |

Postos 5, 6 e 8 rodam **em paralelo** depois do 4. O posto 7 começa quando os sprites e plates dos cartões do lote existem. O 7a roda uma vez sobre o vídeo inteiro (continuidade de câmera entre lotes); 7b e 7c rodam em paralelo depois do passe de animação. O 8b entra sempre que houver correção pedida.

## 2. Passes incrementais (cada cartão sobe de passe; o vídeo inteiro sempre renderiza)
| Passe | O que existe no plano | Render de conferência |
|---|---|---|
| 0 **animatic** | cor/still + texto do visual no tempo certo da voz | vídeo inteiro em draft: ritmo e duração |
| 1 **layout** | cenário/plate + câmera + atores parados nas posições (blocking) | stills por cartão no meio do plano |
| 2 **animação** | keys, estados no `at_word`, entradas e saídas, ciclos | trecho `-from/-to` do lote + folha densa |
| 3 **luz** | rig por cenário, cues de ênfase (spot, rays, exposição, temperatura) nas palavras | stills antes/depois do cue |
| 4 **fx** | FX de alta qualidade (shaders/pixelfx), transições de assinatura, props-piada | trecho do lote + `FxLab` |
| 5 **som** | SFX, música, J-cuts | vídeo com mix |
| 6 **final** | correções do revisor (passando pelo continuísta) | `vf qa` estrito |

Regra: **o vídeo nunca quebra**. Cartão sem cena do passe atual usa a do passe anterior (o animatic preenche tudo desde o início), então qualquer render a qualquer momento é assistível. Estado em `roteiro/planos/sNN.json > passe/status`; `planos.mjs status` mostra o quadro.

## 3. Divisão do trabalho
- **Lotes contíguos:** 4-6 cartões seguidos por animador, para manter continuidade (o mesmo personagem entre planos vizinhos fica com o mesmo agente). Um short de 60 s ≈ 15-25 cartões ≈ 4 animadores em paralelo.
- **Posse de arquivo:** cada sub-agente escreve **só** nos caminhos de `saida` dos seus cartões (`roteiro/cenas/sNN.py`, `out/planos/sNN.png`) e na pasta do seu posto (`public/sprites/<pack>/`, `public/img/plates/<cenario>*`). Nada de editar `vfscene.py`, `edl.json`, `factory.yaml` ou arquivos de outro lote. Assim não há conflito e não precisa de worktree.
- **Faltou algo de outro posto** (um estado de sprite, um prop): o animador **não cria**; registra em `notas` do cartão (`"precisa: sprite heroi estado 'raiva'"`) e usa o mais próximo. O orquestrador repassa ao posto certo.
- **Montagem:** `roteiro/cenas.py` chama `init()`, `load_parts()` e `save()`: junta todas as partes na ordem dos cartões.
- **Revisor independente:** recebe só os cartões, os stills e a folha densa, nunca o código. Responde por cartão: `ok` ou defeito + correção sugerida. Quem corrige é o animador do lote.

## 3b. Um agente por responsabilidade de qualidade

Cada ponto de qualidade tem dono. Quem faz não aprova; o revisor confere pela etapa indicada do
[QA visual](../../edicao/references/qa-cinco-etapas.md).

| Responsabilidade | Agente dono | Regra | Etapa do QA |
|---|---|---|---|
| Posicionamento coeso dos assets | **cenógrafo** (layout do passe 1) + animador do lote | nada mal posicionado, quebrando a harmonia ou tapando a cena | 3 |
| Identidade estável ao corrigir | **continuísta** | correção mexe só no pedido, no mesmo pack; antes × depois lado a lado | 3b |
| Câmera entre trechos | **diretor de câmera** | trechos contínuos e semelhantes: sem corte, zoom-out → zoom-in no mesmo cenário | 5 |
| Luz de cenário (harmonia + ênfase) | **iluminador** | rig único por cenário; cues dinâmicos nas palavras | 6 |
| Qualidade dos FX e shaders | **artista de FX** | antecipação → pico → dissipação; núcleo, borda, secundário; ilumina o entorno | 6 |

Fluxo de uma correção pedida pelo usuário: orquestrador → animador do lote (aplica só o pedido) →
**continuísta** (compara `out/planos/sNN.png` antes × depois e roda `bun ${PLUGIN}/skills/animacao/scripts/elenco-lock.mjs check
--pedido posicao|pose|novo-estado`, que precisa sair 0; o `snapshot` é feito quando o elenco é aprovado) → revisor. Se o continuísta reprovar, a correção volta; nunca
se troca o asset por outro parecido.

Briefings específicos (acrescente ao padrão abaixo):
- **diretor de câmera:** "Leia todos os cartões em ordem. Comece pelos `continuo_sugerido` (mesmo cenário em
  cartões vizinhos): confirme em `continuo_com` ou recuse com motivo. Para cada par confirmado, junte numa `scene`
  só com `cam=junta(...)` (câmera `from → via → to`): zoom-out do enquadramento atual e zoom-in no novo assunto (6-14 frames cada perna,
  ease in-out). Mudando o ângulo no mesmo lugar: dissolve de 0,2-0,3 s com a câmera andando no mesmo sentido,
  chicote, atravessar porta ou cortina de poeira. Corte seco em no máximo 25 % das transições. Corte só onde muda lugar, tempo ou assunto. Escreva só
  `camera` nos cartões e a lista de junções em `notas`."
- **iluminador:** "Leia `${PLUGIN}/skills/animacao/references/luz-fx-shaders.md` (inclui as receitas medidas).
  Escreva primeiro o arco de temperatura por ato; depois o rig de cada cenário (chave, preenchimento, rim,
  práticos, ar), presos ao mundo e iguais em todos os ângulos; depois os cues `luz:` por cartão, cada um numa
  palavra, com ênfase quente sobre ambiente frio. Noite com luma ≥ 0,26. Entregue stills
  antes/depois de cada cue."
- **artista de FX:** "Leia a mesma referência, seção FX e shaders. Procure primeiro em `ref/fx/pixelfx`
  (timing/composição) e no `ShaderFx`. Entregue cada FX com 3 stills (antecipação, pico, dissipação), o pack
  ou o código, e o registro de licença no manifest."
- **continuísta:** "Você recebe o pedido do usuário, os stills antes/depois e os packs. Responda `ok` só se
  o asset é o mesmo (rosto, cabelo, roupa, cores, proporção, acessórios) e só o atributo pedido mudou."

## 4. Briefing padrão de sub-agente (copiar no prompt do Agent)
```
Você é <papel> na esteira da video-factory, projeto <pasta absoluta>.
Cartões: <lista de caminhos roteiro/planos/sNN.json>.
Leia antes: assets/style-bible.md, ${PLUGIN}/skills/animacao/templates/glossario-fx.md,
${PLUGIN}/skills/animacao/templates/edl.schema.md, <referência de estilo, se houver>.
Disponível: sprites em public/sprites/ (estados em cada sprite.json), plates em public/img/plates/.
Escreva SÓ: <caminhos de saída>. Não edite nenhum outro arquivo.
Para cada cartão: <o que fazer no passe N>. Toda ação cai numa palavra (w(i) com os índices de `palavras`).
Confira: python3 roteiro/cenas.py && bun ${PLUGIN}/skills/animacao/scripts/edl-check.mjs data/edl.json
e um still por cartão (${PLUGIN}/bin/vf-mygo -project . -formato <f> -still <frame> -o out/planos/sNN.png).
Responda: arquivos escritos, saída das checagens, caminho dos stills e o que ficou em `notas`.
```

## 5. Ritmo da esteira (cadência de canal)
- Um short por dia é viável com a esteira: pesquisa + roteiro de manhã, voz + cartões, postos 5-8 em paralelo, revisão e render.
- Trabalhe **2-3 vídeos em estágios diferentes** ao mesmo tempo (um no roteiro, um na animação, um no QA): a esteira nunca fica parada esperando aprovação humana.
- A cota de imagens do Flow (20 prompts/dia) é compartilhada: o orquestrador distribui os lotes entre os vídeos da semana.
