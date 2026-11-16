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
| 8 | Som | sub-agente **sonoplasta** | cartões (`sfx`, viradas) → `public/audio/` + entradas de áudio | sem ar morto, J-cuts nas viradas |
| 9 | Revisão | sub-agente **revisor** (nunca quem fez) | stills + folha densa + cartões → lista de defeitos por cartão | 0 defeitos bloqueantes |
| 10 | Render/QA/entrega | orquestrador | `edl.json` → MP4 → `vf qa` | QA estrito código 0 |

Postos 5, 6 e 8 rodam **em paralelo** depois do 4. O posto 7 começa quando os sprites e plates dos cartões do lote existem.

## 2. Passes incrementais (cada cartão sobe de passe; o vídeo inteiro sempre renderiza)
| Passe | O que existe no plano | Render de conferência |
|---|---|---|
| 0 **animatic** | cor/still + texto do visual no tempo certo da voz | vídeo inteiro em draft: ritmo e duração |
| 1 **layout** | cenário/plate + câmera + atores parados nas posições (blocking) | stills por cartão no meio do plano |
| 2 **animação** | keys, estados no `at_word`, entradas e saídas, ciclos | trecho `-from/-to` do lote + folha densa |
| 3 **fx** | FX de exagero, transições de assinatura, props-piada | trecho do lote |
| 4 **som** | SFX, música, J-cuts | vídeo com mix |
| 5 **final** | correções do revisor | `vf qa` estrito |

Regra: **o vídeo nunca quebra**. Cartão sem cena do passe atual usa a do passe anterior (o animatic preenche tudo desde o início), então qualquer render a qualquer momento é assistível. Estado em `roteiro/planos/sNN.json > passe/status`; `planos.mjs status` mostra o quadro.

## 3. Divisão do trabalho
- **Lotes contíguos:** 4-6 cartões seguidos por animador, para manter continuidade (o mesmo personagem entre planos vizinhos fica com o mesmo agente). Um short de 60 s ≈ 15-25 cartões ≈ 4 animadores em paralelo.
- **Posse de arquivo:** cada sub-agente escreve **só** nos caminhos de `saida` dos seus cartões (`roteiro/cenas/sNN.py`, `out/planos/sNN.png`) e na pasta do seu posto (`public/sprites/<pack>/`, `public/img/plates/<cenario>*`). Nada de editar `vfscene.py`, `edl.json`, `factory.yaml` ou arquivos de outro lote. Assim não há conflito e não precisa de worktree.
- **Faltou algo de outro posto** (um estado de sprite, um prop): o animador **não cria**; registra em `notas` do cartão (`"precisa: sprite heroi estado 'raiva'"`) e usa o mais próximo. O orquestrador repassa ao posto certo.
- **Montagem:** `roteiro/cenas.py` chama `init()`, `load_parts()` e `save()`: junta todas as partes na ordem dos cartões.
- **Revisor independente:** recebe só os cartões, os stills e a folha densa, nunca o código. Responde por cartão: `ok` ou defeito + correção sugerida. Quem corrige é o animador do lote.

## 4. Briefing padrão de sub-agente (copiar no prompt do Agent)
```
Você é <papel> na esteira da video-factory, projeto <pasta absoluta>.
Cartões: <lista de caminhos roteiro/planos/sNN.json>.
Leia antes: assets/style-bible.md, ${PLUGIN}/skills/animacao/templates/glossario-fx.md,
${PLUGIN}/skills/animacao/templates/edl.schema.md, <referência de estilo, se houver>.
Disponível: sprites em public/sprites/ (estados em cada sprite.json), plates em public/img/plates/.
Escreva SÓ: <caminhos de saída>. Não edite nenhum outro arquivo.
Para cada cartão: <o que fazer no passe N>. Toda ação cai numa palavra (w(i) com os índices de `palavras`).
Confira: python3 roteiro/cenas.py && node ${PLUGIN}/skills/animacao/scripts/edl-check.mjs data/edl.json
e um still por cartão (${PLUGIN}/bin/vf-mygo -project . -formato <f> -still <frame> -o out/planos/sNN.png).
Responda: arquivos escritos, saída das checagens, caminho dos stills e o que ficou em `notas`.
```

## 5. Ritmo da esteira (cadência de canal)
- Um short por dia é viável com a esteira: pesquisa + roteiro de manhã, voz + cartões, postos 5-8 em paralelo, revisão e render.
- Trabalhe **2-3 vídeos em estágios diferentes** ao mesmo tempo (um no roteiro, um na animação, um no QA): a esteira nunca fica parada esperando aprovação humana.
- A cota de imagens do Flow (20 prompts/dia) é compartilhada: o orquestrador distribui os lotes entre os vídeos da semana.
