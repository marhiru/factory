# Referência de comandos do `vf`

Guia completo da CLI da fábrica: o que cada comando faz, todas as opções, exemplos, o que sai e os cuidados.
Resumo rápido no [README](README.md#vf); detalhes de contas e plataformas em
[plataformas.md](skills/edicao/references/plataformas.md). `vf <comando> --help` mostra a ajuda curta de cada um.

**Sumário**
- [Convenções](#convenções)
- [Instalação e diagnóstico](#instalação-e-diagnóstico): `doctor`
- [Projeto](#projeto): `init`
- [Análise de vídeo de referência](#análise-de-vídeo-de-referência): `analyze` · `probe` · `frames` · `keyframes` · `sheet` · `transcribe`
- [Voz e tempo](#voz-e-tempo): `voz` · `transcribe` · `align`
- [Áudio, export e QA](#áudio-export-e-qa): `mix` · `export` · `qa` · `formats`
- [Contas nas plataformas](#contas-nas-plataformas): `auth`
- [Publicação](#publicação): `publish` · `agenda` · `meta`
- [Métricas](#métricas): `analytics` · `relatorio` · `studio`
- [Receitas](#receitas)
- [Onde ficam os arquivos](#onde-ficam-os-arquivos) · [Variáveis de ambiente](#variáveis-de-ambiente) · [Erros comuns](#erros-comuns)

---

## Convenções

**Dois estilos de argumento.**
- Comandos de mídia (`analyze`, `frames`, `qa`, `export`…) usam flags: `vf qa final.mp4 -f yt-short --lenient`.
- Comandos de projeto e plataforma (`init`, `auth`, `publish`, `analytics`, `relatorio`…) usam
  **`chave=valor`**: `vf publish final.mp4 platform=yt mode=publico`. Também aceitam `--chave valor`,
  `--chave=valor` e `--flag` (vale `1`). Valor com espaço vai entre aspas: `title="Meu título"`.
  Listas são separadas por vírgula: `platform=yt,tiktok`, `account=a,b`, `video=ID1,ID2`.

**Plataformas:** `yt` (ou `youtube`), `tiktok` (ou `tt`), `tiktok-biz` (ou `ttb`; só métricas).

**Qual conta é usada** (publish, analytics, relatorio, meta): `account=<apelido>` → `factory.yaml > contas.<plataforma>`
do projeto da pasta atual → conta padrão (`vf auth use`) → a única existente. Com várias contas e nenhuma
padrão, o comando pede para escolher. `account=all` (analytics/relatorio) roda em todas.

**Pasta de saída:** dentro de um projeto (existe `out/`), as saídas vão para `out/<pasta>`; fora dele, para
`./<pasta>`. `o=<pasta>` (ou `out=`) troca o destino.

**Código de saída:** `0` = ok; `1` = QA reprovado (`vf qa`) ou parte das contas falhou (`analytics`); `2` = erro.

---

## Instalação e diagnóstico

```bash
./scripts/build.sh        # compila bin/vf (Rust) e bin/vf-mygo (Go); atualiza os links em ~/.local/bin
./scripts/install.sh      # põe vf, vf-mygo, vf-novo e vf-render-hf no PATH (~/.local/bin)
brew install ffmpeg whisper-cpp yt-dlp uv
```

### `vf doctor`
Confere as dependências externas e os modelos.
```bash
vf doctor
```
Mostra `ok`/`FALTA` para: `ffmpeg` e `ffprobe` (obrigatórios), `whisper-cli` (transcrição), `yt-dlp` (analisar
URL), `bun` (scripts JS/TS e render), `curl` (contas, publicação, métricas), `git` (`vf init` baixa as bibliotecas
de FX), `uv` (`vf voz`). Também diz quantos modelos whisper estão em `~/.cache/vf/models` e quantos núcleos há.

---

## Projeto

### `vf init`
Transforma a pasta atual (ou `dir=`) num projeto da fábrica. **Nunca sobrescreve** arquivos que já existem.
```bash
vf init type=short platform="yt,tiktok"
vf init type=long platform=yt motor=estatico duracao=720
vf init type="short,long" platform="yt,tiktok" idioma=pt canal=~/Projects/canais/historia --install
vf init skills                     # recopia as skills da fábrica para .claude/skills (depois de atualizar a fábrica)
```
| Opção | Valores | Padrão | O que faz |
|---|---|---|---|
| `type=` | `short` · `long` · `"short,long"` | obrigatório | tipo de vídeo |
| `platform=` | `yt` · `tiktok` · `"yt,tiktok"` | `yt` | onde o vídeo vai |
| `motor=` | `cutout-2d` · `estatico` · `kenburns` · `remotion-2d` · `remotion-three` · `ia-video` · `mapas-3d` | `estatico` com long; `cutout-2d` sem | como o vídeo é feito. `estatico` só para longo; `blender-3d` ainda não implementado |
| `idioma=` / `lang=` | `pt`, `en`… | `pt` | idioma da narração e do vídeo |
| `duracao=` | segundos | alvo do formato | duração alvo |
| `canal=` | pasta do canal | — | aplica o kit do canal (`canal.yaml`: voz, elenco, marcas) |
| `dir=` | pasta | pasta atual | onde criar |
| `--install` | flag | não | instala as dependências Bun (precisa para exportar sprites/Remotion) |
| `--offline` | flag | não | não baixa as bibliotecas de FX de referência (`ref/fx/`) |
| `--plugin` | flag | não | liga o plugin no `.claude/settings.json` em vez de copiar as skills |

`type` × `platform` → formatos: long+yt = `yt-long`; short+yt = `yt-short`; short+tiktok = `tiktok`. Longo só
existe no YouTube; com `tiktok`, o TikTok recebe cortes.

**O que cria:** o esqueleto do projeto (`factory.yaml`, `SCRIPT.md`, `roteiro/`, `assets/`, `public/`, `data/`,
`out/`…), os templates do motor, o som procedural em `audio/procedural/`, `ref/fx/pixelfx` e
`ref/fx/pixel-splats`, as **8 skills da fábrica em `.claude/skills/`** (com o caminho real da fábrica no lugar
de `${CLAUDE_PLUGIN_ROOT}`) e um `CLAUDE.md` curto. Depois: `vf auth` (opcional) e `claude` na pasta.

---

## Análise de vídeo de referência

### `vf analyze`
Estudo completo de um vídeo (arquivo ou URL do YouTube/TikTok) numa passada: download (yt-dlp, até 1080p),
cortes e crossfades, keyframe por plano, contact sheets, transcrição com tempo por palavra e métricas de ritmo.
```bash
vf analyze "https://www.youtube.com/shorts/XXXX" -o ref/xxxx --lang pt
vf analyze video.mp4 -o ref/meu --no-transcribe --threshold 0.3
```
| Opção | Padrão | O que faz |
|---|---|---|
| `-o, --out` | `vf_analysis` | pasta de saída |
| `--no-transcribe` | — | pula a fala |
| `--side` | `1280` | largura das keyframes (px) |
| `--threshold` | `0.4` | sensibilidade de corte (distância de histograma 0-1; menor = mais cortes) |
| `--min-shot` | `6` | duração mínima de plano (frames) |
| `--width` | `256` | largura de análise (o decode já sai reduzido) |
| `-j, --jobs` | `2` | processos ffmpeg em paralelo |
| `--hw` | — | decode por hardware (VideoToolbox); só vale para ProRes/HEVC 10-bit 4K |
| `--model` | `large-v3-turbo-q5_0` | modelo whisper.cpp (baixado em `~/.cache/vf/models`) |
| `--lang` | `auto` | idioma da fala |
| `--prompt` | — | nomes próprios e grafias para a transcrição |
| `--dtw` | — | tempos por token mais precisos (um pouco mais lento) |
| `--threads` | `8` | threads do whisper |

**Saída:** `analysis.md` (specs, ritmo, gancho, tabela de planos com a fala), `analysis.json`, `keyframes/`,
`sheets/shots_*.jpg`, `transcript.txt`, `source.*` (quando é URL). Leia o `.md` e **abra as sheets**.

### `vf probe <arquivo>`
Specs do arquivo em JSON: duração, resolução, fps, codec, pix_fmt, bitrate, tamanho, áudio.
```bash
vf probe out/final_yt-short.mp4
```

### `vf frames <arquivo>`
Estatística por frame (luma, croma, movimento, diferença de histograma) e detecção de cortes.
```bash
vf frames video.mp4 -o vf_out --threshold 0.35
```
Opções: `-o` (padrão `vf_out`), `--width`, `-j`, `--hw`, `--threshold`, `--min-shot` (iguais ao `analyze`).
**Saída:** `frames.jsonl`, `shots.json` (planos, com `xfade` quando há crossfade), `probe.json`.

### `vf keyframes <arquivo>`
Uma keyframe por plano (ou a cada N segundos) e contact sheets com timecode.
```bash
vf keyframes video.mp4 -o kf                    # uma por plano (detecta os cortes)
vf keyframes video.mp4 -o kf --every 1          # uma por segundo
vf keyframes video.mp4 -o kf --shots shots.json # usa planos já detectados
```
| Opção | Padrão | O que faz |
|---|---|---|
| `-o, --out` | `vf_out` | pasta |
| `--shots` | — | usar este `shots.json` em vez de detectar |
| `--every` | — | um frame a cada N segundos |
| `--side` | `1280` | largura da keyframe |
| + `--width -j --hw --threshold --min-shot` | | como no `frames` |

**Saída:** `keyframes/k_*.jpg`, `sheets/sheet_*.jpg`.

### `vf sheet <pasta>`
Folha de contato de uma pasta de imagens (curadoria de lotes do Google Flow/IA), marcando as pequenas demais.
```bash
vf sheet assets/flow/lote-03 --cols 6 --rows 4 --min-w 2150
```
| Opção | Padrão | O que faz |
|---|---|---|
| `-o, --out` | `<pasta>/_sheet` | saída |
| `--cols` / `--rows` | `6` / `4` | grade |
| `--side` | `400` | largura de cada miniatura |
| `--min-w` | `2150` | abaixo disso a imagem é marcada `UP` (precisa upscale para o zoom) |

**Saída:** `sheet_*.jpg` e `sheet.json` (`n`, arquivo, largura, altura, `upscale`).

---

## Voz e tempo

### `vf voz`
Narração **local** com Chatterbox (Resemble AI, MIT), com o ajuste fino oficial para **pt-BR**. Roda num ambiente
Python isolado pelo `uv` (criado na primeira vez; os modelos, ~3-4 GB, vão para `~/.cache/huggingface`).
Guia completo: [voz-chatterbox.md](skills/assets/templates/voz-chatterbox.md).
```bash
vf voz voice/texto.txt -o public/voice/voice.wav                       # ajustes de factory.yaml > voz
vf voz voice/texto.txt -o public/voice/voice.wav --ref kit/voz-ref.wav --seed 42
vf voz voice/texto.txt -o public/voice/voice.wav --refazer 3,7          # tomada nova só das frases 3 e 7
vf voz voice/texto.txt --exaggeration 0.7 --cfg 0.3 --tempo 1.05
```
| Opção | Padrão | O que faz |
|---|---|---|
| `<texto>` | obrigatório | arquivo de texto (uma frase por linha; linha em branco = pausa de parágrafo; `#` = comentário) ou `-` para stdin |
| `-o, --out` | `public/voice/voice.wav` | WAV final (48 kHz mono, silêncio inicial cortado) |
| `--ref` | — | WAV de 10-20 s, limpo, em pt-BR: **clona a voz** (só a sua ou com autorização) |
| `--modelo` | `pt-br` com `--lang pt` | `pt-br` · `multilingual` · `en` |
| `--lang` | `pt` (ou `factory.yaml > idioma`) | idioma |
| `--exaggeration` | `0.5` | expressividade (0.25-2.0); alto acelera a fala |
| `--cfg` | `0.5` | ritmo: baixo = mais lento; `0` tira o sotaque de uma referência em outro idioma |
| `--temperature` | `0.8` | variação |
| `--seed` | `42` | semente fixa: mesmo texto + semente = mesmo áudio |
| `--tempo` | `1.0` | velocidade final sem mudar o tom (atempo do ffmpeg) |
| `--pausa` / `--pausa-paragrafo` | `0.25` / `0.6` | silêncio entre frases / parágrafos (s) |
| `--refazer` | — | números das frases para gerar de novo com outra semente |
| `--device` | `auto` | `auto` (GPU do Mac, cai para CPU se falhar) · `mps` · `cpu` · `cuda` |
| `--cache` | `voice/chatterbox` | pasta dos pedaços |

O que faltar na linha de comando vem de `factory.yaml > voz.chatterbox` (`ref`, `modelo`, `exaggeration`,
`cfg_weight`, `temperature`, `seed`, `tempo`) quando `voz.provedor: chatterbox`.
**Cache:** cada frase fica em `voice/chatterbox/NNN-<hash>.wav`; editar uma linha regera só ela.
`voice/chatterbox/pedacos.json` traz início/fim de cada frase (para achar a que refazer).
Todo áudio leva a marca d'água imperceptível PerTh da Resemble AI.

### `vf transcribe <áudio|vídeo>`
Transcrição com tempo por palavra (whisper.cpp com Metal), ou conversão do JSON do ElevenLabs Scribe.
```bash
vf transcribe public/voice/voice.wav -o data --lang pt
vf transcribe public/voice/voice.wav -o data --lang pt --prompt "Ilha Fiscal, Ouro Preto" --dtw
vf transcribe alinhamento.json -o data --elevenlabs
```
| Opção | Padrão | O que faz |
|---|---|---|
| `-o, --out` | `.` | pasta |
| `--elevenlabs` | — | a entrada é o JSON do Speech-to-Text do ElevenLabs |
| `--model` · `--lang` · `--prompt` · `--dtw` · `--threads` | como no `analyze` | |

**Saída:** `words.json` (`[{word, start, end}]`), `transcript.srt`, `transcript.txt`.

### `vf align <beats.json> <words.json>`
Amarra os beats do roteiro às palavras da narração: grava `w0`, `w1`, `start`, `end` em cada beat.
```bash
vf align roteiro/beats.json data/words.json            # sobrescreve beats.json
vf align roteiro/beats.json data/words.json -o roteiro/beats_alinhado.json
```
Imprime uma linha por beat com as palavras casadas; `<- CONFERIR` quando menos da metade casou (a narração
mudou o texto).

---

## Áudio, export e QA

### `vf mix <edl.json>`
Mixa as faixas de áudio do `edl.json` (voz, música com ducking, SFX) num WAV 48 kHz.
```bash
vf mix data/edl.json -o public/audio/mix.wav --duck 12
```
| Opção | Padrão | O que faz |
|---|---|---|
| `-o, --out` | `public/audio/mix.wav` | saída |
| `--root` | `public` | pasta base dos `src` do EDL |
| `--duck` | `10` | quanto a música abaixa sob a voz (dB) |

### `vf export <vídeo>`
Export no preset do formato: enquadramento, encode, loudness em 2 passadas e mux do áudio.
```bash
vf export render.mp4 -f yt-short -o out/final_yt-short.mp4 --audio public/audio/mix.wav
vf export longo.mp4 -f tiktok -o out/final_tiktok.mp4 --fit blur
```
| Opção | Padrão | O que faz |
|---|---|---|
| `-f, --formato` | obrigatório | `yt-long` · `yt-short` · `tiktok` |
| `-o, --out` | obrigatório | arquivo final |
| `--audio` | — | trilha final para mux |
| `--fit` | `pad` | `pad` (barras) · `crop` (corta o centro) · `blur` (fundo desfocado; bom de 16:9 para 9:16) |
| `--fast` | — | encoder por hardware (VideoToolbox): bem mais rápido, qualidade um pouco menor |
| `--no-loudnorm` | — | não normaliza o volume |
| `--reencode` | — | força recodificar |
| `--size` | do formato | tamanho explícito `WxH` |
| `--formats` | embutido | outro `formats.yaml` |

### `vf qa <vídeo>`
QA do vídeo final contra o formato. **Estrito por padrão**: WARN = FAIL e sai com código 1.
```bash
vf qa out/final_yt-short.mp4 -f yt-short -o out/qa_yt-short --voice public/voice/voice.wav
vf qa out/final_yt-long.mp4 -f yt-long --plan data/plan.json --lenient   # rascunho: WARN não reprova
```
| Opção | Padrão | O que faz |
|---|---|---|
| `-f, --formato` | obrigatório | formato de entrega |
| `-o, --out` | `qa` | pasta |
| `--plan` | — | `shots.json`/`plan.json` com flash/xfade/planned (ignora transições planejadas) |
| `--freeze` | do formato | frames parados mínimos para acusar congelamento |
| `--sheet-every` | `10` | miniatura a cada N frames nas sheets |
| `--lenient` | — | rascunho: WARN não reprova |
| `--factory` | `./factory.yaml` | lê `qa_excecoes: [{check, motivo}]` (única forma de aceitar um WARN) |
| `--voice` | — | trilha de voz isolada, para checar buracos na narração |
| `--formats` · `--width -j --hw --threshold --min-shot` | | como acima |

Checa specs, duração, loudness, pretos/brancos/congelados/pops, planos curtos e longos, bitrate, abertura do
áudio, ar morto, gancho visual, repetição de imagem, barras e clipping. **Saída:** `qa.md`, `qa.json`, sheets.

### `vf formats`
Lista os formatos de entrega (resolução, aspecto, duração mínima/máxima/alvo, LUFS, palavras por minuto).
```bash
vf formats                  # yt-long, yt-short, tiktok
vf formats --formats meu.yaml
```

---

## Contas nas plataformas

### `vf auth`
OAuth 2.0 com o navegador; várias contas por plataforma, cada uma com um **apelido**. Os tokens ficam em
`~/.config/vf/auth/<plataforma>/<apelido>.json` (modo 600) e renovam sozinhos.
```bash
vf auth platform=yt client=~/Downloads/client_secret_XXXX.json   # 1ª vez: instala as credenciais do app e loga
vf auth platform=yt account=meu-canal                            # outra conta/canal
vf auth platform=tiktok account=meu-perfil                       # rascunho na inbox + métricas
vf auth platform=tiktok account=meu-perfil direct=1              # + post direto/agendado (app com Direct Post)
vf auth platform=tiktok-biz account=meu-perfil                   # métricas de retenção (Business/Creator; experimental)
vf auth status                                                   # contas, canal, validade; * = padrão
vf auth use platform=yt account=meu-canal                        # muda a conta padrão
vf auth logout platform=yt account=meu-canal                     # apaga o token desta máquina
```
| Ação / opção | O que faz |
|---|---|
| `login` (padrão) | abre o navegador para autorizar |
| `status` (ou `list`) | lista contas de todas as plataformas (ou só as de `platform=`) |
| `use` | define a conta padrão da plataforma |
| `logout` | remove o token da conta |
| `platform=` | `yt` · `tiktok` · `tiktok-biz` (lista com vírgula no login) |
| `account=` / `conta=` | apelido (sem ele, o apelido vem do nome do canal/perfil) |
| `client=` | JSON de credenciais baixado (Google "App para computador" ou `{client_key, client_secret}` do TikTok) |
| `port=` | porta local do retorno do OAuth |
| `direct=1` | TikTok: pede também `video.publish` (post direto) |

Logar de novo na mesma conta só atualiza o token. Contas do YouTube autorizadas antes da permissão de edição
precisam logar de novo para usar `vf meta`. Passo a passo das credenciais: [plataformas.md](skills/edicao/references/plataformas.md#credenciais-do-app-uma-vez-por-máquina).

---

## Publicação

### `vf publish <vídeo.mp4>`
Sobe vídeo longo ou Short. O padrão é **rascunho**; postar ou agendar só quando você pedir.
```bash
# YouTube
vf publish out/final_yt-long.mp4 platform=yt meta=out/publicacao_yt-long.json thumb=out/thumb.jpg     # rascunho (privado)
vf publish out/final_yt-long.mp4 platform=yt mode=publico meta=out/publicacao_yt-long.json            # posta agora
vf publish out/final_yt-short.mp4 platform=yt at="2026-11-20 18:00" meta=out/publicacao_yt-short.json # agenda (hora local)
vf publish out/final_yt-short.mp4 platform=yt mode=nao-listado title="Teste" tags=a,b
# TikTok
vf publish out/final_tiktok.mp4 platform=tiktok                                                       # rascunho na inbox do app
vf publish out/final_tiktok.mp4 platform=tiktok mode=publico caption="Legenda #tema" capa_ms=800        # post direto
vf publish out/final_tiktok.mp4 platform=tiktok at=+3h caption="Legenda #tema"                         # agenda local
# várias plataformas e contas
vf publish out/final_yt-short.mp4 platform=yt,tiktok account=canal-a,canal-b at="2026-11-21 12:00"
```
| Opção | Valores | O que faz |
|---|---|---|
| `platform=` | `yt` · `tiktok` · `yt,tiktok` | obrigatório |
| `mode=` / `modo=` | `rascunho` (padrão) · `privado` · `nao-listado` (só YT) · `amigos` (só TikTok) · `publico` | visibilidade |
| `at=` / `agendar=` | `2026-11-20 18:00` (hora local) · `2026-11-20T18:00-03:00` · `+30m` · `+2h` · `+1d` | agenda; mínimo 15 min à frente; sem `mode=` vale `publico` |
| `meta=` | JSON `{title, description, tags, caption, category}` | metadados (as chaves avulsas vencem) |
| `title=` `description=` `tags=a,b` `caption=` `legenda=` `category=` | | metadados avulsos (`category` padrão `22`) |
| `idioma=` / `lang=` | `pt`… | idioma do vídeo e do áudio (padrão: `factory.yaml > idioma`) |
| `thumb=` | jpg/png | thumbnail (YouTube; canal verificado por telefone) |
| `formato=` | `yt-long` · `yt-short` · `tiktok` | padrão: detectado (vertical e ≤ 3 min = Short) |
| `ia=0` | | não marcar mídia sintética (padrão marca) |
| `kids=1` | | feito para crianças |
| `comentarios=0` `duet=0` `stitch=0` | | TikTok direto: desliga interações |
| `capa_ms=` | ms | TikTok direto: frame da capa |
| `account=` / `o=` | | conta(s) / pasta do registro |

**Comportamento:**
- **YouTube:** `rascunho`/`privado`/`amigos` sobem privados; `at=` usa o agendamento nativo (sobe privado e o
  YouTube publica sozinho na hora, mesmo com o computador desligado; só agenda público). O `vf` recusa título com
  mais de 100 caracteres ou descrição com mais de 5000 bytes, corta tags acima de 500 caracteres, remove `<` `>`,
  acrescenta `#Shorts` nos Shorts e avisa vídeo longo sem capítulos. Projeto do Google Cloud não verificado:
  o YouTube deixa tudo privado mesmo com `mode=publico` (o `vf` avisa).
- **TikTok:** `rascunho` vai para a caixa de entrada do app (legenda no app). Os outros modos usam o post direto
  (`vf auth ... direct=1`); app não auditado só posta `privado`. Sem `caption=`, a legenda é o título + até 5 tags
  como hashtags. `at=` vai para a [agenda local](#vf-agenda).
- **Registro:** `out/publicacao/<plataforma>-<conta>-<data>.json` (id, URL, link do Studio, estado). Anote em
  `factory.yaml > publicacao.enviados`.

### `vf agenda`
Posts agendados do TikTok (a API do TikTok não agenda). Ficam em `~/.config/vf/agenda/` e saem na hora marcada.
```bash
vf agenda install           # uma vez: LaunchAgent roda "vf agenda run" a cada 5 min (Linux: imprime a linha do crontab)
vf agenda                   # (ou list) pendentes, falhos, últimos feitos e se o executor está ligado
vf agenda run               # posta agora o que já venceu
vf agenda cancel id=<id>    # cancela um pendente
vf agenda retry id=<id>     # devolve um que falhou para a fila
vf agenda uninstall         # desliga o executor (os pendentes ficam guardados)
```
O computador precisa estar ligado na hora; desligado, o post sai quando ele ligar. Não mova o vídeo antes do
post. Log em `~/.config/vf/agenda/agenda.log`.

### `vf meta <ID>`
Corrige título, descrição, tags, categoria e idioma de um vídeo **já enviado** ao YouTube. Sem `aplicar=1`, só
mostra o antes × depois.
```bash
vf meta <ID> platform=yt meta=out/metricas/meta_<ID>.json            # prévia
vf meta <ID> platform=yt meta=out/metricas/meta_<ID>.json aplicar=1  # grava
vf meta <ID> platform=yt title="Novo título" tags=a,b aplicar=1 thumb=nova.jpg
```
| Opção | O que faz |
|---|---|
| `meta=` | JSON com os campos a mudar (o que não vier fica como está) |
| `title=` `description=` `tags=` `category=` `idioma=` | campos avulsos |
| `thumb=` | troca a thumbnail junto |
| `formato=` | para as checagens (padrão: ≤ 180 s = short) |
| `aplicar=1` | grava (sem isso, só prévia) |
| `account=` | conta |

Grava o histórico em `out/analytics/meta-<ID>-<data>.json`. O TikTok não deixa editar legenda pela API.

---

## Métricas

### `vf analytics`
Puxa os dados de canal e vídeos para análise (a skill `metricas` lê o JSON).
```bash
vf analytics platform=yt                               # canal + últimos 50 uploads + top do período (28 dias)
vf analytics platform=yt account=all dias=90           # todas as contas, 90 dias
vf analytics platform=yt video=ID1,ID2 comentarios=1   # retenção, tráfego, curva por idade e comentários
vf analytics platform=yt curva=1 max=100               # views acumuladas d1/d3/d7/d28 dos 25 principais
vf analytics platform=tiktok max=60                    # perfil + posts públicos com engajamento
vf analytics                                           # sem platform=: as do factory.yaml com conta autorizada
```
| Opção | Padrão | O que faz |
|---|---|---|
| `platform=` | do projeto | `yt` · `tiktok` |
| `dias=` / `days=` | `28` | período da YouTube Analytics |
| `max=` | `50` | uploads/posts listados |
| `video=` | — | só estes vídeos, com retenção completa, tráfego e curva por idade |
| `retencao=N` | `5` | curva de retenção dos N principais |
| `curva=1` | — | views por dia desde a publicação (comparar na mesma idade) |
| `comentarios=1` | — | comentários dos principais (sem nome de autor) |
| `account=` · `o=` | | conta(s) / pasta |

**Saída:** `out/analytics/<plataforma>-<conta>-<data>.json`:
- `videos[]`: metadata, `tipo` (short/longo), privacidade, agendamento, `total` (tempo real), `periodo`
  (Analytics), `retencao` (marcos, maiores quedas em segundos, curva), `idade` (d1…d28), `trafego`, `comentarios`;
- `videos[].studio` (se o vídeo foi aberto no Studio com a extensão): continuaram/ignorado, impressões, CTR,
  tráfego, geografia, demografia e `primeiras_horas` (h1…h48, pico);
- `medianas` por tipo (inclusive continuaram, CTR e views_h1/h3/h24), `por_tipo`, `trafego` do canal,
  `canal_studio` e `avisos` (por que algo veio vazio: Analytics atrasa 24-72 h, vídeo privado, API desligada).

TikTok: a API normal só lista posts públicos, sem retenção nem tráfego.

### `vf relatorio`
Relatórios no estilo do YouTube Studio, por vídeo ou de todos com filtros, em Markdown + JSON + HTML.
Alias: `vf report`.
```bash
# um vídeo
vf relatorio video=<ID>                                 # 1º dia, 3, 7, 28 primeiros dias e total lado a lado
vf relatorio video=<ID> janela=d7 beats=roteiro/beats.json
vf relatorio video=ID1,ID2 janela=total                         # um relatório por vídeo
# todos, com filtros
vf relatorio                                                    # vídeos públicos, 7 primeiros dias de cada
vf relatorio tipo=short desde=2026-10-01 ordenar=retencao limite=20
vf relatorio busca="império" tag=brasil min_views=100 janela=d28 ordenar=intencionais ordem=asc
vf relatorio janela=2026-11-01..2026-11-15 account=all
vf relatorio privacidade=unlisted
# TikTok
vf relatorio platform=tiktok ordenar=engajamento
vf relatorio platform=tiktok video=7300000000000000000
```
| Opção | Valores | O que faz |
|---|---|---|
| `video=` | IDs | relatório individual (sem `video=`: geral) |
| `janela=` | `d1` · `d3` · `d7` · `d28` · `dN` · `total` · `todas` · `AAAA-MM-DD..AAAA-MM-DD` | individual: `todas` (padrão); geral: `d7` (padrão; uma janela só) |
| `tipo=` | `short` · `longo` · `todos` | filtro |
| `privacidade=` | `public` · `unlisted` · `private` | filtro (sem ele: públicos e não listados) |
| `desde=` / `ate=` | `AAAA-MM-DD` | data de publicação |
| `busca=` | texto | no título ou descrição |
| `tag=` | tag | nas tags ou como #hashtag |
| `min_views=` | número | mínimo de views na janela |
| `ordenar=` | `views` · `intencionais` · `retencao` · `avd` · `pct` · `horas` · `inscritos` · `likes` · `comentarios` · `shares` · `engajamento` · `alcance` · `publicado` | ordem |
| `ordem=` | `desc` (padrão) · `asc` | |
| `limite=` | N | quantos vídeos |
| `max=` | `200` | uploads lidos do canal |
| `beats=` | `roteiro/beats.json` | liga as quedas de retenção ao beat do roteiro |
| `studio=0` | | não usar os dados do Studio |
| `html=0` | | não gerar o HTML |
| `platform=` · `account=` · `o=` | | |

**Linhas (como no Studio):** visualizações, impressões e CTR (Studio), visualizações intencionais, continuaram
assistindo (Studio), duração média, % visualizada, retenção média (média da curva), horas de exibição, gostei,
inscritos (ganhos − perdidos), comentários, compartilhamentos, cada uma com a **mediana do canal** (mesmo tipo,
mesma janela) e a diferença.

**Seções extras:** retenção (pontos 3 s/30 s/25-100 % e maiores quedas, com o beat), primeiras horas (Studio:
views em 1/3/6/12/24/48 h × mediana), fontes de tráfego, geografia e demografia (Studio); no geral, totais,
medianas por tipo, melhores e piores em retenção e "Canal no Studio".

**Saída:** `out/relatorios/<plataforma>-<conta>-<vídeo|geral>-<janela>-<data>.md|json|html`. O HTML tem a curva de
retenção (ou views por hora, ou barras de retenção por vídeo), tabelas que ordenam com um clique e modo claro e
escuro. "—" = sem dado (Analytics atrasada, API não fornece ou Studio processando); "parcial" = vídeo mais novo
que a janela (fica fora da mediana).

### `vf studio`
Dados que só o YouTube Studio mostra (continuaram assistindo, impressões, CTR, views por hora…), capturados pela
**extensão do Chrome** `tools/vf-studio-ext`: ela lê o que o Studio carrega no seu navegador logado e entrega ao
`vf`; os cookies não saem do Chrome. **Não oficial** (viola os termos do YouTube sobre acesso automatizado).
```bash
vf studio instalar                      # registra o vf no Chrome/Brave/Edge/Arc (native messaging)
#   depois: chrome://extensions → Modo do desenvolvedor → Carregar sem compactação → tools/vf-studio-ext
vf studio status                        # host instalado?, vídeos capturados, mapa
vf studio resumo video=<ID>      # um valor atual por cartão + views por hora (JSON)
vf studio resumo canal=UCxxxxxxxxxxxxxxxxxxxxxx
vf studio metricas video=<ID>    # tudo o que foi achado nas capturas (depuração)
vf studio mapear minha_linha=TEXTO:Rótulo do cartão   # liga uma métrica extra a uma linha do relatório
vf studio desinstalar
```
**Coletar:**
- **passivo:** abra Studio → Análises de um vídeo (ou do canal) e navegue; tudo o que a página carregar é guardado
  em `~/.config/vf/studio/dados/<vídeo>/` (até 80 respostas por vídeo);
- **ativo gentil** (botão **Coletar** da extensão): abre a página de análises de cada vídeo numa aba em segundo
  plano, uma página a cada 35-75 s, rolando devagar; até 30 vídeos por rodada e por dia; pula o que foi capturado
  nas últimas 24 h; para se a sessão cair, se o Studio pedir verificação ou se a aba fechar; botão **Parar**.

**Curva das primeiras horas:** cada captura traz as últimas ~48 h por hora; abra o vídeo no Studio em 1 h, 3 h e
24 h depois de publicar para ter a curva desde o início. Valores que o Studio ainda mostra como 0 saem em
`processando`.

Legado (não recomendado, guarda cookies de acesso total à conta no Keychain): `pbpaste | vf studio login
account=<apelido> nome=<consulta>`, `vf studio buscar video=ID nome=<consulta>`, `vf studio logout account=<apelido>`.

---

## Receitas

**Projeto novo pelo terminal, até o rascunho no YouTube**
```bash
mkdir ~/Projects/meu-video && cd ~/Projects/meu-video
vf init type=short platform="yt,tiktok"
vf auth status                                   # conta certa?
claude                                           # "vamos fazer o vídeo sobre <tema>": a skill factory conduz
# ... roteiro, voz, animação, render ...
vf qa out/final_yt-short.mp4 -f yt-short -o out/qa_yt-short --voice public/voice/voice.wav
vf publish out/final_yt-short.mp4 platform=yt meta=out/publicacao_yt-short.json
```

**Narração local e alinhamento**
```bash
vf voz voice/texto.txt -o public/voice/voice.wav --ref ~/Projects/canais/x/kit/voz-ref.wav
vf transcribe public/voice/voice.wav -o data --lang pt
vf align roteiro/beats.json data/words.json
```

**Publicar Short no YouTube e no TikTok no mesmo horário**
```bash
vf auth platform=tiktok account=perfil direct=1        # uma vez
vf agenda install                                      # uma vez
vf publish out/final_yt-short.mp4 platform=yt at="2026-11-20 18:00" meta=out/publicacao_yt-short.json
vf publish out/final_tiktok.mp4 platform=tiktok at="2026-11-20 18:00" caption="Legenda #tema #historia"
vf agenda
```

**Depois de publicar: checagem de 1 h, 3 h e 24 h**
```bash
# abra as análises do vídeo no Studio (extensão captura), depois:
vf relatorio video=<ID> janela=total
vf studio resumo video=<ID>
```

**Revisão semanal do canal**
```bash
vf analytics platform=yt dias=28 curva=1
vf relatorio janela=d7 ordenar=retencao
vf relatorio tipo=short janela=d28 ordenar=views limite=10
# na sessão: /video-factory:metricas auditoria
```

**Corrigir o título de um vídeo longo com CTR baixo**
```bash
vf relatorio video=<ID> janela=d7                      # confirma: impressões ok, CTR abaixo da mediana
# skill metricas (fluxo metadata) escreve out/metricas/meta_<ID>.json
vf meta <ID> platform=yt meta=out/metricas/meta_<ID>.json            # prévia
vf meta <ID> platform=yt meta=out/metricas/meta_<ID>.json aplicar=1  # grava (depois do seu OK)
```

**Estudar um vídeo de referência**
```bash
vf analyze "https://www.youtube.com/shorts/XXXX" -o ref/xxxx --lang pt
open ref/xxxx/analysis.md ref/xxxx/sheets/
```

---

## Onde ficam os arquivos

| Caminho | Conteúdo |
|---|---|
| `~/.config/vf/yt-client.json` · `tiktok-client.json` · `tiktok-biz-client.json` | credenciais dos apps (modo 600) |
| `~/.config/vf/auth/<plataforma>/<apelido>.json` · `padrao` | tokens das contas (modo 600) e a conta padrão |
| `~/.config/vf/agenda/{pendente,feito,falhou,cancelado}/` · `agenda.log` | posts agendados |
| `~/Library/LaunchAgents/com.video-factory.agenda.plist` | executor da agenda (`vf agenda install`) |
| `~/.config/vf/studio/dados/<vídeo ou canal_UC…>/` | capturas da extensão vf studio |
| `~/.config/vf/studio/mapa.json` · `vf-studio-host.sh` | mapa de linhas extras · ponte com o Chrome |
| `~/Library/Application Support/<navegador>/NativeMessagingHosts/com.video_factory.studio.json` | registro da ponte (`vf studio instalar`) |
| `~/.cache/vf/models/` | modelos whisper |
| `~/.cache/huggingface/` · `~/.cache/vf/chatterbox-pt-br/` | modelos do Chatterbox |
| `<projeto>/.claude/skills/` | skills da fábrica no projeto (`vf init`) |
| `<projeto>/out/analytics/` · `out/relatorios/` · `out/publicacao/` · `out/studio/` | saídas de métricas e publicação |

## Variáveis de ambiente

| Variável | Uso |
|---|---|
| `VF_ROOT` | raiz da fábrica (se o `vf` não achar sozinho) |
| `VF_WHISPER` | caminho do `whisper-cli` |
| `VF_YT_CLIENT_ID` · `VF_YT_CLIENT_SECRET` | credenciais do app do YouTube (em vez do JSON) |
| `VF_TIKTOK_CLIENT_KEY` · `VF_TIKTOK_CLIENT_SECRET` · `VF_TIKTOK_REDIRECT` | app do TikTok |
| `VF_TTB_CLIENT_KEY` · `VF_TTB_CLIENT_SECRET` · `VF_TTB_REDIRECT` | app do TikTok for Business |
| `XDG_CONFIG_HOME` | troca a base de `~/.config` |
| `ELEVENLABS_API_KEY` | voz pelo ElevenLabs (skill assets) |
| `HF_TOKEN` | opcional, downloads do Hugging Face (Chatterbox) |

## Erros comuns

| Mensagem | O que fazer |
|---|---|
| "YouTube Data API v3 está DESATIVADA no projeto" | abrir o link da mensagem → Ativar; esperar 2-5 min. Ative também a YouTube Analytics API |
| "autorizada sem todas as permissões" / "sem permissão de edição" | `vf auth platform=yt account=<apelido>` de novo, marcando todas as caixas |
| "essa conta Google não tem canal" | logar de novo escolhendo o canal (conta de marca) |
| "cota da API do Google esgotada" | esperar o dia virar (horário do Pacífico); upload custa ~1600 de 10000 unidades/dia |
| "token recusado (HTTP 401)" | logar de novo; app em modo teste expira em 7 dias |
| "post direto precisa do escopo video.publish" | ativar o Direct Post no app do TikTok e `vf auth platform=tiktok direct=1` |
| relatório cheio de "—" num vídeo novo | normal: a Analytics atrasa 24-72 h; o Studio (extensão) preenche parte antes |
| "nada capturado" no `vf studio` | abrir as análises do vídeo no Studio com a extensão carregada; `vf studio status` |
| "vf não conectado" na extensão | `vf studio instalar` e recarregar a extensão |
| "QA REPROVADO" | corrigir, ou registrar a exceção com motivo em `factory.yaml > qa_excecoes` |
