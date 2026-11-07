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
            if workspace().is_some() {
                let falta = crate::iso::missing_shell_tools();
                if !falta.is_empty() {
                    log(&format!("faltan herramientas de terminal para las personas: {} (se instalan con  sh install.sh --shell-tools-only )", falta.join(", ")));
                }
            }
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
    if v["limits"].is_object() {
        crate::iso::set_limits(&v["limits"]);
    }
    Some(Config { terminal: v["terminal"].as_bool()?, people: v["people"].as_bool().unwrap_or(false), ssh: v["ssh"].as_bool().unwrap_or(false) })
}

/// Raíz de los espacios personales (`homeCrewHelm/<persona>/`): `TM_WORKSPACE` si se fijó; si no, `~/crewhelm` cuando la app permite la terminal a las personas.
pub(crate) fn workspace() -> Option<PathBuf> {
    if let Some(w) = std::env::var("TM_WORKSPACE").ok().filter(|w| !w.trim().is_empty()) {
        return Some(crate::iso::expand_home(w.trim()));
    }
    PEOPLE.load(Ordering::Relaxed).then(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("crewhelm"))
}

/// Espacio personal de una persona: su HOME en la terminal (herdr, shell, preferencias e intermedios persisten acá).
fn personal_home(ws: &Path, who: &str) -> std::io::Result<PathBuf> {
    let home = ws.join("homeCrewHelm").join(crate::iso::slug(who));
    std::fs::create_dir_all(home.join(".config"))?;
    std::fs::create_dir_all(home.join(".cache"))?;
    // configuración por defecto (zsh y bash con colores, eza, fzf, starship…): se actualiza sola mientras la persona no la toque
    install_rc(&home.join(".zshrc"), include_str!("../assets/zshrc"))?;
    install_rc(&home.join(".bashrc"), include_str!("../assets/bashrc"))?;
    // `cw`: acciones de git con palabras simples (actualizar, rama, guardar, enviar); se reescribe en cada sesión
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin)?;
    std::fs::write(bin.join("cw"), include_str!("../assets/cw"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(bin.join("cw"), std::fs::Permissions::from_mode(0o755))?;
    }
    let profile = home.join(".profile");
    if !profile.exists() {
        std::fs::write(&profile, "[ -n \"$BASH_VERSION\" ] && [ -f \"$HOME/.bashrc\" ] && . \"$HOME/.bashrc\"\n")?;
    }
    Ok(home)
}

/// Escribe `content` si el archivo no existe, o si es un archivo de CrewHelm (primera línea `# crewhelm-default vN`, o el `.bashrc` de la
/// primera versión) de una versión anterior. Si la persona borró esa primera línea, el archivo es suyo y no se toca.
fn install_rc(path: &Path, content: &str) -> std::io::Result<()> {
    let version = |t: &str| t.lines().next().and_then(|l| l.strip_prefix("# crewhelm-default v")).and_then(|v| v.trim().parse::<u32>().ok());
    let new_v = version(content).unwrap_or(1);
    let replace = match std::fs::read_to_string(path) {
        Err(_) => true,
        Ok(cur) => version(&cur).is_some_and(|v| v < new_v) || cur.starts_with("# Espacio personal de CrewHelm"),
    };
    if replace {
        std::fs::write(path, content)?;
    }
    Ok(())
}

/// ¿Está `herdr` instalado en esta máquina? (en el PATH o en ~/.local/bin, donde lo deja su instalador)
fn has_herdr() -> bool {
    let local = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/bin/herdr"));
    local.is_some_and(|p| p.is_file()) || std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("herdr").is_file()))
}

/// Última vez que se trajo `origin` de cada proyecto (para no pedirle a GitHub en cada terminal).
static FETCHED: std::sync::LazyLock<Mutex<std::collections::HashMap<PathBuf, std::time::Instant>>> = std::sync::LazyLock::new(Default::default);

/// Actualiza lo que se sabe de GitHub en la carpeta del proyecto (solo las ramas remotas: no toca archivos ni la rama en uso).
async fn refresh_origin(project: &Path) {
    {
        let mut m = FETCHED.lock().unwrap();
        if m.get(project).is_some_and(|t| t.elapsed() < Duration::from_secs(60)) {
            return;
        }
        m.insert(project.to_path_buf(), std::time::Instant::now());
    }
    if !project.join(".git").exists() {
        return;
    }
    let mut c = tokio::process::Command::new("git");
    c.args(["fetch", "-q", "origin"]).current_dir(project).env("GIT_TERMINAL_PROMPT", "0").env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes -o ConnectTimeout=10").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).kill_on_drop(true);
    let _ = tokio::time::timeout(Duration::from_secs(25), async { c.status().await }).await;
}

/// Pedidos que una persona hace desde su terminal con `cw enviar`: se atienden acá, con las credenciales del servicio,
/// que nunca entran a la cápsula. La rama se trae a la carpeta del proyecto DENTRO de un sandbox (sin red ni credenciales:
/// la configuración de la copia de la persona no se ejecuta con permisos del servicio) y recién ahí se sube a GitHub.
struct Pedidos {
    api: Arc<Api>,
    home: PathBuf,
    copy: PathBuf,
    project_dir: PathBuf,
    pid: i64,
    who: String,
    prefix: String,
}

impl Pedidos {
    async fn watch(self, stop: Arc<AtomicBool>) {
        let dir = self.home.join(".crewhelm/pedidos");
        let _ = std::fs::create_dir_all(&dir);
        while !stop.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            let ids: Vec<String> = rd.flatten().filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".go")).map(String::from)).collect();
            for id in ids {
                if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) || dir.join(format!("{id}.fin")).exists() {
                    continue;
                }
                let _ = std::fs::remove_file(dir.join(format!("{id}.go")));
                let read = |ext: &str| std::fs::read_to_string(dir.join(format!("{id}.{ext}"))).unwrap_or_default().chars().take(4000).collect::<String>();
                let (rama, titulo, cuerpo) = (read("rama"), read("titulo"), read("cuerpo"));
                let res = self.enviar(&dir.join(format!("{id}.out")), rama.trim(), titulo.trim(), cuerpo.trim()).await;
                let _ = std::fs::write(dir.join(format!("{id}.fin")), if res { "ok" } else { "error" });
            }
        }
    }

    async fn enviar(&self, out: &Path, rama: &str, titulo: &str, cuerpo: &str) -> bool {
        let say = |s: &str| {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(out) {
                let _ = writeln!(f, "{s}");
            }
        };
        let valida = rama.starts_with(&format!("{}/", self.prefix))
            && rama.len() <= 100
            && !rama.contains("..")
            && !rama.ends_with(['/', '.'])
            && !rama.ends_with(".lock")
            && rama.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.'));
        if !valida {
            say("\x1b[31mNombre de rama no permitido.\x1b[0m");
            return false;
        }
        let git_dir = self.project_dir.join(".git");
        if !git_dir.is_dir() || !self.copy.join(".git").is_dir() {
            say("\x1b[31mEste proyecto no es un repositorio git en el servidor.\x1b[0m");
            return false;
        }
        say("1/3 Llevando tu trabajo al proyecto…");
        let reference = format!("refs/crewhelm/{rama}");
        let mut f = tokio::process::Command::new("bwrap");
        f.args(["--ro-bind", "/", "/", "--bind"]).arg(&git_dir).arg(&git_dir).args(["--dev", "/dev", "--proc", "/proc", "--unshare-net", "--unshare-pid", "--unshare-ipc", "--die-with-parent", "git", "-C"]);
        f.arg(&self.project_dir)
            .args(["-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false", "-c", "core.alternateRefsCommand=/bin/false", "-c", "protocol.file.allow=always", "fetch", "-q"])
            .arg(&self.copy)
            .arg(format!("+refs/heads/{rama}:{reference}"))
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .kill_on_drop(true);
        match tokio::time::timeout(Duration::from_secs(60), f.output()).await {
            Ok(Ok(o)) if o.status.success() => {}
            Ok(Ok(o)) => {
                say(&format!("\x1b[31mNo pude leer tu rama: {}\x1b[0m", crate::tail(&String::from_utf8_lossy(&o.stderr), 200)));
                return false;
            }
            _ => {
                say("\x1b[31mNo pude leer tu rama (tardó demasiado o falta bwrap).\x1b[0m");
                return false;
            }
        }
        say("2/3 Subiendo la rama a GitHub…");
        let mut p = tokio::process::Command::new("git");
        p.arg("-C").arg(&self.project_dir).args(["push", "-q", "origin", &format!("{reference}:refs/heads/{rama}")]).env("GIT_TERMINAL_PROMPT", "0").env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes -o ConnectTimeout=10").kill_on_drop(true);
        let pushed = tokio::time::timeout(Duration::from_secs(90), p.output()).await;
        let _ = tokio::process::Command::new("git").arg("-C").arg(&self.project_dir).args(["update-ref", "-d", &reference]).status().await;
        match pushed {
            Ok(Ok(o)) if o.status.success() => {}
            Ok(Ok(o)) => {
                let e = String::from_utf8_lossy(&o.stderr).to_string();
                let msg = if e.contains("rejected") || e.contains("non-fast-forward") { "esa rama ya existe en el proyecto con otros cambios: creá una rama nueva (cw rama)".to_string() } else { crate::tail(&e, 200) };
                say(&format!("\x1b[31mNo pude subir la rama: {msg}\x1b[0m"));
                return false;
            }
            _ => {
                say("\x1b[31mNo pude subir la rama (sin conexión con GitHub o tardó demasiado).\x1b[0m");
                return false;
            }
        }
        say("3/3 Pidiendo la revisión (Pull Request)…");
        match self.api.call(reqwest::Method::POST, "/api/agent/pr", Some(serde_json::json!({ "project": self.pid, "who": self.who, "branch": rama, "title": titulo, "body": cuerpo }))).await {
            Ok(v) => {
                say(&format!("\x1b[32m✔ Listo. Quien revisa ya puede verlo: {}\x1b[0m", v["url"].as_str().unwrap_or("")));
                true
            }
            Err(e) => {
                say(&format!("\x1b[33mTu rama quedó subida, pero no se abrió el Pull Request: {}\x1b[0m", crate::tail(&e.to_string(), 200)));
                false
            }
        }
    }
}

/// Copia de trabajo de una persona: `~/<proyecto>` (clon local del repositorio del proyecto, con su mismo `origin`).
/// Se crea la primera vez y después se reutiliza tal cual (sus cambios no se tocan). Devuelve el nombre de la carpeta.
async fn working_copy(project: &std::path::Path, home: &std::path::Path, name: &str) -> Option<String> {
    let slug = crate::iso::slug(name);
    let dest = home.join(&slug);
    if dest.join(".git").exists() {
        return Some(slug);
    }
    if !project.join(".git").exists() || dest.exists() {
        return None;
    }
    let quiet = |c: &mut tokio::process::Command| {
        c.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    };
    let mut clone = tokio::process::Command::new("git");
    clone.args(["clone", "--local"]).arg(project).arg(&dest).current_dir(home);
    quiet(&mut clone);
    if !clone.status().await.is_ok_and(|s| s.success()) {
        return None;
    }
    // que `origin` apunte al mismo remoto que el proyecto (GitHub), no a la carpeta local
    if let Ok(o) = tokio::process::Command::new("git").args(["config", "--get", "remote.origin.url"]).current_dir(project).output().await {
        let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !url.is_empty() {
            let mut set = tokio::process::Command::new("git");
            set.args(["remote", "set-url", "origin", &url]).current_dir(&dest);
            quiet(&mut set);
            let _ = set.status().await;
        }
    }
    Some(slug)
}

async fn session(api: &Arc<Api>, sid: &str, job: &Value) -> anyhow::Result<()> {
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
    let stop = Arc::new(AtomicBool::new(false));
    let mut quota: Option<(PathBuf, u64)> = None;
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
                if let Some(n) = job["project"].as_str().filter(|n| !n.is_empty()) {
                    c.env("CREWHELM_PROJECT", crate::iso::slug(n));
                }
                log(&format!("terminal de {who} (owner): HOME = {}", home.display()));
                c
            } else {
                // miembro: cápsula bwrap; ve su espacio (escribible) y el proyecto en solo lectura, nada más. Sin bwrap no abre.
                if !crate::iso::bwrap_ok().await {
                    let _ = ws.send(Message::Binary(b"\r\nEsta terminal necesita `bwrap` en el servidor y no esta disponible.\r\n".to_vec().into())).await;
                    anyhow::bail!("terminal de {who} rechazada: falta bwrap");
                }
                // cuota de disco del espacio personal: lleno = no abre hasta que se libere; cerca del límite = aviso
                let limit = crate::iso::quota_mb() * 1024 * 1024;
                let mut quota_note = String::new();
                if limit > 0 && view.is_none() {
                    let hh = home.clone();
                    let used = tokio::task::spawn_blocking(move || crate::iso::dir_usage(&hh, 25)).await.unwrap_or(0);
                    if used >= limit {
                        let _ = ws.send(Message::Binary(format!("\r\nTu espacio esta lleno: usas {} MB de {} MB. Pedile a un owner que te ayude a liberar lugar o que amplie el limite.\r\n", used >> 20, limit >> 20).into_bytes().into())).await;
                        anyhow::bail!("terminal de {who} rechazada: espacio personal lleno ({} MB)", used >> 20);
                    }
                    if used * 10 >= limit * 8 {
                        quota_note = format!("echo 'Atencion: tu espacio esta al {}% ({} MB de {} MB). Con cw espacio ves que ocupa lugar.'; ", used * 100 / limit, used >> 20, limit >> 20);
                    }
                    quota = Some((home.clone(), limit));
                }
                let tools = crate::iso::person_tools();
                let pname = job["project"].as_str().unwrap_or("proyecto");
                let proj = (target.is_none() && dir.is_dir()).then_some((dir.as_path(), pname));
                // copia de trabajo propia del proyecto dentro del espacio de la persona (escribible): ahí edita, crea archivos y hace commits
                if view.is_none() {
                    if let Some((d, _)) = proj {
                        refresh_origin(d).await;
                    }
                }
                let work = match proj {
                    Some((d, n)) if view.is_none() => working_copy(d, &home, n).await,
                    _ => None,
                };
                if let (Some(w), Some(pid)) = (&work, job["pid"].as_i64()) {
                    let ctx = Pedidos { api: api.clone(), home: home.clone(), copy: home.join(w), project_dir: dir.clone(), pid, who: who.clone(), prefix: crate::iso::ascii_slug(&who) };
                    let f2 = stop.clone();
                    tokio::spawn(async move { ctx.watch(f2).await });
                }
                let mut a = crate::iso::person_args(&crate::iso::PersonPlan { home: &home, project: proj, other_space: target.as_deref(), tools: &tools, work: work.as_deref(), who: &crate::iso::ascii_slug(&who) });
                // aviso de dónde está parada la persona (el nombre sale de slug(): sin comillas ni símbolos)
                let hello = match (&view, proj) {
                    (Some(v), _) => format!("echo 'Espacio de {} (solo lectura) en /espacio. Tu espacio es ~'", crate::iso::slug(v)),
                    (None, Some(_)) if work.is_some() => format!("echo 'Estás en tu copia de trabajo de {0} (~/{0}): acá podés crear y editar. El original compartido está en /proyectos/{0} (solo lectura)'", crate::iso::slug(pname)),
                    (None, Some(_)) => format!("echo 'Proyecto {0}: todavía no tiene repositorio git, por eso solo podés mirar sus archivos en /proyectos/{0}. Pedí que lo creen con el botón «Pedir repositorio» (abajo de la terminal). Tu espacio es ~ (escribible)'", crate::iso::slug(pname)),
                    _ => "echo 'Tu espacio es ~ (escribible)'".to_string(),
                };
                let sh = crate::iso::person_shell();
                let upd = if work.is_some() { "cw actualizar --auto; " } else { "" };
                a.extend(["--".into(), "/bin/bash".into(), "-c".into(), format!("{hello}; {quota_note}{upd}exec {sh}")]);
                let (prog, a, scoped) = crate::iso::with_limits("bwrap", a);
                let mut c = CommandBuilder::new(prog);
                c.args(a);
                c.env_clear();
                c.env("PATH", std::env::var("PATH").unwrap_or_default());
                if scoped {
                    for (k, v) in crate::iso::systemd_env() {
                        c.env(k, v);
                    }
                }
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
    let mut tick = tokio::time::interval(Duration::from_secs(90));
    tick.tick().await;
    let mut warned = false;
    loop {
        tokio::select! {
            _ = tick.tick(), if quota.is_some() => {
                let (hh, limit) = quota.clone().unwrap();
                let used = tokio::task::spawn_blocking(move || crate::iso::dir_usage(&hh, 25)).await.unwrap_or(0);
                if used >= limit / 100 * 125 {
                    let _ = ws.send(Message::Binary(format!("\r\n\x1b[1;31mTu espacio supero el limite ({} MB de {} MB): cierro la terminal. Libera lugar (cw espacio te muestra que ocupa) y vuelve a abrirla.\x1b[0m\r\n", used >> 20, limit >> 20).into_bytes().into())).await;
                    log(&format!("terminal de {who} cerrada: espacio personal {} MB sobre el limite de {} MB", used >> 20, limit >> 20));
                    break;
                } else if used >= limit && !warned {
                    warned = true;
                    let _ = ws.send(Message::Binary(format!("\r\n\x1b[1;33mAtencion: tu espacio llego al limite ({} MB de {} MB). Libera lugar con cw espacio; si sigue creciendo se cierra la terminal.\x1b[0m\r\n", used >> 20, limit >> 20).into_bytes().into())).await;
                } else if used < limit {
                    warned = false;
                }
            }
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
    stop.store(true, Ordering::Relaxed);
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
        let mut a = crate::iso::person_args(&crate::iso::PersonPlan { home: &home, project: None, other_space: None, tools: &tools, work: None, who: "" });
        a.push("--".into());
        a.push(crate::iso::person_shell().into());
        let (prog, a, scoped) = crate::iso::with_limits("bwrap", a);
        let mut c = std::process::Command::new(prog);
        c.args(a).env_clear().env("PATH", std::env::var("PATH").unwrap_or_default()).env("TERM", term);
        if scoped {
            c.envs(crate::iso::systemd_env());
        }
        c.exec()
    };
    Err(err.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(dir: &Path, args: &[&str]) -> String {
        let o = std::process::Command::new("git").args(["-c", "user.email=a@b", "-c", "user.name=a"]).args(args).current_dir(dir).output().unwrap();
        assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }

    /// `cw enviar`: la rama de la persona llega a GitHub (acá, un repositorio local) sin dejar referencias sueltas;
    /// una rama que no es de la persona se rechaza.
    #[tokio::test]
    async fn enviar_sube_la_rama_de_la_persona() {
        if !std::process::Command::new("bwrap").args(["--ro-bind", "/", "/", "--dev", "/dev", "true"]).status().is_ok_and(|s| s.success()) {
            return;
        }
        let d = std::env::temp_dir().join(format!("tm-env-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (gh, proj, home) = (d.join("gh.git"), d.join("proj"), d.join("home"));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&proj).unwrap();
        sh(&d, &["init", "-q", "--bare", "gh.git"]);
        sh(&proj, &["init", "-q", "-b", "main"]);
        std::fs::write(proj.join("a.txt"), "hola").unwrap();
        sh(&proj, &["add", "."]);
        sh(&proj, &["commit", "-q", "-m", "a"]);
        sh(&proj, &["remote", "add", "origin", gh.to_str().unwrap()]);
        sh(&proj, &["push", "-q", "origin", "main"]);
        let copy = home.join("proj");
        sh(&home, &["clone", "-q", "--local", proj.to_str().unwrap(), "proj"]);
        sh(&copy, &["switch", "-q", "-c", "marta/tema"]);
        std::fs::write(copy.join("b.txt"), "nuevo").unwrap();
        sh(&copy, &["add", "."]);
        sh(&copy, &["commit", "-q", "-m", "b"]);
        let cfg = crate::Cfg { url: "http://127.0.0.1:1".into(), token: String::new(), cmd: String::new(), poll: 1, timeout: 1, max: 1, fallback_dir: None, once: true };
        let p = Pedidos { api: Arc::new(Api { http: reqwest::Client::new(), cfg }), home: home.clone(), copy: copy.clone(), project_dir: proj.clone(), pid: 1, who: "Marta".into(), prefix: "marta".into() };
        let out = d.join("out");
        // sin servidor, el Pull Request no se abre (devuelve false) pero la rama ya tiene que estar subida
        assert!(!p.enviar(&out, "marta/tema", "t", "").await);
        assert!(sh(&gh, &["branch", "--list", "marta/tema"]).contains("marta/tema"), "salida: {}", std::fs::read_to_string(&out).unwrap_or_default());
        assert!(sh(&proj, &["for-each-ref", "refs/crewhelm"]).is_empty());
        // una rama ajena o con trucos no se acepta
        assert!(!p.enviar(&d.join("out2"), "otra/cosa", "t", "").await);
        assert!(!p.enviar(&d.join("out3"), "marta/../main", "t", "").await);
        assert!(sh(&gh, &["branch", "--list", "otra/cosa"]).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

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
