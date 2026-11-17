---
name: canal
description: "Preparo e operação de um canal da fábrica de vídeo (YouTube/TikTok): define nicho, promessa, formato fixo, voz, elenco, estilo visual e regras de originalidade num canal.yaml; monta o banco de pautas com nota; checklist de criação do canal (nome, @, avatar, banner, sobre, playlists, padrões de upload); cadência de produção na esteira; e revisão semanal de métricas (viewed vs swiped, APV, retenção). Use quando o usuário quiser criar ou planejar um canal, escolher nicho, montar pauta de vídeos, ou revisar o desempenho de um canal."
argument-hint: "novo <nome> | pauta | revisar"
---

# Canal (antes do primeiro vídeo, e toda semana)

Ferramental JS/TS: siga [Bun e tsgo/tsrs](../factory/references/ferramental.md) ao executar ou alterar scripts e templates.

Um canal da fábrica é um **formato repetível**: o mesmo tipo de história, a mesma voz, o mesmo mundo visual e a mesma estrutura de roteiro, com temas sempre novos. O vídeo é feito pela `factory`; esta skill decide **o quê** e **para quem**, e mede o resultado.

Pasta do canal: `~/Projects/canais/<canal>/` com `canal.yaml`, `pauta.md`, `style-bible.md`, `metricas.md` e `kit/` (sprites do elenco fixo, plates/sets recorrentes, voz, logo). Os projetos de vídeo apontam para ela (`novo.sh --canal <pasta>`).

## Canal cartoon
Ao definir elenco e mundo recorrentes, use [cartoon-identidade.md](templates/cartoon-identidade.md): fontes dos desenhos, kit visual/sonoro, marcas próprias e decisões de idioma/legenda/mix. O exemplo da ponte é referência técnica, não identidade de canal.

## `novo <nome>`
1. Pergunte, numa pergunta só, o que faltar: **nicho/tema**, **idioma/público**, **formato** (short animado, longo estático ou os dois).
2. Copie `templates/canal.yaml` e `templates/pauta.md` para a pasta do canal e preencha. Use as referências estudadas (`skills/analise/referencias/`) **pela gramática**, nunca pelo tema.
3. **Teste de originalidade** (obrigatório, ver abaixo) antes de produzir.
4. Crie o kit: style bible (`assets` skill), elenco fixo (sprites, se animado), voz fixa (voice_id no ElevenLabs), 3-5 cenários recorrentes.
5. Siga `templates/setup.md` para criar o canal na plataforma (o usuário faz os cliques; você gera textos e imagens).
6. Produza 3 vídeos-piloto pela `factory` antes de publicar o primeiro: o formato precisa se provar repetível.

## `pauta`
Mantenha `pauta.md` com 30+ ideias com nota (critérios no template). Produza sempre a de maior nota que ainda não foi feita, alternando subtemas. Ideia com nota < 7 não entra na fila.

## `revisar` (semanal)
Puxe as métricas dos últimos vídeos com `vf analytics platform=yt|tiktok` (depois de `vf auth`; retenção por vídeo com `video=ID`, ver `skills/edicao/references/plataformas.md`), ou o usuário exporta do Studio/cola os números, para `metricas.md` e tire **uma** decisão por semana:
| Sinal | Leitura | Ação |
|---|---|---|
| Viewed vs. swiped < 70% | gancho/primeiro frame fracos | reescrever os 3 próximos ganchos; frame 0 = a ação absurda |
| APV < 80% num short | queda no meio | olhar o gráfico: o beat da queda vira regra no template |
| Um tema explode (10x a média) | subnicho validado | 3 vídeos derivados na pauta (mesma engrenagem, outro caso) |
| Views caem em série | fadiga do formato ou cópia | trocar o tipo de mecanismo/cenário, nunca a voz ou o elenco |

## Regras de originalidade (anti-"conteúdo inautêntico")
O YouTube agrupa vídeos parecidos (*semantic IDs*) e desmonetiza conteúdo repetitivo/inautêntico. Por isso:
- Copie **gramática** (ritmo, estrutura, tipos de FX), nunca tema, título, personagens, voz, paleta ou mundo de outro canal.
- Cada vídeo tem pesquisa própria com fonte e um ângulo que o canal de referência não fez.
- O canal tem **pelo menos 3 marcas próprias**: elenco, voz, paleta/mundo, bordão de abertura, ou um elemento visual recorrente.
- Nada de reenviar o mesmo vídeo com mudanças mínimas; recortes de um longo viram shorts com edição própria (`recorte`).
