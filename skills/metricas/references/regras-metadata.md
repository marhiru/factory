# Regras de metadata por plataforma

Duas camadas: **limites da API** (o `vf` recusa ou corrige sozinho) e **boas práticas** (o editor de
metadata aplica; o revisor confere). Base de títulos do projeto: `skills/edicao/templates/publicacao.md`.

## Limites duros (o `vf publish`/`vf meta` checa)
| | YouTube | TikTok (API) |
|---|---|---|
| Título | ≤ 100 caracteres, sem `<` `>` | — (a legenda faz o papel) |
| Descrição / legenda | ≤ 5000 bytes, sem `<` `>` | ≤ 2200 caracteres (UTF-16) no post direto; rascunho (inbox) não leva legenda |
| Tags | soma ≤ 500 caracteres (vírgulas contam; tag com espaço conta aspas) | não existe: hashtags na legenda |
| Hashtags | > 60 na descrição → o YouTube ignora todas; as 3 primeiras aparecem perto do título | sem limite formal; use poucas |
| Capítulos | 1º em 00:00, ≥ 3, cada ≥ 10 s | — |
| Edição depois | `vf meta <ID> aplicar=1` (escopo youtube.force-ssl) | só no app |

## YouTube longo
- **Título**: o essencial nos primeiros ~60 caracteres (corta em busca/sugeridos e no celular). Termo que a
  pessoa buscaria + promessa ou curiosidade. Específico vence genérico (nome, número, lugar, época).
- **Thumbnail + título se completam**: a thumb não repete o título; no máximo 3-4 palavras; um assunto grande.
- **Descrição**: as 2 primeiras linhas (~150 caracteres) aparecem na busca: promessa + palavra-chave, em frase
  natural. Depois capítulos (dos beats `tipo: capitulo`), fontes, créditos, até 3 hashtags.
- **Tags**: peso pequeno na descoberta; servem para grafias alternativas e nome próprio (5-10 bastam).
- **Idioma**: `defaultLanguage`/`defaultAudioLanguage` corretos (o `vf` usa `factory.yaml > idioma`).

## YouTube Shorts (até 3 min, vertical)
- O título aparece sobre o vídeo no feed: **complementa o gancho falado, não repete**. ≤ 40 caracteres.
- O que decide é o primeiro segundo (imagem + fala), não a metadata; título novo raramente salva um Short
  com retenção baixa.
- Descrição curta (fontes/créditos) e `#Shorts` (o `vf` acrescenta se faltar). Ligue o Short ao vídeo longo
  relacionado no Studio (campo "vídeo relacionado"; não há na API).
- Thumbnail personalizada de Short não é confiável pela API: escolha o frame no app.

## TikTok
- **Legenda**: a primeira linha (≤ 80 caracteres) é o que aparece; ela reforça o gancho e diz do que se trata
  com as palavras que alguém digitaria na busca do TikTok. 3-5 hashtags **específicas** do tema (nada de
  `#fyp`/`#viral` genéricas). O texto na tela e a fala também são lidos pela busca: a palavra-chave deve
  aparecer em pelo menos um deles.
- `caption=` no `vf publish` (post direto) ou, sem ela, título + até 5 tags viram hashtags.
- Marque conteúdo de IA (`ia=1`, padrão) quando houver pessoas, lugares ou eventos realistas gerados.
- Capa: `capa_ms=<ms>` aponta o frame do gancho.

## Rubrica de título (0-2 cada, total 10)
| Critério | 0 | 1 | 2 |
|---|---|---|---|
| Clareza | não dá para saber do que é | entende com esforço | entende em 1 leitura |
| Curiosidade honesta | nenhuma ou isca que o vídeo não paga | alguma | lacuna clara que o vídeo fecha |
| Especificidade | genérico | um detalhe | nome/número/lugar concreto |
| Busca / contexto | sem termo buscável (longo) ou repete o gancho (Short) | parcial | termo no começo (longo) / complementa o gancho (Short) |
| Tamanho | estoura o limite visível | no limite | cabe com folga |

Recomende o maior total; empate → o mais curto. O revisor reprova qualquer título que prometa algo que o
roteiro não entrega (vale mais que qualquer pontuação).

## Limiares
Ponto de partida quando o canal ainda não tem mediana própria (< 5 vídeos do tipo). **A mediana do canal
vence sempre** (`medianas` no JSON do `vf analytics`).
| Sinal | Short (YT) | Longo (YT) | TikTok |
|---|---|---|---|
| % visto médio (averageViewPercentage) | bom ≥ 70 %; > 100 % = replays | bom ≥ 40-50 % | — (API não dá) |
| Retenção nos primeiros 3 s / 30 s | queda forte em 0-3 s = gancho | queda > 30 % em 30 s = abertura lenta | — |
| `relativeRetentionPerformance` | < 0,4 no trecho = pior que vídeos de mesma duração | idem | — |
| Engajamento (likes+coment+shares)/views | — | — | compare com a mediana; shares por mil é o sinal mais forte |

Sinais de diagnóstico rápido:
- views baixas + % visto ≥ mediana → **embalagem** (título/thumb/primeiro segundo);
- views ok + % visto baixo → **conteúdo/ritmo** (roteiro);
- tráfego quase todo de "feed de Shorts" e nada de "busca" num longo → título/descrição sem termo buscável;
- muitos inscritos ganhos num vídeo → formato a repetir (`pauta`).
