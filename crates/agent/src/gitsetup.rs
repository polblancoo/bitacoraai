//! Asistente de git: el servidor le pide a este agente que mire o prepare el repositorio de un proyecto (traerlo de GitHub,
//! crearlo nuevo o dejarlo solo local). La subida usa una clave de despliegue propia del repositorio, guardada en
//! `~/.config/tm-agent/keys` y registrada en GitHub; el token de la persona se usa en esta operación y no se guarda.
use crate::Api;
use serde_json::{json, Value};
use std::{path::{Path, PathBuf}, sync::Arc, time::Duration};
use tokio::process::Command;

pub async fn serve(api: Arc<Api>) {
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let Ok(list) = api.get("/api/agent/git/pending").await else { continue };
        for job in list.as_array().into_iter().flatten().cloned() {
            let Some(id) = job["id"].as_str().map(String::from) else { continue };
            let api = api.clone();
            tokio::spawn(async move {
                let res = match run_job(&job["payload"]).await {
                    Ok(v) => json!({ "ok": true, "data": v }),
                    Err(e) => json!({ "ok": false, "error": e }),
                };
                let _ = api.call(reqwest::Method::POST, &format!("/api/agent/git/{id}/result"), Some(res)).await;
            });
        }
    }
}

async fn run(c: &mut Command, secs: u64) -> Result<String, String> {
    c.env("GIT_TERMINAL_PROMPT", "0").stdin(std::process::Stdio::null()).kill_on_drop(true);
    let out = tokio::time::timeout(Duration::from_secs(secs), c.output()).await.map_err(|_| "tardó demasiado".to_string())?.map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(crate::tail(&String::from_utf8_lossy(&out.stderr), 300))
    }
}

async fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    run(Command::new("git").arg("-C").arg(dir).args(args), 120).await
}

async fn git_ssh(dir: &Path, ssh: &str, args: Vec<String>) -> Result<String, String> {
    run(Command::new("git").arg("-C").arg(dir).args(&args).env("GIT_SSH_COMMAND", ssh), 180).await
}

async fn status(dir: &Path) -> Value {
    let is_repo = dir.join(".git").exists();
    let mut v = json!({ "exists": dir.is_dir(), "is_repo": is_repo, "empty": dir.is_dir() && std::fs::read_dir(dir).map(|mut r| r.next().is_none()).unwrap_or(false) });
    if is_repo {
        v["origin"] = json!(git(dir, &["config", "--get", "remote.origin.url"]).await.unwrap_or_default());
        v["branch"] = json!(git(dir, &["branch", "--show-current"]).await.unwrap_or_default());
        v["dirty"] = json!(!git(dir, &["status", "--porcelain"]).await.unwrap_or_default().is_empty());
        v["commits"] = json!(git(dir, &["rev-parse", "-q", "--verify", "HEAD"]).await.is_ok());
    }
    v
}

const IGNORE: &str = "# creado por CrewHelm: lo que no conviene subir nunca\ntarget/\nnode_modules/\n.env\n.env.*\n*.pem\n*.key\nid_rsa*\nid_ed25519*\n.tm/\n.DS_Store\n";

/// `git init` + `.gitignore` básico (si no hay) + primer commit si todavía no hay ninguno.
async fn init_local(dir: &Path, steps: &mut Vec<String>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("no pude crear la carpeta: {e}"))?;
    if !dir.join(".git").exists() {
        git(dir, &["init", "-q", "-b", "main"]).await?;
        steps.push("Repositorio git creado en la carpeta del proyecto".into());
    }
    if git(dir, &["rev-parse", "-q", "--verify", "HEAD"]).await.is_err() {
        let ignore = dir.join(".gitignore");
        if !ignore.exists() {
            std::fs::write(&ignore, IGNORE).map_err(|e| e.to_string())?;
            steps.push("Se agregó un .gitignore para no subir claves ni carpetas pesadas (target/, node_modules/, .env…)".into());
        }
        git(dir, &["add", "-A"]).await?;
        git(dir, &["-c", "user.name=CrewHelm", "-c", "user.email=crewhelm@localhost", "commit", "-q", "--allow-empty", "-m", "Primer commit"]).await?;
        steps.push("Primer commit hecho".into());
    }
    Ok(())
}

fn host_of(base: &str) -> String {
    base.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string()
}

/// Crea (si falta) la clave del repositorio y la registra en GitHub como clave de despliegue con permiso de escritura.
/// Devuelve el comando ssh que usa git en este repositorio.
async fn deploy_key(p: &Value, repo: &str, steps: &mut Vec<String>) -> Result<String, String> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("no sé cuál es la carpeta personal")?;
    let keydir = home.join(".config/tm-agent/keys");
    std::fs::create_dir_all(&keydir).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&keydir, std::fs::Permissions::from_mode(0o700));
    }
    let key = keydir.join(repo.replace('/', "__"));
    let keys = key.to_string_lossy().to_string();
    if keys.contains([' ', '"', '\'', '\\', '\n']) {
        return Err("la carpeta personal tiene espacios o comillas: no puedo guardar la clave ahí".into());
    }
    if !key.exists() {
        run(Command::new("ssh-keygen").args(["-q", "-t", "ed25519", "-N", "", "-C", &format!("crewhelm {repo}"), "-f", &keys]), 30).await.map_err(|e| format!("ssh-keygen: {e}"))?;
        steps.push("Clave de subida creada en esta máquina".into());
    }
    let public = std::fs::read_to_string(format!("{keys}.pub")).map_err(|e| e.to_string())?.trim().to_string();
    let (api, token) = (p["api"].as_str().unwrap_or("https://api.github.com"), p["token"].as_str().unwrap_or(""));
    let host = std::fs::read_to_string("/etc/hostname").unwrap_or_default().trim().to_string();
    let r = reqwest::Client::new()
        .post(format!("{api}/repos/{repo}/keys"))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "crewhelm-agent")
        .json(&json!({ "title": format!("CrewHelm ({host})"), "key": public, "read_only": false }))
        .send()
        .await
        .map_err(|_| "no pude hablar con GitHub".to_string())?;
    let status = r.status();
    if status.is_success() {
        steps.push("Clave de subida registrada en GitHub (solo para este repositorio)".into());
    } else {
        let body: Value = r.json().await.unwrap_or(Value::Null);
        let text = body.to_string();
        if !(status.as_u16() == 422 && text.contains("already in use")) {
            return Err(match status.as_u16() {
                404 | 403 => "GitHub no me dejó registrar la clave de subida: tu cuenta tiene que ser administradora de ese repositorio".to_string(),
                _ => format!("GitHub rechazó la clave de subida ({status}): {}", crate::tail(body["message"].as_str().unwrap_or(""), 160)),
            });
        }
    }
    Ok(format!("ssh -i {keys} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new -o BatchMode=yes"))
}

async fn configure(dir: &Path, p: &Value, repo: &str, ssh: &str) -> Result<String, String> {
    let url = format!("git@{}:{repo}.git", host_of(p["base"].as_str().unwrap_or("https://github.com")));
    if git(dir, &["remote", "get-url", "origin"]).await.is_ok() {
        git(dir, &["remote", "set-url", "origin", &url]).await?;
    } else {
        git(dir, &["remote", "add", "origin", &url]).await?;
    }
    git(dir, &["config", "core.sshCommand", ssh]).await?;
    Ok(url)
}

async fn retry<T, F: std::future::Future<Output = Result<T, String>>>(mut f: impl FnMut() -> F) -> Result<T, String> {
    let mut last = String::new();
    for i in 0..3 {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => last = e,
        }
        if i < 2 {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
    Err(last)
}

// ---------------------------------------------------------------- recuperar archivos y ramas desde la web (sin escribir comandos)

/// git dentro de una cápsula sin red: la copia de la persona la maneja ella, así que su configuración (filtros, hooks) no corre con permisos del servicio.
async fn sgit(copy: &Path, sandbox: bool, args: &[&str]) -> Result<String, String> {
    let mut c;
    if sandbox {
        c = Command::new("bwrap");
        c.args(["--ro-bind", "/", "/", "--bind"]).arg(copy).arg(copy).args(["--dev", "/dev", "--proc", "/proc", "--unshare-net", "--unshare-pid", "--unshare-ipc", "--die-with-parent", "git"]);
    } else {
        c = Command::new("git");
    }
    c.arg("-C").arg(copy).args(["-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false", "-c", "core.quotepath=false", "-c", "protocol.file.allow=always"]).args(args);
    c.env_clear().env("PATH", std::env::var("PATH").unwrap_or_default()).env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1");
    run(&mut c, 60).await
}

fn safe_rel(p: &str) -> bool {
    !p.is_empty() && p.len() < 400 && !p.contains('\0') && !p.starts_with('/') && p.split('/').all(|s| !s.is_empty() && s != ".." && s != "." && s != ".git")
}

fn safe_branch(b: &str) -> bool {
    !b.is_empty() && b.len() <= 100 && !b.contains("..") && !b.starts_with(['/', '-']) && !b.ends_with(['/', '.']) && !b.ends_with(".lock") && b.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.'))
}

fn is_sha(s: &str) -> bool {
    (4..=40).contains(&s.len()) && s.chars().all(|c| c.is_ascii_hexdigit())
}

struct Rc {
    copy: PathBuf,
    sandbox: bool,
}

impl Rc {
    async fn g(&self, a: &[&str]) -> Result<String, String> {
        sgit(&self.copy, self.sandbox, a).await
    }
    async fn base_of(&self, f: &str) -> Option<String> {
        for b in ["main", "master"] {
            let r = format!("refs/remotes/base/{b}");
            if self.g(&["cat-file", "-e", &format!("{r}:{f}")]).await.is_ok() {
                return Some(r);
            }
        }
        None
    }
}

async fn recover(p: &Value, project_dir: &Path) -> Result<Value, String> {
    let owner = p["owner"].as_bool() == Some(true);
    let who = p["who"].as_str().unwrap_or("");
    let copy = if owner {
        project_dir.to_path_buf()
    } else {
        let ws = crate::term::workspace().ok_or("Este servidor no usa espacios personales")?;
        ws.join("homeCrewHelm").join(crate::iso::slug(who)).join(crate::iso::slug(p["project"].as_str().unwrap_or("")))
    };
    if !copy.join(".git").exists() {
        return Err("Todavía no tenés una copia de trabajo de este proyecto: abrí su terminal una vez y volvé a probar.".into());
    }
    let sandbox = !owner;
    if sandbox && !crate::iso::bwrap_ok().await {
        return Err("Falta bwrap en el servidor: no se puede mirar la copia de trabajo de forma segura.".into());
    }
    let prefix = if owner { String::new() } else { format!("{}/", crate::iso::ascii_slug(who)) };
    let path = p["path"].as_str().unwrap_or("").trim_start_matches("./").to_string();
    let rc = Rc { copy: copy.clone(), sandbox };
    match p["action"].as_str().unwrap_or("") {
        "list" => {
            // `diff --name-status -z` (no `status`): `run` recorta los espacios del principio y rompería la primera línea
            let st = rc.g(&["diff", "--name-status", "-z", "HEAD"]).await?;
            let mut changed = Vec::new();
            let mut parts = st.split('\0');
            while let Some(code) = parts.next() {
                let Some(f) = parts.next() else { break };
                if code.starts_with('R') || code.starts_with('C') {
                    parts.next();
                }
                if !safe_rel(f) {
                    continue;
                }
                if code.starts_with('D') {
                    changed.push(json!({ "path": f, "state": "borrado" }));
                } else if code.starts_with('M') || code.starts_with('T') {
                    changed.push(json!({ "path": f, "state": "cambiado" }));
                }
            }
            let log = rc.g(&["log", "--diff-filter=D", "--name-only", "--format=", "-n", "60"]).await.unwrap_or_default();
            let mut seen = std::collections::HashSet::new();
            let deleted: Vec<String> = log.lines().filter(|l| !l.is_empty() && safe_rel(l) && !copy.join(l).exists() && seen.insert(l.to_string())).take(40).map(String::from).collect();
            Ok(json!({ "changed": changed, "deleted": deleted }))
        }
        "versions" => {
            if !safe_rel(&path) {
                return Err("Ruta no válida".into());
            }
            let log = rc.g(&["log", "-n", "25", "--format=%h%x09%at%x09%an%x09%s", "--", &path]).await.unwrap_or_default();
            let mut versions = Vec::new();
            for l in log.lines() {
                let f: Vec<&str> = l.splitn(4, '\t').collect();
                if f.len() == 4 && rc.g(&["cat-file", "-e", &format!("{}:{path}", f[0])]).await.is_ok() {
                    versions.push(json!({ "rev": f[0], "at": f[1].parse::<i64>().unwrap_or(0), "who": f[2], "msg": f[3] }));
                    if versions.len() >= 12 {
                        break;
                    }
                }
            }
            Ok(json!({ "versions": versions, "base": rc.base_of(&path).await.is_some(), "exists_now": copy.join(&path).exists() }))
        }
        "restore" => {
            if !safe_rel(&path) {
                return Err("Ruta no válida".into());
            }
            let rev = p["rev"].as_str().unwrap_or("");
            let rev = if rev == "base" { rc.base_of(&path).await.ok_or("El proyecto no tiene ese archivo")? } else if is_sha(rev) { rev.to_string() } else { return Err("Versión no válida".into()) };
            rc.g(&["cat-file", "-e", &format!("{rev}:{path}")]).await.map_err(|_| "Esa versión no tiene el archivo".to_string())?;
            let target = copy.join(&path);
            let mut backup = None;
            if target.is_file() {
                let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) % 86400;
                let b = format!("{path}.antes-de-recuperar-{:02}{:02}{:02}", secs / 3600, secs / 60 % 60, secs % 60);
                std::fs::copy(&target, copy.join(&b)).map_err(|e| format!("no pude respaldar lo que había: {e}"))?;
                backup = Some(b);
            }
            if let Some(d) = target.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            rc.g(&["restore", "--source", &rev, "--worktree", "--", &path]).await?;
            Ok(json!({ "restored": path, "backup": backup }))
        }
        "branches" => {
            let cur = rc.g(&["branch", "--show-current"]).await.unwrap_or_default();
            let fmt = "--format=%(refname:short)%09%(committerdate:unix)%09%(subject)";
            let pat = if prefix.is_empty() { "refs/heads/".to_string() } else { format!("refs/heads/{prefix}") };
            let local_txt = rc.g(&["for-each-ref", "--sort=-committerdate", fmt, &pat]).await?;
            let mut local = Vec::new();
            let mut names = std::collections::HashSet::new();
            for l in local_txt.lines() {
                let f: Vec<&str> = l.splitn(3, '\t').collect();
                if f.len() == 3 {
                    names.insert(f[0].to_string());
                    local.push(json!({ "name": f[0], "at": f[1].parse::<i64>().unwrap_or(0), "msg": f[2], "current": f[0] == cur.trim() }));
                }
            }
            // ramas que se borraron: salen del historial de movimientos de la copia
            let rl = rc.g(&["reflog", "--format=%H%x09%gs", "-n", "400"]).await.unwrap_or_default();
            let rows: Vec<(&str, &str)> = rl.lines().filter_map(|l| l.split_once('\t')).collect();
            let mut deleted = Vec::new();
            let mut dseen = std::collections::HashSet::new();
            for (i, (_, gs)) in rows.iter().enumerate() {
                let Some(rest) = gs.strip_prefix("checkout: moving from ") else { continue };
                let Some((from, _)) = rest.split_once(" to ") else { continue };
                if !from.starts_with(&prefix) || !safe_branch(from) || names.contains(from) || !dseen.insert(from.to_string()) {
                    continue;
                }
                if let Some((sha, _)) = rows.get(i + 1) {
                    let at = rc.g(&["log", "-1", "--format=%at%x09%s", sha]).await.unwrap_or_default();
                    let (t, m) = at.split_once('\t').unwrap_or(("0", ""));
                    deleted.push(json!({ "name": from, "sha": &sha[..12.min(sha.len())], "at": t.parse::<i64>().unwrap_or(0), "msg": m }));
                }
                if deleted.len() >= 15 {
                    break;
                }
            }
            // ramas que ya se enviaron a revisión (están en el proyecto) y no están en la copia
            // en el repositorio del proyecto: ramas que llegaron de GitHub (origin) o que se enviaron a revisión sin GitHub
            let mut sent: Vec<Value> = Vec::new();
            let mut sseen = std::collections::HashSet::new();
            for (pat, lstrip) in [(format!("refs/heads/{prefix}"), 2), (format!("refs/remotes/origin/{prefix}"), 3)] {
                let fmt = format!("--format=%(refname:lstrip={lstrip})%09%(committerdate:unix)%09%(subject)");
                let txt = run(Command::new("git").arg("-C").arg(project_dir).args(["for-each-ref", "--sort=-committerdate", &fmt, &pat]), 20).await.unwrap_or_default();
                for l in txt.lines() {
                    let f: Vec<&str> = l.splitn(3, '\t').collect();
                    if f.len() == 3 && safe_branch(f[0]) && !names.contains(f[0]) && sseen.insert(f[0].to_string()) {
                        sent.push(json!({ "name": f[0], "at": f[1].parse::<i64>().unwrap_or(0), "msg": f[2] }));
                    }
                }
            }
            sent.truncate(15);
            Ok(json!({ "local": local, "deleted": deleted, "sent": sent, "dirty": !rc.g(&["status", "--porcelain"]).await.unwrap_or_default().is_empty() }))
        }
        "branch_restore" => {
            let name = p["name"].as_str().unwrap_or("");
            if !safe_branch(name) || !name.starts_with(&prefix) {
                return Err("Solo podés recuperar ramas tuyas".into());
            }
            if rc.g(&["rev-parse", "-q", "--verify", &format!("refs/heads/{name}")]).await.is_ok() {
                return Err("Esa rama ya existe en tu copia".into());
            }
            if p["from"].as_str() == Some("sent") {
                let mut src = format!("refs/heads/{name}");
                let mut sha = run(Command::new("git").arg("-C").arg(project_dir).args(["rev-parse", "-q", "--verify", &src]), 20).await.unwrap_or_default();
                if sha.is_empty() {
                    src = format!("refs/remotes/origin/{name}");
                    sha = run(Command::new("git").arg("-C").arg(project_dir).args(["rev-parse", "-q", "--verify", &src]), 20).await.map_err(|_| "Esa rama no está en el proyecto".to_string())?;
                }
                let sha = sha.trim().to_string();
                if !is_sha(&sha) {
                    return Err("Esa rama no está en el proyecto".into());
                }
                rc.g(&["fetch", "-q", &project_dir.to_string_lossy(), &format!("+{src}:refs/crewhelm/recuperada")]).await.map_err(|e| format!("no pude traerla: {e}"))?;
                rc.g(&["branch", name, "refs/crewhelm/recuperada"]).await?;
                let _ = rc.g(&["update-ref", "-d", "refs/crewhelm/recuperada"]).await;
            } else {
                let sha = p["sha"].as_str().unwrap_or("");
                if !is_sha(sha) || rc.g(&["cat-file", "-e", &format!("{sha}^{{commit}}")]).await.is_err() {
                    return Err("No encontré esa versión de la rama".into());
                }
                rc.g(&["branch", name, sha]).await?;
            }
            let dirty = !rc.g(&["status", "--porcelain"]).await.unwrap_or_default().is_empty();
            let switched = p["switch"].as_bool() == Some(true) && !dirty && rc.g(&["switch", "-q", name]).await.is_ok();
            Ok(json!({ "branch": name, "switched": switched, "dirty": dirty }))
        }
        "branch_switch" => {
            let name = p["name"].as_str().unwrap_or("");
            if !safe_branch(name) || !name.starts_with(&prefix) {
                return Err("Solo podés pasar a ramas tuyas".into());
            }
            if !rc.g(&["status", "--porcelain"]).await.unwrap_or_default().is_empty() {
                return Err("Tenés cambios sin guardar: guardalos primero (botón «💾 Guardar»)".into());
            }
            rc.g(&["switch", "-q", name]).await?;
            Ok(json!({ "branch": name }))
        }
        _ => Err("Acción no válida".into()),
    }
}

// ---------------------------------------------------------------- revisión sin GitHub: ver, juntar y devolver trabajo en la carpeta del proyecto

/// git en la carpeta del proyecto sin hooks (lo que se revisa lo escribió otra persona).
async fn pgit(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new("git");
    c.arg("-C").arg(dir).args(["-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false", "-c", "core.quotepath=false"]).args(args);
    run(&mut c, 120).await
}

/// Rama principal del proyecto: main, master o, si no hay, la que está abierta.
async fn base_branch(dir: &Path) -> Result<String, String> {
    for b in ["main", "master"] {
        if pgit(dir, &["rev-parse", "-q", "--verify", &format!("refs/heads/{b}")]).await.is_ok() {
            return Ok(b.to_string());
        }
    }
    let cur = pgit(dir, &["branch", "--show-current"]).await.unwrap_or_default();
    if cur.is_empty() { Err("El proyecto todavía no tiene una rama principal (hace falta al menos un guardado).".into()) } else { Ok(cur) }
}

async fn review(p: &Value, dir: &Path) -> Result<Value, String> {
    if !dir.join(".git").exists() {
        return Err("La carpeta del proyecto todavía no es un repositorio".into());
    }
    let base = base_branch(dir).await?;
    let branch = p["branch"].as_str().unwrap_or("").to_string();
    let need_branch = || if safe_branch(&branch) { Ok(()) } else { Err("Nombre de rama no válido".to_string()) };
    match p["action"].as_str().unwrap_or("") {
        "info" => {
            let mut out = Vec::new();
            for b in p["branches"].as_array().into_iter().flatten().filter_map(|b| b.as_str()).filter(|b| safe_branch(b)).take(40) {
                let exists = pgit(dir, &["rev-parse", "-q", "--verify", &format!("refs/heads/{b}")]).await.is_ok();
                if !exists {
                    out.push(json!({ "branch": b, "exists": false }));
                    continue;
                }
                let commits: i64 = pgit(dir, &["rev-list", "--count", &format!("{base}..{b}")]).await.unwrap_or_default().trim().parse().unwrap_or(0);
                let (mut files, mut add, mut del) = (0, 0i64, 0i64);
                for l in pgit(dir, &["diff", "--numstat", "--no-renames", &format!("{base}...{b}")]).await.unwrap_or_default().lines() {
                    let f: Vec<&str> = l.splitn(3, '\t').collect();
                    if f.len() == 3 {
                        files += 1;
                        add += f[0].parse::<i64>().unwrap_or(0);
                        del += f[1].parse::<i64>().unwrap_or(0);
                    }
                }
                let mergeable = pgit(dir, &["merge-tree", "--write-tree", "--no-messages", &base, b]).await.is_ok();
                out.push(json!({ "branch": b, "exists": true, "commits": commits, "files": files, "add": add, "del": del, "mergeable": mergeable }));
            }
            Ok(json!({ "base": base, "branches": out }))
        }
        "files" => {
            need_branch()?;
            let range = format!("{base}...{branch}");
            let num = pgit(dir, &["diff", "--numstat", "--no-renames", "-z", &range]).await?;
            let st = pgit(dir, &["diff", "--name-status", "--no-renames", "-z", &range]).await?;
            let mut status = std::collections::HashMap::new();
            let mut it = st.split('\0');
            while let (Some(c), Some(f)) = (it.next(), it.next()) {
                status.insert(f.to_string(), if c.starts_with('A') { "added" } else if c.starts_with('D') { "removed" } else { "modified" });
            }
            let mut budget = 150_000usize;
            let mut files = Vec::new();
            for e in num.split('\0').filter(|e| !e.is_empty()).take(60) {
                let f: Vec<&str> = e.splitn(3, '\t').collect();
                if f.len() != 3 || !safe_rel(f[2]) {
                    continue;
                }
                let binary = f[0] == "-";
                let patch = if binary { String::new() } else { pgit(dir, &["diff", "--no-renames", "--no-color", &range, "--", f[2]]).await.unwrap_or_default() };
                let cut: String = patch.chars().take(8000.min(budget)).collect();
                budget = budget.saturating_sub(cut.len());
                files.push(json!({ "name": f[2], "status": status.get(f[2]).copied().unwrap_or("modified"), "add": f[0].parse::<i64>().unwrap_or(0), "del": f[1].parse::<i64>().unwrap_or(0), "patch": cut, "cut": cut.len() < patch.len(), "binary": binary }));
            }
            Ok(json!({ "files": files }))
        }
        "merge" => {
            need_branch()?;
            if pgit(dir, &["rev-parse", "-q", "--verify", &format!("refs/heads/{branch}")]).await.is_err() {
                return Err("La rama ya no está en el proyecto".into());
            }
            if pgit(dir, &["branch", "--show-current"]).await.unwrap_or_default() != base {
                return Err(format!("La carpeta del proyecto no está en la rama principal («{base}»): pasala a «{base}» antes de integrar."));
            }
            if !pgit(dir, &["status", "--porcelain", "--untracked-files=no"]).await.unwrap_or_default().is_empty() {
                return Err("La carpeta del proyecto tiene cambios sin guardar: guardalos antes de integrar.".into());
            }
            let who: String = p["who"].as_str().unwrap_or("CrewHelm").chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.')).take(60).collect();
            let msg: String = format!("Integra «{}» (de {})", p["title"].as_str().unwrap_or(&branch).chars().filter(|c| !c.is_control()).take(150).collect::<String>(), p["author"].as_str().unwrap_or("?").chars().filter(|c| !c.is_control()).take(60).collect::<String>());
            let id = ["-c", "user.name=CrewHelm", "-c", "user.email=crewhelm@localhost"];
            let squash = p["squash"].as_bool() == Some(true);
            let res = if squash {
                match pgit(dir, &["merge", "--squash", &branch]).await {
                    Ok(_) => pgit(dir, &[id[0], id[1], id[2], id[3], "commit", "-q", "-m", &format!("{msg} — por {who}")]).await,
                    Err(e) => Err(e),
                }
            } else {
                pgit(dir, &[id[0], id[1], id[2], id[3], "merge", "--no-ff", "-q", "-m", &format!("{msg} — por {who}"), &branch]).await
            };
            if let Err(e) = res {
                // git cuenta los conflictos por la salida normal, no por el error: se miran los archivos sin resolver
                let conflict = dir.join(".git/MERGE_HEAD").exists() || !pgit(dir, &["diff", "--name-only", "--diff-filter=U"]).await.unwrap_or_default().is_empty();
                let _ = pgit(dir, &["merge", "--abort"]).await;
                let _ = pgit(dir, &["reset", "-q", "--hard", "HEAD"]).await;
                return Err(if conflict { "Tiene conflictos con la rama principal: la persona tiene que actualizar su copia (cw actualizar) y volver a enviarlo".into() } else { format!("No pude integrar: {e}") });
            }
            let sha = pgit(dir, &["rev-parse", "--short", "HEAD"]).await.unwrap_or_default();
            // la rama de la persona ya no hace falta en el proyecto (queda en su copia)
            let _ = pgit(dir, &["branch", "-D", &branch]).await;
            Ok(json!({ "merged": true, "sha": sha }))
        }
        _ => Err("Acción no válida".into()),
    }
}

async fn run_job(p: &Value) -> Result<Value, String> {
    let dir = crate::iso::expand_home(p["dir"].as_str().unwrap_or(""));
    if !dir.is_absolute() {
        return Err("la carpeta del proyecto tiene que ser una ruta completa".into());
    }
    let mode = p["mode"].as_str().unwrap_or("");
    let mut steps: Vec<String> = Vec::new();
    match mode {
        "usage" => {
            // espacio que ocupa cada persona en esta máquina (para el panel del CTO)
            let Some(root) = crate::term::workspace().map(|w| w.join("homeCrewHelm")) else { return Ok(json!({ "people": [], "limit_mb": 0 })) };
            let mut people = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&root) {
                for e in rd.flatten().take(60) {
                    if !e.path().is_dir() {
                        continue;
                    }
                    let path = e.path();
                    let bytes = tokio::task::spawn_blocking(move || crate::iso::dir_usage(&path, 8)).await.unwrap_or(0);
                    people.push(json!({ "name": e.file_name().to_string_lossy(), "mb": bytes >> 20 }));
                }
            }
            return Ok(json!({ "people": people, "limit_mb": crate::iso::quota_mb() }));
        }
        "status" => return Ok(status(&dir).await),
        "recover" => return recover(p, &dir).await,
        "review" => return review(p, &dir).await,
        "mirror" => {
            // «todo en el servidor»: se baja todo lo que el proyecto tiene en GitHub (ramas y etiquetas) y se le quita la subida;
            // al volver a permitir GitHub se la devuelve
            if !dir.join(".git").exists() {
                return Ok(json!({ "skipped": true, "why": "la carpeta todavía no es un repositorio" }));
            }
            if git(&dir, &["config", "--get", "remote.origin.url"]).await.unwrap_or_default().is_empty() {
                return Ok(json!({ "skipped": true, "why": "no tiene origen en GitHub" }));
            }
            if p["lock"].as_bool() == Some(false) {
                let _ = git(&dir, &["config", "--unset-all", "remote.origin.pushurl"]).await;
                return Ok(json!({ "unlocked": true }));
            }
            git(&dir, &["fetch", "-q", "--tags", "origin", "+refs/heads/*:refs/remotes/origin/*"]).await.map_err(|e| format!("no pude bajar de GitHub: {e}"))?;
            let remotes = git(&dir, &["for-each-ref", "--format=%(refname:lstrip=3)", "refs/remotes/origin"]).await.unwrap_or_default();
            let locals = git(&dir, &["for-each-ref", "--format=%(refname:short)", "refs/heads"]).await.unwrap_or_default();
            let have: std::collections::HashSet<&str> = locals.lines().collect();
            let (mut total, mut created) = (0, 0);
            for b in remotes.lines().filter(|b| !b.is_empty() && *b != "HEAD" && safe_branch(b)).take(500) {
                total += 1;
                if !have.contains(b) && git(&dir, &["branch", "-q", b, &format!("refs/remotes/origin/{b}")]).await.is_ok() {
                    created += 1;
                }
            }
            let tags = git(&dir, &["tag", "--list"]).await.unwrap_or_default().lines().count();
            let commits: i64 = git(&dir, &["rev-list", "--all", "--count"]).await.unwrap_or_default().trim().parse().unwrap_or(0);
            // sin subida: un `git push` desde cualquier terminal falla (el origen queda solo para leer)
            git(&dir, &["config", "remote.origin.pushurl", "crewhelm-sin-subida:todo-en-el-servidor"]).await?;
            return Ok(json!({ "branches": total, "created": created, "tags": tags, "commits": commits }));
        }

        "sync" => {
            // después de integrar un Pull Request: la carpeta del proyecto se pone al día con GitHub (solo si se puede sin pisar nada)
            if !dir.join(".git").exists() {
                return Err("La carpeta todavía no es un repositorio".into());
            }
            git(&dir, &["fetch", "-q", "origin"]).await.map_err(|e| format!("no pude traer las novedades de GitHub: {e}"))?;
            let cur = git(&dir, &["branch", "--show-current"]).await.unwrap_or_default();
            let cur = cur.trim().to_string();
            let remote = format!("refs/remotes/origin/{cur}");
            if cur.is_empty() || git(&dir, &["rev-parse", "-q", "--verify", &remote]).await.is_err() {
                steps.push("Novedades traídas (la carpeta no está en una rama que exista en GitHub, no se tocó)".into());
            } else if !git(&dir, &["status", "--porcelain", "--untracked-files=no"]).await.unwrap_or_default().trim().is_empty() {
                steps.push("Novedades traídas, pero la carpeta tiene cambios sin guardar: no se actualizó".into());
            } else if git(&dir, &["merge", "-q", "--ff-only", &remote]).await.is_ok() {
                steps.push(format!("La carpeta del proyecto quedó al día con «{cur}» de GitHub"));
            } else {
                steps.push("Novedades traídas, pero la carpeta tiene cambios propios que GitHub no tiene: no se actualizó".into());
            }
            let mut st = status(&dir).await;
            st["steps"] = json!(steps);
            return Ok(st);
        }
        "local" => {
            init_local(&dir, &mut steps).await?;
        }
        "attach" | "clone" => {
            let repo = p["repo"].as_str().unwrap_or("").to_string();
            let is_repo = dir.join(".git").exists();
            if mode == "clone" && is_repo {
                return Err("La carpeta ya es un repositorio: usá «Conectar con GitHub» en lugar de traerlo".into());
            }
            if mode == "attach" && !is_repo {
                return Err("La carpeta todavía no es un repositorio".into());
            }
            if mode == "clone" && dir.is_dir() && std::fs::read_dir(&dir).map(|mut r| r.next().is_some()).unwrap_or(false) {
                return Err("La carpeta ya tiene archivos: elegí «Crear uno nuevo» para subirlos, o vaciala para traer el repositorio".into());
            }
            let ssh = deploy_key(p, &repo, &mut steps).await?;
            if mode == "clone" {
                if let Some(parent) = dir.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let url = format!("git@{}:{repo}.git", host_of(p["base"].as_str().unwrap_or("https://github.com")));
                let parent = dir.parent().unwrap_or(Path::new("/")).to_path_buf();
                let d = dir.to_string_lossy().to_string();
                retry(|| git_ssh(&parent, &ssh, vec!["clone".into(), "-q".into(), url.clone(), d.clone()])).await.map_err(|e| format!("no pude traer el repositorio: {e}"))?;
                steps.push(format!("Repositorio {repo} traído a la carpeta del proyecto"));
            }
            configure(&dir, p, &repo, &ssh).await?;
            if mode == "attach" {
                retry(|| git_ssh(&dir, &ssh, vec!["ls-remote".into(), "-q".into(), "origin".into()])).await.map_err(|e| format!("la carpeta quedó configurada pero GitHub no me dejó entrar al repositorio: {e}"))?;
                steps.push(format!("Carpeta conectada con {repo}"));
            }
        }
        "new" => {
            let (name, private) = (p["name"].as_str().unwrap_or(""), p["private"].as_bool().unwrap_or(true));
            let (api, token) = (p["api"].as_str().unwrap_or("https://api.github.com"), p["token"].as_str().unwrap_or(""));
            init_local(&dir, &mut steps).await?;
            let r = reqwest::Client::new()
                .post(format!("{api}/user/repos"))
                .bearer_auth(token)
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "crewhelm-agent")
                .json(&json!({ "name": name, "private": private, "description": "Creado desde CrewHelm" }))
                .send()
                .await
                .map_err(|_| "no pude hablar con GitHub".to_string())?;
            let code = r.status();
            let body: Value = r.json().await.unwrap_or(Value::Null);
            if !code.is_success() {
                return Err(if code.as_u16() == 422 { "Ya existe un repositorio con ese nombre en tu cuenta de GitHub: elegí otro nombre".to_string() } else { format!("GitHub no me dejó crear el repositorio ({code}): {}", crate::tail(body["message"].as_str().unwrap_or(""), 160)) });
            }
            let repo = body["full_name"].as_str().ok_or("GitHub no devolvió el nombre del repositorio")?.to_string();
            steps.push(format!("Repositorio {repo} creado en GitHub ({})", if private { "privado" } else { "público" }));
            let ssh = deploy_key(p, &repo, &mut steps).await?;
            configure(&dir, p, &repo, &ssh).await?;
            let branch = git(&dir, &["branch", "--show-current"]).await.unwrap_or_default();
            let branch = if branch.is_empty() { "main".to_string() } else { branch };
            retry(|| git_ssh(&dir, &ssh, vec!["push".into(), "-q".into(), "-u".into(), "origin".into(), branch.clone()])).await.map_err(|e| format!("el repositorio se creó pero no pude subir el contenido: {e}"))?;
            steps.push("Contenido subido a GitHub".into());
            let mut st = status(&dir).await;
            st["repo"] = json!(repo);
            st["steps"] = json!(steps);
            return Ok(st);
        }
        _ => return Err("opción no válida".into()),
    }
    let mut st = status(&dir).await;
    if let Some(repo) = p["repo"].as_str().filter(|r| !r.is_empty()) {
        st["repo"] = json!(repo);
    }
    st["steps"] = json!(steps);
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn solo_local_crea_el_repositorio_con_gitignore_y_primer_commit() {
        let d = std::env::temp_dir().join(format!("tm-gs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let dir = d.join("proyecto");
        let ya_esta = run_job(&json!({ "mode": "status", "dir": dir })).await.unwrap();
        assert_eq!(ya_esta["exists"], false);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "hola").unwrap();
        std::fs::write(dir.join(".env"), "SECRETO=1").unwrap();
        let r = run_job(&json!({ "mode": "local", "dir": dir })).await.unwrap();
        assert_eq!((r["is_repo"].clone(), r["commits"].clone(), r["branch"].clone()), (json!(true), json!(true), json!("main")));
        let files = git(&dir, &["ls-files"]).await.unwrap();
        assert!(files.contains("a.txt") && !files.contains(".env"), "archivos: {files}");
        // repetirlo no rompe nada ni duplica commits
        run_job(&json!({ "mode": "local", "dir": dir })).await.unwrap();
        assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]).await.unwrap(), "1");
        // traer un repositorio sobre una carpeta que ya es repositorio se rechaza
        assert!(run_job(&json!({ "mode": "clone", "dir": dir, "repo": "a/b" })).await.is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn recuperar_archivos_y_ramas() {
        let d = std::env::temp_dir().join(format!("cw-rec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        let sh = |args: &[&str]| {
            let o = std::process::Command::new("git").arg("-C").arg(&d).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).output().unwrap();
            assert!(o.status.success(), "{args:?}: {}", String::from_utf8_lossy(&o.stderr));
        };
        sh(&["init", "-q", "-b", "main"]);
        std::fs::write(d.join("a.txt"), "v1").unwrap();
        std::fs::write(d.join("sub/b.txt"), "bb").unwrap();
        sh(&["add", "-A"]);
        sh(&["commit", "-q", "-m", "uno"]);
        std::fs::write(d.join("a.txt"), "v2").unwrap();
        sh(&["commit", "-qam", "dos"]);
        sh(&["rm", "-q", "sub/b.txt"]);
        sh(&["commit", "-qm", "borro b"]);
        let q = |extra: Value| {
            let mut v = json!({ "mode": "recover", "dir": d.to_string_lossy(), "owner": true, "who": "t", "project": "p" });
            for (k, x) in extra.as_object().unwrap() { v[k] = x.clone(); }
            v
        };
        // archivo borrado en un guardado anterior
        let l = run_job(&q(json!({ "action": "list" }))).await.unwrap();
        assert_eq!(l["deleted"][0], "sub/b.txt");
        let v = run_job(&q(json!({ "action": "versions", "path": "sub/b.txt" }))).await.unwrap();
        let rev = v["versions"][0]["rev"].as_str().unwrap().to_string();
        run_job(&q(json!({ "action": "restore", "path": "sub/b.txt", "rev": rev }))).await.unwrap();
        assert_eq!(std::fs::read_to_string(d.join("sub/b.txt")).unwrap(), "bb");
        // archivo cambiado: se pisa con la versión vieja y lo de hoy queda al lado
        std::fs::write(d.join("a.txt"), "roto").unwrap();
        let v = run_job(&q(json!({ "action": "versions", "path": "a.txt" }))).await.unwrap();
        let old = v["versions"].as_array().unwrap().last().unwrap()["rev"].as_str().unwrap().to_string();
        let r = run_job(&q(json!({ "action": "restore", "path": "a.txt", "rev": old }))).await.unwrap();
        assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "v1");
        let b = r["backup"].as_str().unwrap();
        assert_eq!(std::fs::read_to_string(d.join(b)).unwrap(), "roto");
        // rutas peligrosas
        for bad in ["../x", "/etc/passwd", ".git/config", "a/../../x"] {
            assert!(run_job(&q(json!({ "action": "restore", "path": bad, "rev": "base" }))).await.is_err(), "{bad}");
        }
        // rama borrada: se vuelve a crear desde el historial
        sh(&["switch", "-q", "-c", "t/trabajo"]);
        std::fs::write(d.join("n.txt"), "n").unwrap();
        sh(&["add", "-A"]);
        sh(&["commit", "-qm", "nuevo"]);
        sh(&["switch", "-q", "main"]);
        sh(&["branch", "-D", "t/trabajo"]);
        let br = run_job(&q(json!({ "action": "branches" }))).await.unwrap();
        let del = br["deleted"].as_array().unwrap();
        assert_eq!(del[0]["name"], "t/trabajo");
        let sha = del[0]["sha"].as_str().unwrap().to_string();
        let x = run_job(&q(json!({ "action": "branch_restore", "name": "t/trabajo", "sha": sha, "from": "reflog", "switch": true }))).await.unwrap();
        assert_eq!(x["switched"], true);
        assert!(d.join("n.txt").exists());
        assert!(run_job(&q(json!({ "action": "branch_restore", "name": "../x", "sha": "abcd1234" }))).await.is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn git_en_capsula() {
        if !crate::iso::bwrap_ok().await {
            return;
        }
        let d = std::env::temp_dir().join(format!("cw-caps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        git(&d, &["init", "-q", "-b", "main"]).await.unwrap();
        std::fs::write(d.join("a"), "1").unwrap();
        git(&d, &["add", "-A"]).await.unwrap();
        git(&d, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "x"]).await.unwrap();
        std::fs::write(d.join("a"), "2").unwrap();
        // restaurar dentro de la cápsula escribe en la copia y funciona sin red
        sgit(&d, true, &["restore", "--source", "HEAD", "--worktree", "--", "a"]).await.unwrap();
        assert_eq!(std::fs::read_to_string(d.join("a")).unwrap(), "1");
        // un hook malicioso de la copia no se ejecuta
        std::fs::write(d.join(".git/hooks/post-checkout"), "#!/bin/sh\ntouch /tmp/cw-hook-pwned\n").unwrap();
        let _ = sgit(&d, true, &["switch", "-q", "-c", "otra"]).await;
        assert!(!std::path::Path::new("/tmp/cw-hook-pwned").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn bajar_todo_y_cortar_github() {
        let d = std::env::temp_dir().join(format!("cw-mirror-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let g = |dir: &Path, a: &[&str]| {
            let o = std::process::Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "protocol.file.allow=always"]).args(a).output().unwrap();
            assert!(o.status.success(), "{a:?}: {}", String::from_utf8_lossy(&o.stderr));
        };
        // «GitHub»: repositorio con dos ramas y una etiqueta
        let (gh, work) = (d.join("gh"), d.join("work"));
        std::fs::create_dir_all(&gh).unwrap();
        g(&gh, &["init", "-q", "-b", "main"]);
        std::fs::write(gh.join("a"), "1").unwrap();
        g(&gh, &["add", "-A"]);
        g(&gh, &["commit", "-qm", "uno"]);
        g(&gh, &["branch", "ana/otra"]);
        g(&gh, &["tag", "v1"]);
        std::process::Command::new("git").args(["clone", "-q"]).arg(&gh).arg(&work).status().unwrap();
        // alguien sube una rama nueva a GitHub después del clon
        g(&gh, &["branch", "bob/nueva"]);
        let dir = work.to_string_lossy().to_string();
        let r = run_job(&json!({ "mode": "mirror", "dir": dir, "lock": true })).await.unwrap();
        assert!(r["created"].as_i64().unwrap() >= 2, "{r}");
        assert_eq!(r["tags"], 1);
        let b = git(&work, &["branch", "--list"]).await.unwrap();
        assert!(b.contains("bob/nueva") && b.contains("ana/otra"), "{b}");
        // sin subida a GitHub
        assert!(git(&work, &["push", "-q", "origin", "main"]).await.is_err());
        // al permitir GitHub otra vez se devuelve la subida
        run_job(&json!({ "mode": "mirror", "dir": dir, "lock": false })).await.unwrap();
        assert!(git(&work, &["config", "--get", "remote.origin.pushurl"]).await.is_err());
        // carpeta sin origen: no hace nada
        let sola = d.join("sola");
        std::fs::create_dir_all(&sola).unwrap();
        g(&sola, &["init", "-q"]);
        let r = run_job(&json!({ "mode": "mirror", "dir": sola.to_string_lossy(), "lock": true })).await.unwrap();
        assert_eq!(r["skipped"], true);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn revisar_y_juntar_sin_github() {
        let d = std::env::temp_dir().join(format!("cw-rev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let g = |a: &[&str]| {
            let o = std::process::Command::new("git").arg("-C").arg(&d).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).output().unwrap();
            assert!(o.status.success(), "{a:?}: {}", String::from_utf8_lossy(&o.stderr));
        };
        g(&["init", "-q", "-b", "main"]);
        std::fs::write(d.join("a.txt"), "uno\n").unwrap();
        g(&["add", "-A"]);
        g(&["commit", "-qm", "base"]);
        // rama limpia
        g(&["switch", "-qc", "marta/tema"]);
        std::fs::write(d.join("b.txt"), "nuevo\n").unwrap();
        g(&["add", "-A"]);
        g(&["commit", "-qm", "b"]);
        // rama que choca con main
        g(&["switch", "-q", "main"]);
        g(&["switch", "-qc", "juan/choque"]);
        std::fs::write(d.join("a.txt"), "juan\n").unwrap();
        g(&["commit", "-qam", "j"]);
        g(&["switch", "-q", "main"]);
        std::fs::write(d.join("a.txt"), "main\n").unwrap();
        g(&["commit", "-qam", "m"]);
        let q = |extra: Value| {
            let mut v = json!({ "mode": "review", "dir": d.to_string_lossy() });
            for (k, x) in extra.as_object().unwrap() { v[k] = x.clone(); }
            v
        };
        let i = run_job(&q(json!({ "action": "info", "branches": ["marta/tema", "juan/choque", "no/existe"] }))).await.unwrap();
        assert_eq!(i["base"], "main");
        let b = i["branches"].as_array().unwrap();
        assert_eq!((b[0]["files"].as_i64(), b[0]["mergeable"].clone()), (Some(1), json!(true)));
        assert_eq!(b[1]["mergeable"], false);
        assert_eq!(b[2]["exists"], false);
        let f = run_job(&q(json!({ "action": "files", "branch": "marta/tema" }))).await.unwrap();
        assert_eq!(f["files"][0]["name"], "b.txt");
        assert_eq!(f["files"][0]["status"], "added");
        assert!(f["files"][0]["patch"].as_str().unwrap().contains("+nuevo"));
        // con conflicto: se rechaza y la carpeta queda limpia
        let e = run_job(&q(json!({ "action": "merge", "branch": "juan/choque", "title": "t", "author": "juan", "who": "ana" }))).await.unwrap_err();
        assert!(e.contains("conflictos"), "{e}");
        assert!(git(&d, &["status", "--porcelain"]).await.unwrap().is_empty());
        // limpia: se junta y la rama se borra del proyecto
        let m = run_job(&q(json!({ "action": "merge", "branch": "marta/tema", "title": "Mi tema", "author": "marta", "who": "ana" }))).await.unwrap();
        assert_eq!(m["merged"], true);
        assert!(d.join("b.txt").exists());
        assert!(!git(&d, &["branch", "--list", "marta/tema"]).await.unwrap().contains("marta"));
        assert!(git(&d, &["log", "-1", "--format=%s"]).await.unwrap().contains("Integra «Mi tema» (de marta)"));
        // cambios sin guardar en la carpeta: no se integra encima
        std::fs::write(d.join("a.txt"), "sucio").unwrap();
        let e = run_job(&q(json!({ "action": "merge", "branch": "juan/choque", "title": "t", "author": "j", "who": "ana" }))).await.unwrap_err();
        assert!(e.contains("sin guardar"), "{e}");
        // nombres peligrosos
        assert!(run_job(&q(json!({ "action": "files", "branch": "../x" }))).await.is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
