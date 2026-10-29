//! tm-agent: vigila el servidor y, cuando le asignan una tarea a esta IA, abre un agente en la carpeta de
//! trabajo del proyecto, comenta y pasa la tarea a «En curso» y «Hecho».
use anyhow::{bail, Context, Result};
use reqwest::Method;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{process::Command, sync::Semaphore};

const HELP: &str = "tm-agent: pone a trabajar a una IA cuando le asignan una tarea en rust-tui-manager.

Variables de entorno:
  TM_TOKEN          token de la IA (obligatorio; se crea en la web: 🤖 IAs → «+»)
  TM_AGENT_CMD      comando que lanza al agente (obligatorio), por ejemplo:
                    claude -p \"$TM_TASK_PROMPT\" --permission-mode acceptEdits
  TM_URL            servidor (por defecto http://127.0.0.1:7878)
  TM_AGENT_POLL     segundos entre consultas (20)
  TM_AGENT_TIMEOUT  segundos máximos por tarea (1800)
  TM_AGENT_MAX      tareas en paralelo (1)
  TM_AGENT_DIR      carpeta a usar si el proyecto no tiene una definida o no existe en esta máquina
  TM_AGENT_ONCE=1   procesa lo pendiente y sale

El comando corre en la carpeta de trabajo del proyecto con TM_TASK_PROMPT, TM_TASK_ID, TM_TASK_TITLE,
TM_PROJECT_ID, TM_URL y TM_TOKEN en el entorno. Si termina bien, la tarea pasa a Hecho; si falla,
vuelve a Por hacer con un comentario y, mientras el agente siga corriendo, no se reintenta
hasta que alguien la modifique.

Alarma de asistencia: dentro del agente, `tm-agent ask \"¿Puedo borrar X?\"` comenta en la tarea con 🙋
(avisa a admins y responsables, por la web, Telegram y email) y espera la respuesta humana: la imprime
por stdout y el agente la recibe como resultado del comando. TM_ASK_TIMEOUT = segundos de espera (900).";

struct Cfg {
    url: String,
    token: String,
    cmd: String,
    poll: u64,
    timeout: u64,
    max: usize,
    fallback_dir: Option<PathBuf>,
    once: bool,
}

fn env_num<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

impl Cfg {
    fn from_env() -> Result<Self> {
        let token = std::env::var("TM_TOKEN").context("falta TM_TOKEN (el token de la IA)")?;
        let cmd = std::env::var("TM_AGENT_CMD").context("falta TM_AGENT_CMD (el comando que lanza al agente)")?;
        Ok(Self {
            url: std::env::var("TM_URL").unwrap_or_else(|_| "http://127.0.0.1:7878".into()).trim_end_matches('/').into(),
            token,
            cmd,
            poll: env_num("TM_AGENT_POLL", 20).max(2),
            timeout: env_num("TM_AGENT_TIMEOUT", 1800),
            max: env_num("TM_AGENT_MAX", 1usize).max(1),
            fallback_dir: std::env::var("TM_AGENT_DIR").ok().filter(|d| !d.is_empty()).map(PathBuf::from),
            once: std::env::var("TM_AGENT_ONCE").is_ok_and(|v| v == "1"),
        })
    }
}

struct Api {
    http: reqwest::Client,
    cfg: Cfg,
}

impl Api {
    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value> {
        let mut r = self.http.request(method, format!("{}{path}", self.cfg.url)).bearer_auth(&self.cfg.token);
        if let Some(b) = body {
            r = r.json(&b);
        }
        let resp = r.send().await.with_context(|| format!("no se pudo conectar con {}", self.cfg.url))?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            bail!("{path}: {status} {text}");
        }
        Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
    }
    async fn get(&self, path: &str) -> Result<Value> {
        self.call(Method::GET, path, None).await
    }
    async fn say(&self, task: i64, body: &str) {
        if let Err(e) = self.call(Method::POST, &format!("/api/tasks/{task}/comments"), Some(json!({ "body": body }))).await {
            log(&format!("no pude comentar la tarea {task}: {e}"));
        }
    }
    async fn set_status(&self, task: i64, status: &str) -> Result<Value> {
        self.call(Method::PATCH, &format!("/api/tasks/{task}"), Some(json!({ "status": status }))).await
    }
}

fn log(msg: &str) {
    println!("[tm-agent] {msg}");
}

fn tail(s: &str, max: usize) -> String {
    let t: Vec<char> = s.trim().chars().collect();
    if t.len() <= max { t.into_iter().collect() } else { format!("…{}", t[t.len() - max..].iter().collect::<String>()) }
}

struct Job {
    id: i64,
    title: String,
    description: String,
    project_id: i64,
    project: String,
    repo: String,
    work_dir: String,
    updated_at: String,
}

async fn prompt_for(api: &Api, job: &Job, dir: &std::path::Path) -> String {
    let comments = api.get(&format!("/api/tasks/{}/comments", job.id)).await.unwrap_or(Value::Null);
    let notes: Vec<String> = comments
        .as_array()
        .map(|a| a.iter().rev().take(10).rev().map(|c| format!("- {}: {}", c["username"].as_str().unwrap_or("?"), c["body"].as_str().unwrap_or(""))).collect())
        .unwrap_or_default();
    format!(
        "Te asignaron una tarea en el tablero de comando.\n\nProyecto: {}\nRepositorio: {}\nCarpeta de trabajo: {}\n\nTarea #{}: {}\n\n{}\n\n{}\nSi necesitás una autorización, una respuesta o que una persona haga algo, ejecutá `tm-agent ask \"tu pregunta\"` (usa TM_TASK_ID y TM_TOKEN del entorno): avisa a las personas y devuelve su respuesta por stdout.\n\nTrabajá dentro de la carpeta de trabajo. Al terminar, respondé con un resumen breve de lo que hiciste y de cualquier cosa que necesite revisión humana.",
        job.project,
        if job.repo.is_empty() { "(sin definir)" } else { &job.repo },
        dir.display(),
        job.id,
        job.title,
        if job.description.is_empty() { "(sin descripción)" } else { &job.description },
        if notes.is_empty() { String::new() } else { format!("Comentarios recientes:\n{}\n", notes.join("\n")) },
    )
}

/// Devuelve el `updated_at` de la tarea tras el último cambio, para no reintentarla sin que nadie la toque.
async fn run(api: &Api, job: &Job) -> Option<String> {
    let dir = [Some(PathBuf::from(&job.work_dir)).filter(|d| !job.work_dir.is_empty() && d.is_dir()), api.cfg.fallback_dir.clone().filter(|d| d.is_dir())]
        .into_iter()
        .flatten()
        .next();
    let Some(dir) = dir else {
        api.say(job.id, &format!("⚠️ No puedo empezar: la carpeta de trabajo «{}» no existe en la máquina donde corre este agente (se puede usar TM_AGENT_DIR).", job.work_dir)).await;
        return Some(job.updated_at.clone());
    };
    let claimed = match api.set_status(job.id, "doing").await {
        Ok(t) => t,
        Err(e) => {
            log(&format!("no pude tomar la tarea {}: {e}", job.id));
            return None;
        }
    };
    log(&format!("▶ #{} «{}» en {}", job.id, job.title, dir.display()));
    api.say(job.id, &format!("▶ Empiezo con esta tarea. Trabajo en `{}`.", dir.display())).await;
    let prompt = prompt_for(api, job, &dir).await;

    let mut cmd = if cfg!(windows) { let mut c = Command::new("cmd"); c.arg("/C"); c } else { let mut c = Command::new("sh"); c.arg("-c"); c };
    cmd.arg(&api.cfg.cmd)
        .current_dir(&dir)
        .env("TM_TASK_PROMPT", &prompt)
        .env("TM_TASK_ID", job.id.to_string())
        .env("TM_TASK_TITLE", &job.title)
        .env("TM_PROJECT_ID", job.project_id.to_string())
        .env("TM_URL", &api.cfg.url)
        .env("TM_TOKEN", &api.cfg.token)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let out = match tokio::time::timeout(Duration::from_secs(api.cfg.timeout), async { cmd.spawn()?.wait_with_output().await }).await {
        Ok(Ok(o)) => Ok(o),
        Ok(Err(e)) => Err(format!("no pude lanzar el comando: {e}")),
        Err(_) => Err(format!("se pasó del tiempo máximo ({} s) y lo cancelé", api.cfg.timeout)),
    };
    let ok_text = |o: &std::process::Output| {
        let so = String::from_utf8_lossy(&o.stdout);
        let se = String::from_utf8_lossy(&o.stderr);
        if so.trim().is_empty() { se.into_owned() } else { so.into_owned() }
    };
    match out {
        Ok(o) if o.status.success() => {
            api.say(job.id, &format!("✅ Terminé.\n\n{}", tail(&ok_text(&o), 1800))).await;
            log(&format!("✅ #{} hecha", job.id));
            api.set_status(job.id, "done").await.ok().and_then(|t| t["updated_at"].as_str().map(String::from))
        }
        other => {
            let why = match &other {
                Ok(o) => format!("terminó con error ({})\n\n{}", o.status, tail(&ok_text(o), 1500)),
                Err(e) => e.clone(),
            };
            api.say(job.id, &format!("❌ No pude completarla: {why}\n\nLa devuelvo a «Por hacer»; mientras siga corriendo, no la reintento hasta que alguien la modifique.")).await;
            log(&format!("❌ #{} falló", job.id));
            api.set_status(job.id, "todo").await.ok().and_then(|t| t["updated_at"].as_str().map(String::from)).or(claimed["updated_at"].as_str().map(String::from))
        }
    }
}

async fn pending(api: &Api, me: i64, skip: &HashMap<i64, String>, running: &HashSet<i64>) -> Result<Vec<Job>> {
    let mut jobs = Vec::new();
    for team in api.get("/api/teams").await?.as_array().cloned().unwrap_or_default() {
        for p in api.get(&format!("/api/teams/{}/projects", team["id"])).await?.as_array().cloned().unwrap_or_default() {
            let tasks = api.get(&format!("/api/projects/{}/tasks", p["id"])).await?;
            for t in tasks.as_array().cloned().unwrap_or_default() {
                let id = t["id"].as_i64().unwrap_or(0);
                if t["assignee_id"].as_i64() != Some(me) || t["status"] != "todo" || running.contains(&id) {
                    continue;
                }
                if skip.get(&id).is_some_and(|u| Some(u.as_str()) == t["updated_at"].as_str()) {
                    continue;
                }
                jobs.push(Job {
                    id,
                    title: t["title"].as_str().unwrap_or("").into(),
                    description: t["description"].as_str().unwrap_or("").into(),
                    project_id: p["id"].as_i64().unwrap_or(0),
                    project: p["name"].as_str().unwrap_or("").into(),
                    repo: p["repo"].as_str().unwrap_or("").into(),
                    work_dir: p["work_dir"].as_str().unwrap_or("").into(),
                    updated_at: t["updated_at"].as_str().unwrap_or("").into(),
                });
            }
        }
    }
    Ok(jobs)
}

/// `tm-agent ask «pregunta»`: levanta la alarma 🙋 en la tarea actual y espera la primera respuesta de otra persona.
async fn ask(question: String) -> Result<()> {
    if question.trim().is_empty() {
        bail!("uso: tm-agent ask \"pregunta o pedido\"");
    }
    let task: i64 = std::env::var("TM_TASK_ID").ok().and_then(|v| v.parse().ok()).context("falta TM_TASK_ID (lo define tm-agent al lanzar al agente)")?;
    let cfg = Cfg {
        url: std::env::var("TM_URL").unwrap_or_else(|_| "http://127.0.0.1:7878".into()).trim_end_matches('/').into(),
        token: std::env::var("TM_TOKEN").context("falta TM_TOKEN")?,
        cmd: String::new(),
        poll: 5,
        timeout: 0,
        max: 1,
        fallback_dir: None,
        once: true,
    };
    let wait = env_num("TM_ASK_TIMEOUT", 900u64);
    let api = Api { http: reqwest::Client::new(), cfg };
    let me = api.get("/api/me").await?["id"].as_i64().context("respuesta inesperada de /api/me")?;
    let sent = api.call(Method::POST, &format!("/api/tasks/{task}/comments"), Some(json!({ "body": format!("🙋 {}", question.trim()) }))).await?;
    let after = sent["id"].as_i64().unwrap_or(0);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(wait);
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(api.cfg.poll)).await;
        let Ok(list) = api.get(&format!("/api/tasks/{task}/comments")).await else { continue };
        if let Some(c) = list.as_array().into_iter().flatten().find(|c| c["id"].as_i64().unwrap_or(0) > after && c["user_id"].as_i64() != Some(me)) {
            println!("{}: {}", c["username"].as_str().unwrap_or("?"), c["body"].as_str().unwrap_or(""));
            return Ok(());
        }
    }
    bail!("nadie respondió en {wait} s; seguí sin esa respuesta o dejá la tarea documentada")
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("tm-agent {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("ask") {
        return ask(std::env::args().skip(2).collect::<Vec<_>>().join(" ")).await;
    }
    let cfg = Cfg::from_env().inspect_err(|_| eprintln!("{HELP}\n"))?;
    let (poll, max, once) = (cfg.poll, cfg.max, cfg.once);
    let api = Arc::new(Api { http: reqwest::Client::new(), cfg });
    let me = api.get("/api/me").await.context("no pude iniciar sesión con TM_TOKEN")?;
    let my_id = me["id"].as_i64().context("respuesta inesperada de /api/me")?;
    log(&format!("conectado como {} (id {my_id}) a {}; reviso cada {poll} s, hasta {max} en paralelo", me["username"].as_str().unwrap_or("?"), api.cfg.url));

    let sem = Arc::new(Semaphore::new(max));
    let running: Arc<Mutex<HashSet<i64>>> = Default::default();
    let skip: Arc<Mutex<HashMap<i64, String>>> = Default::default();
    loop {
        let (skip_now, running_now) = (skip.lock().unwrap().clone(), running.lock().unwrap().clone());
        let mut started = 0;
        match pending(&api, my_id, &skip_now, &running_now).await {
            Ok(jobs) => {
                for job in jobs {
                    let Ok(permit) = sem.clone().try_acquire_owned() else { break };
                    started += 1;
                    running.lock().unwrap().insert(job.id);
                    let (api, running, skip) = (api.clone(), running.clone(), skip.clone());
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Some(updated) = run(&api, &job).await {
                            skip.lock().unwrap().insert(job.id, updated);
                        }
                        running.lock().unwrap().remove(&job.id);
                    });
                }
            }
            Err(e) => log(&format!("no pude consultar el servidor: {e}")),
        }
        if once {
            let _all = sem.acquire_many(max as u32).await?;
            if started == 0 {
                return Ok(());
            }
            continue;
        }
        tokio::time::sleep(Duration::from_secs(poll)).await;
    }
}
