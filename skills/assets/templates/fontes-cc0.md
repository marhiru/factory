# Bancos de imagem, vídeo e 3D (CC0 / stock)

| Fonte | Tipo | Licença | Atribuição |
|---|---|---|---|
| Wikimedia Commons | fotos históricas, mapas, obras | domínio público / CC0 / CC-BY(-SA) (varia por arquivo) | se CC-BY/SA: autor + licença na descrição |
| Biblioteca Nacional, Library of Congress, Rijksmuseum, Met Open Access | acervo histórico | domínio público | não exigida (recomendada) |
| NASA, ESA, USGS | espaço, satélite, Terra | domínio público (NASA/USGS); ESA CC-BY-SA IGO | ESA: sim |
| Pexels, Pixabay | fotos e vídeos stock | licença própria (uso comercial livre, sem revenda) | não exigida |
| Unsplash | fotos | licença Unsplash | não exigida |
| Kenney.nl | modelos 3D, sprites, UI, áudio | CC0 | não |
| Poly Haven | HDRIs, texturas, modelos | CC0 | não |
| Freesound (filtro CC0) | SFX | CC0 | não |
| OpenStreetMap / Natural Earth | mapas | ODbL / domínio público | OSM: "© OpenStreetMap contributors" |

## Como buscar e registrar
1. Busque pelo `visual` do beat, em inglês e em português.
2. Baixe para `public/img/` ou `public/clips/` com o nome `bNN-descricao.ext`.
3. Registre no `assets/manifest.yaml`:
```yaml
  - id: b03-porto
    tipo: imagem
    origem: cc0
    caminho: public/img/b03-porto.jpg
    url: https://commons.wikimedia.org/wiki/File:...
    autor: "Marc Ferrez"
    licenca: "Domínio público"
    beats: [3]
    status: aprovado
```
4. Confira cada imagem antes de usar: abra o arquivo e veja se mostra o que o beat diz, se não tem marca d'água nem texto embutido e se a resolução basta.

## Procedural quando não houver imagem
- Cards de dado (número grande, comparação, linha do tempo) no Remotion: clipes `text` com `style: number|title|label`.
- Mapas: Natural Earth + d3-geo no Remotion, ou a skill `mapas-3d-videos-virais`.
- Diagramas: SVG desenhado no componente.
