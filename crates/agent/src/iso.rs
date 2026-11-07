//! Aislamiento de las IAs (ver `Informes/plan-aislamiento-fases-1-3.md`):
//! fase 1, una copia de trabajo por tarea con su rama y el push hecho por `tm-agent`;
//! fase 2, sandbox por ejecución con `bwrap`; fase 3, red solo hacia destinos permitidos.
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream, UnixListener, UnixStream},
    process::Command,
};

/// Destinos a los que una IA aislada puede salir por HTTPS (el sufijo cubre subdominios). Un admin suma más desde la app.
pub const DEFAULT_ALLOW: &[&str] = &[
    "anthropic.com", "claude.ai", "openai.com", "openrouter.ai", "generativelanguage.googleapis.com", "oauth2.googleapis.com", "accounts.google.com",
    "github.com", "githubusercontent.com", "gitlab.com", "crates.io", "rust-lang.org", "npmjs.org", "npmjs.com", "pypi.org", "pythonhosted.org",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Net {
    /// sin aislar la red (solo con `TM_SANDBOX_NET=open`)
    Open,
    /// sin red propia: sale por el proxy que filtra destinos
    Proxy,
    /// sin red salvo el puente al servidor de CrewHelm
    None,
}

#[derive(Clone, Debug)]
pub struct Settings {
    /// raíz del modo servidor (`TM_WORKSPACE`); sin ella, todo vive dentro de la carpeta de cada proyecto
    pub workspace: Option<PathBuf>,
    /// copia de trabajo por tarea con su rama
    pub copy: bool,
    pub sandbox: bool,
    pub net: Net,
    pub allow_hosts: Vec<String>,
    pub extra_rw: Vec<PathBuf>,
    pub extra_ro: Vec<PathBuf>,
}

fn paths_env(key: &str) -> Vec<PathBuf> {
    std::env::var(key).unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()).map(expand_home).collect()
}

pub fn expand_home(p: &str) -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match (p.strip_prefix("~/").or_else(|| (p == "~").then_some("")), home) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(p),
    }
}

impl Settings {
    /// `level` es el aislamiento de la ficha de la IA (`""`, `copy` o `sandbox`); `ai_pr`, si el proyecto abre Pull Requests (necesita rama);
    /// `server_hosts`, los destinos extra que fijó un admin en la app.
    pub fn resolve(level: &str, ai_pr: bool, server_hosts: &str) -> Self {
        let env_sandbox = std::env::var("TM_SANDBOX").is_ok_and(|v| v == "bwrap");
        let sandbox = level == "sandbox" || env_sandbox;
        let copy = sandbox || level == "copy" || ai_pr || std::env::var("TM_WORKTREE").is_ok_and(|v| v == "1");
        let net = match std::env::var("TM_SANDBOX_NET").as_deref() {
            Ok("open") => Net::Open,
            Ok("none") => Net::None,
            _ => Net::Proxy,
        };
        let mut allow_hosts: Vec<String> = DEFAULT_ALLOW.iter().map(|s| s.to_string()).collect();
        let extra = std::env::var("TM_SANDBOX_NET_ALLOW").unwrap_or_default();
        for h in extra.split(',').chain(server_hosts.split(|c: char| c == ',' || c.is_whitespace())) {
            let h = h.trim().trim_start_matches("*.").to_lowercase();
            if !h.is_empty() && !allow_hosts.contains(&h) {
                allow_hosts.push(h);
            }
        }
        Self {
            workspace: std::env::var("TM_WORKSPACE").ok().filter(|w| !w.trim().is_empty()).map(|w| expand_home(w.trim())),
            copy,
            sandbox,
            net,
            allow_hosts,
            extra_rw: paths_env("TM_SANDBOX_RW"),
            extra_ro: paths_env("TM_SANDBOX_RO"),
        }
    }
}

/// Nombre seguro para usar en rutas.
/// Como `slug` pero solo con a-z, 0-9 y guiones (nombres de ramas de git).
pub fn ascii_slug(s: &str) -> String {
    let t: String = s.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
    if t.is_empty() { "x".into() } else { t.chars().take(30).collect() }
}

pub fn slug(s: &str) -> String {
    let t: String = s.to_lowercase().chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '-' }).collect::<String>().split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
    if t.is_empty() { "x".into() } else { t }
}

// ---------------------------------------------------------------- fase 1: copia de trabajo y rama

async fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = tokio::time::timeout(
        Duration::from_secs(180),
        Command::new("git").arg("-C").arg(dir).args(args).env("GIT_TERMINAL_PROMPT", "0").env("GIT_ASKPASS", "true").stdin(Stdio::null()).output(),
    )
    .await
    .map_err(|_| format!("git {} tardó demasiado", args.first().unwrap_or(&"")))?
    .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub async fn is_git(dir: &Path) -> bool {
    git(dir, &["rev-parse", "--git-dir"]).await.is_ok() && git(dir, &["rev-parse", "--verify", "-q", "HEAD"]).await.is_ok()
}

/// La copia de trabajo de una tarea.
#[derive(Clone, Debug)]
pub struct Work {
    /// dónde trabaja la IA
    pub dir: PathBuf,
    /// repositorio del proyecto (de donde sale la copia y adonde vuelve la rama)
    pub origin: PathBuf,
    pub branch: Option<String>,
    pub base: Option<String>,
}

pub fn branch_of(task: i64) -> String {
    format!("tm/tarea-{task}")
}

pub fn work_path(ws: Option<&Path>, project_dir: &Path, project: &str, task: i64) -> PathBuf {
    match ws {
        Some(w) => w.join("proyectos").join(slug(project)).join("tareas").join(task.to_string()),
        None => project_dir.join(".crewhelm").join("wt").join(format!("tarea-{task}")),
    }
}

/// Espacio personal de la IA para lo intermedio de una tarea.
pub fn mine_path(ws: Option<&Path>, project_dir: &Path, project: &str, ia: &str, task: i64) -> PathBuf {
    match ws {
        Some(w) => w.join("homeCrewHelm").join(slug(ia)).join(slug(project)).join(task.to_string()),
        None => project_dir.join(".crewhelm").join(slug(ia)).join(task.to_string()),
    }
}

async fn has_branch(repo: &Path, name: &str) -> bool {
    git(repo, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{name}")]).await.is_ok()
}

/// Rama de la que sale la copia: la de la tarea madre si existe; si no, `main`, `master` o la actual.
async fn base_branch(repo: &Path, parent: Option<i64>) -> String {
    if let Some(p) = parent {
        let b = branch_of(p);
        if has_branch(repo, &b).await {
            return b;
        }
    }
    for c in ["main", "master"] {
        if has_branch(repo, c).await {
            return c.into();
        }
    }
    git(repo, &["rev-parse", "--abbrev-ref", "HEAD"]).await.unwrap_or_else(|_| "HEAD".into())
}

/// Crea (o reutiliza) la copia aislada de la tarea: un clon local (los objetos se enlazan, casi no ocupa) en la rama `tm/tarea-N`.
/// Si la carpeta del proyecto no es un repositorio, no hay copia: se trabaja directo en ella.
pub async fn prepare(project_dir: &Path, ws: Option<&Path>, project: &str, task: i64, parent: Option<i64>) -> Result<Work, String> {
    if !is_git(project_dir).await {
        return Ok(Work { dir: project_dir.to_path_buf(), origin: project_dir.to_path_buf(), branch: None, base: None });
    }
    let branch = branch_of(task);
    let dir = work_path(ws, project_dir, project, task);
    let base = base_branch(project_dir, parent).await;
    if dir.join(".git").exists() {
        // otra vuelta de la misma tarea: se sigue sobre la misma rama
        let _ = git(&dir, &["checkout", "-q", &branch]).await;
        return Ok(Work { dir, origin: project_dir.to_path_buf(), branch: Some(branch), base: Some(base) });
    }
    if let Some(p) = dir.parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("no pude crear {}: {e}", p.display()))?;
    }
    let existing = has_branch(project_dir, &branch).await;
    let from = if existing { branch.as_str() } else { base.as_str() };
    let out = Command::new("git").args(["clone", "--local", "--quiet", "--branch", from]).arg(project_dir).arg(&dir).env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).output().await.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("no pude armar la copia de trabajo: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    if !existing {
        git(&dir, &["checkout", "-q", "-b", &branch]).await?;
    }
    Ok(Work { dir, origin: project_dir.to_path_buf(), branch: Some(branch), base: Some(base) })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Pushed {
    /// subida al remoto `origin` del proyecto
    Remote(String),
    /// la rama quedó en el repositorio del proyecto (no tiene remoto)
    LocalOnly,
    Failed(String),
}

#[derive(Debug)]
pub struct Outcome {
    pub branch: String,
    pub commits: usize,
    pub files: usize,
    pub pushed: Pushed,
}

/// Quita usuario y clave de una URL de remoto antes de mostrarla.
pub fn clean_url(u: &str) -> String {
    match (u.find("://"), u.rfind('@')) {
        (Some(i), Some(a)) if a > i => format!("{}{}", &u[..i + 3], &u[a + 1..]),
        _ => u.to_string(),
    }
}

/// Al terminar la IA: guarda lo que dejó en un commit, lleva la rama al repositorio del proyecto y la sube al remoto. Lo hace `tm-agent`, fuera del sandbox.
pub async fn finish(work: &Work, ia: &str, task: i64, title: &str) -> Option<Outcome> {
    let branch = work.branch.clone()?;
    let base = work.base.clone()?;
    let dir = &work.dir;
    if !git(dir, &["status", "--porcelain"]).await.unwrap_or_default().is_empty() {
        let _ = git(dir, &["add", "-A"]).await;
        let name = format!("user.name={ia}");
        let mail = format!("user.email={}@crewhelm.local", slug(ia));
        let msg = format!("Tarea #{task}: {}", title.chars().take(120).collect::<String>());
        if let Err(e) = git(dir, &["-c", &name, "-c", &mail, "commit", "-q", "-m", &msg]).await {
            return Some(Outcome { branch, commits: 0, files: 0, pushed: Pushed::Failed(format!("no pude guardar los cambios: {e}")) });
        }
    }
    // sin cambios nuevos desde la última subida: no hay nada que informar
    let (head, pushed_before) = (git(dir, &["rev-parse", "HEAD"]).await.ok(), git(dir, &["rev-parse", "-q", "--verify", &format!("refs/remotes/origin/{branch}")]).await.ok());
    if head.is_some() && head == pushed_before {
        return None;
    }
    let range = format!("origin/{base}..HEAD");
    let commits = git(dir, &["rev-list", "--count", &range]).await.ok().and_then(|n| n.parse().ok()).unwrap_or(0);
    if commits == 0 {
        return None;
    }
    let files = git(dir, &["diff", "--name-only", &format!("origin/{base}...HEAD")]).await.map(|s| s.lines().count()).unwrap_or(0);
    // 1) la rama vuelve al repositorio del proyecto
    if let Err(e) = git(dir, &["push", "-q", "origin", &format!("HEAD:refs/heads/{branch}")]).await {
        return Some(Outcome { branch, commits, files, pushed: Pushed::Failed(format!("no pude llevar la rama al proyecto: {e}")) });
    }
    // 2) y, si el proyecto tiene remoto, se sube de ahí (con las credenciales del servidor)
    let pushed = match git(&work.origin, &["remote", "get-url", "origin"]).await {
        Ok(url) => match git(&work.origin, &["push", "-q", "-u", "origin", &branch]).await {
            Ok(_) => Pushed::Remote(clean_url(&url)),
            Err(e) => Pushed::Failed(format!("la rama quedó en el proyecto pero no pude subirla a {}: {}", clean_url(&url), e.lines().last().unwrap_or(""))),
        },
        Err(_) => Pushed::LocalOnly,
    };
    Some(Outcome { branch, commits, files, pushed })
}

pub fn outcome_text(o: &Outcome) -> String {
    let what = format!("{} commit{} y {} archivo{}", o.commits, if o.commits == 1 { "" } else { "s" }, o.files, if o.files == 1 { "" } else { "s" });
    match &o.pushed {
        Pushed::Remote(url) => format!("📤 Dejé mi trabajo en la rama `{}` ({what}) y la subí a {url}. Podés abrir el Pull Request desde la tarea.", o.branch),
        Pushed::LocalOnly => format!("📤 Dejé mi trabajo en la rama `{}` ({what}) en el repositorio del proyecto (no tiene remoto, no la subí a ningún lado).", o.branch),
        Pushed::Failed(e) => format!("⚠️ Mi trabajo ({what}) está en la rama `{}` pero {e}", o.branch),
    }
}

// ---------------------------------------------------------------- fase 2: sandbox con bwrap

/// ¿Hay `bwrap` y funciona en esta máquina?
pub async fn bwrap_ok() -> bool {
    Command::new("bwrap").args(["--ro-bind", "/", "/", "--dev", "/dev", "--unshare-pid", "--die-with-parent", "true"]).stdout(Stdio::null()).stderr(Stdio::null()).status().await.is_ok_and(|s| s.success())
}

/// Carpetas de herramientas y sesiones de los CLI de IA que se montan si existen (la sesión de Claude, Gemini…; compiladores y runtimes).
fn default_rw(home: &Path) -> Vec<PathBuf> {
    [".claude", ".claude.json", ".gemini", ".codex", ".config/opencode", ".local/share/opencode", ".cargo", ".npm"].iter().map(|p| home.join(p)).filter(|p| p.exists()).collect()
}
fn default_ro(home: &Path) -> Vec<PathBuf> {
    [".local/bin", ".local/share/claude", ".rustup", ".nvm", ".volta", ".bun", ".local/share/pnpm"].iter().map(|p| home.join(p)).filter(|p| p.exists()).collect()
}

pub struct Plan<'a> {
    pub work: &'a Path,
    pub mine: &'a Path,
    /// HOME dentro de la cápsula; si no es la carpeta real, se monta como espacio de la IA
    pub ia_home: Option<&'a Path>,
    pub exe: &'a Path,
    pub net: Net,
    /// carpeta con los sockets del proxy y del servidor (fase 3)
    pub sock_dir: Option<&'a Path>,
    pub extra_rw: &'a [PathBuf],
    pub extra_ro: &'a [PathBuf],
    /// `false` para una shell interactiva (con `--new-session` no hay terminal de control)
    pub new_session: bool,
}

/// Argumentos de `bwrap`: todo en solo lectura salvo lo que se da; el home real queda vacío.
pub fn bwrap_args(real_home: &Path, p: &Plan) -> Vec<String> {
    let s = |x: &Path| x.to_string_lossy().to_string();
    let mut a: Vec<String> = ["--die-with-parent", "--unshare-pid", "--unshare-ipc", "--unshare-uts", "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"].iter().map(|x| x.to_string()).collect();
    if p.new_session {
        a.push("--new-session".into());
    }
    for d in ["/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/opt", "/etc"] {
        let path = Path::new(d);
        match std::fs::read_link(path) {
            Ok(t) => a.extend(["--symlink".into(), s(&t), d.into()]),
            Err(_) if path.exists() => a.extend(["--ro-bind".into(), d.into(), d.into()]),
            Err(_) => {}
        }
    }
    if p.net == Net::Open {
        // el DNS de systemd-resolved vive en /run
        if Path::new("/run/systemd/resolve").exists() {
            a.extend(["--ro-bind".into(), "/run/systemd/resolve".into(), "/run/systemd/resolve".into()]);
        }
    } else {
        a.push("--unshare-net".into());
    }
    // el home real aparece vacío; sobre él solo se monta lo que se da
    a.extend(["--tmpfs".into(), s(real_home)]);
    let (rw, ro): (Vec<PathBuf>, Vec<PathBuf>) = if p.ia_home.is_some() { (Vec::new(), Vec::new()) } else { (default_rw(real_home), default_ro(real_home)) };
    for d in rw.iter().chain(p.extra_rw) {
        if d.exists() {
            a.extend(["--bind".into(), s(d), s(d)]);
        }
    }
    for d in ro.iter().chain(p.extra_ro) {
        if d.exists() {
            a.extend(["--ro-bind".into(), s(d), s(d)]);
        }
    }
    // credenciales de publicación: nunca visibles aunque se monte ~/.cargo
    for f in [".cargo/credentials.toml", ".cargo/credentials"] {
        let path = real_home.join(f);
        if path.exists() && rw.iter().chain(p.extra_rw).any(|d| path.starts_with(d)) {
            a.extend(["--ro-bind".into(), "/dev/null".into(), s(&path)]);
        }
    }
    if let Some(h) = p.ia_home {
        a.extend(["--bind".into(), s(h), s(h)]);
    }
    // el programa tiene que existir adentro (`tm-agent ask` y el puente de red)
    if let Some(dir) = p.exe.parent() {
        if !ro.iter().any(|d| dir.starts_with(d)) && !["/usr", "/bin", "/opt"].iter().any(|r| dir.starts_with(r)) && !p.extra_ro.iter().any(|d| dir.starts_with(d)) {
            a.extend(["--ro-bind".into(), s(p.exe), s(p.exe)]);
        }
    }
    for d in [p.work, p.mine] {
        a.extend(["--bind".into(), s(d), s(d)]);
    }
    if let Some(sd) = p.sock_dir {
        a.extend(["--bind".into(), s(sd), s(sd)]);
    }
    let home_in = p.ia_home.map(s).unwrap_or_else(|| s(real_home));
    a.extend(["--setenv".into(), "HOME".into(), home_in, "--chdir".into(), s(p.work)]);
    a
}

/// Cápsula para PERSONAS (terminal web y SSH): el home real del servidor no existe ahí adentro, ni siquiera como carpetas vacías.
/// El espacio personal aparece en `/home/crewhelm`, el proyecto (solo lectura) en `/proyecto`, el espacio ajeno que mira un admin
/// (solo lectura) en `/espacio` y las pocas herramientas permitidas en `/tools`.
pub struct PersonPlan<'a> {
    pub home: &'a Path,
    /// carpeta del proyecto y su nombre: aparece (solo lectura) en `/proyectos/<nombre>`
    pub project: Option<(&'a Path, &'a str)>,
    pub other_space: Option<&'a Path>,
    /// archivos sueltos que se ofrecen en `/tools` (por ejemplo `herdr`)
    pub tools: &'a [PathBuf],
    /// copia de trabajo (carpeta dentro del espacio personal): si existe, la shell arranca ahí
    pub work: Option<&'a str>,
    /// nombre de la persona en minúsculas y solo ascii (prefijo de sus ramas: `<persona>/<tema>`)
    pub who: &'a str,
}

pub const PERSON_HOME: &str = "/home/crewhelm";
pub const PERSON_PATH: &str = "/usr/local/bin:/usr/bin:/bin:/tools:/home/crewhelm/.local/bin";

pub fn person_args(p: &PersonPlan) -> Vec<String> {
    let s = |x: &Path| x.to_string_lossy().to_string();
    let mut a: Vec<String> = ["--die-with-parent", "--unshare-pid", "--unshare-ipc", "--unshare-uts", "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"].iter().map(|x| x.to_string()).collect();
    for d in ["/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/opt", "/etc"] {
        let path = Path::new(d);
        match std::fs::read_link(path) {
            Ok(t) => a.extend(["--symlink".into(), s(&t), d.into()]),
            Err(_) if path.exists() => a.extend(["--ro-bind".into(), d.into(), d.into()]),
            Err(_) => {}
        }
    }
    // red abierta, como en una shell normal (el DNS de systemd-resolved vive en /run)
    if Path::new("/run/systemd/resolve").exists() {
        a.extend(["--ro-bind".into(), "/run/systemd/resolve".into(), "/run/systemd/resolve".into()]);
    }
    a.extend(["--bind".into(), s(p.home), PERSON_HOME.into()]);
    let proj_at = p.project.map(|(d, name)| (d, format!("/proyectos/{}", slug(name))));
    if let Some((d, at)) = &proj_at {
        a.extend(["--ro-bind".into(), s(d), at.clone()]);
    }
    if let Some(d) = p.other_space {
        a.extend(["--ro-bind".into(), s(d), "/espacio".into()]);
    }
    for t in p.tools.iter().filter(|t| t.is_file()) {
        if let Some(n) = t.file_name() {
            a.extend(["--ro-bind".into(), s(t), format!("/tools/{}", n.to_string_lossy())]);
        }
    }
    let chdir = if p.other_space.is_some() { "/espacio".to_string() } else if let Some(w) = p.work { format!("{PERSON_HOME}/{w}") } else if let Some((_, at)) = &proj_at { at.clone() } else { PERSON_HOME.to_string() };
    a.extend(["--setenv".into(), "HOME".into(), PERSON_HOME.into(), "--setenv".into(), "PATH".into(), PERSON_PATH.into(), "--chdir".into(), chdir]);
    // herdr abre una sesión por proyecto (la shell lo usa para arrancar siempre en la carpeta del proyecto)
    if let Some((_, name)) = p.project {
        a.extend(["--setenv".into(), "CREWHELM_PROJECT".into(), slug(name)]);
    }
    if !p.who.is_empty() {
        a.extend(["--setenv".into(), "CW_WHO".into(), p.who.into()]);
    }
    a
}

/// Herramientas de terminal que se ofrecen dentro de la cápsula (en `/tools`) cuando están instaladas fuera de `/usr`, por ejemplo en
/// `~/.local/bin` o `~/.cargo/bin`: son binarios sueltos, no dejan ver nada más del home.
pub const TOOL_NAMES: &[&str] = &["herdr", "eza", "starship", "fzf", "bat", "fd", "rg", "zoxide"];

pub fn person_tools() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut dirs = vec![home.join(".local/bin"), home.join(".cargo/bin")];
    dirs.extend(std::env::var_os("PATH").into_iter().flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>()));
    TOOL_NAMES
        .iter()
        .filter_map(|n| dirs.iter().map(|d| d.join(n)).find(|p| p.is_file()))
        // lo que ya vive en /usr se ve sin ofrecerlo
        .filter(|p| !p.starts_with("/usr") && !p.starts_with("/bin") && !p.starts_with("/opt"))
        .collect()
}

/// El shell de las personas: `zsh` si el sistema lo tiene (con la configuración por defecto de CrewHelm), si no `bash`.
pub fn person_shell() -> &'static str {
    if Path::new("/usr/bin/zsh").exists() { "/usr/bin/zsh" } else { "/bin/bash" }
}

/// Herramientas de terminal recomendadas que no están en ningún lado (ni en el sistema ni en `~/.local/bin`).
pub fn missing_shell_tools() -> Vec<&'static str> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut dirs = vec![home.join(".local/bin"), home.join(".cargo/bin")];
    dirs.extend(std::env::var_os("PATH").into_iter().flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>()));
    ["zsh", "eza", "starship", "fzf", "rg"].into_iter().filter(|n| !dirs.iter().any(|d| d.join(n).is_file())).collect()
}

/// Herramientas de solo lectura para la shell de `sandbox-shell` (las mismas que usa una ejecución).
pub fn paths_for_shell() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut v = default_ro(&home);
    v.extend(paths_env("TM_SANDBOX_RO"));
    v
}

// ---------------------------------------------------------------- fase 3: red hacia destinos permitidos

pub fn host_allowed(host: &str, allow: &[String]) -> bool {
    let h = host.trim_end_matches('.').to_lowercase();
    !h.is_empty() && allow.iter().any(|a| h == *a || h.ends_with(&format!(".{a}")))
}

fn public_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v) => !(v.is_private() || v.is_loopback() || v.is_link_local() || v.is_unspecified() || v.is_broadcast() || v.octets()[0] == 100 && (64..128).contains(&v.octets()[1])),
        std::net::IpAddr::V6(v) => !(v.is_loopback() || v.is_unspecified() || (v.segments()[0] & 0xfe00) == 0xfc00 || (v.segments()[0] & 0xffc0) == 0xfe80),
    }
}

/// Registro de lo bloqueado en una ejecución.
pub type Blocked = Arc<Mutex<BTreeSet<String>>>;

async fn handle_proxy(mut c: UnixStream, allow: Arc<Vec<String>>, blocked: Blocked) {
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 8192 {
        match c.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    let text = String::from_utf8_lossy(&head).to_string();
    let first = text.lines().next().unwrap_or("");
    let mut it = first.split_whitespace();
    let (method, target) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
    let deny = |why: &str| format!("HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nCrewHelm bloqueó esta conexión: {why}\n");
    if method != "CONNECT" {
        let _ = c.write_all(deny("solo se permite HTTPS").as_bytes()).await;
        return;
    }
    let (host, port) = target.rsplit_once(':').unwrap_or((target, "443"));
    if port != "443" || !host_allowed(host, &allow) {
        blocked.lock().unwrap().insert(format!("{host}:{port}"));
        let _ = c.write_all(deny("destino no permitido").as_bytes()).await;
        return;
    }
    let addrs: Vec<std::net::SocketAddr> = match tokio::net::lookup_host((host, 443)).await {
        Ok(a) => a.filter(|a| public_ip(a.ip())).collect(),
        Err(_) => Vec::new(),
    };
    let mut up = None;
    for a in addrs {
        if let Ok(Ok(s)) = tokio::time::timeout(Duration::from_secs(15), TcpStream::connect(a)).await {
            up = Some(s);
            break;
        }
    }
    let Some(mut up) = up else {
        let _ = c.write_all(b"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n").await;
        return;
    };
    if c.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.is_err() {
        return;
    }
    let _ = tokio::io::copy_bidirectional(&mut c, &mut up).await;
}

async fn handle_server(mut c: UnixStream, addr: String) {
    if let Ok(mut up) = TcpStream::connect(&addr).await {
        let _ = tokio::io::copy_bidirectional(&mut c, &mut up).await;
    }
}

/// Lo que corre fuera de la cápsula durante una ejecución: el proxy filtrado y el puente al servidor.
pub struct NetGuard {
    pub dir: PathBuf,
    pub blocked: Blocked,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for NetGuard {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Levanta los sockets `p.sock` (proxy HTTPS filtrado; solo en modo `Proxy`) y `s.sock` (servidor de CrewHelm, `host:puerto`).
pub fn start_net(task: i64, mode: Net, allow: Vec<String>, server_addr: String) -> std::io::Result<NetGuard> {
    let dir = std::env::temp_dir().join(format!("tmr-{}-{task}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let blocked: Blocked = Default::default();
    let mut tasks = Vec::new();
    let srv = UnixListener::bind(dir.join("s.sock"))?;
    tasks.push(tokio::spawn(async move {
        while let Ok((c, _)) = srv.accept().await {
            tokio::spawn(handle_server(c, server_addr.clone()));
        }
    }));
    if mode == Net::Proxy {
        let prx = UnixListener::bind(dir.join("p.sock"))?;
        let (allow, b) = (Arc::new(allow), blocked.clone());
        tasks.push(tokio::spawn(async move {
            while let Ok((c, _)) = prx.accept().await {
                tokio::spawn(handle_proxy(c, allow.clone(), b.clone()));
            }
        }));
    }
    Ok(NetGuard { dir, blocked, tasks })
}

pub const PROXY_PORT: u16 = 3128;
pub const SERVER_PORT: u16 = 3129;

/// Dentro de la cápsula: `tm-agent __netbridge <carpeta de sockets> <modo> -- comando…`. Abre 127.0.0.1:3128 (proxy) y :3129 (servidor)
/// reenviando a los sockets de afuera, y corre el comando con el proxy en el entorno.
pub async fn netbridge(args: Vec<String>) -> i32 {
    let (Some(dir), Some(mode)) = (args.first().map(PathBuf::from), args.get(1).cloned()) else { return 2 };
    let Some(sep) = args.iter().position(|a| a == "--") else { return 2 };
    let cmd = &args[sep + 1..];
    if cmd.is_empty() {
        return 2;
    }
    let mut ports = vec![(SERVER_PORT, dir.join("s.sock"))];
    if mode == "proxy" {
        ports.push((PROXY_PORT, dir.join("p.sock")));
    }
    for (port, sock) in ports {
        let Ok(l) = TcpListener::bind(("127.0.0.1", port)).await else { return 3 };
        tokio::spawn(async move {
            while let Ok((mut c, _)) = l.accept().await {
                let sock = sock.clone();
                tokio::spawn(async move {
                    if let Ok(mut up) = UnixStream::connect(&sock).await {
                        let _ = tokio::io::copy_bidirectional(&mut c, &mut up).await;
                    }
                });
            }
        });
    }
    let mut child = Command::new(&cmd[0]);
    child.args(&cmd[1..]).env("TM_URL", format!("http://127.0.0.1:{SERVER_PORT}")).env("NO_PROXY", "127.0.0.1,localhost").env("no_proxy", "127.0.0.1,localhost");
    if mode == "proxy" {
        let p = format!("http://127.0.0.1:{PROXY_PORT}");
        for k in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
            child.env(k, &p);
        }
    }
    match child.status().await {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("[tm-agent] no pude lanzar el comando dentro de la cápsula: {e}");
            127
        }
    }
}

/// Texto para la tarea con lo que la cápsula no dejó salir.
pub fn blocked_text(b: &Blocked) -> Option<String> {
    // la telemetría de las herramientas (se bloquea igual) no se informa: solo ensuciaría cada tarea
    const RUIDO: [&str; 4] = ["datadoghq.com", "statsig", "sentry.io", "segment.io"];
    let v: Vec<String> = b.lock().unwrap().iter().filter(|h| !RUIDO.iter().any(|r| h.contains(r))).cloned().collect();
    (!v.is_empty()).then(|| format!("🌐 El aislamiento bloqueó conexiones a: {}. Si hacen falta, un admin puede sumarlas en «destinos de red permitidos».", v.join(", ")))
}

/// Une las líneas de la salida de un proceso por si se quiere mostrar (solo para pruebas y mensajes).
#[allow(dead_code)]
pub async fn read_lines<R: tokio::io::AsyncRead + Unpin>(r: R) -> Vec<String> {
    let mut l = BufReader::new(r).lines();
    let mut out = Vec::new();
    while let Ok(Some(x)) = l.next_line().await {
        out.push(x);
    }
    out
}

// ---------------------------------------------------------------- limpieza

/// ¿Cuántos días pasaron desde `iso` (fecha RFC 3339)?
pub fn days_since(iso: &str) -> Option<i64> {
    let t = chrono_like(iso)?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    Some((now - t) / 86400)
}

/// Segundos Unix de `YYYY-MM-DDTHH:MM:SS…` (UTC; ignora fracciones y desfase, suficiente para contar días).
fn chrono_like(s: &str) -> Option<i64> {
    let (d, t) = s.split_once('T')?;
    let mut dp = d.split('-').map(|x| x.parse::<i64>().ok());
    let (y, m, day) = (dp.next()??, dp.next()??, dp.next()??);
    let mut tp = t.split(|c: char| c == ':' || c == '.' || c == '+' || c == 'Z').map(|x| x.parse::<i64>().ok());
    let (hh, mm, ss) = (tp.next()??, tp.next()??, tp.next().flatten().unwrap_or(0));
    // días desde 1970 (algoritmo de Howard Hinnant)
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + hh * 3600 + mm * 60 + ss)
}

#[cfg(test)]
mod tests {
    /// La cápsula de una persona no deja ver nada del home real del servidor (ni los nombres de carpetas).
    #[test]
    fn la_capsula_de_una_persona_no_ve_el_home_real() {
        let ok = std::process::Command::new("bwrap").args(["--ro-bind", "/", "/", "--dev", "/dev", "true"]).status().is_ok_and(|s| s.success());
        if !ok {
            return;
        }
        let d = std::env::temp_dir().join(format!("tm-person-{}", std::process::id()));
        let (home, proj) = (d.join("espacio"), d.join("proy"));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("f.txt"), "hola").unwrap();
        let mut a = person_args(&PersonPlan { home: &home, project: Some((&proj, "Mi Proyecto")), other_space: None, tools: &[], work: None, who: "" });
        let script = "echo P=$CREWHELM_PROJECT; ls -A /home; echo --; pwd; echo --; cat /proyectos/mi-proyecto/f.txt; touch /proyectos/mi-proyecto/x 2>&1 | head -1; touch ~/mio && echo escribe; ls /root /home/pablo 2>&1 | head -2";
        a.extend(["--".into(), "/bin/bash".into(), "-c".into(), script.into()]);
        let out = std::process::Command::new("bwrap").args(a).env_clear().output().unwrap();
        let t = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
        assert!(t.starts_with("P=mi-proyecto\ncrewhelm\n--\n/proyectos/mi-proyecto\n--\nhola"), "salida: {t}");
        assert!(t.contains("escribe") && !t.contains("pablo\n") && !proj.join("x").exists(), "salida: {t}");
        assert!(home.join("mio").exists());
        // con copia de trabajo: la shell arranca en ~/<proyecto> y ahí se puede crear
        std::fs::create_dir_all(home.join("mi-proyecto")).unwrap();
        let mut a = person_args(&PersonPlan { home: &home, project: Some((&proj, "Mi Proyecto")), other_space: None, tools: &[], work: Some("mi-proyecto"), who: "marta" });
        a.extend(["--".into(), "/bin/bash".into(), "-c".into(), "pwd; mkdir nueva && touch nueva/a.txt && echo ok".into()]);
        let out = std::process::Command::new("bwrap").args(a).env_clear().output().unwrap();
        let t = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
        assert!(t.starts_with("/home/crewhelm/mi-proyecto\nok"), "salida: {t}");
        assert!(home.join("mi-proyecto/nueva/a.txt").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tm-iso-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    fn sh(dir: &Path, cmd: &str) -> String {
        let o = std::process::Command::new("sh").arg("-c").arg(cmd).current_dir(dir).output().unwrap();
        assert!(o.status.success(), "{cmd}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }
    fn repo(name: &str) -> PathBuf {
        let d = tmp(name);
        sh(&d, "git init -q -b main . && git config user.email t@t && git config user.name t && echo uno > a.txt && git add . && git commit -qm base");
        d
    }

    #[test]
    fn destinos_por_sufijo() {
        let a: Vec<String> = DEFAULT_ALLOW.iter().map(|s| s.to_string()).collect();
        assert!(host_allowed("api.anthropic.com", &a));
        assert!(host_allowed("github.com", &a));
        assert!(host_allowed("static.crates.io", &a));
        assert!(!host_allowed("evilgithub.com", &a));
        assert!(!host_allowed("github.com.evil.io", &a));
        assert!(!host_allowed("storage.googleapis.com", &a));
        assert!(!host_allowed("", &a));
    }

    #[test]
    fn ips_privadas_no_son_publicas() {
        for ip in ["127.0.0.1", "10.1.2.3", "192.168.0.5", "172.16.0.1", "169.254.1.1", "100.64.0.1", "::1", "fd00::1", "fe80::1"] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_ip("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn url_sin_credenciales() {
        assert_eq!(clean_url("https://user:tok@github.com/a/b.git"), "https://github.com/a/b.git");
        assert_eq!(clean_url("git@github.com:a/b.git"), "git@github.com:a/b.git");
    }

    #[test]
    fn dias_desde() {
        assert_eq!(chrono_like("1970-01-02T00:00:00Z"), Some(86400));
        assert_eq!(chrono_like("2026-11-06T12:00:00.123+00:00").map(|s| s / 86400), Some(20763));
        assert!(days_since("2020-01-01T00:00:00Z").unwrap() > 2000);
    }

    #[test]
    fn argumentos_de_bwrap() {
        let home = tmp("home");
        let (work, mine) = (tmp("work"), tmp("mine"));
        let exe = std::env::current_exe().unwrap();
        let a = bwrap_args(&home, &Plan { work: &work, mine: &mine, ia_home: None, exe: &exe, net: Net::Proxy, sock_dir: None, extra_rw: &[], extra_ro: &[], new_session: true });
        let j = a.join(" ");
        assert!(j.contains("--unshare-net"));
        assert!(j.contains(&format!("--tmpfs {}", home.display())));
        assert!(j.contains(&format!("--bind {} {}", work.display(), work.display())));
        assert!(j.contains("--chdir"));
        let open = bwrap_args(&home, &Plan { work: &work, mine: &mine, ia_home: None, exe: &exe, net: Net::Open, sock_dir: None, extra_rw: &[], extra_ro: &[], new_session: true });
        assert!(!open.join(" ").contains("--unshare-net"));
    }

    #[tokio::test]
    async fn copia_de_trabajo_rama_y_push() {
        let proj = repo("proj");
        // un remoto «origin» para el proyecto
        let remote = tmp("remote");
        sh(&remote, "git init -q --bare .");
        sh(&proj, &format!("git remote add origin {}", remote.display()));
        let ws = tmp("ws");
        let w = prepare(&proj, Some(&ws), "Mi Proyecto", 7, None).await.unwrap();
        assert_eq!(w.branch.as_deref(), Some("tm/tarea-7"));
        assert!(w.dir.starts_with(&ws) && w.dir.ends_with("proyectos/mi-proyecto/tareas/7"));
        assert_eq!(sh(&w.dir, "git rev-parse --abbrev-ref HEAD"), "tm/tarea-7");
        // la IA escribe y no commitea
        std::fs::write(w.dir.join("nuevo.txt"), "hola").unwrap();
        std::fs::write(w.dir.join("a.txt"), "dos").unwrap();
        let o = finish(&w, "claude-opus", 7, "Probar").await.expect("hay cambios");
        assert_eq!((o.commits, o.files), (1, 2));
        assert!(matches!(o.pushed, Pushed::Remote(_)), "{:?}", o.pushed);
        // la rama está en el proyecto y en el remoto; main del proyecto no cambió
        assert!(sh(&proj, "git branch --list tm/tarea-7").contains("tm/tarea-7"));
        assert!(sh(&remote, "git branch --list").contains("tm/tarea-7"));
        assert_eq!(sh(&proj, "cat a.txt"), "uno");
        assert!(sh(&proj, "git log -1 --format=%an tm/tarea-7").contains("claude-opus"));
        // segunda vuelta: reutiliza la copia y sin cambios no hay nada que subir
        let w2 = prepare(&proj, Some(&ws), "Mi Proyecto", 7, None).await.unwrap();
        assert_eq!(w2.dir, w.dir);
        assert!(finish(&w2, "claude-opus", 7, "Probar").await.is_none());
    }

    #[tokio::test]
    async fn sin_remoto_queda_local_y_sin_repo_no_hay_copia() {
        let proj = repo("proj2");
        let w = prepare(&proj, None, "p", 3, None).await.unwrap();
        assert!(w.dir.ends_with(".crewhelm/wt/tarea-3"));
        std::fs::write(w.dir.join("x"), "x").unwrap();
        let o = finish(&w, "ia", 3, "t").await.unwrap();
        assert_eq!(o.pushed, Pushed::LocalOnly);
        let plain = tmp("plain");
        let w = prepare(&plain, None, "p", 4, None).await.unwrap();
        assert_eq!((w.dir.clone(), w.branch), (plain, None));
    }

    #[tokio::test]
    async fn subtarea_sale_de_la_rama_de_la_madre() {
        let proj = repo("proj3");
        sh(&proj, "git branch tm/tarea-1 && git checkout -q tm/tarea-1 && echo m > madre.txt && git add . && git commit -qm madre && git checkout -q main");
        let w = prepare(&proj, None, "p", 2, Some(1)).await.unwrap();
        assert_eq!(w.base.as_deref(), Some("tm/tarea-1"));
        assert!(w.dir.join("madre.txt").exists());
    }

    #[tokio::test]
    async fn la_capsula_no_ve_el_home_ni_escribe_fuera() {
        if !bwrap_ok().await {
            eprintln!("sin bwrap: prueba omitida");
            return;
        }
        let home = PathBuf::from(std::env::var("HOME").unwrap());
        let (work, mine) = (tmp("w2"), tmp("m2"));
        let exe = std::env::current_exe().unwrap();
        let mut args = bwrap_args(&home, &Plan { work: &work, mine: &mine, ia_home: None, exe: &exe, net: Net::Proxy, sock_dir: None, extra_rw: &[], extra_ro: &[], new_session: true });
        args.extend(["--".into(), "sh".into(), "-c".into(), format!("echo ok > {w}/dentro.txt; ls -A {h} | head -50; touch /usr/x 2>&1; touch /etc/x 2>&1; curl -s -m 3 https://example.com >/dev/null 2>&1 && echo SALE_A_INTERNET", w = work.display(), h = home.display())]);
        let o = Command::new("bwrap").args(&args).output().await.unwrap();
        let out = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
        assert!(work.join("dentro.txt").exists(), "{out}");
        assert!(![".ssh", "sshCompartido", ".lazyssh", ".lesshst", "Documentos", "Descargas"].iter().any(|x| out.contains(x)), "se ve el home: {out}");
        assert!(out.contains("Read-only file system"), "{out}");
        assert!(!out.contains("SALE_A_INTERNET"), "{out}");
    }

    #[tokio::test]
    async fn el_proxy_deja_pasar_lo_permitido_y_bloquea_el_resto() {
        if !bwrap_ok().await {
            eprintln!("sin bwrap: prueba omitida");
            return;
        }
        // «servidor» de CrewHelm de mentira
        let srv = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let srv_addr = srv.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            while let Ok((mut c, _)) = srv.accept().await {
                tokio::spawn(async move {
                    let mut b = [0u8; 1024];
                    let _ = c.read(&mut b).await;
                    let _ = c.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhola-server").await;
                });
            }
        });
        let allow = vec!["example.com".to_string()];
        let net = start_net(9001, Net::Proxy, allow, srv_addr).unwrap();
        let home = PathBuf::from(std::env::var("HOME").unwrap());
        let (work, mine) = (tmp("w3"), tmp("m3"));
        let exe = std::env::current_exe().unwrap();
        let mut args = bwrap_args(&home, &Plan { work: &work, mine: &mine, ia_home: None, exe: &exe, net: Net::Proxy, sock_dir: Some(&net.dir), extra_rw: &[], extra_ro: &[], new_session: true });
        // el puente es un subcomando del binario real de tm-agent
        let agent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/tm-agent");
        if !agent.exists() {
            eprintln!("falta target/debug/tm-agent (cargo build -p crewhelm-agent): prueba omitida");
            return;
        }
        args.extend(["--ro-bind".into(), agent.to_string_lossy().to_string(), "/tmp/agent-bin".into()]);
        args.extend(["--".into(), "/tmp/agent-bin".into(), "__netbridge".into(), net.dir.to_string_lossy().to_string(), "proxy".into(), "--".into(), "sh".into(), "-c".into(),
            "echo SERVER=$(curl -s -m 5 $TM_URL/x); echo BLOQ=$(curl -s -m 5 https://github.com 2>&1 | head -c 80); echo CODE=$(curl -s -m 8 -o /dev/null -w '%{http_code}' https://example.com)".into()]);
        let o = Command::new("bwrap").args(&args).output().await.unwrap();
        let out = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
        assert!(out.contains("SERVER=hola-server"), "{out}");
        assert!(!out.contains("BLOQ=<"), "github no debería pasar: {out}");
        assert!(net.blocked.lock().unwrap().iter().any(|h| h.starts_with("github.com")), "{out} / {:?}", net.blocked.lock().unwrap());
    }
}
