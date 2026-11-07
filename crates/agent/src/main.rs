//! tm-agent: vigila el servidor y, cuando le asignan una tarea a esta IA, abre un agente en la carpeta de
//! trabajo del proyecto, comenta y pasa la tarea a «En curso» y «Hecho».
use anyhow::{bail, Context, Result};
use reqwest::Method;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{process::Command, sync::Semaphore};

mod gitsetup;
mod iso;
mod term;

const HELP: &str = "tm-agent: pone a trabajar a una IA cuando le asignan una tarea en CrewHelm.

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
  TM_SSH_KEYS=1        con TM_WORKSPACE: las claves SSH que cada persona carga en su perfil entran a esta cuenta, cada una a su espacio (miembros en sandbox)
  TM_AGENT_ONCE=1   procesa lo pendiente y sale

El comando corre en la carpeta de trabajo del proyecto con TM_TASK_PROMPT, TM_TASK_ID, TM_TASK_TITLE,
TM_PROJECT_ID, TM_URL y TM_TOKEN en el entorno. Si termina bien, la tarea pasa a Hecho; si falla,
vuelve a Por hacer con un comentario y, mientras el agente siga corriendo, no se reintenta
hasta que alguien la modifique.

Flujo de tareas: una IA revisora cierra su etapa con `tm-agent verdict ok \"comentario\"` o `tm-agent verdict return \"qué corregir\"`.
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

/// Carpeta por defecto de un proyecto sin ruta: ~/proyectos/<nombre en minúsculas y con guiones>.
fn default_dir(project: &str) -> PathBuf {
    let slug: String = project.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect::<String>().split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
    expand_home("~/proyectos").join(if slug.is_empty() { "proyecto" } else { &slug })
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
    parent: Option<i64>,
}

async fn prompt_for(api: &Api, job: &Job, dir: &std::path::Path, mine: &std::path::Path, branch: Option<&str>) -> String {
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
    let pr_note = branch
        .map(|b| format!("Trabajás en una copia aislada de esta tarea, en la rama `{b}`. No hagas `git commit` ni `git push`: cuando termines, el sistema guarda tus cambios en esa rama y la sube; la persona abre el Pull Request desde la tarea. Si no cambiaste archivos del repositorio, no se crea nada.\n\n"))
        .unwrap_or_default();
    format!(
        "{term_note}{pr_note}Te asignaron una tarea en el tablero de comando.\n\nProyecto: {}\nRepositorio: {}\nCarpeta de trabajo: {}\n\nTarea #{}: {}\n\n{}\n\n{}\nSi necesitás una autorización, una respuesta o que una persona haga algo, ejecutá `tm-agent ask \"tu pregunta\"` (usa TM_TASK_ID y TM_TOKEN del entorno): avisa a las personas y devuelve su respuesta por stdout. Si lo que pedís es que una persona ejecute o permita un comando, escribilo dentro de la pregunta en un bloque de código (tres comillas invertidas, un comando por línea): la app lo muestra en un recuadro para copiar y permitir con un clic.\n\nTrabajá dentro de la carpeta de trabajo. Los archivos que generes y que no sean parte del código del repositorio (informes, presentaciones, datos, borradores) guardalos en tu carpeta de esta tarea: {}. Lo que quede ahí es lo que se entrega a la etapa siguiente, y al aprobarse la última etapa se copia a la carpeta final de la tarea. Al terminar, respondé con un resumen breve de lo que hiciste y de cualquier cosa que necesite revisión humana.",
        job.project,
        if job.repo.is_empty() { "(sin definir)" } else { &job.repo },
        dir.display(),
        job.id,
        job.title,
        if job.description.is_empty() { "(sin descripción)" } else { &job.description },
        if notes.is_empty() { String::new() } else { format!("Comentarios recientes:\n{}\n", notes.join("\n")) },
        mine.display(),
    )
}

/// Devuelve el `updated_at` de la tarea tras el último cambio, para no reintentarla sin que nadie la toque.
async fn run(api: &Api, job: &Job) -> Option<String> {
    // carpeta de trabajo: si el proyecto no tiene una (y no hay TM_AGENT_DIR) se usa ~/proyectos/<nombre del proyecto>;
    // si no existe se crea sola (ruta absoluta y sin «..»); si no se puede, se explica por qué
    let own = !job.work_dir.is_empty();
    let wanted = if own || api.cfg.fallback_dir.is_some() { expand_home(&job.work_dir) } else { default_dir(&job.project) };
    let mut why = String::new();
    if (own || api.cfg.fallback_dir.is_none()) && !wanted.is_dir() && wanted.is_absolute() && !wanted.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        match std::fs::create_dir_all(&wanted) {
            Ok(()) => api.say(job.id, &format!("📁 {} Creé la carpeta {}.", if own { "La carpeta de trabajo no existía." } else { "El proyecto no tenía carpeta de trabajo; usé una por defecto." }, wanted.display())).await,
            Err(e) => why = format!(" No pude crearla: {e}."),
        }
    }
    let dir = [Some(wanted.clone()).filter(|d| (own || api.cfg.fallback_dir.is_none()) && d.is_dir()), api.cfg.fallback_dir.clone().filter(|d| d.is_dir())]
        .into_iter()
        .flatten()
        .next();
    let Some(dir) = dir else {
        api.say(job.id, &format!("⚠️ No puedo empezar: la carpeta de trabajo «{}» no existe en la máquina donde corre este agente.{why} Corregí la ruta en Configuración → Proyecto, o definí TM_AGENT_DIR.", wanted.display())).await;
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
    // aislamiento de esta IA (ficha en la app), destinos de red permitidos y si el proyecto abre Pull Requests
    let me_allow = api.get("/api/me/allow").await.ok();
    let ai_pr = !job.repo.is_empty() && api.get(&format!("/api/projects/{}/github/settings", job.project_id)).await.ok().and_then(|v| v["ai_pr"].as_bool()).unwrap_or(false);
    let iso = iso::Settings::resolve(
        me_allow.as_ref().and_then(|v| v["isolation"].as_str()).unwrap_or(""),
        ai_pr,
        me_allow.as_ref().and_then(|v| v["net_allow"].as_str()).unwrap_or(""),
    );
    let me_name = api.get("/api/me").await.ok().and_then(|v| v["username"].as_str().map(String::from)).unwrap_or_else(|| "ia".into());
    // si el aislamiento está pedido y no se puede cumplir, la tarea no corre sin él (falla cerrado)
    let claimed_at = claimed["updated_at"].as_str().map(String::from);
    let fail = |why: String| refuse(api, job.id, claimed_at.clone(), why);
    if iso.sandbox && !iso::bwrap_ok().await {
        return fail("el aislamiento de esta IA está activado pero `bwrap` no está instalado o no funciona en esta máquina (instalá bubblewrap, o quitá el aislamiento en la ficha de la IA).".into()).await;
    }
    // copia de trabajo propia de la tarea, en su rama (si la carpeta del proyecto es un repositorio)
    let work = if iso.copy {
        match iso::prepare(&dir, iso.workspace.as_deref(), &job.project, job.id, job.parent).await {
            Ok(w) => w,
            Err(e) => return fail(e).await,
        }
    } else {
        iso::Work { dir: dir.clone(), origin: dir.clone(), branch: None, base: None }
    };
    log(&format!("▶ #{} «{}» en {}", job.id, job.title, work.dir.display()));
    let aislada = work.branch.as_ref().map(|b| format!(" (copia aislada, rama `{b}`)")).unwrap_or_default();
    api.say(job.id, &format!("▶ Empiezo con esta tarea. Trabajo en `{}`{aislada}{}.", work.dir.display(), if iso.sandbox { " dentro de un sandbox" } else { "" })).await;
    // espacio propio de esta tarea para lo intermedio (y final/, que llena el servidor al aprobarse el flujo)
    let mine = iso::mine_path(iso.workspace.as_deref(), &dir, &job.project, &me_name, job.id);
    let _ = std::fs::create_dir_all(&mine);
    let _ = std::fs::create_dir_all(dir.join(".crewhelm/_tareas").join(job.id.to_string()).join("final"));
    let _ = std::fs::write(dir.join(".crewhelm/.gitignore"), "*\n");
    let prompt = prompt_for(api, job, &work.dir, &mine, work.branch.as_deref()).await;

    // Lista de comandos prohibidos que un admin edita desde la app (ficha de la IA).
    let deny = api.get("/api/me/deny").await.ok().and_then(|v| v["deny"].as_str().map(String::from)).unwrap_or_default();
    // Comandos permitidos: los permanentes de la IA («/permitir siempre») y los de «una vez» de esta tarea, que se consumen al empezar.
    publish_lists(api).await;
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
    let envs: Vec<(&str, String)> = vec![
        ("TM_TASK_PROMPT", prompt),
        ("TM_DENY", deny),
        ("TM_ALLOW", allow),
        ("TM_ALLOW_ALL", if allow_all { "1" } else { "0" }.into()),
        ("TM_LISTS_FROM_APP", if from_app { "1" } else { "0" }.into()),
        ("TM_TASK_ID", job.id.to_string()),
        ("TM_TASK_DIR", mine.to_string_lossy().into()),
        ("TM_TASK_TITLE", job.title.clone()),
        ("TM_PROJECT_ID", job.project_id.to_string()),
        ("TM_URL", api.cfg.url.clone()),
        ("TM_TOKEN", api.cfg.token.clone()),
    ];
    // fase 3: proxy filtrado y puente al servidor, fuera de la cápsula
    let net = if iso.sandbox && iso.net != iso::Net::Open {
        match iso::start_net(job.id, iso.net, iso.allow_hosts.clone(), server_addr(&api.cfg.url)) {
            Ok(n) => Some(n),
            Err(e) => return fail(format!("no pude preparar la red del sandbox: {e}")).await,
        }
    } else {
        None
    };
    let mut cmd = if iso.sandbox {
        let real_home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/root"));
        let ia_home = iso.workspace.as_ref().map(|w| w.join("homeCrewHelm").join(iso::slug(&me_name)));
        if let Some(h) = &ia_home {
            let _ = std::fs::create_dir_all(h);
        }
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("tm-agent"));
        // el script que lanza a la IA (y sus listas) tiene que verse adentro
        let mut ro = iso.extra_ro.clone();
        ro.extend(lists_dir(&api.cfg.cmd));
        let mut args = iso::bwrap_args(&real_home, &iso::Plan { work: &work.dir, mine: &mine, ia_home: ia_home.as_deref(), exe: &exe, net: iso.net, sock_dir: net.as_ref().map(|n| n.dir.as_path()), extra_rw: &iso.extra_rw, extra_ro: &ro, new_session: true });
        args.push("--".into());
        if let Some(n) = &net {
            args.extend([exe.to_string_lossy().to_string(), "__netbridge".into(), n.dir.to_string_lossy().to_string(), if iso.net == iso::Net::Proxy { "proxy" } else { "none" }.into(), "--".into()]);
        }
        args.extend(["sh".into(), "-c".into(), api.cfg.cmd.clone()]);
        let mut c = Command::new("bwrap");
        c.args(args).env_clear();
        // adentro solo entra lo necesario: nada de variables del servidor
        for (k, v) in std::env::vars() {
            if ["PATH", "LANG", "TERM", "USER", "LOGNAME"].contains(&k.as_str()) || ["LC_", "TM_", "ANTHROPIC_", "CLAUDE_", "OPENAI_", "GEMINI_", "GOOGLE_API", "OPENROUTER_"].iter().any(|p| k.starts_with(p)) {
                c.env(k, v);
            }
        }
        c
    } else if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&api.cfg.cmd);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(&api.cfg.cmd);
        c
    };
    for (k, v) in &envs {
        cmd.env(k, v);
    }
    // sin sandbox, la carpeta de trabajo es la de la copia (o la del proyecto); con sandbox la fija bwrap
    cmd.current_dir(if iso.sandbox { Path::new("/") } else { work.dir.as_path() })
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
    // fase 1: lo que dejó la IA se guarda en la rama de la tarea y se sube desde acá, fuera del sandbox
    if run_ok {
        if let Some(o) = iso::finish(&work, &me_name, job.id, &job.title).await {
            api.say(job.id, &iso::outcome_text(&o)).await;
        }
    }
    if let Some(t) = net.as_ref().and_then(|n| iso::blocked_text(&n.blocked)) {
        api.say(job.id, &t).await;
    }
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
            api.set_status(job.id, "todo").await.ok();
            // None: no se saltea, se vuelve a tomar en la próxima vuelta (con los comentarios nuevos)
            None
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

/// No se pudo empezar (aislamiento que no se puede cumplir, copia que no se arma…): se avisa y la tarea vuelve a «Por hacer».
async fn refuse(api: &Api, task: i64, claimed_at: Option<String>, why: String) -> Option<String> {
    api.say(task, &format!("⚠️ No empiezo: {why}")).await;
    api.set_status(task, "todo").await.ok().and_then(|t| t["updated_at"].as_str().map(String::from)).or(claimed_at)
}

/// `host:puerto` de la dirección del servidor (para el puente del sandbox).
fn server_addr(url: &str) -> String {
    let https = url.starts_with("https://");
    let rest = url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or("");
    if rest.contains(':') { rest.to_string() } else { format!("{rest}:{}", if https { 443 } else { 80 }) }
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
    let auto = ["↪", "✅", "📤", "❌", "📝", "🙋", "🔁", "▶", "↩", "🧭", "⛔"];
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
                    parent: t["parent_id"].as_i64(),
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

/// `tm-agent verdict ok|return «texto»`: el veredicto de una IA revisora en el flujo de la tarea actual (aprueba y sigue, o devuelve a la etapa de trabajo con el motivo).
async fn verdict(args: Vec<String>) -> Result<()> {
    let ok = match args.first().map(String::as_str) {
        Some("ok") => true,
        Some("return") => false,
        _ => bail!("uso: tm-agent verdict ok \"comentario opcional\"  |  tm-agent verdict return \"qué hay que corregir\""),
    };
    let reason = args[1..].join(" ");
    if !ok && reason.trim().is_empty() {
        bail!("para devolver hay que explicar qué corregir: tm-agent verdict return \"qué hay que corregir\"");
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
    let api = Api { http: reqwest::Client::new(), cfg };
    api.call(Method::POST, &format!("/api/tasks/{task}/flow/verdict"), Some(json!({ "ok": ok, "reason": reason.trim() }))).await?;
    println!("{}", if ok { "Aprobado: la tarea pasó a la etapa siguiente." } else { "Devuelta: la tarea volvió a la etapa de trabajo con tu motivo." });
    Ok(())
}

/// `tm-agent sandbox-shell [carpeta]`: una shell dentro del sandbox, con el espacio propio de esta IA como HOME y la red abierta.
/// Sirve para iniciar sesión del CLI de la IA (por ejemplo `claude`) una sola vez en su espacio.
async fn sandbox_shell(dir: Option<String>) -> Result<()> {
    let ws = std::env::var("TM_WORKSPACE").ok().filter(|w| !w.trim().is_empty()).map(|w| iso::expand_home(w.trim())).context("falta TM_WORKSPACE (la raíz del modo servidor)")?;
    let name = std::env::var("TM_IA").ok().filter(|n| !n.is_empty()).context("falta TM_IA (el nombre de la IA cuyo espacio querés abrir)")?;
    if !iso::bwrap_ok().await {
        bail!("`bwrap` no está instalado o no funciona en esta máquina");
    }
    let home = ws.join("homeCrewHelm").join(iso::slug(&name));
    std::fs::create_dir_all(&home)?;
    let work = dir.map(PathBuf::from).unwrap_or_else(|| home.clone());
    let real_home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/root"));
    let exe = std::env::current_exe()?;
    let ro = iso::paths_for_shell();
    let mut args = iso::bwrap_args(&real_home, &iso::Plan { work: &work, mine: &home, ia_home: Some(&home), exe: &exe, net: iso::Net::Open, sock_dir: None, extra_rw: &[], extra_ro: &ro, new_session: false });
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".into());
    args.extend(["--".into(), shell]);
    println!("[tm-agent] sandbox de {name}: HOME = {} (sesión del CLI, memoria y notas de esta IA). Salí con `exit`.", home.display());
    let st = Command::new("bwrap").args(args).env("PATH", std::env::var("PATH").unwrap_or_default()).status().await?;
    std::process::exit(st.code().unwrap_or(1));
}

/// Limpieza: copias de trabajo y espacios de tareas terminadas hace 30 días o más. A los 23 días se avisa en la tarea; si pasan 7 días
/// desde el aviso se borra. Un archivo `.keep` dentro de la carpeta (o reabrir la tarea) la conserva. Lo ya subido a la rama no se pierde.
async fn sweep(api: &Api, me: &str) {
    let ws = std::env::var("TM_WORKSPACE").ok().filter(|w| !w.trim().is_empty()).map(|w| iso::expand_home(w.trim()));
    for team in api.get("/api/teams").await.ok().and_then(|t| t.as_array().cloned()).unwrap_or_default() {
        for p in api.get(&format!("/api/teams/{}/projects", team["id"])).await.ok().and_then(|t| t.as_array().cloned()).unwrap_or_default() {
            let tasks = api.get(&format!("/api/projects/{}/tasks", p["id"])).await.ok().and_then(|t| t.as_array().cloned()).unwrap_or_default();
            let (pname, wd) = (p["name"].as_str().unwrap_or(""), iso::expand_home(p["work_dir"].as_str().unwrap_or("")));
            let mut roots = vec![iso::work_path(ws.as_deref(), &wd, pname, 0).parent().map(PathBuf::from).unwrap_or_default(), iso::mine_path(ws.as_deref(), &wd, pname, me, 0).parent().map(PathBuf::from).unwrap_or_default()];
            roots.dedup();
            // modo servidor: también los espacios de las personas y de las demás IAs (homeCrewHelm/<quien>/<proyecto>/<tarea>)
            if let Some(w) = ws.as_deref() {
                if let Ok(rd) = std::fs::read_dir(w.join("homeCrewHelm")) {
                    for who in rd.flatten() {
                        let r = who.path().join(iso::slug(pname));
                        if r.is_dir() && !roots.contains(&r) {
                            roots.push(r);
                        }
                    }
                }
            }
            for root in roots {
                let Ok(rd) = std::fs::read_dir(&root) else { continue };
                for e in rd.flatten() {
                    let path = e.path();
                    let name = e.file_name().to_string_lossy().to_string();
                    let Some(id) = name.trim_start_matches("tarea-").parse::<i64>().ok().filter(|_| path.is_dir()) else { continue };
                    let Some(t) = tasks.iter().find(|t| t["id"].as_i64() == Some(id)) else { continue };
                    let aviso = root.join(format!("{name}.aviso"));
                    if t["status"].as_str() != Some("done") && t["archived"].as_bool() != Some(true) {
                        let _ = std::fs::remove_file(&aviso);
                        continue;
                    }
                    if path.join(".keep").exists() {
                        continue;
                    }
                    let age = iso::days_since(t["updated_at"].as_str().unwrap_or("")).unwrap_or(0);
                    let notice_age = std::fs::metadata(&aviso).and_then(|m| m.modified()).ok().and_then(|m| m.elapsed().ok()).map(|d| (d.as_secs() / 86400) as i64);
                    match notice_age {
                        None if age >= 23 => {
                            let _ = std::fs::write(&aviso, "aviso de borrado");
                            api.say(id, &format!("🧹 Los archivos intermedios de esta tarea (`{}`) se borrarán en 7 días porque la tarea está terminada. Lo que quedó en la rama `tm/tarea-{id}` no se pierde. Para conservarlos, reabrí la tarea o creá un archivo `.keep` en esa carpeta.", path.display())).await;
                        }
                        Some(n) if n >= 7 && age >= 30 => {
                            if std::fs::remove_dir_all(&path).is_ok() {
                                let _ = std::fs::remove_file(&aviso);
                                api.say(id, &format!("🧹 Borré los archivos intermedios de esta tarea (`{}`), como se avisó.", path.display())).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
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
    if std::env::args().nth(1).as_deref() == Some("__netbridge") {
        std::process::exit(iso::netbridge(std::env::args().skip(2).collect()).await);
    }
    if std::env::args().nth(1).as_deref() == Some("ssh-session") {
        let mut it = std::env::args().skip(2);
        let (who, level, root) = (it.next().unwrap_or_default(), it.next().unwrap_or_default(), it.next().unwrap_or_default());
        if let Err(e) = term::ssh_session(&who, &level, &root).await {
            eprintln!("CrewHelm: {e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("sandbox-shell") {
        return sandbox_shell(std::env::args().nth(2)).await;
    }
    if std::env::args().nth(1).as_deref() == Some("verdict") {
        return verdict(std::env::args().skip(2).collect()).await;
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

    // la terminal y la entrada por SSH se activan desde la app (Conexión del proyecto); las variables TM_AGENT_TERMINAL y TM_SSH_KEYS siguen valiendo
    if !once {
        tokio::spawn(term::serve(api.clone()));
        tokio::spawn(gitsetup::serve(api.clone()));
        tokio::spawn(term::sync_keys(api.clone()));
    }
    let sem = Arc::new(Semaphore::new(max));
    let running: Arc<Mutex<HashSet<i64>>> = Default::default();
    let skip: Arc<Mutex<HashMap<i64, String>>> = Default::default();
    let mut first = true;
    let me_name = me["username"].as_str().unwrap_or("ia").to_string();
    let mut last_sweep = std::time::Instant::now() - Duration::from_secs(7 * 3600);
    loop {
        if !once && last_sweep.elapsed() > Duration::from_secs(6 * 3600) {
            last_sweep = std::time::Instant::now();
            sweep(&api, &me_name).await;
        }
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
