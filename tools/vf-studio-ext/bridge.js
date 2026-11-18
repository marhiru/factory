// Ponte (world isolado): recebe as capturas do inject.js e repassa ao service worker.
// No modo ativo, rola a página devagar para o Studio carregar os cartões que só aparecem ao rolar.
chrome.runtime.onMessage.addListener((m) => {
  if (!m || m.tipo !== "rolar") return;
  let passos = 0;
  const rolaveis = () => [document.scrollingElement, ...document.querySelectorAll("*")]
    .filter((e) => e && e.scrollHeight > e.clientHeight + 50 && (e === document.scrollingElement || /(auto|scroll)/.test(getComputedStyle(e).overflowY)));
  const passo = () => {
    passos += 1;
    let mexeu = false;
    for (const e of rolaveis()) {
      const antes = e.scrollTop;
      e.scrollTop = antes + 500;
      if (e.scrollTop !== antes) mexeu = true;
    }
    if (mexeu && passos < 30) setTimeout(passo, 1200 + Math.random() * 800);
  };
  passo();
});

window.addEventListener("message", (ev) => {
  if (ev.source !== window || ev.origin !== location.origin) return;
  const m = ev.data;
  if (!m || m.marca !== "vf-studio-captura" || !/^([A-Za-z0-9_-]{11}|canal_UC[A-Za-z0-9_-]{10,30})$/.test(m.video || "")) return;
  chrome.runtime.sendMessage({ tipo: "captura", video: m.video, endpoint: m.endpoint, pagina: m.pagina, dados: m.dados }).catch(() => {});
});
