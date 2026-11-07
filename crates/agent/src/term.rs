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
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::{client::IntoClientRequest, Message}};

/// Consulta cada segundo si hay terminales pendientes y atiende cada una en su propia tarea.
pub async fn serve(api: Arc<Api>) {
    let active: Arc<Mutex<HashSet<String>>> = Default::default();
    let (mut cfg, mut n, mut said) = (Config::default(), 0u32, false);
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        // la app dice si esta máquina es la terminal de algún proyecto y si abre espacios para las personas (sin variables de entorno)
        if n % 10 == 0 {
            if let Some(c) = fetch_config(&api).await {
                PEOPLE.store(c.people, Ordering::Relaxed);
                cfg = c;
            }
        }
        n = n.wrapping_add(1);
        if !(cfg.terminal || std::env::var("TM_AGENT_TERMINAL").is_ok_and(|v| v == "1")) {
            continue;
        }
        if !said {
            said = true;
            log(&format!("terminal habilitada: los owners abren un shell en esta máquina desde la web{}", if workspace().is_some() { "; las demás personas entran a su espacio personal dentro de un sandbox" } else { "" }));
        }
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

/// Lo que la app le indica a este agente sobre su terminal.
#[derive(Default, Clone)]
struct Config {
    terminal: bool,
    people: bool,
    ssh: bool,
}

/// Un owner dejó prendido «permitir la terminal a las personas del equipo» en la app.
static PEOPLE: AtomicBool = AtomicBool::new(false);

async fn fetch_config(api: &Api) -> Option<Config> {
    let v = api.get("/api/agent/term/config").await.ok()?;
    Some(Config { terminal: v["terminal"].as_bool()?, people: v["people"].as_bool().unwrap_or(false), ssh: v["ssh"].as_bool().unwrap_or(false) })
}

/// Raíz de los espacios personales (`homeCrewHelm/<persona>/`): `TM_WORKSPACE` si se fijó; si no, `~/crewhelm` cuando la app permite la terminal a las personas.
fn workspace() -> Option<PathBuf> {
    if let Some(w) = std::env::var("TM_WORKSPACE").ok().filter(|w| !w.trim().is_empty()) {
        return Some(crate::iso::expand_home(w.trim()));
    }
    PEOPLE.load(Ordering::Relaxed).then(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("crewhelm"))
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
    // un owner mirando el espacio de otra persona: solo lectura, dentro de la cápsula
    let view = job["view"].as_str().filter(|v| !v.is_empty()).map(String::from);
    let real_home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/root"));
    let mut cmd = match (workspace(), who.is_empty()) {
        (Some(root), false) => {
            let home = personal_home(&root, &who)?;
            let target = view.as_ref().map(|v| root.join("homeCrewHelm").join(crate::iso::slug(v)));
            if view.is_some() && !target.as_ref().is_some_and(|t| t.is_dir()) {
                let _ = ws.send(Message::Binary(b"\r\nEsa persona todavia no tiene espacio personal en este servidor.\r\n".to_vec().into())).await;
                return Ok(());
            }
            if owner && view.is_none() {
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
                let tools = crate::iso::person_tools();
                let pname = job["project"].as_str().unwrap_or("proyecto");
                let proj = (target.is_none() && dir.is_dir()).then_some((dir.as_path(), pname));
                let mut a = crate::iso::person_args(&crate::iso::PersonPlan { home: &home, project: proj, other_space: target.as_deref(), tools: &tools });
                // aviso de dónde está parada la persona (el nombre sale de slug(): sin comillas ni símbolos)
                let hello = match (&view, proj) {
                    (Some(v), _) => format!("echo 'Espacio de {} (solo lectura) en /espacio. Tu espacio es ~'", crate::iso::slug(v)),
                    (None, Some(_)) => format!("echo 'Proyecto {0}: sus archivos están en /proyectos/{0} (solo lectura). Tu espacio es ~ (escribible)'", crate::iso::slug(pname)),
                    _ => "echo 'Tu espacio es ~ (escribible)'".to_string(),
                };
                a.extend(["--".into(), "/bin/bash".into(), "-c".into(), format!("{hello}; exec /bin/bash")]);
                let mut c = CommandBuilder::new("bwrap");
                c.args(a);
                c.env_clear();
                c.env("PATH", std::env::var("PATH").unwrap_or_default());
                log(&format!("terminal de {who} ({}) en sandbox: HOME = {}", if view.is_some() { "owner, mirando otro espacio en solo lectura" } else { "miembro" }, home.display()));
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

// ---------- entrada por SSH al usuario de servicio (TM_SSH_KEYS=1): cada persona con su clave y su espacio ----------

const KEYS_BEGIN: &str = "# BEGIN crewhelm (lo escribe tm-agent: no edites este bloque)";
const KEYS_END: &str = "# END crewhelm";

/// Cada minuto copia las claves públicas de las personas (las cargan en su perfil) a `~/.ssh/authorized_keys`, en un bloque propio.
/// Cada clave solo puede abrir `tm-agent ssh-session <persona> <nivel>`: sin comandos sueltos, túneles, agente ni X11.
pub async fn sync_keys(api: Arc<Api>) {
    let Ok(exe) = std::env::current_exe() else { return };
    let exe = exe.to_string_lossy().to_string();
    if exe.contains(['"', '\n', '\\']) {
        log("entrada por SSH: la ruta de tm-agent tiene caracteres que no puedo usar en authorized_keys");
        return;
    }
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else { return };
    let mut last: Option<String> = None;
    loop {
        // se activa desde la app («Permitir entrar por SSH»); TM_SSH_KEYS=1 sigue valiendo como alternativa
        if let Some(cfg) = fetch_config(&api).await {
            PEOPLE.store(cfg.people, Ordering::Relaxed);
            let on = (cfg.ssh || std::env::var("TM_SSH_KEYS").is_ok_and(|v| v == "1")) && workspace().is_some();
            let mut block = String::new();
            if on {
                let root = workspace().unwrap_or_default().to_string_lossy().to_string();
                if root.contains(['"', '\n', '\\', ' ']) {
                    log("entrada por SSH: la carpeta de espacios no puede tener espacios ni comillas");
                } else if let Ok(list) = api.get("/api/agent/ssh-keys").await {
                    for k in list.as_array().into_iter().flatten() {
                        let (Some(who), Some(key), Some(level)) = (k["who"].as_str(), k["key"].as_str(), k["level"].as_str()) else { continue };
                        let safe = !who.is_empty() && who.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
                        if !safe || !matches!(level, "owner" | "member") || key.contains(['\n', '\r', '"']) {
                            continue;
                        }
                        block.push_str(&format!("restrict,pty,command=\"{exe} ssh-session {who} {level} {root}\" {key} crewhelm:{who}\n"));
                    }
                } else {
                    block = last.clone().unwrap_or_default();
                }
            }
            // sin nada que escribir y nada escrito antes, no se toca authorized_keys
            if Some(&block) != last.as_ref() && !(last.is_none() && block.is_empty()) {
                match write_keys(&home, &block) {
                    Ok(()) => { log(&format!("entrada por SSH: authorized_keys actualizado ({} claves)", block.lines().count())); last = Some(block); }
                    Err(e) => log(&format!("no pude escribir authorized_keys: {e}")),
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

fn write_keys(home: &Path, block: &str) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let dir = home.join(".ssh");
    std::fs::create_dir_all(&dir)?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    let path = dir.join("authorized_keys");
    let cur = std::fs::read_to_string(&path).unwrap_or_default();
    let mut keep = String::new();
    let mut skipping = false;
    for l in cur.lines() {
        if l == KEYS_BEGIN { skipping = true; continue; }
        if l == KEYS_END { skipping = false; continue; }
        if !skipping { keep.push_str(l); keep.push('\n'); }
    }
    if !block.is_empty() {
        keep.push_str(&format!("{KEYS_BEGIN}\n{block}{KEYS_END}\n"));
    }
    let tmp = dir.join("authorized_keys.tm");
    std::fs::write(&tmp, keep)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(tmp, path)
}

/// Comando forzado de una clave: abre el shell de la persona. Owner: shell completa con su espacio como HOME; miembro: dentro de `bwrap`.
pub async fn ssh_session(who: &str, level: &str, root: &str) -> anyhow::Result<()> {
    use std::os::unix::process::CommandExt;
    if std::env::var_os("SSH_ORIGINAL_COMMAND").is_some_and(|c| !c.is_empty()) {
        anyhow::bail!("solo se permiten sesiones interactivas (ssh usuario@servidor, sin comando)");
    }
    // sshd arranca este comando sin el entorno del agente: la carpeta de espacios viene en la línea de la clave
    let root = Some(root).filter(|r| !r.is_empty()).map(PathBuf::from).or_else(workspace).ok_or_else(|| anyhow::anyhow!("falta la carpeta de espacios personales"))?;
    let home = personal_home(&root, who)?;
    let real_home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/root"));
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".into());
    let term = std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".into());
    println!("CrewHelm · {who} · tu espacio: {} (se conserva entre sesiones)", home.display());
    let err = if level == "owner" {
        let path = std::env::var("PATH").unwrap_or_default();
        std::process::Command::new(&shell)
            .current_dir(&home)
            .env("HOME", &home)
            .env("TERM", term)
            .env("PATH", format!("{}:{}:{path}", real_home.join(".local/bin").display(), real_home.join(".cargo/bin").display()))
            .env("CARGO_HOME", real_home.join(".cargo"))
            .env("RUSTUP_HOME", real_home.join(".rustup"))
            .exec()
    } else {
        if !crate::iso::bwrap_ok().await {
            anyhow::bail!("esta entrada necesita `bwrap` en el servidor y no está disponible");
        }
        let tools = crate::iso::person_tools();
        let mut a = crate::iso::person_args(&crate::iso::PersonPlan { home: &home, project: None, other_space: None, tools: &tools });
        a.extend(["--".into(), "/bin/bash".into()]);
        std::process::Command::new("bwrap").args(a).env_clear().env("PATH", std::env::var("PATH").unwrap_or_default()).env("TERM", term).exec()
    };
    Err(err.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorized_keys_conserva_lo_ajeno_y_reemplaza_el_bloque() {
        let d = std::env::temp_dir().join(format!("tm-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join(".ssh")).unwrap();
        std::fs::write(d.join(".ssh/authorized_keys"), "ssh-ed25519 AAAAmia yo@pc\n").unwrap();
        write_keys(&d, "restrict,pty,command=\"x ssh-session ana member\" ssh-ed25519 AAAAana crewhelm:ana\n").unwrap();
        write_keys(&d, "restrict,pty,command=\"x ssh-session bob owner\" ssh-ed25519 AAAAbob crewhelm:bob\n").unwrap();
        let t = std::fs::read_to_string(d.join(".ssh/authorized_keys")).unwrap();
        assert!(t.contains("AAAAmia") && t.contains("AAAAbob") && !t.contains("AAAAana"));
        assert_eq!(t.matches(KEYS_BEGIN).count(), 1);
        write_keys(&d, "").unwrap();
        let t = std::fs::read_to_string(d.join(".ssh/authorized_keys")).unwrap();
        assert_eq!(t, "ssh-ed25519 AAAAmia yo@pc\n");
        let _ = std::fs::remove_dir_all(&d);
    }
}
