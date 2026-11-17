# Plataformas: autorização, publicação, agenda e analíticos

`vf auth`, `vf publish` e `vf analytics` falam direto com o YouTube e o TikTok com OAuth 2.0 (PKCE, retorno
em 127.0.0.1). Tokens ficam em `~/.config/vf/auth/<plataforma>.json` (modo 600), um por máquina, valendo
para todos os projetos. O padrão é **rascunho** (`mode=rascunho`); postar público, não listado ou agendado só
acontece quando o usuário pede explicitamente aquele vídeo, conta e horário.

## Credenciais do app (uma vez por máquina)

A API exige um app registrado em nome do usuário. O `vf` não traz credenciais embutidas.

**YouTube** (Google Cloud):
1. console.cloud.google.com → novo projeto → ative **YouTube Data API v3** e **YouTube Analytics API**.
2. Tela de consentimento OAuth: tipo Externo, adicione a própria conta como usuário de teste.
3. Credenciais → ID do cliente OAuth → tipo **App para computador** → baixe o JSON.
4. Salve como `~/.config/vf/yt-client.json` (ou exporte `VF_YT_CLIENT_ID` e `VF_YT_CLIENT_SECRET`).
5. `vf auth platform=yt`. O navegador abre, você autoriza, o terminal confirma.

Projetos de API não verificados pelo Google só conseguem enviar vídeos **privados**, e é isso que a fábrica
faz de qualquer jeito. Thumbnail personalizada exige canal verificado por telefone.

**TikTok** (developers.tiktok.com):
1. Crie um app → produtos **Login Kit** (Desktop) e **Content Posting API**.
2. Escopos: `user.info.basic`, `user.info.stats`, `video.list`, `video.upload`. Para postar direto (público,
   privado, agendado) adicione o **Direct Post** (escopo `video.publish`) e autorize com
   `vf auth platform=tiktok direct=1`. Sem o Direct Post aprovado o TikTok recusa o login com `direct=1`.
3. Redirect URI: `http://localhost:8723/callback/` (exatamente; para outro, defina `redirect_uri` no JSON).
4. `~/.config/vf/tiktok-client.json` com `{"client_key": "...", "client_secret": "..."}` (ou
   `VF_TIKTOK_CLIENT_KEY`/`VF_TIKTOK_CLIENT_SECRET`).
5. `vf auth platform=tiktok`.

Apps em modo sandbox/não auditados só funcionam para as contas de teste cadastradas no app, e o post direto
de app não auditado só sai como `SELF_ONLY` (privado): use `mode=privado` até a auditoria.

O JSON baixado pode ser passado direto: `vf auth platform=yt client=~/Downloads/client_secret_....json` (o `vf`
valida, guarda em `~/.config/vf/yt-client.json` com modo 600 e já abre o login). O token renova sozinho.

## TikTok Business: retenção (experimental)

A API normal do TikTok (Display API) não dá retenção, tempo médio nem alcance. Esses números saem da
**Accounts API do TikTok for Business** (`business-api.tiktok.com/open_api/v1.3`), que o `vf relatorio
platform=tiktok` usa quando existe uma conta `tiktok-biz`:
1. A conta TikTok precisa ser **Business ou Creator** (no app: Configurações → Conta).
2. business-api.tiktok.com → My Apps → crie um app com acesso às contas TikTok (perfil, lista de posts,
   insights de vídeo). Redirect URI: `http://localhost:8724/callback/`.
3. `~/.config/vf/tiktok-biz-client.json` com `{"client_key": "<app id>", "client_secret": "..."}`
   (opcional `"scopes"` e `"redirect_uri"`; ou `VF_TTB_CLIENT_KEY`/`VF_TTB_CLIENT_SECRET`).
4. `vf auth platform=tiktok-biz account=<mesmo apelido do tiktok>`.

Campos lidos de `/business/video/list/`: `video_views`, `likes`, `comments`, `shares`, `reach`,
`video_duration`, `average_time_watched`, `total_time_watched`, `full_video_watched_rate`,
`impression_sources`, `video_view_retention`. Retenção média = tempo médio ÷ duração. Os dados atrasam
24-48 h e 0 costuma significar "ainda sem dado". **Experimental**: a documentação oficial não pôde ser
conferida linha a linha; se o TikTok recusar algo, o `vf` mostra a resposta crua para ajustar (escopos em
`"scopes"` no JSON do app). `tiktok-biz` serve só para métricas; para subir vídeo use `platform=tiktok`.

## YouTube Studio: extensão do Chrome (`vf studio`, não oficial)

Métricas que a API pública não entrega (ex.: **"Continuaram assistindo"** dos Shorts, assistiram × deslizaram)
vêm dos dados que o próprio Studio carrega. A extensão `tools/vf-studio-ext` lê essas respostas **dentro do
seu Chrome logado**: os cookies nunca saem do navegador e ela não fabrica requisições.

**Instalar (uma vez):**
1. `vf studio instalar`: registra o `vf` como host de native messaging do Chrome/Brave/Edge/Arc
   (`~/Library/Application Support/<navegador>/NativeMessagingHosts/com.video_factory.studio.json`).
2. `chrome://extensions` → Modo do desenvolvedor → **Carregar sem compactação** → `tools/vf-studio-ext`
   (id `gepfeoocoahfmnnkpmlakpffedddabmm`). O ícone mostra se o `vf` está conectado.

**Coletar:**
- **Passivo**: abra Studio → Análises de um vídeo e navegue pelas abas; o que a página carregar é entregue ao
  `vf` em `~/.config/vf/studio/dados/<vídeo>/`.
- **Ativo gentil** (botão **Coletar** da extensão): pede ao `vf` os vídeos públicos mais recentes (ou use os
  IDs que você colar) e abre as páginas de análise **numa aba em segundo plano, uma página a cada 35-75 s**
  (intervalo aleatório), só "visão geral" por padrão (ou geral + alcance + engajamento). Limites: até 30 vídeos
  por rodada e por dia; pula o que foi capturado nas últimas 24 h; para sozinho se a sessão cair, se o Studio
  pedir verificação ou se você fechar a aba. Só roda quando você clica; botão **Parar** a qualquer momento.

**Usar no relatório:**
1. `vf studio metricas video=<ID>`: lista as métricas achadas nas capturas (nomes em MAIÚSCULAS + valor).
2. `vf studio mapear continuaram=<NOME_DA_MÉTRICA>` (fração 0-1 vira %; `continuaram=` vazio desmapeia).
3. `vf relatorio video=<ID> studio=1`: preenche a linha na janela principal.
- `vf studio status` mostra host, capturas e mapa; `vf studio desinstalar` remove o host.

**Riscos (aceitos pelo usuário):** é coleta fora da API oficial (os termos do YouTube restringem acesso
automatizado); o modo ativo é lento de propósito para parecer navegação normal, mas continua sendo automação.
Os nomes das métricas internas podem mudar sem aviso: se a linha sumir, rode `vf studio metricas` e remapeie.

### Legado: cURL com cookies (`vf studio login`)
Prefira a extensão. O caminho antigo guarda os cookies da sessão (acesso total à conta Google) no Keychain:

1. **Capturar** (uma vez por tipo de consulta): no Chrome, abra o Studio → Análises do vídeo (Short) → o
   cartão com a métrica. DevTools (⌥⌘I) → Network → filtre `youtubei` → ache a requisição cuja resposta tem o
   número (aba Response) → botão direito → Copy → **Copy as cURL (bash)**.
2. `pbpaste | vf studio login account=<apelido> nome=shorts`
   - cookie + `x-goog-authuser`/`x-goog-pageid` (qual conta/canal) vão para o **Keychain do macOS**
     (serviço `vf-studio`), pela API do sistema: nada em arquivo de texto nem na linha de comando;
   - o modelo da consulta (URL, corpo, cabeçalhos **sem** cookie/autorização) vai para
     `~/.config/vf/studio/<apelido>/<nome>.json`, com o id do vídeo capturado.
3. `vf studio buscar account=<apelido> nome=shorts video=<ID>`: repete a consulta trocando o vídeo, monta o
   `SAPISIDHASH` na hora, salva a resposta em `out/studio/` e lista as métricas achadas (nomes em MAIÚSCULAS).
4. `vf studio mapear continuaram=<NOME_DA_MÉTRICA>`: diz qual métrica é a linha do relatório (fração 0-1 vira %).
5. `vf relatorio video=<ID> studio=1`: preenche "Continuaram assistindo" (e o que mais for mapeado) na janela
   principal do relatório.
- `vf studio status` lista sessões e consultas; `vf studio logout account=<apelido>` apaga sessão e modelos.
- HTTP 401/403 = sessão expirou (ou você saiu da conta): capture de novo. Sair da conta no navegador
  invalida os cookies guardados.

## Várias contas

Um app (credenciais) por plataforma serve para **quantas contas/canais você quiser**; cada conta autoriza o
app uma vez e ganha um **apelido**.
```
vf auth platform=yt account=canal-historia     # o Google pergunta qual conta/canal (select_account)
vf auth platform=yt account=canal-ciencia
vf auth status                                 # lista: * = padrão, nome do canal, validade, "usada por este projeto"
vf auth use platform=yt account=canal-ciencia  # muda a conta padrão
vf auth logout platform=yt account=canal-historia
```
- Sem `account=` no login, o apelido vem do nome do canal/perfil. Logar de novo na mesma conta só atualiza o token
  (reaproveita o apelido; com outro `account=` ele renomeia).
- **Qual conta é usada** em `publish`/`analytics`: `account=` explícito → `factory.yaml > contas.<plataforma>` do
  projeto → conta padrão → a única existente. Com várias contas e nenhuma padrão, o comando pede para escolher.
- `vf publish ... account=a,b` sobe o rascunho em mais de uma conta; `vf analytics platform=yt account=all` puxa
  todas. Os arquivos saem como `out/<pasta>/<plataforma>-<conta>-<data>.json`.
- Tokens em `~/.config/vf/auth/<plataforma>/<apelido>.json` (modo 600); o token antigo de conta única é migrado
  sozinho para o apelido `principal`.
- Canal da fábrica (`canal.yaml`): registre os apelidos em `contas:`; o projeto copia para `factory.yaml > contas`.

## Erros comuns

| Mensagem do `vf` | Causa | Solução |
|---|---|---|
| "… API está DESATIVADA no projeto" | autorizar a conta (OAuth) não liga a API; é um interruptor do projeto no Cloud | abrir o link da mensagem → **Ativar**; esperar 2-5 min. Ative as duas: YouTube Data API v3 e YouTube Analytics API |
| "autorizada sem todas as permissões" | alguma caixa ficou desmarcada na tela do Google | `vf auth platform=yt account=<apelido>` de novo, marcando tudo |
| "essa conta Google não tem canal" | escolheu a conta pessoal em vez da conta do canal (brand account) | refazer o login escolhendo o canal |
| "cota da API do Google esgotada" | 10.000 unidades/dia; um upload custa ~1.600 | esperar o dia virar (horário do Pacífico) ou pedir aumento |
| "token recusado (HTTP 401)" | acesso revogado ou refresh expirado (apps em modo teste expiram em 7 dias) | refazer o login; para não expirar, publique o app (sai do modo teste) |

`vf analytics` sem `platform=` usa as plataformas do projeto (`factory.yaml > plataformas`) que têm conta
autorizada; uma conta com erro não interrompe as outras (o comando sai com código 1 no fim).

## Publicar e agendar (`vf publish`)

Só depois de `vf qa` estrito com código 0 **e** do OK do usuário. Longo e Short usam o mesmo comando; o formato
vem do arquivo (vertical e ≤ 3 min = Short) ou de `formato=`.
```
vf publish out/final_yt-long.mp4  platform=yt meta=out/publicacao_yt-long.json thumb=out/thumb.jpg   # rascunho (privado)
vf publish out/final_yt-long.mp4  platform=yt mode=publico meta=...                                  # posta agora
vf publish out/final_yt-short.mp4 platform=yt at="2026-11-20 18:00" meta=...                         # agenda (hora local)
vf publish out/final_yt-short.mp4 platform=yt mode=nao-listado
vf publish out/final_tiktok.mp4   platform=tiktok                                                    # rascunho na inbox do app
vf publish out/final_tiktok.mp4   platform=tiktok mode=publico caption="..." capa_ms=800             # post direto
vf publish out/final_tiktok.mp4   platform=tiktok at=+3h caption="..."                               # agenda local
```
| `mode=` | YouTube | TikTok |
|---|---|---|
| `rascunho` (padrão) | privado, você publica no Studio | caixa de entrada do app (legenda no app) |
| `privado` | privado | post direto `SELF_ONLY` |
| `nao-listado` | não listado | não existe (erro) |
| `amigos` | privado | post direto `MUTUAL_FOLLOW_FRIENDS` |
| `publico` | público | post direto `PUBLIC_TO_EVERYONE` (app auditado) |

- `at=` aceita `2026-11-20 18:00` (hora local), `2026-11-20T18:00-03:00`, `+30m`, `+2h`, `+1d`; mínimo 15 min à
  frente. `at=` sem `mode=` vale `publico`.
  - **YouTube**: agenda nativa (`publishAt`): sobe agora como privado e o YouTube publica sozinho na hora, com o
    computador desligado. Só agenda público.
  - **TikTok**: a API não agenda. O pedido vai para a **agenda local** (`~/.config/vf/agenda/`) e é postado por
    `vf agenda run`. Ligue uma vez: `vf agenda install` (LaunchAgent a cada 5 min no macOS; no Linux imprime a
    linha do crontab). O computador precisa estar ligado na hora; se não estiver, posta quando ligar. Não mova
    o arquivo de vídeo até lá. `vf agenda` lista, `vf agenda cancel id=...` cancela, `vf agenda retry id=...`
    repete um que falhou.
- Metadata: `meta=` (JSON `{title, description, tags, caption}` de `templates/publicacao.md` ou da skill
  `metricas`) ou `title=`/`description=`/`tags=`/`caption=`. O `vf` checa os limites do YouTube (título ≤ 100,
  descrição ≤ 5000 bytes, tags ≤ 500), põe `#Shorts` em Short e avisa vídeo longo sem capítulos.
  `idioma=` (ou `factory.yaml > idioma`) vira o idioma do vídeo e do áudio.
- `containsSyntheticMedia`/`is_aigc`: `ia=1` por padrão (`ia=0` quando não há mídia sintética realista).
- TikTok direto: `comentarios=0`, `duet=0`, `stitch=0` desligam interações; o `vf` consulta o
  `creator_info` e recusa privacidade que a conta não permite ou vídeo maior que o limite dela.
- **Projeto do Google Cloud não verificado**: o YouTube trava todo upload da API como privado, mesmo com
  `mode=publico` (o `vf` avisa). Para postar público pela API, peça a auditoria do projeto
  (https://support.google.com/youtube/contact/yt_api_form); até lá, publique no Studio.
- A resposta fica em `out/publicacao/<plataforma>-<conta>-<data>.json`; anote em `factory.yaml > publicacao.enviados`.

## Corrigir metadata depois (`vf meta`, YouTube)
```
vf meta <ID> platform=yt meta=out/metricas/meta_<ID>.json          # prévia: antes × depois
vf meta <ID> platform=yt meta=out/metricas/meta_<ID>.json aplicar=1 # grava (pede o escopo youtube.force-ssl)
```
Só muda os campos presentes no JSON. Contas autorizadas antes desta versão: rode `vf auth platform=yt
account=<apelido>` de novo para ganhar o escopo de edição. TikTok não edita legenda pela API.

## Analíticos (`vf analytics`)

```
vf analytics platform=yt                               # canal + últimos 50 uploads + top do período (28 dias)
vf analytics platform=yt video=ID1,ID2 comentarios=1   # retenção, tráfego, curva por idade, comentários
vf analytics platform=yt curva=1 dias=90 max=100       # curva d1/d3/d7/d28 dos 25 principais
vf analytics platform=tiktok max=60                    # perfil + posts públicos (views, likes, coment., shares, engajamento)
```
Saída em `out/analytics/<plataforma>-<conta>-<data>.json` (`account=<apelido>` ou `account=all`). Quem lê e
tira conclusões é a skill **`metricas`** (auditoria, upload novo × mesma idade, retenção × roteiro, metadata).
- O canal e os vídeos vêm da **YouTube Data API v3** (tempo real: views totais, privacidade, agendamento,
  metadata); o período, a retenção e o tráfego da **YouTube Analytics API** (atraso de 24-72 h). Se a Data API
  estiver desligada no projeto do Cloud, o comando para com o link para ativar.
- `avisos` no JSON explica resultado vazio: Analytics atrasado, vídeos ainda privados, canal sem views.
- Shorts × longos: `tipo` de cada vídeo (pelo `creatorContentType` da Analytics) e `por_tipo` do canal.
- TikTok: a Display API só lista posts públicos e não dá retenção/tráfego/comentários (TikTok Studio).
