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

mod term;

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
  TM_AGENT_TERMINAL=1  habilita la terminal desde la web: un owner abre un shell en esta máquina (carpeta del proyecto)
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

/// `~` y `~/carpeta` se resuelven contra la carpeta personal (el sistema no lo hace fuera de una shell).
/// Cuenta al servidor qué dicen whitelist.txt / blacklist.txt (junto al script del agente) para que se vean y se puedan pasar a la app.
async fn publish_lists(api: &Api) {
    if let Some(dir) = lists_dir(&api.cfg.cmd) {
        let read = |f: &str| {
            std::fs::read_to_string(dir.join(f)).unwrap_or_default().lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect::<Vec<_>>().join("\n")
        };
        let _ = api.call(Method::POST, "/api/me/base-lists", Some(json!({ "allow": read("whitelist.txt"), "deny": read("blacklist.txt") }))).await;
    }
}

/// Carpeta de las listas del script del agente: `TM_AGENT_CONF` o la carpeta del script de `TM_AGENT_CMD`.
fn lists_dir(cmd: &str) -> Option<std::path::PathBuf> {
    if let Ok(d) = std::env::var("TM_AGENT_CONF") {
        if !d.trim().is_empty() {
            return Some(d.trim().into());
        }
    }
    let first = cmd.split_whitespace().next()?.trim_matches(|c| c == '\'' || c == '"');
    let p = std::path::Path::new(first);
    p.is_absolute().then(|| p.parent().map(|d| d.to_path_buf())).flatten()
}

fn expand_home(p: &str) -> PathBuf {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    match (p.strip_prefix("~/").or_else(|| (p == "~").then_some("")), home) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(p),
    }
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
            fallback_dir: std::env::var("TM_AGENT_DIR").ok().filter(|d| !d.is_empty()).map(|d| expand_home(&d)),
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
    // lo que una persona hizo en la terminal de esta tarea (la guarda tm-server en la carpeta del proyecto)
    let term_note = std::fs::read_to_string(dir.join(format!(".tm/terminal-tarea-{}.md", job.id)))
        .ok()
        .map(|t| {
            let v: Vec<char> = t.chars().collect();
            let tail: String = v[v.len().saturating_sub(6000)..].iter().collect();
            format!("Una persona trabajó en la terminal de esta tarea; esto es lo último que quedó anotado en .tm/terminal-tarea-{}.md (seguí desde ahí):\n{tail}\n\n", job.id)
        })
        .unwrap_or_default();
    format!(
        "{term_note}Te asignaron una tarea en el tablero de comando.\n\nProyecto: {}\nRepositorio: {}\nCarpeta de trabajo: {}\n\nTarea #{}: {}\n\n{}\n\n{}\nSi necesitás una autorización, una respuesta o que una persona haga algo, ejecutá `tm-agent ask \"tu pregunta\"` (usa TM_TASK_ID y TM_TOKEN del entorno): avisa a las personas y devuelve su respuesta por stdout. Si lo que pedís es que una persona ejecute o permita un comando, escribilo dentro de la pregunta en un bloque de código (tres comillas invertidas, un comando por línea): la app lo muestra en un recuadro para copiar y permitir con un clic.\n\nTrabajá dentro de la carpeta de trabajo. Al terminar, respondé con un resumen breve de lo que hiciste y de cualquier cosa que necesite revisión humana.",
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
    // carpeta de trabajo nueva: se crea sola (ruta absoluta y sin «..»); si no se puede, se explica por qué
    let wanted = expand_home(&job.work_dir);
    let mut why = String::new();
    if !job.work_dir.is_empty() && !wanted.is_dir() && wanted.is_absolute() && !job.work_dir.split(['/', '\\']).any(|c| c == "..") {
        match std::fs::create_dir_all(&wanted) {
            Ok(()) => api.say(job.id, &format!("📁 La carpeta de trabajo no existía: la creé en {}.", wanted.display())).await,
            Err(e) => why = format!(" No pude crearla: {e}."),
        }
    }
    let dir = [Some(expand_home(&job.work_dir)).filter(|d| !job.work_dir.is_empty() && d.is_dir()), api.cfg.fallback_dir.clone().filter(|d| d.is_dir())]
        .into_iter()
        .flatten()
        .next();
    let Some(dir) = dir else {
        api.say(job.id, &format!("⚠️ No puedo empezar: la carpeta de trabajo «{}» no existe en la máquina donde corre este agente.{why} Corregí la ruta en Configuración → Proyecto, o definí TM_AGENT_DIR.", job.work_dir)).await;
        return Some(job.updated_at.clone());
    };
    // cuotas: tope de la tarea, cuota mensual del proyecto y del equipo (las fija una persona en la app)
    if let Ok(b) = api.call(Method::POST, &format!("/api/tasks/{}/budget-check", job.id), None).await {
        if b["ok"] == false {
            api.say(job.id, &format!("⛔ No empiezo: {}.", b["reason"].as_str().unwrap_or("se agotó la cuota"))).await;
            log(&format!("⛔ #{} sin cuota", job.id));
            return Some(job.updated_at.clone());
        }
    }
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

    // Lista de comandos prohibidos que un admin edita desde la app (ficha de la IA).
    let deny = api.get("/api/me/deny").await.ok().and_then(|v| v["deny"].as_str().map(String::from)).unwrap_or_default();
    // Comandos permitidos: los permanentes de la IA («/permitir siempre») y los de «una vez» de esta tarea, que se consumen al empezar.
    publish_lists(api).await;
    let me_allow = api.get("/api/me/allow").await.ok();
    let mut allow = me_allow.as_ref().and_then(|v| v["allow"].as_str().map(String::from)).unwrap_or_default();
    // «Autorización total»: la IA puede todo salvo lo prohibido (TM_DENY siempre gana)
    let from_app = me_allow.as_ref().and_then(|v| v["from_app"].as_bool()).unwrap_or(false);
    let allow_all = me_allow.as_ref().and_then(|v| v["full"].as_bool()).unwrap_or(false);
    if let Ok(v) = api.call(Method::POST, &format!("/api/tasks/{}/grants/claim", job.id), None).await {
        for c in v["once"].as_array().into_iter().flatten().filter_map(|c| c.as_str()) {
            allow.push('\n');
            allow.push_str(c);
        }
    }
    let mut cmd = if cfg!(windows) { let mut c = Command::new("cmd"); c.arg("/C"); c } else { let mut c = Command::new("sh"); c.arg("-c"); c };
    cmd.arg(&api.cfg.cmd)
        .current_dir(&dir)
        .env("TM_TASK_PROMPT", &prompt)
        .env("TM_DENY", deny)
        .env("TM_ALLOW", allow)
        .env("TM_ALLOW_ALL", if allow_all { "1" } else { "0" })
        .env("TM_LISTS_FROM_APP", if from_app { "1" } else { "0" })
        .env("TM_TASK_ID", job.id.to_string())
        .env("TM_TASK_TITLE", &job.title)
        .env("TM_PROJECT_ID", job.project_id.to_string())
        .env("TM_URL", &api.cfg.url)
        .env("TM_TOKEN", &api.cfg.token)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let started = std::time::Instant::now();
    let out = match tokio::time::timeout(Duration::from_secs(api.cfg.timeout), async { cmd.spawn()?.wait_with_output().await }).await {
        Ok(Ok(o)) => Ok(o),
        Ok(Err(e)) => Err(format!("no pude lanzar el comando: {e}")),
        Err(_) => Err(format!("se pasó del tiempo máximo ({} s) y lo cancelé", api.cfg.timeout)),
    };
    // si el comando devuelve el JSON de Claude (`--output-format json`), de ahí salen los tokens y el costo reales
    let usage = out.as_ref().ok().and_then(|o| parse_usage(&String::from_utf8_lossy(&o.stdout)));
    // algunos agentes (opencode) devuelven 0 aunque fallen: sin nada en la salida y un «Error» en el canal de errores, es un fallo
    let silent_error = out.as_ref().is_ok_and(|o| o.stdout.iter().all(u8::is_ascii_whitespace) && String::from_utf8_lossy(&o.stderr).contains("Error"));
    let run_ok = out.as_ref().is_ok_and(|o| o.status.success()) && !usage.as_ref().is_some_and(|u| u.is_error) && !silent_error;
    let u = usage.as_ref();
    let report = json!({
        "seconds": started.elapsed().as_secs(), "ok": run_ok,
        "input_tokens": u.map_or(0, |u| u.input), "output_tokens": u.map_or(0, |u| u.output),
        "cache_create": u.map_or(0, |u| u.cache_create), "cache_read": u.map_or(0, |u| u.cache_read),
        "cost_usd": u.map_or(0.0, |u| u.cost), "model": u.map_or("", |u| u.model.as_str()),
    });
    api.call(Method::POST, &format!("/api/tasks/{}/run", job.id), Some(report)).await.ok();
    let ok_text = |o: &std::process::Output| {
        if let Some(t) = usage.as_ref().and_then(|u| u.text.clone()) {
            return t;
        }
        let so = String::from_utf8_lossy(&o.stdout);
        let se = String::from_utf8_lossy(&o.stderr);
        if so.trim().is_empty() { se.into_owned() } else { so.into_owned() }
    };
    match out {
        Ok(_) if api.get(&format!("/api/tasks/{}/grants", job.id)).await.is_ok_and(|g| g["pending"].as_array().is_some_and(|a| !a.is_empty())) => {
            api.say(job.id, "🔁 Me autorizaron comandos mientras trabajaba: vuelvo a empezar con esos permisos.").await;
            api.set_status(job.id, "todo").await.ok();
            None
        }
        Ok(_) if run_ok && wrote_while_working(api, job.id, claimed["updated_at"].as_str().unwrap_or("")).await => {
            api.say(job.id, "🔁 Mientras trabajaba escribieron comentarios nuevos: la retomo leyéndolos.").await;
            api.set_status(job.id, "todo").await.ok().and_then(|t| t["updated_at"].as_str().map(String::from))
        }
        Ok(o) if run_ok => {
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

/// Gasto de una ejecución, leído del JSON de `claude -p --output-format json`.
struct Usage {
    input: i64,
    output: i64,
    cache_create: i64,
    cache_read: i64,
    cost: f64,
    model: String,
    text: Option<String>,
    is_error: bool,
}

fn parse_usage(stdout: &str) -> Option<Usage> {
    let t = stdout.trim();
    let v: Value = serde_json::from_str(t).or_else(|_| serde_json::from_str(t.lines().last().unwrap_or(""))).ok()?;
    v.get("total_cost_usd")?;
    let mut u = Usage { input: 0, output: 0, cache_create: 0, cache_read: 0, cost: v["total_cost_usd"].as_f64().unwrap_or(0.0), model: String::new(), text: None, is_error: v["is_error"].as_bool().unwrap_or(false) };
    let n = |x: &Value| x.as_i64().unwrap_or(0);
    match v["modelUsage"].as_object().filter(|m| !m.is_empty()) {
        Some(models) => {
            u.model = models.keys().cloned().collect::<Vec<_>>().join(", ");
            for m in models.values() {
                u.input += n(&m["inputTokens"]);
                u.output += n(&m["outputTokens"]);
                u.cache_create += n(&m["cacheCreationInputTokens"]);
                u.cache_read += n(&m["cacheReadInputTokens"]);
            }
        }
        None => {
            u.input = n(&v["usage"]["input_tokens"]);
            u.output = n(&v["usage"]["output_tokens"]);
            u.cache_create = n(&v["usage"]["cache_creation_input_tokens"]);
            u.cache_read = n(&v["usage"]["cache_read_input_tokens"]);
        }
    }
    u.text = v["result"].as_str().map(String::from).or_else(|| u.is_error.then(|| format!("{} ({})", v["subtype"].as_str().unwrap_or("error"), v["terminal_reason"].as_str().unwrap_or("sin detalle"))));
    Some(u)
}

/// ¿Una persona comentó mientras yo trabajaba? (no cuentan las respuestas a pedidos, autorizaciones ni notas automáticas, que ya leí)
async fn wrote_while_working(api: &Api, task: i64, since: &str) -> bool {
    let Ok(me) = api.get("/api/me").await else { return false };
    let me = me["id"].as_i64();
    let auto = ["↪", "✅", "📤", "❌", "📝", "🙋", "🔁", "▶"];
    api.get(&format!("/api/tasks/{task}/comments")).await.ok().and_then(|c| c.as_array().cloned()).is_some_and(|a| {
        a.iter().any(|c| c["user_id"].as_i64() != me && c["created_at"].as_str().unwrap_or("") > since && !auto.iter().any(|p| c["body"].as_str().unwrap_or("").starts_with(p)))
    })
}

/// ¿El último comentario de la tarea es de otra persona (no mío)? Entonces me están esperando.
async fn waiting_for_me(api: &Api, task: i64, me: i64) -> bool {
    api.get(&format!("/api/tasks/{task}/comments")).await.ok().and_then(|c| c.as_array().and_then(|a| a.last().cloned())).is_some_and(|c| c["user_id"].as_i64() != Some(me))
}

/// Tareas para esta IA: las de «Por hacer» y, como red de seguridad, las que quedaron en «En curso» sin que yo las esté
/// corriendo: al arrancar (el proceso anterior se cortó a la mitad) o cuando alguien comentó después de mi último mensaje.
async fn pending(api: &Api, me: i64, skip: &HashMap<i64, String>, running: &HashSet<i64>, first: bool) -> Result<Vec<Job>> {
    let mut jobs = Vec::new();
    for team in api.get("/api/teams").await?.as_array().cloned().unwrap_or_default() {
        for p in api.get(&format!("/api/teams/{}/projects", team["id"])).await?.as_array().cloned().unwrap_or_default() {
            let tasks = api.get(&format!("/api/projects/{}/tasks", p["id"])).await?;
            for t in tasks.as_array().cloned().unwrap_or_default() {
                let id = t["id"].as_i64().unwrap_or(0);
                if t["assignee_id"].as_i64() != Some(me) || running.contains(&id) || t["waiting"].as_bool() == Some(true) {
                    continue;
                }
                match t["status"].as_str() {
                    Some("todo") => {}
                    Some("doing") if first || waiting_for_me(api, id, me).await => {}
                    _ => continue,
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
    // comenta con 🙋 y abre una tarea de pedido para la persona autorizada (ejecutor) que corresponda
    let sent = match api.call(Method::POST, &format!("/api/tasks/{task}/help"), Some(json!({ "body": question.trim() }))).await {
        Ok(v) => json!({ "id": v["comment_id"] }),
        Err(_) => api.call(Method::POST, &format!("/api/tasks/{task}/comments"), Some(json!({ "body": format!("🙋 {}", question.trim()) }))).await?,
    };
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
    publish_lists(&api).await;
    log(&format!("conectado como {} (id {my_id}) a {}; reviso cada {poll} s, hasta {max} en paralelo", me["username"].as_str().unwrap_or("?"), api.cfg.url));

    if std::env::var("TM_AGENT_TERMINAL").is_ok_and(|v| v == "1") && !once {
        tokio::spawn(term::serve(api.clone()));
    }
    let sem = Arc::new(Semaphore::new(max));
    let running: Arc<Mutex<HashSet<i64>>> = Default::default();
    let skip: Arc<Mutex<HashMap<i64, String>>> = Default::default();
    let mut first = true;
    loop {
        let (skip_now, running_now) = (skip.lock().unwrap().clone(), running.lock().unwrap().clone());
        let mut started = 0;
        let result = pending(&api, my_id, &skip_now, &running_now, first).await;
        if result.is_ok() {
            first = false;
        }
        match result {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilde_se_expande() {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        assert_eq!(expand_home("~/Backups"), home.join("Backups"));
        assert_eq!(expand_home("/tmp/x"), PathBuf::from("/tmp/x"));
    }
}
