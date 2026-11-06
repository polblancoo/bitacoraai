//! Terminal integrada por el agente (`TM_AGENT_TERMINAL=1`): cuando una persona (owner) abre una terminal
//! del proyecto desde la web, este agente abre un shell en la carpeta del proyecto de ESTA máquina y lo
//! conecta al servidor. La conexión sale desde acá: no hacen falta claves, puertos abiertos ni IPs.
use crate::{log, Api};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde_json::Value;
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::{client::IntoClientRequest, Message}};

/// Consulta cada segundo si hay terminales pendientes y atiende cada una en su propia tarea.
pub async fn serve(api: Arc<Api>) {
    log("terminal habilitada (TM_AGENT_TERMINAL=1): las personas con rol owner pueden abrir un shell en esta máquina desde la web");
    let active: Arc<Mutex<HashSet<String>>> = Default::default();
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let Ok(list) = api.get(&format!("/api/agent/term/pending?herdr={}&ws={}", u8::from(has_herdr()), u8::from(workspace().is_some()))).await else { continue };
        for job in list.as_array().into_iter().flatten() {
            let Some(sid) = job["sid"].as_str().map(String::from) else { continue };
            if !active.lock().unwrap().insert(sid.clone()) {
                continue;
            }
            let (api, active, job) = (api.clone(), active.clone(), job.clone());
            tokio::spawn(async move {
                if let Err(e) = session(&api, &sid, &job).await {
                    log(&format!("terminal {sid}: {e}"));
                }
                active.lock().unwrap().remove(&sid);
            });
        }
    }
}

/// Raíz del modo servidor (`TM_WORKSPACE`): ahí cada persona tiene su espacio en `homeCrewHelm/<persona>/`.
fn workspace() -> Option<PathBuf> {
    std::env::var("TM_WORKSPACE").ok().filter(|w| !w.trim().is_empty()).map(|w| crate::iso::expand_home(w.trim()))
}

/// Espacio personal de una persona: su HOME en la terminal (herdr, shell, preferencias e intermedios persisten acá).
fn personal_home(ws: &Path, who: &str) -> std::io::Result<PathBuf> {
    let home = ws.join("homeCrewHelm").join(crate::iso::slug(who));
    std::fs::create_dir_all(home.join(".config"))?;
    let rc = home.join(".bashrc");
    if !rc.exists() {
        std::fs::write(&rc, "# Espacio personal de CrewHelm: editá este archivo, es solo tuyo y se conserva entre sesiones.\nexport PATH=\"$HOME/.local/bin:$PATH\"\nexport EDITOR=${EDITOR:-nano}\nPS1='\\[\\e[36m\\]\\u@crewhelm\\[\\e[0m\\] \\w \\$ '\n")?;
    }
    let profile = home.join(".profile");
    if !profile.exists() {
        std::fs::write(&profile, "[ -n \"$BASH_VERSION\" ] && [ -f \"$HOME/.bashrc\" ] && . \"$HOME/.bashrc\"\n")?;
    }
    Ok(home)
}

/// ¿Está `herdr` instalado en esta máquina? (en el PATH o en ~/.local/bin, donde lo deja su instalador)
fn has_herdr() -> bool {
    let local = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/bin/herdr"));
    local.is_some_and(|p| p.is_file()) || std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("herdr").is_file()))
}

async fn session(api: &Api, sid: &str, job: &Value) -> anyhow::Result<()> {
    let (cols, rows) = (job["cols"].as_u64().unwrap_or(80).clamp(10, 500) as u16, job["rows"].as_u64().unwrap_or(24).clamp(2, 200) as u16);
    let dir = PathBuf::from(job["dir"].as_str().unwrap_or(""));
    let url = format!("{}/api/agent/term/{sid}", api.cfg.url.replacen("http", "ws", 1));
    let mut req = url.into_client_request()?;
    req.headers_mut().insert("authorization", format!("Bearer {}", api.cfg.token).parse()?);
    let (mut ws, _) = connect_async(req).await?;

    let pair = native_pty_system().openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;
    let who = job["who"].as_str().unwrap_or("").to_string();
    let owner = job["owner"].as_bool().unwrap_or(false);
    let real_home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/root"));
    let mut cmd = match (workspace(), who.is_empty()) {
        (Some(root), false) => {
            let home = personal_home(&root, &who)?;
            if owner {
                // owner: shell completa con su espacio como HOME (las herramientas del servicio siguen en el PATH)
                let mut c = CommandBuilder::new_default_prog();
                let path = std::env::var("PATH").unwrap_or_default();
                c.env("PATH", format!("{}:{}:{path}", real_home.join(".local/bin").display(), real_home.join(".cargo/bin").display()));
                c.env("HOME", &home);
                c.env("CARGO_HOME", real_home.join(".cargo"));
                c.env("RUSTUP_HOME", real_home.join(".rustup"));
                log(&format!("terminal de {who} (owner): HOME = {}", home.display()));
                c
            } else {
                // miembro: cápsula bwrap; ve su espacio (escribible) y el proyecto en solo lectura, nada más. Sin bwrap no abre.
                if !crate::iso::bwrap_ok().await {
                    let _ = ws.send(Message::Binary(b"\r\nEsta terminal necesita `bwrap` en el servidor y no esta disponible.\r\n".to_vec().into())).await;
                    anyhow::bail!("terminal de {who} rechazada: falta bwrap");
                }
                let exe = std::env::current_exe()?;
                let mut ro = crate::iso::paths_for_shell();
                if dir.is_dir() {
                    ro.push(dir.clone());
                }
                let mut a = crate::iso::bwrap_args(&real_home, &crate::iso::Plan { work: &home, mine: &home, ia_home: Some(&home), exe: &exe, net: crate::iso::Net::Open, sock_dir: None, extra_rw: &[], extra_ro: &ro, new_session: false });
                if dir.is_dir() {
                    a.extend(["--chdir".into(), dir.to_string_lossy().to_string()]);
                }
                a.extend(["--".into(), std::env::var("SHELL").unwrap_or_else(|_| "bash".into())]);
                let mut c = CommandBuilder::new("bwrap");
                c.args(a);
                c.env_clear();
                c.env("PATH", std::env::var("PATH").unwrap_or_default());
                log(&format!("terminal de {who} (miembro) en sandbox: HOME = {}", home.display()));
                c
            }
        }
        _ => CommandBuilder::new_default_prog(),
    };
    cmd.env("TERM", "xterm-256color");
    if (owner || workspace().is_none()) && !dir.as_os_str().is_empty() && dir.is_dir() {
        cmd.cwd(&dir);
    }
    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    log(&format!("▶ terminal abierta en {}", if dir.is_dir() { dir.display().to_string() } else { "la carpeta por defecto".into() }));
    loop {
        tokio::select! {
            out = rx.recv() => match out {
                Some(b) => ws.send(Message::Binary(b.into())).await?,
                None => {
                    // el shell terminó (por ejemplo con `exit`): avisa y espera un momento la nota que manda el servidor
                    let _ = ws.send(Message::Text("{\"exit\":true}".into())).await;
                    let _ = tokio::time::timeout(Duration::from_secs(5), async {
                        while let Some(Ok(m)) = ws.next().await {
                            if let Message::Text(t) = m {
                                if let Ok(v) = serde_json::from_str::<Value>(&t) {
                                    if let (Some(rel), Some(body)) = (v["note"]["rel"].as_str(), v["note"]["body"].as_str()) {
                                        if let Err(e) = save_note(&dir, rel, body, v["note"]["replace"].as_bool().unwrap_or(false)) { log(&format!("no pude guardar la nota: {e}")); }
                                        break;
                                    }
                                }
                            }
                        }
                    }).await;
                    break;
                }
            },
            msg = ws.next() => match msg {
                Some(Ok(Message::Binary(b))) => { writer.write_all(&b)?; writer.flush()?; }
                Some(Ok(Message::Text(t))) => {
                    let Ok(v) = serde_json::from_str::<Value>(&t) else { continue };
                    if let Some(sz) = v.get("resize") {
                        let _ = pair.master.resize(PtySize { rows: sz[1].as_u64().unwrap_or(24).clamp(2, 200) as u16, cols: sz[0].as_u64().unwrap_or(80).clamp(10, 500) as u16, pixel_width: 0, pixel_height: 0 });
                    }
                    if let (Some(rel), Some(body)) = (v["note"]["rel"].as_str(), v["note"]["body"].as_str()) {
                        if let Err(e) = save_note(&dir, rel, body, v["note"]["replace"].as_bool().unwrap_or(false)) { log(&format!("no pude guardar la nota: {e}")); }
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            },
        }
    }
    let _ = child.kill();
    log("■ terminal cerrada");
    Ok(())
}

/// Agrega la nota de la sesión a `.tm/terminal-*.md` dentro de la carpeta del proyecto (y deja `.tm/` fuera de git).
/// `replace`: el archivo se reescribe (contexto de la tarea); si no, se agrega al final (historial de sesiones).
fn save_note(dir: &Path, rel: &str, body: &str, replace: bool) -> std::io::Result<()> {
    if !dir.is_dir() || !rel.starts_with(".tm/terminal-") || !rel.ends_with(".md") || rel.contains("..") || rel.contains('\\') || rel[4..].contains('/') {
        return Err(std::io::Error::other("ruta de nota no permitida"));
    }
    std::fs::create_dir_all(dir.join(".tm"))?;
    let mut f = std::fs::OpenOptions::new().create(true).write(true).append(!replace).truncate(replace).open(dir.join(rel))?;
    f.write_all(body.as_bytes())?;
    let exclude = dir.join(".git/info/exclude");
    if dir.join(".git").is_dir() && !std::fs::read_to_string(&exclude).unwrap_or_default().lines().any(|l| l == ".tm/") {
        let _ = std::fs::OpenOptions::new().create(true).append(true).open(exclude).and_then(|mut f| f.write_all(b"\n.tm/\n"));
    }
    Ok(())
}
