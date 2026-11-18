// Service worker: entrega as capturas ao vf (native messaging) e conduz o modo ativo GENTIL:
// abre as páginas de análise de cada vídeo numa aba em segundo plano, uma página por vez, com espera
// aleatória longa entre elas, teto por rodada e por dia, e para ao primeiro sinal de problema.
// Nenhuma requisição é fabricada: quem chama a API é o próprio Studio, como se você estivesse navegando.
const HOST = "com.video_factory.studio";
const ESPERA_MIN = 35; // segundos entre páginas (o chrome.alarms não aceita menos de 30 s)
const ESPERA_MAX = 75;
const TETO_DIA = 30; // vídeos por dia no modo ativo
const RECENTE_H = 24; // não recoleta vídeo capturado há menos de 24 h
// o Studio novo usa o "hub" de análises; as abas antigas redirecionam para ele ou para a seção equivalente
const ABAS = {
  geral: ["hub_overview"],
  completo: ["hub_overview", "tab-reach_viewers/period-default", "tab-interest_viewers/period-default"],
};

const hoje = () => new Date().toISOString().slice(0, 10);
const get = async (k, padrao) => (await chrome.storage.local.get(k))[k] ?? padrao;
const set = (o) => chrome.storage.local.set(o);
const nativo = (msg) =>
  new Promise((ok) => {
    try {
      chrome.runtime.sendNativeMessage(HOST, msg, (r) => ok(chrome.runtime.lastError ? { erro: chrome.runtime.lastError.message } : r || { erro: "sem resposta" }));
    } catch (e) {
      ok({ erro: String(e) });
    }
  });

async function registrarCaptura(m) {
  const r = await nativo({ tipo: "captura", video: m.video, endpoint: m.endpoint, pagina: m.pagina, dados: m.dados, quando: new Date().toISOString() });
  const idx = await get("capturas", {});
  const c = idx[m.video] || { endpoints: [] };
  c.quando = Date.now();
  if (!c.endpoints.includes(m.endpoint)) c.endpoints.push(m.endpoint);
  c.erro = r && r.erro ? r.erro : null;
  idx[m.video] = c;
  await set({ capturas: idx, ultimoErroHost: r && r.erro ? r.erro : null });
}

// ---------------- modo ativo
async function iniciar({ ids, max, perfil }) {
  const atual = await get("rodada", null);
  if (atual && atual.ativo) return { erro: "já existe uma rodada em andamento" };
  const idx = await get("capturas", {});
  const dia = await get("dia", { data: hoje(), n: 0 });
  if (dia.data !== hoje()) Object.assign(dia, { data: hoje(), n: 0 });
  const livres = Math.max(0, TETO_DIA - dia.n);
  const fila = ids
    .filter((id) => /^[A-Za-z0-9_-]{11}$/.test(id))
    .filter((id) => !idx[id] || Date.now() - idx[id].quando > RECENTE_H * 3600e3)
    .slice(0, Math.min(max || 10, livres));
  if (!fila.length) return { erro: livres ? "nada a coletar (tudo capturado nas últimas 24 h)" : `teto diário de ${TETO_DIA} vídeos atingido; volte amanhã` };
  const abas = ABAS[perfil] || ABAS.geral;
  const tab = await chrome.tabs.create({ url: url(fila[0], abas[0]), active: false });
  rolarDepois(tab.id);
  await set({ rodada: { ativo: true, fila, abas, i: 0, j: 0, tabId: tab.id, inicio: Date.now(), motivo: null, proxima: agendar() }, dia: { data: dia.data, n: dia.n + 1 } });
  return { ok: true, total: fila.length };
}

const url = (id, aba) => `https://studio.youtube.com/video/${id}/analytics/${aba}`;

// os cartões do hub carregam ao aparecer na tela: depois de abrir a página, rola devagar até o fim
function rolarDepois(tabId) {
  setTimeout(() => chrome.tabs.sendMessage(tabId, { tipo: "rolar" }).catch(() => {}), 8000);
}

function agendar() {
  const s = ESPERA_MIN + Math.random() * (ESPERA_MAX - ESPERA_MIN);
  chrome.alarms.create("vf-proximo", { delayInMinutes: s / 60 });
  return Date.now() + s * 1000;
}

async function parar(motivo) {
  chrome.alarms.clear("vf-proximo");
  const r = await get("rodada", null);
  if (!r) return;
  if (r.tabId) chrome.tabs.remove(r.tabId).catch(() => {});
  await set({ rodada: { ...r, ativo: false, motivo, fim: Date.now() } });
}

chrome.alarms.onAlarm.addListener(async (a) => {
  if (a.name !== "vf-proximo") return;
  const r = await get("rodada", null);
  if (!r || !r.ativo) return;
  let tab;
  try {
    tab = await chrome.tabs.get(r.tabId);
  } catch {
    return parar("a aba da coleta foi fechada");
  }
  // sessão caiu, captcha ou página fora do Studio: para (nunca insiste)
  if (!tab.url || !tab.url.startsWith("https://studio.youtube.com/")) return parar("o Studio saiu da sessão ou pediu verificação: abra o Studio e confira");
  let { i, j } = r;
  j += 1;
  if (j >= r.abas.length) {
    i += 1;
    j = 0;
    if (i < r.fila.length) {
      const dia = await get("dia", { data: hoje(), n: 0 });
      if (dia.data === hoje() && dia.n >= TETO_DIA) return parar(`teto diário de ${TETO_DIA} vídeos`);
      await set({ dia: { data: hoje(), n: (dia.data === hoje() ? dia.n : 0) + 1 } });
    }
  }
  if (i >= r.fila.length) return parar("rodada concluída");
  await chrome.tabs.update(r.tabId, { url: url(r.fila[i], r.abas[j]) });
  rolarDepois(r.tabId);
  await set({ rodada: { ...r, i, j, proxima: agendar() } });
});

chrome.tabs.onRemoved.addListener(async (id) => {
  const r = await get("rodada", null);
  if (r && r.ativo && r.tabId === id) parar("a aba da coleta foi fechada");
});

// ---------------- mensagens
chrome.runtime.onMessage.addListener((m, sender, responder) => {
  (async () => {
    if (m.tipo === "captura") {
      if (!sender.url || !sender.url.startsWith("https://studio.youtube.com/")) return responder({ erro: "origem inválida" });
      await registrarCaptura(m);
      return responder({ ok: true });
    }
    if (sender.id !== chrome.runtime.id || sender.tab) return responder({ erro: "origem inválida" });
    if (m.tipo === "iniciar") return responder(await iniciar(m));
    if (m.tipo === "parar") {
      await parar("parado por você");
      return responder({ ok: true });
    }
    if (["ping", "contas", "listar"].includes(m.tipo)) return responder(await nativo(m));
    responder({ erro: "tipo desconhecido" });
  })();
  return true;
});
