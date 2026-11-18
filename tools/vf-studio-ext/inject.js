// Roda no contexto da página do Studio (world MAIN). Só OBSERVA: copia as respostas das chamadas internas
// (youtubei) que o próprio Studio faz quando você abre as análises de um vídeo. Não faz nenhuma chamada nova.
(() => {
  const MARCA = "vf-studio-captura";
  const MAX = 2 * 1024 * 1024; // respostas maiores que 2 MB são ignoradas
  // vídeo (/video/<id>/analytics/...) ou canal (/channel/UC.../analytics/...) -> "canal_UC..."
  const videoAtual = () => {
    const v = location.pathname.match(/\/video\/([A-Za-z0-9_-]{11})\/analytics/);
    if (v) return v[1];
    const c = location.pathname.match(/\/channel\/(UC[A-Za-z0-9_-]{10,30})\/analytics/);
    return c ? `canal_${c[1]}` : null;
  };
  const enviar = (url, texto) => {
    const video = videoAtual();
    if (!video || !texto || texto.length > MAX) return;
    let dados;
    try { dados = JSON.parse(texto); } catch { return; }
    const endpoint = (new URL(url, location.href).pathname.split("/youtubei/v1/")[1] || "").replace(/[^A-Za-z0-9_\/-]/g, "");
    window.postMessage({ marca: MARCA, video, endpoint, pagina: location.pathname, dados }, location.origin);
  };
  const interessa = (url) => typeof url === "string" && url.includes("/youtubei/v1/") && videoAtual();

  const fetchOriginal = window.fetch;
  window.fetch = async function (...args) {
    const resp = await fetchOriginal.apply(this, args);
    try {
      const url = typeof args[0] === "string" ? args[0] : args[0] && args[0].url;
      if (interessa(url) && resp.ok) resp.clone().text().then((t) => enviar(url, t)).catch(() => {});
    } catch {}
    return resp;
  };

  const abrir = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (metodo, url, ...resto) {
    this.__vfUrl = url;
    return abrir.call(this, metodo, url, ...resto);
  };
  const mandar = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.send = function (...args) {
    if (interessa(this.__vfUrl)) {
      this.addEventListener("load", () => {
        try {
          if (this.status >= 200 && this.status < 300 && (this.responseType === "" || this.responseType === "text")) enviar(this.__vfUrl, this.responseText);
        } catch {}
      });
    }
    return mandar.apply(this, args);
  };
})();
