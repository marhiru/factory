// Ponte (world isolado): recebe as capturas do inject.js e repassa ao service worker.
window.addEventListener("message", (ev) => {
  if (ev.source !== window || ev.origin !== location.origin) return;
  const m = ev.data;
  if (!m || m.marca !== "vf-studio-captura" || !/^[A-Za-z0-9_-]{11}$/.test(m.video || "")) return;
  chrome.runtime.sendMessage({ tipo: "captura", video: m.video, endpoint: m.endpoint, pagina: m.pagina, dados: m.dados }).catch(() => {});
});
