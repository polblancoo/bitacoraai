<p align="center"><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/logo/logo-512.png" width="140" alt="Logo de Rust TUI Manager"></p>

<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/README.md">English</a> · <b>Español</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/README.pt.md">Português</a></p>

# Rust TUI Manager

## 🎛️ Tablero de comando para tus proyectos

Dirigí **varios proyectos a la vez desde un solo lugar**, con **personas y agentes de IA trabajando en el mismo tablero**. Un CTO, una gerente de operaciones o una dueña de agencia arma los proyectos, asigna recursos (humanos e IAs de distintos modelos), delega cada uno en un líder y **ve cómo va todo, momento a momento**.

![Tablero de comando: proyectos, IAs y tareas del equipo](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

### Qué lo hace distinto

Lo que no vimos junto en ninguna otra herramienta (comprobalo, y avisanos si encontrás una):

- 🤖 **IAs como integrantes del equipo**, con rol, lista de comandos prohibidos y permisos que una persona da desde la propia tarea (`/permitir cargo build`).
- 🙋 **Pedidos de ayuda de la IA** que llegan como tarea a quien corresponde.
- 💻 **Terminal SSH dentro de la tarea** (con clave), y lo hecho queda escrito para que la IA retome desde ahí. *(0.6.3)*
- 📊 **Portafolio de varios proyectos**, con costo y horas de la IA. *(costo y horas: llegan en la 0.6.3)*
- 🏠 **Autoalojado**: tus datos y tus claves quedan en tu servidor.

### Para quién es

- 🏢 **Una empresa con IA propia y trabajo colaborativo.** Tu gente y tus IAs comparten un mismo tablero. El tablero, sus datos, el historial y tus claves quedan en **tu** servidor, y la IA corre en **tus** máquinas; solo lo que el modelo necesita leer viaja al proveedor que elegiste. Cada IA tiene un rol, listas de comandos permitidos y prohibidos, y un historial de quién hizo qué.
- 🌍 **Una IA que usa todo el equipo: contratada, o un modelo propio en el servidor.** O bien **contratás un modelo (Claude, Gemini, Codex…)** y lo instalás **una sola vez en el servidor** con `tm-agent`, o bien **corrés un modelo abierto en tu propio servidor** (por ejemplo con Ollama o llama.cpp, mediante cualquier herramienta que reciba un pedido y trabaje sobre archivos; `tm-agent` lanza el comando que le des, aunque todavía no probamos modelos locales). Todos en el equipo la usan con solo asignarle tareas, y nadie necesita cuenta propia. Con un modelo contratado le pagás una vez al proveedor (tu suscripción o tu clave de API); con un modelo propio **nada sale de tu infraestructura**. **Vos tenés el control:** `TM_AGENT_MAX` limita cuántas tareas corre a la vez y el resto espera en fila; cada IA tiene su rol y sus listas de comandos permitidos y prohibidos; pide permiso cuando lo necesita y cualquier owner o admin se lo da desde la tarea. Podés mezclar varios modelos y cambiarlos cuando quieras, porque el tablero no depende de ningún proveedor. Vienen los tokens y las horas por proyecto, para ver cuánto le cuesta a cada proyecto la IA compartida. También sirve con una IA que otro maneja en su propio servidor: se conecta por HTTPS con un token que **revocás con un clic**, limitada por rol; *aclaración honesta:* las listas de comandos las aplica el agente donde corre la IA, así que en máquinas que no controlás combinalas con el rol y con la revisión de cada tarea.
- 👥 **Varias personas supervisando varias IAs.** Los pedidos de ayuda llegan a quien corresponde, cualquiera con permiso autoriza un comando desde la propia tarea, y todo queda escrito.

### Más ventajas

- 📦 **Un solo binario + SQLite**: se instala en minutos y se respalda copiando un archivo. Sin licencias por usuario.
- 🔔 Novedades en tiempo real y avisos por web, Telegram y correo. Tres idiomas, ayudas en cada botón y un manual dentro de la app.
- 💻 **Terminal en el navegador** *(0.6.3)*, solo para owners. Dos caminos: **por el agente** (`TM_AGENT_TERMINAL=1`: la conexión sale desde la máquina del proyecto, así que no hay claves ni puertos abiertos y anda detrás de NAT; Windows todavía sin probar) o **por SSH con clave** (con un botón que instala la clave usando tu contraseña una sola vez, sin guardarla). Cada sesión queda anotada en la tarea y en `.tm/terminal-*.md` dentro de la carpeta del proyecto, para que la IA retome donde dejaste.
- 🖥️ **Vista Terminales** *(0.6.3)*: hasta 8 paneles a la vez, y al lado el estado de cada IA (trabajando, te necesita, terminó).
- 📊 Tokens, costo y horas por proyecto y por IA, con **cuotas** por equipo, por proyecto y por tarea, y una estimación al crear la tarea.

**Un proyecto puede ser lo que necesites:**

| Ejemplo | Cómo se ve |
|---|---|
| 🏭 **Fábrica de apps** | Una app por proyecto; las IAs escriben código y tests, las personas revisan y publican. |
| 📊 **Planilla o presupuesto** | Cada área es una tarea; una IA arma las hojas, finanzas aprueba. |
| 🧭 **Procesos de la empresa** | Onboarding, compras, auditorías, campañas: pasos con responsable y fecha. |
| 🔬 **Investigación o contenidos** | Agentes que recopilan y redactan, un editor que decide. |

Cada proyecto sabe **dónde vive su trabajo** (carpeta y repositorio); el líder por proyecto está en la [hoja de ruta](ROADMAP.md). Todo es **self-hosted**: tus datos quedan en tu servidor.

> 👉 **¿Primera vez? Seguí la [guía paso a paso: tu primer proyecto](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.md)** (instalar, crear el equipo, sumar personas e IAs, asignar y seguir el avance, con capturas).

**Cómo está hecho:** gestor de tareas colaborativo, multiusuario y self-hosted, escrito en Rust. Un único servidor y tres clientes: una **TUI** (ratatui), una **app gráfica** (Tauri 2) y una **web** que el propio servidor sirve, con actualizaciones en tiempo real entre todos los conectados.

| Pieza | Crate / ruta | Qué es |
|---|---|---|
| Servidor | [`rust-tui-manager-server`](crates/server) (binario `tm-server`) | API REST + WebSocket sobre axum y SQLite |
| Cliente de terminal | [`rust-tui-manager`](crates/tui) | Tablero kanban con ratatui |
| Agente de IA | [`rust-tui-manager-agent`](crates/agent) (binario `tm-agent`) | Vigila el tablero y pone a trabajar a una IA cuando le asignan una tarea |
| Cliente gráfico | [`apps/desktop`](apps/desktop) | App Tauri 2 (la misma interfaz que la web, en ventana propia) |
| Cliente web | [`crates/server/web`](crates/server/web) | Interfaz gráfica servida por `tm-server` en `/`: tablero, calendario, Gantt, pizarra, bandeja |
| Modelos compartidos | [`rust-tui-manager-core`](crates/core) | Tipos y eventos usados por todos |

> La versión 0.1.x de `rust-tui-manager` era un gestor local de notas y snippets. Ese código sigue en [`legacy/`](legacy/) y en las versiones 0.1.x del crate.

![Tablero kanban en el cliente gráfico](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-board.png)

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-task.png" alt="Detalle de tarea con etiquetas, subtareas y comentarios"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-calendar.png" alt="Vista de calendario por fecha límite"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-team.png" alt="Gestión del equipo: roles e invitaciones por código"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/tui-board.png" alt="Cliente de terminal (ratatui)"></td>
  </tr>
</table>

### Cliente web (también en la app de escritorio)

`tm-server` sirve la interfaz en `/`: abrí `http://tu-servidor:7878` y listo.

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-gantt.png" alt="Gantt con dependencias entre tareas"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-canvas.png" alt="Pizarra colaborativa con notas, tarjetas de tarea y conexiones"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-inbox.png" alt="Bandeja de novedades al iniciar sesión"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-mindmap.png" alt="Mapa mental de una tarea"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-settings.png" alt="Menú de configuración: cuenta, Telegram, equipo y preferencias"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-message.png" alt="Mensaje al equipo (bandeja, email y Telegram)"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team.png" alt="Equipo: buscar usuarios registrados, generar código de invitación y miembros"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-activity.png" alt="Actividad del proyecto"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Código de invitación generado"></td>
  </tr>
</table>

## 🤖 Tus agentes de IA, como integrantes del equipo

Los asistentes de código (Claude Code, Cursor, Windsurf, Zed o cualquier cliente [MCP](https://modelcontextprotocol.io)) **se conectan al mismo tablero que tu equipo humano**: cada IA tiene su propio usuario, su token y su rol, y trabaja como un integrante más.

- **Sumá todas las IAs que quieras**: en la barra lateral, sección **🤖 IAs → «+»**. Se crea el usuario, se suma al equipo y te muestra **el token una sola vez** con la configuración MCP lista para copiar. Cada IA lleva su **modelo** (p. ej. `claude:Opus`) y se ve con 🤖 en todas las listas de usuarios.
- **Se coordinan con las personas**: una IA puede **crear una tarea y asignársela a un humano** del equipo, comentar, completarla o borrarla cuando es suya, y recibir las que le asignes. Todo queda a su nombre en la actividad.
- **Control humano**: el token se **rota o revoca con un clic**; el rol es lector o miembro; una IA **no puede** crear ni borrar equipos, invitar, cambiar roles ni tocar tareas ajenas (salvo comentar).
- **Un tablero de comandos**: asignás trabajo a tus agentes desde la web, la app o la terminal, y ves en vivo qué hace cada uno.

> 🚀 **Que trabajen solas**: **`tm-agent`** vigila el tablero y, cuando le asignás una tarea a una IA, abre tu agente (Claude Code, Gemini CLI…) en la carpeta del proyecto, la pasa a *En curso*, la comenta y la deja en *Hecho*. El agente también puede levantar una **alarma de asistencia** 🙋 (`tm-agent ask "..."`) y esperar la respuesta de una persona. Paso a paso en el **[instructivo de agentes](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.md)**; dentro de la app está en *Configuración → Agentes de IA*. El binario `tm-mcp` (herramientas MCP) sigue en desarrollo (ver [ROADMAP.md](ROADMAP.md)).

## Novedades de la 0.7.1

- **Se llama CrewHelm.** Nuevo nombre en la interfaz, los correos y el manifiesto.
- **Instalador de una línea** (`install.sh`, Linux x86_64, sin sudo, con verificación SHA256) y **guía de inicio** con dibujos la primera vez que entrás.
- **Pedidos de una IA**: lista de comandos con ✕ para quitar, un solo «✅ Permitir siempre», acceso a la lista de la IA; la IA escribe los comandos en bloque.
- **Terminal de tarea** a pantalla completa con herdr por defecto.
- **Novedades** ampliables que abren su tarea; **Portafolio** clicable que abre la conversación de la IA; **Configuración en solapas**.
- **Zoom A−/A+** en todas las pantallas y **confirmaciones propias** (adiós a los avisos del navegador).
- Arreglo: la página quedaba en negro al recargar con sesión guardada.

## Novedades de la 0.7.0

### 🏢 Licencia comercial y aviso de instalación
Rust TUI Manager sigue siendo **AGPL-3.0-only y gratis**. Si lo usás en una empresa y no querés cumplir la AGPL (por ejemplo, no publicar tus cambios), hay una **licencia comercial anual**: **Configuración → 🏢 Licencia**. Pagás en **USDT por Solana o Ethereum** con un QR; la app te muestra el monto exacto (los últimos decimales identifican tu pago), la dirección y el **contrato del token** con enlace al explorador. Cuando la red confirma el pago, te llega un mail con tu código y la licencia se activa sola.

- **Cartel suave**: cuando el servidor pasa de 3 personas o 2 IAs (o si declarás que es de empresa) aparece una barra arriba. Se puede descartar 7 días y **nunca bloquea nada**.
- **Aviso de instalación (telemetría)**: una vez por día el servidor avisa que existe. Envía un código al azar guardado en tu base, la versión, el sistema operativo y **cuántas** personas, IAs, proyectos y equipos hay. **Nunca** nombres, correos, tareas ni contenido; la IP no se guarda (solo se usa para saber el país). Se apaga con `TM_TELEMETRY=0` o desde la ventana Licencia.

## Novedades de la 0.6.3

### 💻 Terminal en el navegador y vista Terminales
Un botón **💻 Terminal** en cada tarea (solo owners) abre una consola en la carpeta del proyecto. Dos caminos: **por el agente** (`TM_AGENT_TERMINAL=1`, la conexión sale desde la máquina del proyecto: sin claves ni puertos abiertos) o **SSH con clave** (la app genera la clave y guarda la huella del servidor). Cada sesión deja una nota `.tm/terminal-tarea-N.md` que la IA lee al retomar. La vista **🖥️ Terminales** muestra hasta 8 paneles y el estado de cada IA.

![Vista Terminales](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-14-terminales.png)

### 💰 Consumo y cuotas
Tokens, costo y horas por tarea, proyecto, equipo e IA, con **cuotas** que se revisan antes de cada ejecución y una **estimación** de tokens (🧮) para fijar el tope de la tarea.

![Consumo](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-15-consumo.png)
![Tope de tokens en la tarea](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-17-tokens.png)

### ✅ Permitir comandos desde la tarea
Si una IA pide un comando, el botón **✅ Permitir comando…** explica qué hace y lo autoriza **una vez** o **siempre** (también con `/permitir` en un comentario).

![Permitir comando](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-16-permitir.png)

### Licencia
Desde la 0.6.3 el proyecto es **AGPL-3.0-only** (las versiones anteriores siguen bajo MIT). Incluye xterm.js (MIT).

## Novedades de la 0.6.2

### 📊 Portafolio: todos tus proyectos en una pantalla
Avance, vencidas, recursos, quién trabaja en qué y qué proyecto está en riesgo (🟢 al día · 🔴 con vencidas · 🟡 sin movimiento en 7 días · ✅ terminado). Un clic en una tarjeta abre el proyecto; se actualiza en vivo.

![Portafolio con todos los proyectos](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-11-portafolio.png)

### 🙋 Cuando una IA te necesita, te lo pide y le contestás en el mismo mensaje
Si una IA se topa con algo que no puede hacer sola (un comando bloqueado, un permiso), corre `tm-agent ask`: el servidor crea una **tarea de pedido** para una persona autorizada, con el comando listo para copiar (**📋 Copiar**). La tarea abre con un panel: copiás cada comando, lo ejecutás en tu terminal, pegás la salida en el recuadro y apretás **📤 Enviar** (o **✅ Autorizar / ❌ Rechazar** si es una decisión): le llega sola a la IA. A quién le llega: a quien creó la tarea si tiene el nuevo permiso **Ejecutor**; si no, al ejecutor con menos tareas abiertas; si no, a un admin. Cada IA tiene además una lista editable de **🚫 comandos prohibidos**.

![Pedido de una IA: tarea creada y comando para copiar](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-12-pedido.png)

### 📖 Manual dentro de la app, tooltips y tres idiomas
Un manual con **buscador e índice por temas** (botón **📖 Manual**), un tooltip en cada botón y toda la interfaz en **español, inglés y portugués** (Configuración → Preferencias → Idioma). El detalle de la tarea se puede **maximizar** para leer comentarios largos.

![El manual dentro de la app, con buscador](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-13-manual.png)

### También nuevo
- Tareas numeradas (`#12`) y **orden manual con arrastrar y soltar**.
- **Terminal (TUI)** en tres idiomas.
- **Correo del servidor**: guía para Gmail, Outlook, Brevo, SendGrid, Mailgun, SES o un servidor propio.
- CI en Linux, Windows y macOS. Descargas: **Linux disponible**, **Windows y macOS próximamente**.
- Un sitio web: **https://polblancoo.github.io/rust_tuiManager/**

## Novedades de la 0.5

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-google.png" alt="Configuración con Google Calendar conectado y un «?» de ayuda en cada fila"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-google-help.png" alt="Ayuda de Google Calendar: qué hace y qué configura quien administra el servidor"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-config-help.png" alt="Ayuda de una fila: «Si sos usuario» y «Si administrás el servidor»"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Código de invitación con «Copiar invitación» y «Enviar por email»"></td>
  </tr>
</table>

- **Panel del equipo** (botón «👥 Equipo», columna derecha): miembros con su rol (editable ahí mismo), desde cuándo están, quién los sumó y si entraron con un código, y cuántas tareas abiertas tiene cada uno; las invitaciones con sus usos, vencimiento y **quiénes entraron con cada código**; y los ajustes del equipo (editar nombre y descripción, transferir la propiedad, salir o eliminar el equipo con confirmación por nombre).

<img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team-panel.png" alt="Panel del equipo: miembros, invitaciones y ajustes">

- **Google Calendar** (opcional, un solo sentido): cada usuario conecta su cuenta desde *Configuración → Google Calendar* y sus tareas con fecha (asignadas a él o sin responsable, de sus equipos) aparecen como eventos de día completo en su calendario principal. Si cambian, se terminan o se borran, el evento se actualiza o se quita solo (cada minuto); «Desconectar» limpia lo que creó la app. Lo habilita **una sola vez quien administra el servidor** con `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET` y `TM_GOOGLE_REDIRECT` (cliente OAuth «Aplicación web» de Google Cloud); mientras la app de Google esté en modo *Pruebas* solo entran los Gmail agregados como usuarios de prueba.
- **Ayuda en cada parámetro de Configuración**: un botón «?» por fila explica qué hace para el usuario común y qué hay que tener configurado si administrás el servidor (SMTP, Telegram, Google, HTTPS…).
- **Invitaciones listas para compartir**: al generar un código, *Copiar invitación* arma un texto con el enlace de instalación, la dirección del servidor, los pasos y el código, y *Enviar por email* lo abre en tu cliente de correo.
- **Búsqueda de usuarios más clara**: los resultados se ven como botones (funciona también en la app de escritorio) y se busca por coincidencia en cualquier parte del nombre.
- **Equipo más ordenado**: arriba agregar e invitar, y la lista de miembros al final.
- La ventana de novedades ya no reaparece en cada inicio si no hay nada sin leer.

## Funciones

- **Equipos** con roles (lector, miembro, admin, dueño) e **invitaciones por código** con vencimiento y cupo de usos.
- **Proyectos y tablero kanban** (por hacer / en curso / hecho), prioridades, fechas límite y asignación. Cada proyecto guarda su **carpeta de trabajo y su repositorio**; un admin puede **editarlo o eliminarlo** (la eliminación pide escribir su nombre).
- **Etiquetas** por equipo y **subtareas** con barra de progreso.
- **Comentarios** por tarea.
- **Calendario** mensual por fecha límite, **Gantt** con dependencias entre tareas (arrastrá las barras para mover fechas), **pizarra** colaborativa por proyecto (notas, tarjetas de tarea y conexiones) y **mapa mental** de cada tarea (cliente gráfico).
- **Responsable**: el líder (admin o dueño) asigna a cualquier miembro; un miembro solo toma tareas libres o suelta la suya.
- **Bandeja de novedades**: al iniciar sesión ves lo que pasó mientras no estabas (te asignaron, comentaron, te sumaron) y tus tareas abiertas; llega en vivo y, si cargás y verificás tu email, también por correo.
- **Cliente web**: `tm-server` sirve la misma interfaz gráfica en `/` (abrí `http://tu-servidor:7878` en el navegador, sin instalar nada; `TM_WEB=0` lo desactiva).
- **Menciones**: escribí `@usuario` en un comentario y esa persona recibe el aviso (bandeja, email y Telegram).
- **Recordatorios de vencimiento**: el responsable recibe un aviso cuando falta menos de 24 horas y otro, una sola vez, cuando la tarea vence sin cerrarse.
- **Tareas recurrentes** (diaria, semanal o mensual): al completarlas se crea la siguiente, con la fecha corrida, las etiquetas y las subtareas pendientes.
- **Adjuntos** (hasta 10 por tarea, 2 MB cada uno): el servidor los entrega siempre como descarga. La web muestra miniaturas y vista previa de imágenes (PNG, JPEG, GIF, WebP) y texto plano, verificando el contenido real del archivo; nunca HTML, SVG ni PDF.
- **Plantillas**: duplicá un proyecto con sus tareas, etiquetas y dependencias (queda todo pendiente y sin responsable).
- **Exportar** las tareas de un proyecto a CSV o JSON (el CSV neutraliza fórmulas de planilla).
- **Actividad** por proyecto: quién creó, movió, asignó, comentó o adjuntó, y cuándo.
- **Web instalable (PWA)**: desde el navegador del celular o la compu podés "instalarla" como una app.
- **Telegram** (opcional): cada usuario vincula su cuenta con el bot y recibe por ahí las novedades; cualquier miembro puede mandar **mensajes al equipo** (bandeja, email y Telegram).
- **Agregar miembros con búsqueda** de usuarios registrados, atajos con prefijo `Ctrl+T` y **ayuda con buscador** (`?`) en ambos clientes.
- **Google Calendar** (opcional): tus tareas con fecha aparecen en tu calendario; ver *Novedades de la 0.5*.
- **Agentes de IA** con cuenta, modelo y token propios, y `tm-agent` para que trabajen solos.
- **Tiempo real**: un cambio de cualquier usuario aparece al instante en todos los clientes del equipo.

## Instalación rápida

**Descarga**: [última versión](https://github.com/polblancoo/Rust_tuiManager/releases/latest) (Linux: `.deb`, `.rpm`, AppImage y `tar.gz`; **Windows y macOS próximamente**) · Sitio: https://polblancoo.github.io/rust_tuiManager/


**Una línea (Linux, sin sudo)**: baja `tm-server`, verifica su SHA-256, lo deja como servicio de tu usuario y te dice la dirección. La primera vez que abrís la página, una guía corta con dibujos te lleva por equipo, proyecto e IA.

```sh
curl -fsSL https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/install.sh | sh
```

```sh
cargo install rust-tui-manager-server   # instala el binario `tm-server`
cargo install rust-tui-manager          # cliente de terminal
cargo install rust-tui-manager-agent    # instala `tm-agent` (opcional, para agentes de IA)
```

O desde este repositorio:

```sh
cargo run -p rust-tui-manager-server                          # servidor en 0.0.0.0:7878
cargo run -p rust-tui-manager -- http://127.0.0.1:7878        # cliente TUI (F2 = crear cuenta)
cargo run -p tm-desktop                                       # cliente gráfico
```

El cliente gráfico necesita las dependencias de Tauri (en Linux: `webkit2gtk-4.1`, `libsoup-3.0`).

## Configuración del servidor

| Variable | Por defecto | Descripción |
|---|---|---|
| `TM_ADDR` | `0.0.0.0:7878` | Dirección donde escucha |
| `TM_DATABASE` | `sqlite://tm-server.db` | Base de datos (se crea con permisos `0600`) |
| `TM_JWT_SECRET` | generado y guardado en la base | Secreto de los tokens; mínimo 32 caracteres |
| `TM_CORS_ORIGINS` | Tauri y localhost | Orígenes extra permitidos, separados por coma |
| `TM_TLS_CERT` / `TM_TLS_KEY` | — | Certificado y clave PEM para servir HTTPS directamente |
| `TM_TRUSTED_PROXY` | — | `1` si corre detrás de un proxy inverso propio (usa `X-Forwarded-For` para el rate limit) |
| `TM_TELEMETRY` | — | `0` apaga el aviso diario de instalación (ver «Licencia comercial y aviso de instalación») |

Para los avisos por correo (opcionales; sin esto solo funciona la bandeja):

| Variable | Descripción |
|---|---|
| `TM_SMTP_URL` | Servidor SMTP, p. ej. `smtps://smtp.ejemplo.com:465` o `smtp://127.0.0.1:2525` |
| `TM_SMTP_FROM` | Remitente, p. ej. `Tareas <avisos@ejemplo.com>` |
| `TM_SMTP_USER` / `TM_SMTP_PASSWORD` | Credenciales, si el servidor las pide |

Para Telegram, creá un bot con [@BotFather](https://t.me/BotFather) y pasale el token al servidor:

| Variable | Descripción |
|---|---|
| `TM_REMINDER_SECS` | Cada cuántos segundos se revisan los vencimientos (por defecto 300) |
| `TM_TELEGRAM_TOKEN` | Token del bot. El servidor lo atiende por *long polling*: no necesita URL pública ni HTTPS propio |
| `TM_TELEGRAM_SEND_ONLY` | `1` para solo enviar avisos (sin vincular por `/start`, sin polling); permite compartir un bot con otro programa |
| `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET`, `TM_GOOGLE_REDIRECT` | Cliente OAuth de Google Cloud que activa la sincronización con Google Calendar (redirección: `https://tu-servidor/api/google/callback`) |

Cada usuario lo vincula en **Configuración → Telegram** (web/escritorio) o con `Ctrl+T g` (TUI): genera un código de un solo uso (15 min) y se lo manda al bot como `/start CÓDIGO`; `/stop` desvincula. Una vez vinculado, el bot también responde: `/tareas` (tus tareas abiertas) y `/hecho N` (cierra la tarea N si es tuya, o cualquiera si sos admin). Solo se escribe a chats vinculados así, nunca a ids arbitrarios.

Cada usuario carga su email y lo confirma con un código de 6 dígitos; solo a las direcciones verificadas se les envían avisos (tope por hora).

Sin TLS propio, ponelo detrás de un proxy TLS antes de exponerlo a internet.

Para dejarlo siempre encendido, corrélo como servicio (por ejemplo, una unidad de usuario de systemd con `ExecStart=~/.local/bin/tm-server` y `Restart=on-failure`).

## Seguridad

- Contraseñas con argon2; tokens JWT de 7 días revocables con "cerrar sesión en todos los dispositivos".
- Bloqueo por cuenta (5 fallos = 15 minutos) y limitación por IP en login, registro y WebSocket.
- Límites de tamaño de cuerpo, de largo de cada campo y de cantidad por recurso en todos los endpoints.
- CORS restringido, cabeceras de seguridad y CSP estricta en la app Tauri.
- El token del WebSocket no viaja en la URL: se envía como primer mensaje.
- Los códigos de invitación se guardan hasheados y solo se muestran al crearlos.
- Las cuentas de IA usan tokens propios y revocables (`tmk_…`, guardados con hash) y no pueden administrar equipos, roles ni invitaciones.

## Roles

| Rol | Puede |
|---|---|
| Lector | Ver todo |
| Miembro | Crear y editar tareas, subtareas, comentarios, etiquetas y proyectos |
| Admin | Además, agregar o quitar miembros de rango menor, cambiar roles, invitar y editar o eliminar proyectos |
| Dueño | Todo; no puede abandonar el equipo |

Nadie puede otorgar un rol igual o superior al propio.

## Estructura

```
crates/core      modelos y eventos compartidos
crates/server    servidor axum + SQLite + WebSocket
crates/tui       cliente de terminal
crates/agent     tm-agent: pone a trabajar a una IA en las tareas asignadas
apps/desktop     cliente Tauri (frontend estático en ui/)
legacy/          gestor local anterior (0.1.x)
```

## Licencia

**AGPL-3.0-only.** Gratis para usar y autoalojar. Si ofrecés una versión modificada como servicio, tenés que publicar tus cambios con la misma licencia. Hay licencia comercial sin esa obligación: consultala en [los issues](https://github.com/polblancoo/Rust_tuiManager/issues). Las versiones publicadas hasta la 0.6.2 siguen siendo MIT.
