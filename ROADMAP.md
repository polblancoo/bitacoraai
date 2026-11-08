# Hoja de ruta

## Visión: «App Factory» / tablero de comandos

Un CTO (o varios admins) arma varios proyectos, asigna recursos **humanos e IA** (de distintos modelos), delega cada proyecto a un **líder** y sigue todo en vivo desde un solo tablero.

### Fase 1 — Las IAs trabajan de verdad
- [ ] `tm-mcp` (stdio) + skill: mis tareas, crear/actualizar/completar, comentar, reportar, asignar a personas.
- [ ] Modo «escuchar»: el agente recibe asignaciones nuevas sin que le hablen.
- [x] **`tm-agent`** (tarea #7 del tablero, probado contra un servidor de prueba con un agente simulado; falta probarlo con Claude Code real): proceso que corre en la máquina donde está la carpeta del proyecto, vigila el servidor, detecta tareas asignadas a una IA, abre un agente (p. ej. `claude -p`) en la carpeta de trabajo, comenta y pasa la tarea a En curso y Hecho; con tope de concurrencia, tiempo máximo y aprobación humana opcional.
- [x] **Instructivo para quien instala** (`docs/AGENTES.md` y Configuración → Agentes de IA): crear la IA, instalar `tm-mcp` y `tm-agent` en la máquina del proyecto, configurar el comando del agente y dejarlo como servicio.
- [x] Cada IA lleva su **modelo** (ej. `claude:Opus`) y se ve con 🤖 en las listas.
- [x] Editar y eliminar proyectos (con confirmación por nombre) y acceso a los ajustes del equipo desde la web.

### Fase 2 — Jerarquía y ámbito
- [x] **Carpeta de trabajo y repositorio por proyecto** (los define un admin/dueño, botón «⚙ Proyecto»).
- [ ] Entregar la carpeta y el repo a las IAs por MCP para que sepan dónde operar.
- [ ] **Líder de proyecto**: el admin asigna un líder; el líder arma tareas, asigna a humanos/IAs y suma recursos dentro de su proyecto.
- [ ] **Perfil de IA**: modelo/proveedor (etiqueta), descripción y **ámbito** (en qué proyectos actúa, si puede crear tareas y subtareas, asignar a otros, invitar a otras IAs, tope de tareas abiertas).
- [ ] IAs que invitan a IAs: heredan como máximo el ámbito de quien las invita, con cupo y aprobación opcional de un humano.

### Fase 3 — Reportes de avance
- [ ] Estado de cada tarea (en curso, bloqueada, en revisión) y **reporte de progreso** con porcentaje y nota, de humanos y de IAs.
- [ ] Alertas al líder/admin: bloqueos, vencidas, sin actividad.

### Fase 4 — Vista de comando y gráficos
- [x] **Panel de portafolio** (📊 en la barra lateral): tarjetas de todos los proyectos con avance, recursos (personas/IAs), vencidas, sin asignar, estado (al día, con vencidas, sin movimiento) y última actividad, más lo último que hicieron las IAs. Pendiente: líder por proyecto, bloqueadas y gráficos de tendencia.
- [ ] Debajo del tablero de cada proyecto: **avance** (burndown/acumulado), reparto por estado y responsable, y **timeline** de hitos y actividad.

### Fase 5 — Control
- [ ] Aprobaciones humanas para acciones sensibles de las IAs, y límites de uso por IA.

## En desarrollo

### IAs como integrantes del equipo
- [x] Usuarios **bot** con token de API propio, revocable, rol lector/miembro; alta, rotación y revocación en la barra lateral de la web (varias IAs por equipo).
- [x] Una IA crea tareas y las asigna a personas; solo modifica o borra las que creó o le asignaron.
- [ ] Marcar con 🤖 a las IAs en el panel del equipo y en la TUI.
- [x] Acciones de administración (equipos, roles, invitaciones) bloqueadas para bots.
- [ ] Aprobación humana opcional para acciones sensibles de una IA.
- [ ] Herramienta «mis tareas pendientes» y modo «escuchar» para que un agente reciba asignaciones.
- [ ] Invitar por email directo desde el servidor (si hay SMTP), con `TM_PUBLIC_URL`.
- [ ] `tm-mcp`: servidor MCP (stdio) sobre la API REST: equipos, tareas (crear, editar, completar, asignar), comentarios, menciones, bandeja y actividad.
- [ ] *Skill* (`SKILL.md`) que explica a la IA cómo usar el gestor; instalable en Claude Code y compatible con otras plataformas de código vía MCP.
- [x] Guía paso a paso del primer proyecto con capturas (`docs/GUIA.md`).
- [ ] Documentar la instalación por plataforma (Claude Code, Cursor, Windsurf, Zed, Claude Desktop).

### Integración con GitHub
- [x] Conexión por Device Flow (cada usuario con su cuenta y sus permisos): Configuración → GitHub → Conectar.
- [ ] Notificaciones (PR, issues, menciones, CI fallido) en la bandeja.
- [ ] Comentar, crear/cerrar issues, aprobar o mergear PR, re-ejecutar workflows (con confirmación).
- [ ] Crear tarea desde un issue o PR; visor de archivos de solo lectura.

### Negocio y marca (anotado 2026-10-31)
- [ ] Modelo **open-core**: núcleo abierto (AGPL) + módulo cerrado con licencia firmada para funciones de empresa (revisión de trabajo de IAs, auditoría, SSO, MCP, GitHub/GitLab).
- [ ] **Marca y dominio**: reservar dominio de BitacoraAI, registrar marca, nombre del repo y de los crates (`bitacora-*`).
- [ ] **BitacoraAI Cloud**: versión alojada con actualizaciones, copias de seguridad y soporte.
- [x] Precios en Supabase (editables en el panel); **precio fundador** USD 290 de por vida para los primeros 5 clientes; diferencia a pagar al superar el plan; términos aceptados antes de pagar.

## Más adelante

### Promoción (anotado para hacer después)
- [ ] Sección destacada en el README con GIF/captura de una IA trabajando en el equipo.
- [ ] Release con notas «con onda» sobre la función de IAs + video corto.
- [ ] Publicar `tm-mcp` en crates.io y registrarlo en directorios de servidores MCP.
- [ ] Publicar el skill en repositorios/marketplaces de skills.
- [ ] Post de lanzamiento (Reddit r/rust, Hacker News «Show HN», comunidades de MCP).
- [ ] Revisar y rotar el secreto de Google compartido en el chat.

### Pendientes técnicos
- [ ] **Probar en Windows y macOS** y armar los instaladores con GitHub Actions (el servidor ya no depende de `/dev/urandom`).
- [ ] Botones de conectar Google y GitHub en la TUI.
- [ ] Probar eliminar/transferir equipo desde el navegador y los instaladores en instalación limpia.
- [ ] Instaladores para Windows y macOS.
