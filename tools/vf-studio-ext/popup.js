const $ = (id) => document.getElementById(id);
const msg = (m) => chrome.runtime.sendMessage(m);
const txt = (el, s, erro) => { el.textContent = s; el.className = erro ? "erro" : ""; };

async function init() {
  const ping = await msg({ tipo: "ping" });
  if (ping && ping.ok) txt($("host"), `Conectado ao vf (${ping.versao || "ok"}).`);
  else txt($("host"), `vf não conectado: rode "vf studio instalar" no terminal e recarregue a extensão. (${(ping && ping.erro) || "?"})`, true);

  const contas = await msg({ tipo: "contas" });
  for (const c of (contas && contas.contas) || []) {
    const o = document.createElement("option");
    o.value = c.apelido;
    o.textContent = c.nome ? `${c.apelido} (${c.nome})` : c.apelido;
    if (c.padrao) o.selected = true;
    $("conta").appendChild(o);
  }

  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  const m = tab && tab.url && tab.url.match(/studio\.youtube\.com\/video\/([A-Za-z0-9_-]{11})\/analytics/);
  const { capturas = {} } = await chrome.storage.local.get("capturas");
  if (m) {
    const c = capturas[m[1]];
    txt($("aqui"), c ? `Este vídeo: capturado ${new Date(c.quando).toLocaleString("pt-BR")} (${c.endpoints.length} resposta(s)).` : "Este vídeo: navegue pelas abas de análise para capturar.");
  } else {
    txt($("aqui"), `${Object.keys(capturas).length} vídeo(s) já capturados. Abra as análises de um vídeo no Studio para capturar ao navegar.`);
  }
  atualizar();
  setInterval(atualizar, 1000);
}

async function atualizar() {
  const { rodada, ultimoErroHost } = await chrome.storage.local.get(["rodada", "ultimoErroHost"]);
  if (!rodada) return txt($("rodada"), ultimoErroHost ? `Erro ao entregar ao vf: ${ultimoErroHost}` : "", !!ultimoErroHost);
  if (rodada.ativo) {
    const s = Math.max(0, Math.round((rodada.proxima - Date.now()) / 1000));
    txt($("rodada"), `Coletando ${rodada.i + 1}/${rodada.fila.length} (página ${rodada.j + 1}/${rodada.abas.length}) · próxima em ${s} s`);
  } else {
    txt($("rodada"), `Última rodada: ${rodada.motivo || "encerrada"} (${rodada.i + (rodada.motivo === "rodada concluída" ? 0 : 1)}/${rodada.fila.length}).`, rodada.motivo && !/concluída|você/.test(rodada.motivo));
  }
}

$("iniciar").addEventListener("click", async () => {
  let ids = $("ids").value.split(/[\s,;]+/).map((s) => s.trim()).filter(Boolean);
  const max = Math.min(30, Math.max(1, parseInt($("max").value, 10) || 10));
  if (!ids.length) {
    txt($("rodada"), "Pedindo a lista de vídeos ao vf…");
    const r = await msg({ tipo: "listar", conta: $("conta").value || null, max: 50 });
    if (!r || r.erro) return txt($("rodada"), `vf: ${(r && r.erro) || "sem resposta"}`, true);
    ids = r.videos.map((v) => v.id);
  }
  const r = await msg({ tipo: "iniciar", ids, max, perfil: $("perfil").value });
  if (!r || r.erro) return txt($("rodada"), (r && r.erro) || "falhou", true);
  atualizar();
});

$("parar").addEventListener("click", async () => {
  await msg({ tipo: "parar" });
  atualizar();
});

init();
