---
name: factory
description: "Skill principal da fábrica de vídeo com IA: cria e conduz um projeto de vídeo para YouTube (longo ou Shorts) e TikTok, do tema à entrega. Shorts/TikTok ANIMADOS (cutout 2D, mygo); vídeo longo de 10-15 min ESTÁTICO com tema (imagens do Google Flow/Nano Banana via MCP, zoom leve, cards, HyperFrames) com esquemática de hook rate. Travas automáticas: roteiro-check, edl-check e QA estrito. Use quando o usuário quiser começar um vídeo novo, continuar um projeto da fábrica (factory.yaml), ver o status, ou gerar shorts a partir de um vídeo longo."
argument-hint: "novo <nome> --formato yt-long|yt-short|tiktok[,...] --motor <motor> | status | continuar"
---

# Video Factory — skill principal

Ferramental JS/TS: siga [Bun e tsgo/tsrs](../factory/references/ferramental.md) ao executar ou alterar scripts e templates.

Você é o diretor de produção. Esta skill abre o projeto, decide qual skill de etapa usar e mantém `factory.yaml` em dia. As etapas vivem em skills irmãs do plugin, e você as carrega quando chegar a vez delas:

| Etapa | Skill | Saída principal |
|---|---|---|
| Canal (uma vez + semanal) | `video-factory:canal` | `canal.yaml`, pauta com nota, kit (voz, elenco, cenários), métricas |
| 0. Referência (opcional) | `video-factory:analise` | `ref/<slug>/analysis.md` + sheets |
| 1. Roteiro | `video-factory:roteiro` | `SCRIPT.md`, `roteiro/roteiro.txt`, `roteiro/beats.json` |
| 2. Assets + voz | `video-factory:assets` | `assets/manifest.yaml`, `public/…`, `data/words.json` |
| 2b. Imagens do Flow (motor `estatico`) | `video-factory:assets` | lotes de 24 imagens via MCP `google-flow` → curadoria → `public/img/planos/` |
| 3a. Elenco | `video-factory:animacao` | `public/sprites/<pack>/` (expressões + piscada + ciclos) |
| 3b. Cenários | `video-factory:animacao` | `public/img/plates/` (fundos sem personagens) |
| 3c. Animação | `video-factory:animacao` | `data/edl.json` com cenas `scene` (ou perfil `estatico`), aprovado pelo `edl-check` |
| 3d. Câmera, luz e FX | `video-factory:animacao` | câmera contínua, rig de luz + cues, FX de shader/pixelfx (agentes próprios) |
| 4. Edição, export e QA | `video-factory:edicao` | `out/final_<formato>.mp4`, `out/qa_<formato>/qa.md` |
| 5. Rascunho, postagem e agenda | `video-factory:edicao` | `vf publish` (rascunho; posta/agenda só se o usuário pedir) |
| 6. Métricas e metadata | `video-factory:metricas` | `vf analytics` → relatório, `meta_<ID>.json`, lições para o roteiro |

Caminhos do plugin: `${CLAUDE_PLUGIN_ROOT}` (no shell, use o valor absoluto). A CLI fica em `VF=${CLAUDE_PLUGIN_ROOT}/bin/vf`. Se ela não existir, compile com `${CLAUDE_PLUGIN_ROOT}/scripts/build.sh` (precisa de Rust/cargo). Para usar fora do plugin (terminal do usuário), `${CLAUDE_PLUGIN_ROOT}/scripts/install.sh` põe `vf`, `vf-mygo`, `vf-novo` e `vf-render-hf` no PATH (`~/.local/bin`). Rode `$VF doctor` uma vez por sessão para checar ffmpeg, whisper-cli, yt-dlp e Bun.

## Conceitos cartoon distribuídos por etapa
O conteúdo de `cartoon-3d-short` foi integrado às skills da fábrica: análise de look (`analise`), história de causa/efeito (`roteiro`), desenho e som procedural (`assets`), cartoon 2D e cards em mundo 3D (`animacao`), revisão/render/QA (`edicao`) e identidade de canal (`canal`). Cada skill aponta para a referência de sua etapa; não depende de uma skill externa instalada.

O template local é uma demonstração técnica da ponte: reutilize o motor e crie história, layout, elenco e tempos próprios. As preferências daquele exemplo (inglês, ausência de legendas, ganho de música e modelo de revisão) não substituem o `factory.yaml` nem pedidos do usuário. Origem e licença: `skills/animacao/templates/remotion-three/ORIGEM.md`.

## Entrada pelo terminal: `vf init` (fluxo do usuário)

O usuário pode começar sem a sessão: dentro da pasta do projeto,
```
vf init type="short" platform="yt,tiktok"      # ou type=long, ou type="short,long" (longo + cortes)
vf auth platform="yt,tiktok"                   # opcional: rascunho na plataforma e analíticos
claude                                         # e pede o vídeo; esta skill assume daqui
```
- `type` × `platform` → `formatos`: long+yt = `yt-long`; short+yt = `yt-short`; short+tiktok = `tiktok`.
  Longo só no YouTube; com tiktok, o TikTok recebe cortes. Motor padrão: `estatico` com longo, `cutout-2d` sem.
  Opções: `motor=`, `idioma=`, `duracao=`, `canal=`, `dir=`, `--install`, `--offline`.
- O `vf init` roda o `novo.sh --existente` (nunca sobrescreve arquivos do usuário), grava `plataformas` no
  `factory.yaml`, copia o som procedural para `audio/procedural/`, liga `ref/fx/pixelfx` e `ref/fx/pixel-splats`
  e cria `.claude/settings.json` (plugin habilitado) e um `CLAUDE.md` curto.
- Se o projeto já tem `factory.yaml`, siga direto para `continuar`. Plataforma sem `vf auth` não bloqueia
  nada: só o rascunho e os analíticos ficam indisponíveis (`vf auth status`).

## Comandos

### `novo <nome>`
0. Se o vídeo é de um canal da fábrica, leia o `canal.yaml` (formato, motor, voz, template de roteiro, marcas próprias) e use a ideia de maior nota da `pauta.md`. Sem canal definido para uma série, sugira a skill `canal` primeiro.
1. Pergunte só o que falta, numa única pergunta: **formato(s)** (`yt-long`, `yt-short`, `tiktok`; dá para combinar, ex. `yt-short,tiktok`), **tema** e **motor** (ver tabela abaixo). Se o usuário já disse, não pergunte.
2. Crie o projeto:
   ```
   ${CLAUDE_PLUGIN_ROOT}/scripts/novo.sh ~/Projects/<nome> --formato <f1,f2> --motor <motor> --idioma pt [--canal ~/Projects/canais/<canal>] --install
   ```
   O motor padrão é `cutout-2d`, e o `novo.sh` já copia o exportador de sprites e escolhe `renderer: mygo` quando `bin/vf-mygo` existe. O `--install` (Bun) é necessário para exportar sprites (Remotion headless) e para o renderer Remotion.
3. Preencha `estilo`, `publico` e `handle` no `factory.yaml` com o que souber (com canal, venha do `canal.yaml`) e siga para a etapa 0 ou 1.

### `status`
Leia `factory.yaml` e mostre numa linha por etapa o estado e o próximo passo concreto.

### `continuar`
Retome da primeira etapa `pendente` ou `em_andamento`, carregando a skill da etapa.

### `recorte` (longo → shorts)
Depois de entregue um `yt-long`, carregue `video-factory:edicao` e siga `templates/recorte.md`. Cada corte entra em `derivados` no `factory.yaml`.

## Modo esteira (sub-agentes)
Para shorts com 12+ planos, vídeos longos, ou vários vídeos na semana, produza pela esteira de `templates/esteira.md`:
1. Depois do `vf align`: `bun ${CLAUDE_PLUGIN_ROOT}/skills/factory/scripts/planos.mjs --lote 5` gera os **cartões de plano** (`roteiro/planos/sNN.json`: janela de tempo, palavras, visual, cenário, critérios, saída). `etapas.cartoes: feito`.
2. Passe 0 (animatic) para o vídeo inteiro já renderizar. Depois suba os cartões de passe: layout → animação → luz → fx → som → final.
3. Em paralelo: sub-agentes de **elenco**, **cenários** e **som**; depois um **animador por lote** de cartões contíguos, o **diretor de câmera** (vídeo inteiro), o **iluminador** e o **artista de FX**. Toda correção pedida passa pelo **continuísta** (esteira, seção 3b: um agente dono por responsabilidade de qualidade). Cada um escreve só em `roteiro/cenas/sNN.py` e nos arquivos do seu posto (`load_parts()` no `cenas.py` junta tudo). Use o briefing padrão do template.
4. Um **revisor** independente (sem acesso ao código) confere stills e folha densa contra os critérios de cada cartão.
5. Só o orquestrador mexe em `factory.yaml`, `edl.json` final, render e commits. `planos.mjs status` mostra o quadro.

Referências de vocabulário para os briefings: `skills/animacao/templates/glossario-fx.md` e `skills/roteiro/templates/glossario-roteiro.md`.

## Motores

| Motor | Quando | Como renderiza |
|---|---|---|
| **`cutout-2d`** (padrão) | qualquer vídeo animado com personagens e props: explicativos, histórias, curiosidades | sprites (expressões, piscada, ciclos) sobre plates, em cenas `scene` no `edl.json`, com mygo |
| **`estatico`** (padrão do `yt-long`) | vídeo longo de 10-15 min com tema: imagens caprichadas, zoom-in leve, cards, acabamento de filme | imagens do Google Flow (MCP) + `estatico.py` → `edl.json` perfil `estatico` → **HyperFrames** |
| `remotion-2d` | motion graphics, texto cinético, infográficos, combinado com cutout | template Remotion/mygo orientado a `data/edl.json` |
| `ia-video` | planos gerados por Veo/Kling/Runway etc. (image-to-video a partir do elenco e dos plates) | prompts por plano (`data/shots.yaml`) → clipes `video` |
| `kenburns` | **só** quando o usuário pede um vídeo de fotos (documentário de arquivo) | clipes `image` com `motion`; não é animação |
| `remotion-three` | personagens 2D cartoon em mundo 3D com nanquim | template local `skills/animacao/templates/remotion-three/`, Remotion + three.js; guia `skills/animacao/references/2d-in-3d-world.md` |
| `blender-3d` (planejado) | **só `yt-short`/`tiktok`**: elenco cutout em cards num mundo 3D PBR (estilo de alta qualidade estudado) | especificação em `skills/animacao/templates/mundo-3d-blender.md`; nunca em `yt-long` |
| `mapas-3d` | curiosidades de geografia/história em mapa de satélite 3D | **delegue à skill `mapas-3d-videos-virais`** |

O renderer é plugável: roteiro, assets e edição só trocam arquivos neutros (`beats.json`, `words.json`, `edl.json`). Há três renderers prontos para o `edl.json`:
- **`mygo`** (`bin/vf-mygo`, Go + Metal): **padrão para `cutout-2d`, `remotion-2d` e `ia-video`**. Cerca de 5x mais rápido que o Remotion e ~40x menos CPU. Exemplo: um short animado (110 s, 33 cenas, 141 atores) renderiza em 30 s em 2K.
- **`hyperframes`** (HTML + GSAP, Chrome headless): **padrão para vídeo longo** (`estatico`, e `kenburns`/`ia-video` em `yt-long`). Cards tipográficos nítidos, transições, grão/vinheta/graduação. 60 s em 39 s no M4; paridade com o mygo SSIM ≥ 0,99. Não desenha clipes `scene` (cutout): esses ficam no mygo.
- **`remotion`**: quando o vídeo precisa de um componente React sob medida, de Remotion Studio, ou é 3D.
Comparação e limites na skill `animacao`. Um motor novo só precisa ler o `edl.json` e gerar um MP4. Para ajustar o look a uma referência, use a skill `look-match-loop`.

## Fluxo e regras

1. **Referência** (se o usuário mandou links ou citou um canal): rode `video-factory:analise` em 1-3 vídeos e anote em `factory.yaml > referencias` o que copiar de **fórmula** (ritmo, gancho, estrutura). Nunca copie roteiro, personagens, marca ou planos.
2. **Roteiro**: gere com `video-factory:roteiro`. No `yt-long`, o `roteiro-check.mjs` precisa sair 0 **antes** de mostrar ao usuário (esquemática de hook rate). Depois **pare para aprovação**. É o único gate humano obrigatório (`etapas.aprovacao_roteiro`).
2b. **Animação de verdade (trava obrigatória):** o vídeo precisa ser animado. Elenco em sprites, cenários e cenas `scene` com ações amarradas às palavras, pela skill `animacao`. `etapas.animacao` só vira `feito` quando:
   - `bun ${CLAUDE_PLUGIN_ROOT}/skills/animacao/scripts/edl-check.mjs data/edl.json` sai com código 0 (≥ 70% do tempo com animação real, nenhum trecho > 6 s sem);
   - a folha de stills de todas as cenas foi conferida.

   Stills com Ken Burns, cartelas e o animatic do `beats2edl` **não** são animação. Erro já cometido e que não pode se repetir: um short de história saiu só com Ken Burns e foi marcado `animacao: feito`.

   **Exceção única: motor `estatico`** (só `yt-long`, vídeo de 10-15 min com tema). Aqui o movimento é o zoom/pan leve, e a trava é o perfil estático do mesmo `edl-check` (lido do `edl.json`): todo still se mexe num ritmo perceptível, planos de 1,5-9 s, gancho com plano médio ≤ 3,2 s, interrupção de padrão a cada ≤ 40 s, nenhuma imagem repetida em 120 s, cards de capítulo e acabamento (grão + vinheta). O capricho fica nas imagens (style bible + referências fixas no Flow) e no roteiro (`roteiro-check.mjs`).
3. Depois do roteiro aprovado, **siga sem pedir confirmação** por assets, voz, animação, edição e QA. Só pare para algo que dependa do usuário: gerar a voz no ElevenLabs sem API key, login no Google Flow (MCP), credenciais, ou uma decisão criativa nova.
3b. **Cota do Google Flow** (motor `estatico`): 20 prompts/dia, **sempre 24 imagens por prompt** (480/dia). Controle global em `~/.cache/vf/flow-cota.json` pelo `flow-lotes.mjs`. Cota esgotada = `etapas.assets: aguardando_cota` + data de retomada em `notas`, e pare. O `continuar` retoma no outro dia.
4. **Ao fim de cada etapa**: atualize `etapas.<etapa>` em `factory.yaml`, salve tudo em disco, **faça um commit no repositório do projeto** (`git add -A && git commit -m "<etapa>: <o que ficou pronto>"`; o `.gitignore` do projeto já deixa de fora renders e node_modules) e diga numa linha o que ficou pronto.
5. **Multi-formato**: um roteiro só por formato principal. Shorts e TikTok podem dividir o mesmo vídeo vertical, mas exporte e rode o QA por formato (safe zones e durações diferem). Um yt-long nunca vira short por reenquadramento cego: use `recorte`.
6b. **Publicação/analíticos**: com `plataformas` e `vf auth` feitos, pergunte uma vez **como** subir: rascunho (padrão), postar agora ou agendar (data/hora), e em qual conta (`vf publish`, ver `skills/edicao/references/plataformas.md`). Postar ou agendar só com resposta explícita do usuário. Antes de subir, a skill `metricas` (fluxo `metadata`) pode revisar título/descrição/tags. Depois de publicado, a skill `metricas` (fluxos `upload` e `retencao`) alimenta `analiticos`, o próximo roteiro e a pauta do canal.
6. **Entrega**: só com `edl-check` aprovado e `vf qa` (estrito, o padrão) saindo com código 0. WARN também reprova; a única saída é uma `qa_excecoes` com motivo concreto. Registre em `entregas` o arquivo e o resultado do QA, e entregue o caminho absoluto do MP4 junto do resumo do QA.

## MCP
O plugin declara o servidor MCP `google-flow` (Google Flow / Nano Banana, `.mcp.json`). Instalação, login e loop de geração: `skills/assets/templates/mcp-google-flow.md`. Se as ferramentas `flow_*` não estiverem disponíveis, peça ao usuário para rodar o login (`! bunx --bun @park-sang-gwon/google-flow-mcp@0.3.1 doctor`) e recarregar a sessão. Não troque para outro gerador sem perguntar.

## Specs por formato (fonte: `${CLAUDE_PLUGIN_ROOT}/formats.yaml`, ou `$VF formats`)

| | yt-long | yt-short | tiktok |
|---|---|---|---|
| Tamanho | 1920x1080 (ou 4K) | 1080x1920 (ou 1440x2560) | 1080x1920 |
| Duração | 4-60 min (alvo 12; QA estrito 8-20 min) | até 3 min (alvo 60 s; QA estrito 30-75 s) | 7 s-10 min (alvo 35 s; QA estrito 21-60 s) |
| Fala | ~160 wpm | ~222 wpm (3,7 palavras/s) | ~222 wpm |
| Loudness | -14 LUFS, -1 dBTP | -14 LUFS, -1 dBTP | -14 LUFS, -1 dBTP |
| Safe zone | — | topo 140, base 380, dir. 120 px | topo 160, base 480, dir. 140 px |

## Performance (produção em escala)

- `vf` decodifica direto no tamanho de análise e calcula tudo numa passada. Num Apple M4, 110 s de vídeo 1440x2560 levam ~4,5 s de análise e ~7 s de transcrição (whisper.cpp com Metal). Os dois rodam em paralelo dentro de `vf analyze`.
- O decode por software já é o mais rápido no Apple Silicon. `--hw` (VideoToolbox) só compensa em codecs pesados.
- Render: `vf-mygo` faz 22 s de vertical 1080p em 3,4 s, contra 16,7 s do Remotion. No Remotion, use `--concurrency` conforme os núcleos e JPEG como intermediário (já é o padrão do template). Para iterar, renderize stills em vez do vídeo inteiro.
- Cada render do Remotion copia `public/` para a pasta temp: mantenha `public/` enxuto (só o que o vídeo usa) e aponte `TMPDIR` para um disco com espaço.

## QA visual por etapa

Para vídeos animados com personagens/props ou composição 3D, conduza as revisões de
[QA visual](../edicao/references/qa-cinco-etapas.md): modelagem, animação/contato, posicionamento,
identidade estável após correções (3b), integridade dos assets, câmera e luz/FX (6). Inicie-as na produção e consolide a evidência antes da entrega.
Mantenha seus estados separados do QA técnico: ausência de preto/freeze não aprova contato,
modelo ou enquadramento. Retome defeitos concretos sem exigir nova aprovação do usuário.
