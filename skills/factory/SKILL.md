---
name: factory
description: "Skill principal da fábrica de vídeo com IA: cria e conduz um projeto de vídeo ANIMADO para YouTube (longo ou Shorts) e TikTok, do tema à entrega (referência → roteiro → voz → elenco/sprites → cenários → animação cutout → edição → QA), com travas que impedem entregar sem animação. Use quando o usuário quiser começar um vídeo novo, continuar um projeto da fábrica (factory.yaml), ver o status, ou gerar shorts a partir de um vídeo longo."
argument-hint: "novo <nome> --formato yt-long|yt-short|tiktok[,...] --motor <motor> | status | continuar"
---

# Video Factory — skill principal

Você é o diretor de produção. Esta skill abre o projeto, decide qual skill de etapa usar e mantém `factory.yaml` em dia. As etapas vivem em skills irmãs do plugin, e você as carrega quando chegar a vez delas:

| Etapa | Skill | Saída principal |
|---|---|---|
| 0. Referência (opcional) | `video-factory:analise` | `ref/<slug>/analysis.md` + sheets |
| 1. Roteiro | `video-factory:roteiro` | `SCRIPT.md`, `roteiro/roteiro.txt`, `roteiro/beats.json` |
| 2. Assets + voz | `video-factory:assets` | `assets/manifest.yaml`, `public/…`, `data/words.json` |
| 3a. Elenco | `video-factory:animacao` | `public/sprites/<pack>/` (expressões + piscada + ciclos) |
| 3b. Cenários | `video-factory:animacao` | `public/img/plates/` (fundos sem personagens) |
| 3c. Animação | `video-factory:animacao` | `data/edl.json` com cenas `scene`, aprovado pelo `edl-check` |
| 4. Edição, export e QA | `video-factory:edicao` | `out/final_<formato>.mp4`, `out/qa_<formato>/qa.md` |

Caminhos do plugin: `${CLAUDE_PLUGIN_ROOT}` (no shell, use o valor absoluto). A CLI fica em `VF=${CLAUDE_PLUGIN_ROOT}/bin/vf`. Se ela não existir, compile com `${CLAUDE_PLUGIN_ROOT}/scripts/build.sh` (precisa de Rust/cargo). Rode `$VF doctor` uma vez por sessão para checar ffmpeg, whisper-cli, yt-dlp e node.

## Comandos

### `novo <nome>`
1. Pergunte só o que falta, numa única pergunta: **formato(s)** (`yt-long`, `yt-short`, `tiktok`; dá para combinar, ex. `yt-short,tiktok`), **tema** e **motor** (ver tabela abaixo). Se o usuário já disse, não pergunte.
2. Crie o projeto:
   ```
   ${CLAUDE_PLUGIN_ROOT}/scripts/novo.sh ~/Projects/<nome> --formato <f1,f2> --motor <motor> --idioma pt --install
   ```
   O motor padrão é `cutout-2d`, e o `novo.sh` já copia o exportador de sprites e escolhe `renderer: mygo` quando `bin/vf-mygo` existe. O `--install` (npm) é necessário para exportar sprites (Remotion headless) e para o renderer Remotion.
3. Preencha `estilo`, `publico` e `canal` no `factory.yaml` com o que souber e siga para a etapa 0 ou 1.

### `status`
Leia `factory.yaml` e mostre numa linha por etapa o estado e o próximo passo concreto.

### `continuar`
Retome da primeira etapa `pendente` ou `em_andamento`, carregando a skill da etapa.

### `recorte` (longo → shorts)
Depois de entregue um `yt-long`, carregue `video-factory:edicao` e siga `templates/recorte.md`. Cada corte entra em `derivados` no `factory.yaml`.

## Motores

| Motor | Quando | Como renderiza |
|---|---|---|
| **`cutout-2d`** (padrão) | qualquer vídeo animado com personagens e props: explicativos, histórias, curiosidades | sprites (expressões, piscada, ciclos) sobre plates, em cenas `scene` no `edl.json`, com mygo |
| `remotion-2d` | motion graphics, texto cinético, infográficos, combinado com cutout | template Remotion/mygo orientado a `data/edl.json` |
| `ia-video` | planos gerados por Veo/Kling/Runway etc. (image-to-video a partir do elenco e dos plates) | prompts por plano (`data/shots.yaml`) → clipes `video` |
| `kenburns` | **só** quando o usuário pede um vídeo de fotos (documentário de arquivo) | clipes `image` com `motion`; não é animação |
| `remotion-three` | personagens 2D cartoon em mundo 3D com nanquim | **delegue à skill `cartoon-3d-short`** |
| `mapas-3d` | curiosidades de geografia/história em mapa de satélite 3D | **delegue à skill `mapas-3d-videos-virais`** |

O renderer é plugável: roteiro, assets e edição só trocam arquivos neutros (`beats.json`, `words.json`, `edl.json`). Há dois renderers prontos para o `edl.json`:
- **`mygo`** (`bin/vf-mygo`, Go + Metal): **padrão para `cutout-2d`, `remotion-2d` e `ia-video`**. Cerca de 5x mais rápido que o Remotion e ~40x menos CPU. Exemplo: um short animado (110 s, 33 cenas, 141 atores) renderiza em 30 s em 2K.
- **`remotion`**: quando o vídeo precisa de um componente React sob medida, de Remotion Studio, ou é 3D.
Comparação e limites na skill `animacao`. Um motor novo só precisa ler o `edl.json` e gerar um MP4. Para ajustar o look a uma referência, use a skill `look-match-loop`.

## Fluxo e regras

1. **Referência** (se o usuário mandou links ou citou um canal): rode `video-factory:analise` em 1-3 vídeos e anote em `factory.yaml > referencias` o que copiar de **fórmula** (ritmo, gancho, estrutura). Nunca copie roteiro, personagens, marca ou planos.
2. **Roteiro**: gere com `video-factory:roteiro` e **pare para aprovação**. É o único gate humano obrigatório (`etapas.aprovacao_roteiro`).
2b. **Animação de verdade (trava obrigatória):** o vídeo precisa ser animado. Elenco em sprites, cenários e cenas `scene` com ações amarradas às palavras, pela skill `animacao`. `etapas.animacao` só vira `feito` quando:
   - `node ${CLAUDE_PLUGIN_ROOT}/skills/animacao/scripts/edl-check.mjs data/edl.json` sai com código 0 (≥ 70% do tempo com animação real, nenhum trecho > 6 s sem);
   - a folha de stills de todas as cenas foi conferida.

   Stills com Ken Burns, cartelas e o animatic do `beats2edl` **não** são animação. Erro já cometido e que não pode se repetir: um short de história saiu só com Ken Burns e foi marcado `animacao: feito`.
3. Depois do roteiro aprovado, **siga sem pedir confirmação** por assets, voz, animação, edição e QA. Só pare para algo que dependa do usuário: gerar a voz no ElevenLabs sem API key, credenciais, ou uma decisão criativa nova.
4. **Ao fim de cada etapa**: atualize `etapas.<etapa>` em `factory.yaml`, salve tudo em disco e diga numa linha o que ficou pronto.
5. **Multi-formato**: um roteiro só por formato principal. Shorts e TikTok podem dividir o mesmo vídeo vertical, mas exporte e rode o QA por formato (safe zones e durações diferem). Um yt-long nunca vira short por reenquadramento cego: use `recorte`.
6. **Entrega**: só com `edl-check` aprovado e `vf qa` sem `FAIL`. Registre em `entregas` o arquivo e o resultado do QA, e entregue o caminho absoluto do MP4 junto do resumo do QA.

## Specs por formato (fonte: `${CLAUDE_PLUGIN_ROOT}/formats.yaml`, ou `$VF formats`)

| | yt-long | yt-short | tiktok |
|---|---|---|---|
| Tamanho | 1920x1080 (ou 4K) | 1080x1920 (ou 1440x2560) | 1080x1920 |
| Duração | 4-60 min (alvo 12) | até 3 min (alvo 60 s) | 7 s-10 min (alvo 35 s) |
| Fala | ~160 wpm | ~222 wpm (3,7 palavras/s) | ~222 wpm |
| Loudness | -14 LUFS, -1 dBTP | -14 LUFS, -1 dBTP | -14 LUFS, -1 dBTP |
| Safe zone | — | topo 140, base 380, dir. 120 px | topo 160, base 480, dir. 140 px |

## Performance (produção em escala)

- `vf` decodifica direto no tamanho de análise e calcula tudo numa passada. Num Apple M4, 110 s de vídeo 1440x2560 levam ~4,5 s de análise e ~7 s de transcrição (whisper.cpp com Metal). Os dois rodam em paralelo dentro de `vf analyze`.
- O decode por software já é o mais rápido no Apple Silicon. `--hw` (VideoToolbox) só compensa em codecs pesados.
- Render: `vf-mygo` faz 22 s de vertical 1080p em 3,4 s, contra 16,7 s do Remotion. No Remotion, use `--concurrency` conforme os núcleos e JPEG como intermediário (já é o padrão do template). Para iterar, renderize stills em vez do vídeo inteiro.
- Cada render do Remotion copia `public/` para a pasta temp: mantenha `public/` enxuto (só o que o vídeo usa) e aponte `TMPDIR` para um disco com espaço.
