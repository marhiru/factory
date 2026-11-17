# Pacote de publicação

Gere `out/publicacao_<formato>.md` com:

## Título
- YouTube longo: ≤ 60 caracteres visíveis; promessa ou curiosidade + termo buscável. Gere 5 opções e recomende 1.
- Shorts: ≤ 40 caracteres, complementa o gancho (não repete).
- **Short de história/mecanismo** (medido no canal de referência, 152 títulos): 4-7 palavras, curiosidade sem
  entregar o mecanismo. Formas: "Por que X não funciona", "O X que Y", "Como X funcionava", "O instinto que faz
  você Y", "O emprego que ninguém quer no Z". O canal estudado usa o título como descrição inteira (sem tags nem
  hashtags) e aposta tudo no título + primeiro segundo; na fábrica a descrição curta continua valendo para
  fontes e créditos (abaixo).
- TikTok: a legenda é o "título"; primeira linha ≤ 80 caracteres + 3-5 hashtags específicas.

## Descrição (YouTube)
```
<2 linhas: a promessa do vídeo, com a palavra-chave principal>

00:00 <capítulo 1>
01:42 <capítulo 2>
...

Fontes:
- <fato> — <fonte/URL> (do SCRIPT.md)

Créditos de imagem/som:
- <asset> — <autor>, <licença> (todo item do manifest com atribuição obrigatória)
```
Capítulos: gerados dos `start` dos beats `tipo: capitulo` (primeiro em 00:00, mínimo 3, ≥ 10 s cada).

## Thumbnail (YouTube longo)
- 1280x720, < 2 MB, JPG.
- Um assunto grande + no máximo 3-4 palavras que **não** repetem o título.
- Gere 2-3 variações a partir de um frame forte: `$VF keyframes out/final_yt-long.mp4 -o out/thumb_kf` e escolha nas sheets.
- Contraste alto. Legível a 160 px de largura (teste reduzindo a imagem).

## Checklist por plataforma
| | yt-long | yt-short | tiktok |
|---|---|---|---|
| Arquivo | `out/final_yt-long.mp4` | `out/final_yt-short.mp4` (≤ 180 s, 9:16) | `out/final_tiktok.mp4` |
| QA | sem FAIL | sem FAIL | sem FAIL |
| Extras | capítulos, thumbnail, tela final, cards | título curto; liga ao vídeo longo relacionado | legenda + hashtags; capa = frame do gancho |
| Divulgação de IA | marcar "conteúdo alterado ou sintético" quando houver pessoas, lugares ou eventos realistas gerados por IA | idem | rótulo "AI-generated" quando aplicável |
