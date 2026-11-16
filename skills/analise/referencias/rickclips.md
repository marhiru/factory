# Referência: canal de shorts "história absurda → mecanismo" (estudo de 2026-11-16)

Estudo feito com `vf analyze` em 3 shorts (6,5M, 29M e 34M de views) e nos títulos/views de 152 shorts do canal. Serve só pela **fórmula**. Nunca copie roteiro, personagens, marca ou planos.

## O que os números dizem
| | Short com mais FX (6,5M) | Top 2 (29M) | Top 1 (34M) |
|---|---|---|---|
| Duração | 59,4 s | ~70 s | ~65 s |
| Fala | 220 wpm, ocupa 96% | 197 wpm, 99% | 218 wpm, 95% |
| Maior pausa | 0,22 s | 0,07 s | 0,24 s |
| Planos | 20 (19 cortes/min, mediana 2,7 s) | 14 (12/min) | 18 (16/min) |
| Fala começa em | 0,04 s | 0,07 s | 0,02 s |

- **Não existe silêncio.** A narração é contínua do frame 0 ao fim, sem respiro maior que 0,25 s. O ritmo vem da fala, não do corte.
- **Planos longos são permitidos quando a câmera trabalha:** o 1º plano do short com FX dura 9,5 s, mas tem 4 enquadramentos dentro dele (push, travelling, aérea, reveal), costurados por dissolves de 2-3 frames.
- **Pivô do canal:** os shorts antigos de meme (títulos com 💀😭, 50k-500k views) perderam para o formato novo de história + explicação (5-34M). O formato é o que escala, não o humor solto.

## Fórmula do roteiro (os 3 seguem igual)
1. **Gancho = a ação estranha ou a consequência absurda, afirmada (nunca perguntada).** Até ~5 s.
   - "Se você plantar uma árvore do lado de uma estrada vazia, os carros vão virar direto nela."
   - "Ele colocou uma bola de bronze na boca de um dragão e disse ao imperador: se isso cair, corra."
   - "Ele serrou buracos num trilho a cada 12 metros."
2. **"Veja," / "É que…"**: a 2ª frase sempre abre com o mesmo conector e arma o cenário em uma frase.
3. **Escalada com quem duvida:** a solução óbvia piora tudo (a placa dobra os acidentes), ou alguém ri do protagonista (a corte, o chefe, os jornais). Cada passo aumenta a aposta.
4. **Personagem-piada recorrente:** alguém que lucra ou comenta o absurdo (o guincho de cadeira de praia, "o cara do brinquedo"). Volta 2-3 vezes e paga uma piada no clímax.
5. **Virada física no ~55-70%:** o absurdo chega ao máximo (o xerife bate devagar, todas as bolas caem de uma vez, o trilho "pula como cobra").
6. **Mecanismo no fim (últimos 10-15 s):** a explicação real, curta, com o **nome do fenômeno** dito uma vez ("isso se chama fixação no alvo") ou só mostrado. A explicação reinterpreta tudo o que se viu.
7. **Última frase = callback visual** ("…mesmo quando é só um toco"). Acaba na palavra, sem conclusão nem CTA.

Títulos que funcionam (4-7 palavras, curiosidade sem spoiler): "Por que trilhos perfeitos não funcionam", "O jarro antigo que avisou um império", "O instinto que faz você bater", "Como X funcionava", "O emprego que ninguém quer no Japão".

## Catálogo de FX (short com mais FX), com a receita na fábrica
| Momento | FX | Por que funciona | Receita (cutout `scene` / mygo ou `cartoon-3d-short`) |
|---|---|---|---|
| Abertura | 1 plano longo com vários enquadramentos + dissolves de 2-3 f | o olho nunca descansa, sem corte duro | vários clipes `scene` curtos do mesmo cenário com `transition_in: {kind: fade, dur: 0.08}` e câmeras diferentes |
| "toda semana" | **placa-contador** "DIAS SEM ACIDENTE 7 → 0" com flip | piada visual que conta a história sem fala | prop `fx` com estados `n7`/`n0` e key `state` no `at_word` do impacto |
| Batida | carro sobe na vertical, **fica parado no ar ~0,8 s** e cai; pneu sai rolando | o exagero cômico mora no *hold* | keys `rot` 0 → 85 (0,15 s, `out`), segura, 85 → 0 (0,2 s, `in`); `fx` `burst` + poeira; pneu = ator com `keys x` + `rot` |
| Placa gigante | letreiro de marquise com lâmpadas piscando ("NÃO BATA NESTA ÁRVORE") | o prop absurdo é a piada da escalada | sprite com ciclo de 2 frames (lâmpadas acesas/apagadas) a 6 fps |
| Fúria | olhos vermelhos brilhando, chão rachando, destroços flutuando | dá peso cômico à raiva | estado `rage` (olhos vermelhos) + `idle: jitter` + `fx` rachadura (`drop`) + detritos com `idle: float` |
| Clímax (machado) | raio no machado → **onda de choque**: anel branco, céu se abre, god rays, parede de poeira | hipérbole no ponto mais alto da história | `transition_in: flash` + `fx` anel (ciclo de 8 f escalando) + câmera `push` rápido + `burst`; o personagem-piada cai da cadeira em seguida |
| "como isso continua acontecendo?" | **zoom-through no olho**: a câmera entra na pupila e a próxima cena está dentro dela | transição que vira a chave para a explicação | clipe de 0,3 s: câmera `scale` 1,2 → 6 centrada no olho (`x`/`y`), `ease: in` + `transition_in: fade 0.1` no plano seguinte |
| Explicação | **muda o clima**: noite, chuva, neblina, faróis, POV do motorista | marca "agora é a ciência" sem dizer | plates noturnos/chuva + overlay de chuva (ciclo `fx`); câmera em primeira pessoa |
| Nome do fenômeno | aparece **dentro do mundo** (placa verde de rodovia "FIXAÇÃO NO ALVO") | texto sem cartela, sem quebrar a imersão | prop/plate com o termo pintado; câmera passa por ele no `at_word` |
| "travam nele" | olhos hipnóticos (espiral) | traduz o conceito em imagem | estado `hypno` com ciclo de espiral (8 f) |
| Fim | plano aberto com o toco + a placa piscando | callback do gancho | último clipe repete o cenário do 1º com o objeto mudado |
| Legendas | 1 palavra por vez, branca, bold, contorno preto, ~62% da altura | leitura sem som, sem cobrir o rosto | `captions: {words_per_page: 1, style: "pop"}` na safe zone |

## Regras que entram na fábrica
- Shorts de história: narração contínua (maior pausa ≤ 0,25 s; o QA estrito já cobra `max_voice_gap_s`).
- Um FX de exagero por ato: impacto com *hold* no ato 1, fúria/hipérbole no clímax, mudança de clima na explicação.
- Pelo menos 1 transição de assinatura (zoom-through, whip, match cut) na passagem história → explicação.
- Termos e números aparecem **no mundo** (placa, letreiro, contador) antes de virar cartela.
