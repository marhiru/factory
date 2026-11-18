# Sinais do Studio: o que cada número diz e o que fazer

Dados capturados pela extensão `vf studio` (não oficial) e lidos por `vf studio resumo`, `vf relatorio` e
`vf analytics` (`videos[].studio`). Complementam a API oficial: chegam **antes** dela (horas, não dias) e trazem
o que ela não tem (continuaram × ignorado, impressões e CTR, views por hora).

## O funil: diagnostique na ordem, pare no primeiro elo fraco

```
Short:  aparece no feed → CONTINUARAM ASSISTINDO → % visto / retenção → gostei/compartilhou → inscreveu
Longo:  IMPRESSÕES → CTR → retenção (30 s, 50 %) → duração média → inscreveu
```
Consertar um elo de baixo não salva um elo de cima quebrado: se metade desliza no primeiro segundo, mexer no
final do vídeo não muda nada.

| Sinal | Fonte | Lê assim | Se estiver fraco (contra a mediana do canal) |
|---|---|---|---|
| **Continuaram assistindo** (Shorts) | Studio | % de quem viu o Short no feed e não deslizou | o **primeiro segundo** (imagem + primeira palavra) não segurou: reescreva o gancho (skill `roteiro`, `ganchos.md`), primeiro quadro com movimento e assunto claro, texto na tela que complete a fala. Título/descrição não mudam isso |
| **Impressões** (longo) | Studio | quantas vezes a thumb foi mostrada | o YouTube não está testando o vídeo: tema/busca (pauta), título sem termo buscável, ou canal sem sinal ainda |
| **CTR das impressões** (longo) | Studio | % de quem viu a thumb e clicou | **embalagem**: thumb + título (fluxo `metadata`). A Ajuda do YouTube diz que metade dos canais/vídeos fica entre 2 % e 10 %; compare com a mediana do canal e com vídeos da mesma origem de tráfego |
| **Views por hora** (primeiras 1/3/6/24 h) | Studio | velocidade de distribuição | ver "Primeiras horas" abaixo |
| **Fontes de tráfego** | Studio / API | de onde veio a audiência | Short com quase tudo do Feed dos Shorts é o normal; longo sem "Pesquisa"/"Sugeridos" depois de 7 dias = título/descrição sem termo de busca ou tema sem demanda |
| **% visto / retenção** | API (24-72 h) | quanto do vídeo as pessoas veem | fluxo `retencao`, cruzando quedas com os beats |
| **Inscritos gerados** | API/Studio | conversão | formato a repetir quando alto; sem chamada clara/sem promessa de série quando baixo |

## Primeiras horas (curva por hora)

A série por hora começa no momento da publicação **só se** o vídeo foi aberto no Studio com a extensão nas
primeiras ~48 h (cada captura traz as últimas 48 h). Sem isso, `primeiras_horas.cobre = false`.

Leia nesta ordem:
1. **1ª hora × mediana da 1ª hora** dos Shorts do canal: é o teste inicial do YouTube com o público mais
   próximo (inscritos e quem já assistiu o canal).
2. **Forma da curva:**
   - sobe ou fica alta até a 3ª-6ª hora → o feed está expandindo o teste: não mexa em nada;
   - pico na 1ª-2ª hora e queda forte → o teste inicial não convenceu (cruze com **continuaram assistindo**);
   - plana e baixa desde o início → pouca exposição: horário de publicação, ou o tema não pegou no público
     do canal.
3. **24 h × mediana de 24 h**: o veredito do primeiro dia (fluxo `upload`).

Pontos de checagem sugeridos depois de publicar: **1 h, 3 h e 24 h** (abrir as análises do vídeo no Studio
com a extensão e rodar `vf relatorio video=<ID>`).

## Confiabilidade (diga sempre ao usuário)
- Dado do Studio é **não oficial** (lido da tela do Studio); os nomes podem mudar e um cartão pode sumir.
- `processando` = o Studio mostra 0 porque ainda não calculou (comum nas primeiras horas): não é zero.
- O período de cada cartão está em `periodo` ("Desde a publicação", "48 horas"...): não misture com a janela
  d7/d28 da API.
- Amostra pequena (< 5 Shorts com captura) → hipótese, não conclusão. A mediana com n=0 não compara nada.
- "Continuaram assistindo" é de Shorts; vídeo longo usa impressões/CTR.
