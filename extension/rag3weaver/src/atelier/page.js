import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
mermaid.initialize({ startOnLoad: false, theme: matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "default" });

const $ = (id) => document.getElementById(id);
let state = {};
let bulle = null;

// Ce qu'un rechargement a perdu en route, dit au journal.
function pertes(d) {
  let t = "";
  if ((d.removed_reactions || []).length) t += " — réactions retirées : " + d.removed_reactions.join(", ");
  if ((d.dropped_cursors || []).length) t += " — curseurs oubliés (événements perdus) : " + d.dropped_cursors.join(", ");
  if ((d.reaction_errors || []).length) t += " — erreurs des réactions arrêtées : " + d.reaction_errors.join(" | ");
  return t;
}

function ligne(classe, texte) {
  const d = document.createElement("div");
  d.className = classe;
  d.textContent = new Date().toLocaleTimeString() + " · " + texte;
  $("journal").prepend(d);
}

function message(classe, texte) {
  const d = document.createElement("div");
  d.className = "msg " + classe;
  d.textContent = texte;
  $("fil").append(d);
  $("fil").scrollTop = $("fil").scrollHeight;
  return d;
}

async function charger() {
  state = await (await fetch("/_atelier/state")).json();
  $("nom").textContent = "Atelier — " + state.name;
  $("etat").textContent = "version " + state.version + (state.chat.on ? " · modèle " + state.chat.model : "");
  if (!state.chat.on && !$("fil").querySelector(".eteint")) {
    message("eteint", "Le chat est éteint : " + state.chat.why);
  }
  const choix = $("routes").value;
  $("routes").innerHTML = "";
  for (const r of state.routes) {
    const o = document.createElement("option");
    o.value = r.slice(4);
    o.textContent = r;
    $("routes").append(o);
  }
  if (choix) $("routes").value = choix;
  if (!$("cadre").src && state.routes.length) {
    // D'abord une route qui a une vue : c'est une page à regarder.
    const vue = state.routes.find((r) => state.views[r]) || state.routes[0];
    montrer(vue.slice(4));
  }
}


async function dessiner(outil, surligne) {
  if (!outil) return;
  const url = "/_atelier/graph?tool=" + encodeURIComponent(outil) + (surligne ? "&highlight=" + encodeURIComponent(surligne) : "");
  const r = await fetch(url);
  if (!r.ok) return;
  const el = $("dessin");
  el.removeAttribute("data-processed");
  el.textContent = await r.text();
  await mermaid.run({ nodes: [el] });
}

function montrer(chemin) {
  $("routes").value = chemin;
  $("cadre").src = chemin;
  // Le graphe suit la page : celui de l'outil de la route montrée.
  dessiner((state.route_tools || {})["GET " + chemin]);
}

function rafraichir(chemins) {
  // Une vue touchée : la montrer ; un graphe touché : le dessiner.
  for (const p of chemins || []) {
    if (p.startsWith("views/")) {
      const vue = p.slice(6);
      const route = Object.entries(state.views || {}).find(([k, v]) => v === vue && k.startsWith("GET ") && !k.includes("{"));
      if (route) montrer(route[0].slice(4));
    }
    if (p.endsWith(".mmd")) {
      const outil = Object.entries(state.graphs || {}).find(([, g]) => g.endsWith(p));
      if (outil) dessiner(outil[0]);
    }
  }
  $("cadre").src = $("cadre").src;
  $("cadre").classList.add("vu");
  setTimeout(() => $("cadre").classList.remove("vu"), 1200);
}

const flux = new EventSource("/_atelier/events");
flux.addEventListener("token", (e) => {
  const d = JSON.parse(e.data);
  if (!bulle) bulle = message("agent", "");
  bulle.textContent += d.text;
});
flux.addEventListener("tool_start", (e) => {
  const d = JSON.parse(e.data);
  bulle = null;
  message("outil", "→ " + d.name);
});
flux.addEventListener("answer", () => { bulle = null; });
flux.addEventListener("declared", async (e) => {
  const d = JSON.parse(e.data);
  ligne("declared", "déclaré " + d.path + " (version " + d.version + (d.commit ? ", commit " + String(d.commit).slice(0, 9) : "") + ")" + pertes(d));
  await charger();
  rafraichir([d.path]);
});
flux.addEventListener("reloaded", async (e) => {
  const d = JSON.parse(e.data);
  ligne("reloaded", "rechargé " + (d.paths || []).join(", ") + " (version " + d.version + ")" + pertes(d));
  await charger();
  rafraichir(d.paths);
});
flux.addEventListener("refused", (e) => {
  const d = JSON.parse(e.data);
  ligne("refused", "refusé " + (d.path || (d.paths || []).join(", ")) + " — " + d.error);
});
flux.addEventListener("error", (e) => {
  if (e.data) ligne("refused", JSON.parse(e.data).text);
});

$("routes").addEventListener("change", () => montrer($("routes").value));
$("recharger").addEventListener("click", () => { $("cadre").src = $("cadre").src; });
$("envoi").addEventListener("submit", async (e) => {
  e.preventDefault();
  const texte = $("message").value.trim();
  if (!texte) return;
  $("message").value = "";
  message("moi", texte);
  bulle = null;
  await fetch("/_atelier/chat", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ message: texte }) });
});

await charger();
