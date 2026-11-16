# Glossário de roteiro e retenção (com a tradução para a fábrica)

Termos de dramaturgia, comédia e métricas de plataforma usados nos templates e nas travas. A coluna "Na fábrica" mostra onde o termo vira regra ou `tipo` no `beats.json`.

## 1. Estrutura
| Termo | O que é | Na fábrica |
|---|---|---|
| **Hook (gancho)** | a 1ª frase/imagem que segura o polegar | `tipo: gancho`; 0-3 s no short, 0-5 s no longo |
| **Cold open** | começa no meio da ação, sem apresentação | o gancho é sempre um cold open |
| **Premise** | a situação absurda em uma frase | o gancho do short "história → mecanismo" |
| **Inciting incident** | o evento que tira a história do normal | 1ª frase do `contexto` |
| **Stakes** | o que se perde se der errado | `contexto`/`escalada` |
| **Escalation** | cada passo piora ou aumenta a aposta | `tipo: escalada` (2-4 seguidos) |
| **Rule of three** | dois casos estabelecem o padrão, o terceiro quebra | 3 tentativas: as duas primeiras falham igual, a terceira surpreende |
| **Reversal / peripeteia** | a situação vira para o lado oposto | `tipo: virada` |
| **Climax** | o ponto mais alto da tensão | a `virada` grande (55-70%) |
| **Payoff** | a entrega do que foi prometido | `tipo: payoff` |
| **Resolution / button** | a última frase que fecha com um "clique" | `tipo: fecho` |
| **Three-act structure** | começo, meio, fim (setup, confronto, resolução) | short: atos de ~15/30/15 s |
| **Kishōtenketsu** | 4 tempos: introdução, desenvolvimento, **virada sem conflito**, conclusão | bom para curiosidades sem vilão |

## 2. Engrenagens de roteiro
| Termo | O que é | Na fábrica |
|---|---|---|
| **"But / therefore"** (regra de South Park) | ligar as cenas com "mas" e "então", nunca com "e depois" | revisar frases que começam com "E depois": trocar por "Mas" ou "Então" |
| **"Luckily / Unluckily"** | alternar sorte e azar: cada solução gera um problema | escalada em ziguezague: "Por sorte… / Só que…" |
| **Setup / payoff (Chekhov's gun)** | o que aparece no começo precisa ser usado no fim | todo prop do gancho volta no `payoff` ou `fecho` |
| **Callback** | retomar algo do começo | última frase = imagem do gancho |
| **Runner / running gag** | piada que volta e escala | personagem-piada 2-3 vezes |
| **Comic escalation** | a mesma piada fica maior a cada volta | runner com aposta crescente (cadeira → lucro → cai da cadeira) |
| **Dramatic irony** | o público sabe algo que o personagem não sabe | mostrar o perigo antes de o personagem ver |
| **Underdog vindication ("laugh at a genius")** | riem do protagonista, que no fim estava certo | `escalada` com zombaria + `payoff` que dá razão a ele |
| **Mechanism reveal** | explicar como a coisa funciona por dentro | `payoff` do short "história → mecanismo" |
| **Show, don't tell** | a imagem conta, a fala acrescenta | `visual` nunca repete o `texto` |
| **Specificity** | detalhe concreto (número, nome, objeto) | "quinze por hora", "a cada doze metros" |
| **Concrete nouns / active verbs** | substantivos que viram imagem, verbos de ação | cada frase ilustrável em 1 plano |
| **Connective tissue** | frase-ponte entre beats | "Veja,", "Então,", "Só que…" |

## 3. Retenção e curiosidade
| Termo | O que é | Na fábrica |
|---|---|---|
| **Curiosity gap** | a distância entre o que se sabe e o que se quer saber | o gancho abre, o `payoff` fecha |
| **Open loop** | pergunta deixada em aberto | `tipo: loop` |
| **Re-hook** | novo gancho no meio do vídeo | `tipo: reganho` |
| **Pattern interrupt** | quebra de ritmo visual ou sonoro | cards, mudança de clima, crash zoom |
| **Bucket brigade** | frase curta que puxa para a próxima | "Mas tem um detalhe." |
| **Payoff density** | quantas recompensas (piada, dado, imagem) por minuto | short: uma a cada 5-8 s |
| **Loop ending** | o fim emenda no começo e o vídeo recomeça "sem querer" | última frase que completa a primeira |
| **Promise / title-thumbnail match** | o vídeo entrega o que a capa promete nos primeiros segundos | frame 0 = promessa do título |

## 4. Métricas (YouTube/TikTok)
| Termo | O que é | Meta de referência |
|---|---|---|
| **Viewed vs. swiped away** (Shorts) | % que ficou em vez de passar | > 70-80% |
| **APV (average percentage viewed)** | % médio assistido | short > 90-100% (com replays) |
| **AVD (average view duration)** | tempo médio assistido | longo > 40-50% da duração |
| **Retention graph / audience retention** | curva de quem continua assistindo | sem quedas bruscas (cada queda = beat fraco) |
| **Intro retention** (longo) | % que fica após 30 s | > 70% |
| **CTR (click-through rate)** | cliques / impressões (longo) | 4-10% |
| **Rewatch rate / loops** | replays no short | sobe com loop ending |
| **RPM** | receita por mil views | Shorts ~US$ 0,05-0,10; longo bem maior |

## 5. Voz e ritmo
| Termo | O que é | Na fábrica |
|---|---|---|
| **Read / delivery** | a interpretação do narrador | voz fixa do canal (`canal.yaml > voz`) |
| **Pacing** | velocidade (wpm) e variação | short 210-225 wpm, contínuo |
| **Beat (pausa)** | micro-pausa antes de uma punchline | no estilo estudado: ≤ 0,25 s, nunca mais |
| **Punchline / stinger line** | a frase que fecha a piada | última palavra = a mais forte ("toco") |
| **Sentence-length variation** | frases curtas para tensão, longas para explicar | gancho e viradas ≤ 12 palavras |
