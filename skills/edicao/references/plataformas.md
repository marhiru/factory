# Plataformas: autorização, rascunho e analíticos

`vf auth`, `vf publish` e `vf analytics` falam direto com o YouTube e o TikTok com OAuth 2.0 (PKCE, retorno
em 127.0.0.1). Tokens ficam em `~/.config/vf/auth/<plataforma>.json` (modo 600), um por máquina, valendo
para todos os projetos. A fábrica **nunca publica**: tudo sobe como rascunho e quem posta é o usuário.

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
2. Escopos: `user.info.basic`, `user.info.stats`, `video.list`, `video.upload`.
3. Redirect URI: `http://localhost:8723/callback/` (exatamente; para outro, defina `redirect_uri` no JSON).
4. `~/.config/vf/tiktok-client.json` com `{"client_key": "...", "client_secret": "..."}` (ou
   `VF_TIKTOK_CLIENT_KEY`/`VF_TIKTOK_CLIENT_SECRET`).
5. `vf auth platform=tiktok`.

Apps em modo sandbox/não auditados só funcionam para as contas de teste cadastradas no app.

`vf auth status` mostra o estado; `vf auth logout platform=yt` apaga o token local. O token renova sozinho.

## Rascunho (`vf publish`)

Só depois de `vf qa` estrito com código 0 **e** do OK do usuário para subir:
```
vf publish out/final_yt-short.mp4 platform=yt meta=out/publicacao_yt-short.json [thumb=out/thumb.jpg]
vf publish out/final_tiktok.mp4 platform=tiktok
```
- YouTube: upload resumível com `privacyStatus: private` (o equivalente a rascunho na API) e
  `containsSyntheticMedia: true` por padrão (`ia=0` desliga quando não há mídia sintética realista). Título,
  descrição e tags vêm do `meta=` (JSON `{title, description, tags}` gerado do `templates/publicacao.md`) ou de
  `title=`/`description=`/`tags=`. Revise e publique no Studio (o link sai na resposta).
- TikTok: vai para a **caixa de entrada** do app como rascunho. Legenda, som e postagem são feitos no app;
  a API não aceita legenda nesse modo.
- A resposta fica em `out/publicacao/<plataforma>-<data>.json`; anote em `factory.yaml > publicacao.enviados`.

## Analíticos (`vf analytics`)

```
vf analytics platform=yt                       # top 25 vídeos dos últimos 28 dias
vf analytics platform=yt video=ID1,ID2 dias=7  # por vídeo, com curva de retenção
vf analytics platform=tiktok                   # conta + 20 vídeos recentes (views, likes, comentários, shares)
```
Saída em `out/analytics/<plataforma>-<data>.json`. No YouTube, a retenção (`audienceWatchRatio` por fração
do vídeo) mostra onde o gancho perde gente. Use para:
- **roteiro**: queda antes de 10 % = gancho fraco; queda num ponto fixo = virada lenta ou explicação longa;
- **canal**: alimentar a pauta (temas e formatos que seguram) e registrar em `factory.yaml > analiticos`.
