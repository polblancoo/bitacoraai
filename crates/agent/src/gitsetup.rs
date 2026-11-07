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

async fn run_job(p: &Value) -> Result<Value, String> {
    let dir = crate::iso::expand_home(p["dir"].as_str().unwrap_or(""));
    if !dir.is_absolute() {
        return Err("la carpeta del proyecto tiene que ser una ruta completa".into());
    }
    let mode = p["mode"].as_str().unwrap_or("");
    let mut steps: Vec<String> = Vec::new();
    match mode {
        "status" => return Ok(status(&dir).await),
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
}
