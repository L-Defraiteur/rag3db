"use strict";
const $ = id => document.getElementById(id);
const fragment = new URLSearchParams(location.hash.slice(1));
if (fragment.has("token")) {
  sessionStorage.setItem("chat-token", fragment.get("token"));
  history.replaceState(null, "", location.pathname);
}
const token = sessionStorage.getItem("chat-token") || "";
let session = localStorage.getItem("chat-session") || crypto.randomUUID();
let busy = false;
async function api(path, body) {
  const res = await fetch(path, {method: body === undefined ? "GET" : "POST", headers: {Authorization: `Bearer ${token}`, "Content-Type": "application/json"}, body: body === undefined ? undefined : JSON.stringify(body)});
  if (!res.ok) throw new Error((await res.json()).error || `HTTP ${res.status}`);
  return res;
}
function message(role, text = "") {
  $("messages").querySelector(".empty")?.remove();
  const item = document.createElement("article"); item.className = `message ${role}`;
  const label = document.createElement("div"); label.className = "role"; label.textContent = role === "user" ? "Vous" : role === "error" ? "Erreur" : "Agent";
  const content = document.createElement("div"); content.className = "body"; content.textContent = text;
  item.append(label, content); $("messages").append(item); return content;
}
function tool(name, args, result) {
  const details = document.createElement("details"), summary = document.createElement("summary"), pre = document.createElement("pre");
  summary.textContent = name; pre.textContent = args + (result ? `\n\n${result}` : "");
  details.append(summary, pre); $("messages").append(details); return pre;
}
function setBusy(value) {
  busy = value; $("send").disabled = value; $("stop").disabled = !value; $("new").disabled = value;
  $("status").textContent = value ? "Réflexion…" : "Prêt";
}
async function lists() {
  const [sessions, artifacts] = await Promise.all([api("/api/sessions").then(r => r.json()), api("/api/artifacts").then(r => r.json())]);
  $("sessions").replaceChildren();
  for (const item of sessions.sessions) {
    const b = document.createElement("button"); b.textContent = `${new Date(item.modified * 1000).toLocaleString()} · ${item.id.slice(0, 8)}`; b.className = item.id === session ? "active" : "";
    b.onclick = () => { if (!busy) load(item.id).catch(showError); }; $("sessions").append(b);
  }
  $("artifacts").replaceChildren();
  for (const file of artifacts.artifacts) {
    const b = document.createElement("button"); b.textContent = file.name; b.title = `${file.bytes} octets`;
    b.onclick = async () => { try { const blob = await (await api(`/api/artifacts/${encodeURIComponent(file.name)}`)).blob(); const url = URL.createObjectURL(blob), a = document.createElement("a"); a.href = url; a.download = file.name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000); } catch (e) { showError(e); } };
    $("artifacts").append(b);
  }
}
function showError(e) { message("error", e.message); $("status").textContent = "Erreur"; }
async function load(id) {
  const result = await (await api(`/api/history?session=${encodeURIComponent(id)}`)).json();
  session = id; localStorage.setItem("chat-session", id); $("messages").replaceChildren();
  const calls = new Map();
  for (const turn of result.turns) {
    if (turn.role === "tool") { const target = calls.get(turn.tool_call_id); if (target) target.textContent += `\n\n${turn.content}`; else tool(turn.tool_name || "Outil", "", turn.content); }
    else if (turn.content) message(turn.role, turn.content);
    for (const call of turn.tool_calls || []) calls.set(call.id, tool(call.name, call.arguments));
  }
  await lists(); $("messages").scrollTop = $("messages").scrollHeight;
}
$("new").onclick = () => load(crypto.randomUUID()).catch(showError);
$("stop").onclick = () => { $("status").textContent = "Annulation demandée…"; api("/api/cancel", {session}).catch(showError); };
$("message").onkeydown = e => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); if (!busy) $("composer").requestSubmit(); } };
$("composer").onsubmit = async e => {
  e.preventDefault(); if (busy) return;
  const text = $("message").value.trim(); if (!text) return;
  setBusy(true); message("user", text); $("message").value = "";
  let output = null, ended = false; const calls = new Map();
  function event(ev) {
    if (ev.event === "token") { output ||= message("assistant"); output.textContent += ev.text; }
    else if (ev.event === "tool_start") { calls.set(ev.id, tool(ev.name, ev.arguments)); output = null; $("status").textContent = ev.name; }
    else if (ev.event === "tool_end") { if (calls.has(ev.id)) calls.get(ev.id).textContent += `\n\n${ev.content}`; }
    else if (ev.event === "status") $("status").textContent = ev.text;
    else if (ev.event === "done") {
      ended = true; if (!ev.ok) throw new Error(ev.error);
      if (!output && ev.result.text) message("assistant", ev.result.text);
      if (ev.result.task_accepted === false) message("error", "Aucun résultat accepté par les validations pour cette demande.");
      else if (ev.result.task_accepted === true) message("assistant", "Résultat accepté par les validations.");
    }
    $("messages").scrollTop = $("messages").scrollHeight;
  }
  try {
    const response = await api("/api/chat", {session, message: text}), reader = response.body.getReader(), decoder = new TextDecoder(); let buffer = "";
    while (true) { const {value, done} = await reader.read(); buffer += decoder.decode(value, {stream: !done}); let end; while ((end = buffer.indexOf("\n")) >= 0) { const line = buffer.slice(0, end); buffer = buffer.slice(end + 1); if (line.trim()) event(JSON.parse(line)); } if (done) break; }
    if (!ended) throw new Error("Flux interrompu. Rechargez la conversation pour vérifier son état.");
  } catch (err) { showError(err); }
  finally { setBusy(false); await lists().catch(showError); $("message").focus(); }
};
(async () => { const info = await (await api("/api/info")).json(); $("name").textContent = info.name; $("model").textContent = `${info.model} · ${info.tools.length} outils${info.demo ? " · MODE DÉMONSTRATION" : ""}`; await load(session); })().catch(showError);
