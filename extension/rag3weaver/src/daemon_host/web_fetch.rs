//! **fetch, the teammate's page reader** (the teammate's base, 20 November 2026, page 02 of rag3weaver-archi): a
//! web page as readable text — no script, no style, no markup — bounded and said, with its status, its type, its
//! final address and the redirects it followed. What goes out on the network follows a declared policy
//! (`teammate.fetch`): the schemes allowed, the hosts refused, and local addresses refused by default (a teammate
//! must not read the host's own services or the local network without the manifest saying so). No cookie is kept.
//! A refusal and an HTTP error are said in the reply, never a silent empty page.

use crate::backend::FetchPolicy;
use serde_json::{json, Value};
use std::io::Read;
use std::net::{IpAddr, ToSocketAddrs};

/// The raw bytes read from a response at most: the text is cut at `max_bytes` after extraction.
const RAW_CAP: u64 = 4 * 1024 * 1024;
/// The redirects followed at most.
const MAX_REDIRECTS: usize = 5;

/// An address of this host or of the local network.
fn is_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_loopback() || v.is_private() || v.is_link_local() || v.is_unspecified() || v.is_broadcast(),
        IpAddr::V6(v) => {
            v.is_loopback() || v.is_unspecified() || (v.segments()[0] & 0xfe00) == 0xfc00 || (v.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// The policy applied to one address: its scheme, its host, and where its host resolves.
fn check(policy: &FetchPolicy, url: &str) -> Result<(), String> {
    let uri: ureq::http::Uri = url.parse().map_err(|e| format!("fetch: « {url} » is not an address ({e})"))?;
    let scheme = uri.scheme_str().unwrap_or("");
    if !policy.schemes.iter().any(|s| s.eq_ignore_ascii_case(scheme)) {
        return Err(format!("fetch: the scheme « {scheme} » is not allowed (teammate.fetch.schemes: {})", policy.schemes.join(", ")));
    }
    let host = uri.host().ok_or_else(|| format!("fetch: « {url} » has no host"))?.trim_matches(['[', ']']);
    if policy.deny_hosts.iter().any(|h| h.eq_ignore_ascii_case(host) || host.to_ascii_lowercase().ends_with(&format!(".{}", h.to_ascii_lowercase()))) {
        return Err(format!("fetch: the host « {host} » is refused (teammate.fetch.deny_hosts)"));
    }
    if policy.allow_local {
        return Ok(());
    }
    let port = uri.port_u16().unwrap_or(if scheme.eq_ignore_ascii_case("https") { 443 } else { 80 });
    let addrs: Vec<IpAddr> = (host, port).to_socket_addrs().map_err(|e| format!("fetch: « {host} » does not resolve ({e})"))?.map(|a| a.ip()).collect();
    if host.eq_ignore_ascii_case("localhost") || addrs.iter().any(is_local) {
        let ip = addrs.iter().find(|a| is_local(a)).map(|a| a.to_string()).unwrap_or_else(|| host.to_string());
        return Err(format!("fetch: « {host} » is a local address ({ip}), refused by default (teammate.fetch.allow_local)"));
    }
    Ok(())
}

/// The address a `Location` points to, from the one that answered.
fn join(base: &str, location: &str) -> Result<String, String> {
    if location.contains("://") {
        return Ok(location.to_string());
    }
    let uri: ureq::http::Uri = base.parse().map_err(|e| format!("fetch: {e}"))?;
    let root = format!("{}://{}", uri.scheme_str().unwrap_or("http"), uri.authority().map(|a| a.as_str()).unwrap_or(""));
    if location.starts_with('/') {
        return Ok(format!("{root}{location}"));
    }
    let path = uri.path();
    let dir = &path[..path.rfind('/').map(|i| i + 1).unwrap_or(0)];
    Ok(format!("{root}{dir}{location}"))
}

/// Removes every `<tag …>…</tag>` block (case ignored).
fn drop_blocks(s: &str, tag: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let (open, close) = (format!("<{tag}"), format!("</{tag}>"));
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while let Some(a) = lower[i..].find(&open).map(|a| a + i) {
        out.push_str(&s[i..a]);
        match lower[a..].find(&close) {
            Some(b) => i = a + b + close.len(),
            None => return out,
        }
    }
    out.push_str(&s[i..]);
    out
}

/// HTML as readable text: scripts, styles and comments removed, blocks as lines, tags dropped, the common entities
/// decoded, spaces collapsed.
pub fn html_text(html: &str) -> String {
    let mut s = drop_blocks(html, "script");
    s = drop_blocks(&s, "style");
    s = drop_blocks(&s, "noscript");
    while let Some(a) = s.find("<!--") {
        match s[a..].find("-->") {
            Some(b) => s.replace_range(a..a + b + 3, " "),
            None => s.truncate(a),
        }
    }
    let mut text = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(a) = rest.find('<') {
        text.push_str(&rest[..a]);
        let Some(b) = rest[a..].find('>') else { break };
        let tag = rest[a + 1..a + b].trim_start_matches('/').to_ascii_lowercase();
        let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        let block = ["p", "br", "div", "li", "tr", "h1", "h2", "h3", "h4", "h5", "h6", "section", "article", "header", "footer", "pre", "table", "ul", "ol", "title"];
        text.push_str(if block.contains(&name.as_str()) { "\n" } else { " " });
        rest = &rest[a + b + 1..];
    }
    if !rest.contains('<') {
        text.push_str(rest);
    }
    let decoded = decode_entities(&text);
    decoded
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(a) = rest.find('&') {
        out.push_str(&rest[..a]);
        let after = &rest[a + 1..];
        let end = after.find(';').filter(|e| *e <= 10);
        let decoded = end.and_then(|e| {
            let name = &after[..e];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" | "#39" => Some('\''),
                "nbsp" => Some(' '),
                n if n.starts_with("#x") || n.starts_with("#X") => u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32),
                n if n.starts_with('#') => n[1..].parse::<u32>().ok().and_then(char::from_u32),
                _ => None,
            };
            c.map(|c| (c, e))
        });
        match decoded {
            Some((c, e)) => {
                out.push(c);
                rest = &after[e + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A text type: its body is given as text; any other is said by its size only.
fn is_text(content_type: &str) -> bool {
    let t = content_type.to_ascii_lowercase();
    t.is_empty() || t.starts_with("text/") || t.contains("json") || t.contains("xml") || t.contains("javascript")
}

/// **fetch {url, max_bytes?}**: the page as readable text, bounded at `max_bytes` (the policy's bound at most) with
/// the cut said in `cut_bytes`; the status, the type, the final address, the redirects followed. A status of 400 and
/// more gives `ok: false` with the status and what the page said; a refusal of the policy is an error.
pub fn fetch(policy: &FetchPolicy, args: &Value) -> Result<Value, String> {
    let url = args["url"].as_str().map(str::trim).filter(|u| !u.is_empty()).ok_or("fetch: url is required")?;
    let bound = policy.max_bytes.max(1);
    let max = args["max_bytes"].as_u64().map(|n| (n as usize).clamp(1, bound)).unwrap_or(bound);
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_global(Some(std::time::Duration::from_secs(policy.timeout_s.max(1))))
            .build(),
    );
    let mut current = url.to_string();
    let mut redirects = Vec::new();
    for _ in 0..=MAX_REDIRECTS {
        check(policy, &current)?;
        let mut resp = agent.get(&current).call().map_err(|e| format!("fetch {current}: {e}"))?;
        let status = resp.status().as_u16();
        if (300..400).contains(&status) {
            if let Some(location) = resp.headers().get("location").and_then(|v| v.to_str().ok()) {
                let next = join(&current, location)?;
                redirects.push(json!({ "status": status, "to": next }));
                current = next;
                continue;
            }
        }
        let content_type = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let mut raw = Vec::new();
        resp.body_mut()
            .with_config()
            .limit(RAW_CAP + 1)
            .reader()
            .take(RAW_CAP)
            .read_to_end(&mut raw)
            .map_err(|e| format!("fetch {current}: reading the body: {e}"))?;
        let mut text = if !is_text(&content_type) {
            format!("[{} bytes of {content_type}, not text]", raw.len())
        } else if content_type.to_ascii_lowercase().contains("html") {
            html_text(&String::from_utf8_lossy(&raw))
        } else {
            String::from_utf8_lossy(&raw).to_string()
        };
        let mut cut = 0;
        if text.len() > max {
            let mut at = max;
            while !text.is_char_boundary(at) {
                at -= 1;
            }
            cut = text.len() - at;
            text.truncate(at);
            text.push_str(&format!("\n[cut: {cut} more bytes of text; max_bytes = {max}]"));
        }
        let mut out = json!({
            "ok": status < 400,
            "status": status,
            "content_type": content_type,
            "url": current,
            "redirects": redirects,
            "text": text,
            "cut_bytes": cut,
        });
        if status >= 400 {
            out["error"] = json!(format!("HTTP {status} from {current}"));
        }
        return Ok(out);
    }
    Err(format!("fetch: more than {MAX_REDIRECTS} redirects from {url}"))
}
