//! Idioma de la interfaz de terminal: el texto del código está en español y acá se pasa a inglés o portugués.
//! Idioma: el guardado en client.json o, si no hay, el de la variable LANG; si no es es/en/pt, inglés.
use std::sync::atomic::{AtomicU8, Ordering};

pub const LANGS: [(&str, &str); 3] = [("en", "English"), ("es", "Español"), ("pt", "Português")];
static CUR: AtomicU8 = AtomicU8::new(0);

pub fn code() -> &'static str { LANGS[CUR.load(Ordering::Relaxed) as usize].0 }
pub fn name() -> &'static str { LANGS[CUR.load(Ordering::Relaxed) as usize].1 }

pub fn set(code: &str) {
    if let Some(i) = LANGS.iter().position(|(c, _)| *c == code) { CUR.store(i as u8, Ordering::Relaxed); }
}

/// Pasa al idioma siguiente (en → es → pt → en).
pub fn next() { CUR.store(((CUR.load(Ordering::Relaxed) as usize + 1) % LANGS.len()) as u8, Ordering::Relaxed); }

/// Idioma inicial: el guardado o el del sistema.
pub fn init(saved: Option<&str>) {
    let sys = std::env::var("LC_ALL").ok().filter(|v| !v.is_empty()).or_else(|| std::env::var("LANG").ok()).unwrap_or_default();
    let pick = saved.map(String::from).unwrap_or_else(|| sys.chars().take(2).collect::<String>().to_lowercase());
    set(if LANGS.iter().any(|(c, _)| *c == pick) { &pick } else { "en" });
}

/// Traduce un texto en español; si no está en el diccionario lo devuelve igual.
pub fn t(s: &str) -> String {
    let i = CUR.load(Ordering::Relaxed);
    if i == 1 { return s.to_string(); }
    DICT.iter().find(|e| e.0 == s).map(|e| if i == 0 { e.1 } else { e.2 }).unwrap_or(s).to_string()
}

/// Como `t`, pero reemplaza cada llave doble por los argumentos, en orden.
pub fn tf(s: &str, args: &[&str]) -> String {
    let mut out = t(s);
    for a in args { out = out.replacen("{}", a, 1); }
    out
}

// (español, inglés, portugués)
const DICT: &[(&str, &str, &str)] = &[
    ("Por hacer", "To do", "A fazer"),
    ("En curso", "Doing", "Em andamento"),
    ("Hecho", "Done", "Feito"),
    ("Lector", "Viewer", "Leitor"),
    ("Miembro", "Member", "Membro"),
    ("Admin", "Admin", "Admin"),
    ("Dueño", "Owner", "Dono"),
    ("baja", "low", "baixa"),
    ("normal", "normal", "normal"),
    ("alta", "high", "alta"),
    ("urgente", "urgent", "urgente"),
    (" Ctrl+T ▸ m miembro · r rol · i invitar · j unirse · t nuevo equipo · e email · v verificar · s mensaje · g Telegram · n bandeja · c config · Esc cancela",
     " Ctrl+T ▸ m member · r role · i invite · j join · t new team · e email · v verify · s message · g Telegram · n inbox · c settings · Esc cancels",
     " Ctrl+T ▸ m membro · r papel · i convidar · j entrar · t nova equipe · e e-mail · v verificar · s mensagem · g Telegram · n caixa · c config · Esc cancela"),
    ("Servidor", "Server", "Servidor"),
    ("Usuario", "User", "Usuário"),
    ("Contraseña", "Password", "Senha"),
    ("Crear cuenta", "Create account", "Criar conta"),
    ("Iniciar sesión", "Sign in", "Entrar"),
    ("Enter entrar · Tab campo · F2 login/registro · Esc salir", "Enter sign in · Tab field · F2 login/register · Esc quit", "Enter entrar · Tab campo · F2 login/cadastro · Esc sair"),
    ("sin proyectos — P para crear uno", "no projects — P to create one", "sem projetos — P para criar um"),
    ("{} · equipo: {}", "{} · team: {}", "{} · equipe: {}"),
    ("n tarea · Enter detalle · H/L mover · +/- prioridad · a tomar/soltar · A asignar · C configuración · Ctrl+T equipo · ? ayuda · q salir",
     "n task · Enter detail · H/L move · +/- priority · a take/drop · A assign · C settings · Ctrl+T team · ? help · q quit",
     "n tarefa · Enter detalhe · H/L mover · +/- prioridade · a pegar/soltar · A atribuir · C configuração · Ctrl+T equipe · ? ajuda · q sair"),
    ("prioridad {} ({})", "priority {} ({})", "prioridade {} ({})"),
    ("sin asignar", "unassigned", "sem responsável"),
    ("Subtareas ({}/{})", "Subtasks ({}/{})", "Subtarefas ({}/{})"),
    ("Comentarios ({})", "Comments ({})", "Comentários ({})"),
    ("s subtarea · j/k elegir · Espacio marcar · d borrar subtarea · l etiqueta · c comentar · Esc volver",
     "s subtask · j/k select · Space toggle · d delete subtask · l label · c comment · Esc back",
     "s subtarefa · j/k escolher · Espaço marcar · d apagar subtarefa · l etiqueta · c comentar · Esc voltar"),
    ("Detalle", "Detail", "Detalhe"),
    ("Nueva tarea", "New task", "Nova tarefa"),
    ("Nuevo proyecto", "New project", "Novo projeto"),
    ("Nuevo equipo", "New team", "Nova equipe"),
    ("Agregar miembro (escribí para buscar)", "Add member (type to search)", "Adicionar membro (digite para buscar)"),
    ("Comentario", "Comment", "Comentário"),
    ("Editar título", "Edit title", "Editar título"),
    ("Nueva subtarea", "New subtask", "Nova subtarefa"),
    ("Código de invitación", "Invitation code", "Código de convite"),
    ("Cambiar rol: usuario rol (viewer | member | admin)", "Change role: user role (viewer | member | admin)", "Mudar papel: usuário papel (viewer | member | admin)"),
    ("Asignar a (Tab completa · - quita la asignación)", "Assign to (Tab completes · - clears the assignment)", "Atribuir a (Tab completa · - remove a atribuição)"),
    ("Tu email (para recibir avisos)", "Your email (to receive notices)", "Seu e-mail (para receber avisos)"),
    ("Código de 6 dígitos que llegó a tu email", "6-digit code sent to your email", "Código de 6 dígitos enviado ao seu e-mail"),
    ("Mensaje para todo el equipo (bandeja, email y Telegram)", "Message for the whole team (inbox, email and Telegram)", "Mensagem para toda a equipe (caixa, e-mail e Telegram)"),
    ("Etiqueta (existente: alterna · nueva: se crea)", "Label (existing: toggles · new: is created)", "Etiqueta (existente: alterna · nova: é criada)"),
    ("↑/↓ elegir · Tab completa · Ctrl+R rol: {} · Enter agrega", "↑/↓ select · Tab completes · Ctrl+R role: {} · Enter adds", "↑/↓ escolher · Tab completa · Ctrl+R papel: {} · Enter adiciona"),
    ("↑/↓ elegir · Tab completa · Enter asigna", "↑/↓ select · Tab completes · Enter assigns", "↑/↓ escolher · Tab completa · Enter atribui"),
    ("cambiar columna", "change column", "mudar de coluna"),
    ("mover selección", "move selection", "mover seleção"),
    ("detalle y comentarios", "detail and comments", "detalhe e comentários"),
    ("nueva tarea", "new task", "nova tarefa"),
    ("editar título", "edit title", "editar título"),
    ("borrar tarea", "delete task", "apagar tarefa"),
    ("mover tarea de estado", "move task to another status", "mover tarefa de estado"),
    ("subir / bajar prioridad", "raise / lower priority", "subir / baixar prioridade"),
    ("tomar la tarea / soltar la mía", "take the task / drop mine", "pegar a tarefa / soltar a minha"),
    ("asignar a un miembro (admin o dueño)", "assign to a member (admin or owner)", "atribuir a um membro (admin ou dono)"),
    ("proyecto siguiente / anterior", "next / previous project", "projeto seguinte / anterior"),
    ("equipo anterior / siguiente", "previous / next team", "equipe anterior / seguinte"),
    ("nuevo proyecto", "new project", "novo projeto"),
    ("nuevo equipo", "new team", "nova equipe"),
    ("agregar miembro con búsqueda (admin+)", "add member with search (admin+)", "adicionar membro com busca (admin+)"),
    ("cambiar rol de un miembro (admin+)", "change a member's role (admin+)", "mudar o papel de um membro (admin+)"),
    ("generar código de invitación (admin+)", "generate invitation code (admin+)", "gerar código de convite (admin+)"),
    ("unirse con un código", "join with a code", "entrar com um código"),
    ("agregar miembro (↑/↓ Tab Ctrl+R rol)", "add member (↑/↓ Tab Ctrl+R role)", "adicionar membro (↑/↓ Tab Ctrl+R papel)"),
    ("cambiar rol", "change role", "mudar papel"),
    ("código de invitación", "invitation code", "código de convite"),
    ("unirse con código", "join with code", "entrar com código"),
    ("cargar mi email para recibir avisos", "set my email to receive notices", "cadastrar meu e-mail para receber avisos"),
    ("verificar el código del email", "verify the email code", "verificar o código do e-mail"),
    ("bandeja: novedades y mis tareas", "inbox: news and my tasks", "caixa: novidades e minhas tarefas"),
    ("menú de configuración (email, Telegram, equipo, sesión)", "settings menu (email, Telegram, team, session)", "menu de configuração (e-mail, Telegram, equipe, sessão)"),
    ("enviar un mensaje a todo el equipo", "send a message to the whole team", "enviar uma mensagem a toda a equipe"),
    ("vincular Telegram (te da el código para el bot)", "link Telegram (gives you the code for the bot)", "vincular Telegram (dá o código para o bot)"),
    ("poner/quitar etiqueta (la crea si no existe)", "add/remove label (created if missing)", "pôr/tirar etiqueta (é criada se não existir)"),
    ("nueva subtarea", "new subtask", "nova subtarefa"),
    ("marcar subtarea (detalle)", "toggle subtask (detail)", "marcar subtarefa (detalhe)"),
    ("Espacio", "Space", "Espaço"),
    ("comentar", "comment", "comentar"),
    ("cerrar sesión en todos los dispositivos", "sign out on all devices", "sair em todos os dispositivos"),
    ("esta ayuda (escribí para buscar)", "this help (type to search)", "esta ajuda (digite para buscar)"),
    ("salir", "quit", "sair"),
    ("(detalle)", "(detail)", "(detalhe)"),
    ("buscar: ", "search: ", "buscar: "),
    ("sin coincidencias", "no matches", "sem resultados"),
    ("Atajos · Esc cierra", "Shortcuts · Esc closes", "Atalhos · Esc fecha"),
    ("Novedades ({} sin leer)", "News ({} unread)", "Novidades ({} não lidas)"),
    ("  nada nuevo", "  nothing new", "  nada de novo"),
    ("Mis tareas abiertas ({})", "My open tasks ({})", "Minhas tarefas abertas ({})"),
    ("j/k elegir · Enter ir a la tarea · Esc cerrar (marca como leído)", "j/k select · Enter go to task · Esc close (marks as read)", "j/k escolher · Enter ir à tarefa · Esc fechar (marca como lido)"),
    ("Bandeja", "Inbox", "Caixa de entrada"),
    ("{} (sin verificar)", "{} (unverified)", "{} (não verificado)"),
    ("sin cargar", "not set", "não cadastrado"),
    ("el servidor no tiene el bot", "the server has no bot", "o servidor não tem o bot"),
    ("✓ vinculado · Enter desvincula", "✓ linked · Enter unlinks", "✓ vinculado · Enter desvincula"),
    ("sin vincular · Enter vincula", "not linked · Enter links", "não vinculado · Enter vincula"),
    ("Email para avisos", "Email for notices", "E-mail para avisos"),
    ("Verificar el email", "Verify the email", "Verificar o e-mail"),
    ("código de 6 dígitos", "6-digit code", "código de 6 dígitos"),
    ("Bandeja de novedades", "News inbox", "Caixa de novidades"),
    ("{} sin leer", "{} unread", "{} não lidas"),
    ("Enviar un mensaje al equipo", "Send a message to the team", "Enviar uma mensagem à equipe"),
    ("Agregar un miembro", "Add a member", "Adicionar um membro"),
    ("con búsqueda", "with search", "com busca"),
    ("Cambiar el rol de un miembro", "Change a member's role", "Mudar o papel de um membro"),
    ("Generar código de invitación", "Generate invitation code", "Gerar código de convite"),
    ("Unirme a un equipo con código", "Join a team with a code", "Entrar numa equipe com código"),
    ("Crear un equipo nuevo", "Create a new team", "Criar uma nova equipe"),
    ("Cerrar sesión en todos los dispositivos", "Sign out on all devices", "Sair em todos os dispositivos"),
    ("cierra el TUI", "closes the TUI", "fecha o TUI"),
    ("Atajos y ayuda", "Shortcuts and help", "Atalhos e ajuda"),
    ("Idioma / Language", "Language", "Idioma"),
    ("Enter cambia", "Enter switches", "Enter muda"),
    ("Usuario: {}   ·   Equipo: {}", "User: {}   ·   Team: {}", "Usuário: {}   ·   Equipe: {}"),
    ("j/k elegir · Enter abrir · Esc cerrar", "j/k select · Enter open · Esc close", "j/k escolher · Enter abrir · Esc fechar"),
    ("Configuración", "Settings", "Configuração"),
    ("✗ ya no sos parte de ese equipo", "✗ you are no longer part of that team", "✗ você não faz mais parte dessa equipe"),
    ("ℹ {} agregado al equipo como {}", "ℹ {} added to the team as {}", "ℹ {} adicionado à equipe como {}"),
    ("no hay un miembro llamado {}", "there is no member named {}", "não há um membro chamado {}"),
    ("ℹ te enviamos un código de 6 dígitos: Ctrl+T v para confirmarlo", "ℹ we sent you a 6-digit code: Ctrl+T v to confirm it", "ℹ enviamos um código de 6 dígitos: Ctrl+T v para confirmá-lo"),
    ("ℹ mensaje enviado a {} personas", "ℹ message sent to {} people", "ℹ mensagem enviada a {} pessoas"),
    ("ℹ email verificado: vas a recibir avisos por correo", "ℹ email verified: you will receive notices by email", "ℹ e-mail verificado: você vai receber avisos por e-mail"),
    ("ℹ {} ahora es {}", "ℹ {} is now {}", "ℹ {} agora é {}"),
    ("ℹ te uniste a {}", "ℹ you joined {}", "ℹ você entrou em {}"),
    ("ℹ Telegram desvinculado", "ℹ Telegram unlinked", "ℹ Telegram desvinculado"),
    ("✗ primero creá o elegí un equipo", "✗ first create or pick a team", "✗ primeiro crie ou escolha uma equipe"),
    ("ℹ código de invitación (rol miembro, 7 días, 10 usos): {}", "ℹ invitation code (member role, 7 days, 10 uses): {}", "ℹ código de convite (papel membro, 7 dias, 10 usos): {}"),
    ("ℹ Telegram: escribile a @{} → /start {}  (un solo uso, vence en {} min)", "ℹ Telegram: message @{} → /start {}  (single use, expires in {} min)", "ℹ Telegram: escreva para @{} → /start {}  (uso único, vence em {} min)"),
    ("✗ atajo de equipo desconocido (m r i j t e v n)", "✗ unknown team shortcut (m r i j t e v n)", "✗ atalho de equipe desconhecido (m r i j t e v n)"),
    ("✗ solo admin o dueño asigna a otros (con a tomás o soltás la tuya)", "✗ only admin or owner can assign to others (with a you take or drop yours)", "✗ só admin ou dono atribui a outros (com a você pega ou solta a sua)"),
    ("rol inválido: usá viewer, member o admin", "invalid role: use viewer, member or admin", "papel inválido: use viewer, member ou admin"),
];
