<p align="center"><a href="https://github.com/polblancoo/bitacoraai/blob/main/docs/GUIDE.md">English</a> · <b>Español</b> · <a href="https://github.com/polblancoo/bitacoraai/blob/main/docs/GUIA.pt.md">Português</a></p>

# Guía paso a paso: tu primer proyecto en el tablero de comando

BitacoraAI es un **tablero de comando**: un lugar donde ves y dirigís varios proyectos a la vez, con personas e IAs como recursos. Esta guía es para quien va a **dirigir trabajo con un equipo de personas y de IAs**: un CTO, una jefa de operaciones, una dueña de agencia… El proyecto puede ser una **app**, una **planilla de presupuesto**, un **proceso de onboarding**, una campaña, una auditoría: BitacoraAI no sabe de apps, sabe de **tareas, responsables y avance**. En 10 minutos tenés el primero armado.

> Las capturas son de una empresa de ejemplo («Mi empresa») con tres proyectos muy distintos.

**Lo que vas a hacer**

1. [Instalar el servidor](#1-instalar-el-servidor)
2. [Crear tu cuenta y tu equipo](#2-crear-tu-cuenta-y-tu-equipo)
3. [Crear los proyectos](#3-crear-los-proyectos)
4. [Sumar personas](#4-sumar-personas)
5. [Sumar IAs](#5-sumar-ias)
6. [Armar y asignar tareas](#6-armar-y-asignar-tareas)
7. [Seguir el avance](#7-seguir-el-avance)
8. [Delegar a un líder](#8-delegar-a-un-líder)
9. [Dirigir varios proyectos a la vez](#9-dirigir-varios-proyectos-a-la-vez)
10. [Cuando una IA necesita que hagas algo](#10-cuando-una-ia-necesita-que-hagas-algo)
11. [Idioma, tooltips y este manual](#11-idioma-tooltips-y-este-manual)
12. [Configurar el correo del servidor](#12-configurar-el-correo-del-servidor)
13. [Permitir comandos desde la tarea](#13-permitir-comandos-desde-la-tarea)
14. [Terminal en el navegador](#14-terminal-en-el-navegador)
15. [Consumo y cuotas](#15-consumo-y-cuotas)
16. [Licencia comercial y aviso de instalación](#16-licencia-comercial-y-aviso-de-instalacion)

---

## 1. Instalar el servidor

El servidor guarda todo (equipos, tareas, usuarios) y sirve la web. Se instala una vez.

1. Descargá `tm-server-linux-x86_64.tar.gz` de la [última versión](https://github.com/polblancoo/bitacoraai/releases/latest) (o `cargo install bitacora-server`).
2. Descomprimí y ejecutá `./tm-server`.
3. Abrí `http://localhost:7878` en el navegador.

Para que otras personas entren desde otras máquinas, poné el servidor en una dirección que alcancen (idealmente con HTTPS: ver [Configuración del servidor](https://github.com/polblancoo/bitacoraai/blob/main/README.es.md)). Si solo vas a probar, alcanza con `localhost`.

**Instalación de una línea (Linux x86_64, sin sudo):** `curl -fsSL https://raw.githubusercontent.com/polblancoo/bitacoraai/main/install.sh | sh`. Baja el servidor del release, **verifica su suma SHA256** (si no coincide, no instala), lo deja en `~/.local/bin` y crea un servicio de usuario que arranca solo. Opciones: `--port N`, `--no-service`, `--with-agent`, `--with-claude`, `--with-gemini`, `--with-codex`, `--with-opencode` (instalan el programa de esa IA; `--with-all-ais` los cuatro) y `--version X`. La primera vez que entrás, una **guía de inicio** con dibujos te lleva por equipo, proyecto e IA (se repite desde Configuración → Preferencias → 🧭 Guía de inicio).

Prefieras terminal o ventana propia, también hay un cliente de terminal y una app de escritorio en la misma página de descargas.

## 2. Crear tu cuenta y tu equipo

1. En la pantalla de inicio elegí **Crear cuenta** (usuario y contraseña de 8 caracteres o más).
2. En la barra lateral, **Equipos → «+»**, poné el nombre de tu organización (por ejemplo «Mi empresa»). Quedás como **Dueño**.

Un **equipo** es el grupo de personas e IAs que comparten proyectos. Podés tener varios.

## 3. Crear los proyectos

En **Proyectos → «+»** creá uno por cada cosa que quieras llevar adelante. En el ejemplo: una app, una planilla y un proceso, cada uno con su color.

![Tablero del proyecto «App móvil de reservas»](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-01-tablero.png)

Cada proyecto tiene su **tablero** (Por hacer · En curso · Hecho). Con **+ Nueva tarea** cargás el trabajo: título, descripción, prioridad, fechas y responsable. Los mismos datos se ven también en **calendario**, **Gantt** y **pizarra**.

![Otro proyecto: una planilla de presupuesto, con responsables humanos y de IA](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-08-otro-proyecto.png)

Cada proyecto guarda **dónde vive su trabajo**: con **⚙ Proyecto** (admin o dueño) definís la carpeta de trabajo y el repositorio, y se muestran arriba del tablero.

![Editar el proyecto: nombre, carpeta de trabajo y repositorio](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-09-proyecto.png)

> 💡 Tip: no hace falta que sea software. Una planilla, un proceso de onboarding o una auditoría se manejan igual: tareas con responsable y fecha.

## 4. Sumar personas

Botón **👥 Equipo** (arriba a la derecha) → **➕ Agregar / invitar**.

- **Ya tiene cuenta**: escribí parte de su nombre, elegí el rol y **Agregar**.
- **Todavía no tiene cuenta**: elegí un rol y **Generar código**. Usá **Copiar invitación** (texto listo para WhatsApp o mail con enlace de descarga, servidor y código) o **Enviar por email**.

![Invitar con un código](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-07-invitar.png)

| Rol | Qué puede hacer |
|---|---|
| **Lector** | Mirar. |
| **Miembro** | Crear, editar y comentar tareas; tomar las libres. |
| **Admin** | Además, asignar tareas a otros, agregar gente y generar invitaciones. |
| **Dueño** | Todo, incluido transferir o eliminar el equipo. |

El panel del equipo muestra a cada integrante con su rol, desde cuándo está, quién lo sumó y cuántas tareas abiertas tiene:

![Panel del equipo con personas e IAs](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-06-panel-equipo.png)

## 5. Sumar IAs

Las IAs (Claude Code, Cursor, Windsurf, Zed, Gemini u otro cliente compatible con [MCP](https://modelcontextprotocol.io)) trabajan **como un integrante más**. Tantas como necesites, de distintos modelos.

1. En la barra lateral, **🤖 IAs → «+»**.
2. Poné un nombre que te diga qué es (`claude-backend`, `gemini-datos`) y su rol: **Miembro** (crea y edita sus tareas) o **Lector** (solo mira).

   ![Nueva IA en el equipo](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-02-nueva-ia.png)

3. Se muestra **su token una sola vez**, con la configuración MCP lista para copiar. Es la «llave» de esa IA.

   ![Token de la IA y configuración MCP](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-03-token.png)

4. Si lo perdés o lo querés cortar: clic en la IA → **Nuevo token** (el anterior deja de servir) o **Revocar**.

   ![Ficha de una IA](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-03-ia.png)

**Qué puede y qué no puede hacer una IA**

- ✅ Crear tareas y **asignárselas a personas** del equipo, comentar, completar o borrar **las que creó ella o le asignaron**.
- ❌ Crear o borrar equipos, invitar gente, cambiar roles, o tocar tareas ajenas (salvo comentar).
- Todo lo que hace queda **a su nombre** en la actividad, y se ve con 🤖 en el panel.

> 🚧 **Estado**: las cuentas de IA y sus tokens ya funcionan contra la API del servidor. El binario `tm-mcp` y el *skill* que las conectan directo a tu plataforma de código están en desarrollo (ver el [ROADMAP](https://github.com/polblancoo/bitacoraai/blob/main/ROADMAP.md)).

## 6. Armar y asignar tareas

- **Nueva tarea** en cualquier columna; abrila para sumar descripción, fechas, etiquetas, subtareas, adjuntos y dependencias.
- **Responsable**: el admin asigna a quien quiera (persona o IA); un miembro toma las libres o suelta la suya.
- **Menciones** `@usuario` en los comentarios avisan a esa persona.
- Una IA puede crear una tarea y **dejarla asignada a una persona**, para pedirle algo.

## 7. Seguir el avance

Todo se actualiza **en vivo**: no hace falta recargar.

| Vista | Para qué |
|---|---|
| **Tablero** | Qué está por hacer, en curso y hecho. |
| **Gantt** | Cronograma con fechas y dependencias; las vencidas se marcan. |
| **Actividad** | Quién hizo qué y cuándo, personas e IAs juntas. |
| **Calendario** | Qué vence cada día. |
| **🔔 Bandeja** | Novedades: asignaciones, menciones, mensajes. También por email y Telegram. |

![Gantt del proyecto, con tareas de personas y de IAs](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-04-gantt.png)

![Actividad: el historial del proyecto, con las acciones de las IAs a su nombre](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-05-actividad.png)

Además, **Google Calendar** (tus tareas con fecha aparecen en tu calendario) se conecta desde **Configuración**.

## 8. Delegar a un líder

Hoy, para delegar un proyecto: sumá a la persona con rol **Admin** (puede asignar tareas, agregar gente y generar invitaciones) y asignale las tareas principales. Ella arma su parte y suma a su gente.

> 🚧 **Próximamente**: **líder por proyecto** (con alcance solo en ese proyecto), **ámbito configurable por IA** (qué proyectos toca, si puede invitar a otras IAs) y **reportes de avance** con gráficos. Mirá la [hoja de ruta](https://github.com/polblancoo/bitacoraai/blob/main/ROADMAP.md).

## 9. Dirigir varios proyectos a la vez

**📊 Portafolio** es el primer botón de la lista de *Proyectos*. Muestra todos los proyectos del equipo en una sola pantalla:

- **Arriba**: proyectos, avance general, tareas en curso, vencidas y proyectos en riesgo.
- **Una tarjeta por proyecto**: barra de avance, quién tiene tareas abiertas (las IAs con 🤖), última actividad, próximo vencimiento y cuántas acciones hicieron las IAs en 24 h.
- **Estado**: 🟢 al día · 🔴 con tareas vencidas · 🟡 sin movimiento en 7 días · ✅ terminado · ⚪ sin tareas.
- **Abajo**: lo último que hicieron las IAs. Un clic en una tarjeta abre el proyecto, y todo se actualiza en vivo.

![Portafolio con todos los proyectos](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-11-portafolio.png)

**Tareas numeradas.** Cada tarea muestra su número (`#12`) en la tarjeta y en el detalle, para nombrarla sin ambigüedad en mensajes y comentarios.

**Orden manual.** Arrastrá una tarea dentro de su columna para cambiar el orden en que se hace; el orden que dejás manda sobre la prioridad. También podés elegir el responsable al crearla.

## 10. Cuando una IA necesita que hagas algo

Una IA puede toparse con algo que no puede hacer sola: un comando bloqueado, un permiso, una decisión. Entonces **te lo pide** y espera tu respuesta:

1. La IA corre `tm-agent ask "…"`. En su tarea aparece un comentario con 🙋 y se **crea una tarea de pedido** («🙋 Pedido de #12: …») asignada a una persona. El comentario te dice cuál es.
2. Abrí la tarea de pedido: arriba está el panel **🙋 Pedido de una IA** con **cada comando en su propia línea y un botón 📋 Copiar**. Lo pegás en tu terminal y lo ejecutás.
3. **¿Dónde va la salida?** En el recuadro **«Pegá la salida acá»** del mismo panel, y **📤 Enviar salida a la IA**. Se publica como comentario de esa tarea y **le llega sola a la IA** (también aparece en su tarea original). Escribir un comentario común en la tarea de pedido hace lo mismo.
4. Si el pedido es una **decisión o un permiso** («¿puedo borrar X?»), usá **✅ Autorizar** o **❌ Rechazar** (con el motivo en el recuadro). Para un comando que la IA tiene bloqueado, «autorizar» no alcanza: ejecutalo vos y enviá la salida.
5. La IA lo lee en su siguiente consulta (cada 10 s) y sigue. Al enviar, el pedido pasa solo a Hecha.

![Pedido de una IA: tarea creada, comando para copiar](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-12-pedido.png)

**¿A quién le llega?** A los miembros con el permiso **Ejecutor** (ventana del equipo → tilde *Ejecutor* junto a cada persona; solo lo cambia un admin, y las IAs no pueden serlo):

1. a quien creó la tarea, si es ejecutor;
2. si no (por ejemplo la creó otra IA), al ejecutor con menos tareas abiertas;
3. si no hay ejecutores, a un admin del equipo.

**Comandos prohibidos.** En la ficha de cada IA, **🚫 Comandos prohibidos** (uno por línea, ej. `Bash(rm:*)`). La IA no puede ejecutarlos: si los necesita, te lo pide como arriba.

## 11. Idioma, tooltips y este manual

- **Idioma**: *Configuración → Preferencias → Idioma* (español, inglés o portugués). Por defecto usa el del navegador.
- **Tooltips**: dejá el cursor sobre un botón para ver qué hace.
- **Este manual** está dentro de la app: botón **📖 Manual** (abajo a la izquierda, o *Configuración → Ayuda*). Tiene buscador y un índice por temas con enlaces.

![El manual dentro de la app, con buscador](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-13-manual.png)

## 12. Configurar el correo del servidor

Para que la app mande **emails** (códigos de verificación, asignaciones, menciones, recordatorios) el **servidor** necesita una cuenta de correo saliente. Lo configura quien administra el servidor, una sola vez; los usuarios solo verifican su email desde *Configuración → Mi email*.

**¿Qué archivo se edita?** Si usás el servicio de systemd (`tm-server.service`), creá **`~/.config/tm-server/mail.env`** (permisos `600`, nunca dentro del repositorio) con estas cuatro variables:

```
TM_SMTP_URL=smtps://smtp.gmail.com:465
TM_SMTP_USER=tucuenta@gmail.com
TM_SMTP_PASSWORD=la-contraseña-de-aplicación
TM_SMTP_FROM=Tablero de comando <tucuenta@gmail.com>
```

y en `~/.config/systemd/user/tm-server.service` agregá, junto a los otros `EnvironmentFile`:

```
EnvironmentFile=-%h/.config/tm-server/mail.env
```

Después: `systemctl --user daemon-reload && systemctl --user restart tm-server`. Si lanzás `tm-server` a mano o con Docker, definí las mismas variables en el entorno (`export …` o `-e …`).

**Según tu proveedor** cambian `TM_SMTP_URL`, el usuario y la clave (verificá los datos en la documentación de tu proveedor):

| Proveedor | `TM_SMTP_URL` | `TM_SMTP_USER` | `TM_SMTP_PASSWORD` |
|---|---|---|---|
| Gmail | `smtps://smtp.gmail.com:465` | tu dirección completa | contraseña de aplicación (hace falta verificación en 2 pasos): https://myaccount.google.com/apppasswords |
| Outlook / Microsoft 365 | `smtp://smtp.office365.com:587?tls=required` | tu dirección completa | tu clave o contraseña de aplicación; en Microsoft 365 el administrador debe habilitar *SMTP autenticado* |
| Yahoo | `smtps://smtp.mail.yahoo.com:465` | tu dirección completa | contraseña de aplicación |
| Zoho | `smtps://smtp.zoho.com:465` | tu dirección completa | tu clave o contraseña de aplicación |
| Brevo | `smtp://smtp-relay.brevo.com:587?tls=required` | tu login de Brevo | clave SMTP |
| SendGrid | `smtps://smtp.sendgrid.net:465` | `apikey` | tu API key |
| Mailgun | `smtps://smtp.mailgun.org:465` | `postmaster@tu-dominio` | clave SMTP del dominio |
| Amazon SES | `smtps://email-smtp.REGIÓN.amazonaws.com:465` | usuario SMTP de SES | clave SMTP de SES |
| Servidor propio (Postfix…) | `smtp://localhost:25` | — | — |

- `smtps://` usa el puerto 465 (TLS directo); `smtp://…:587?tls=required` usa STARTTLS.
- Poné usuario y clave en `TM_SMTP_USER` y `TM_SMTP_PASSWORD` (no dentro de la URL) y no hace falta escapar caracteres especiales.
- `TM_SMTP_FROM` es el remitente; la mayoría de los proveedores exige que sea una dirección tuya o de tu dominio verificado.

**Probar**: *Configuración → Mi email*, escribí tu dirección y debería llegar el código de 6 dígitos. Si no llega, mirá el log del servidor (`journalctl --user -u tm-server -n 50`): una clave rechazada o un puerto bloqueado aparece ahí.

> Gmail y otros servicios gratuitos limitan los envíos (Gmail, unos 500 por día) y reescriben el remitente. Para empezar alcanza; con muchos usuarios conviene un servicio de envío con dominio propio.

## 13. Permitir comandos desde la tarea

Cuando una IA te pide un comando, abrí la tarea de pedido y tocá **✅ Permitir comando…**. El cuadro muestra el comando, **explica qué hace** (con aviso rojo si es peligroso) y te deja elegir **una vez** o **siempre**. También podés escribir en un comentario `/permitir cmd` (una vez) o `/permitir siempre cmd` (solo owners y admins). La IA recibe los permisos en `TM_ALLOW` y retoma la tarea.

![Permitir comando](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-16-permitir.png)

## 14. Terminal en el navegador

Solo para **owners**. El botón **💻 Terminal** de la tarea abre una consola en la carpeta del proyecto. Hay dos formas de conectarla (se elige por proyecto):

- **Por el agente** (recomendada): ejecutá `tm-agent` con `TM_AGENT_TERMINAL=1` en la máquina del proyecto. La conexión sale de esa máquina: sin claves ni puertos abiertos, y anda detrás de NAT.
- **SSH con clave**: la app genera una clave ed25519 (**🔑 Generar clave**) y la instala con **🔐 Instalar clave con contraseña** (la contraseña no se guarda). La huella del servidor se fija la primera vez y si cambia, se rechaza.

Cada sesión deja un comentario 📝 y una nota `.tm/terminal-tarea-N.md` en la carpeta del proyecto, que la IA lee al retomar. En el menú, **🖥️ Terminales** muestra hasta 8 paneles a la vez y el estado de cada IA (🙋 te necesita · ⚙️ trabajando · ⏳ en cola · ✅ terminó).

![Vista Terminales](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-14-terminales.png)

## 15. Consumo y cuotas

El botón **💰 Consumo** muestra tokens, costo y horas por IA y por tarea, con barras de **cuota** del equipo, del proyecto y de la tarea. En una tarea, **🪙 Tokens a gastar** fija un tope y **🧮 Estimar** propone un valor según el historial. La cuota se revisa **antes** de cada ejecución: si no alcanza, la IA no empieza y lo avisa en un comentario. Para cortar en dólares usá `TM_AGENT_MAX_USD` en el agente (1 USD o más). El Portafolio suma el consumo del mes.

![Consumo](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-15-consumo.png)
![Tope de tokens](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/guia-17-tokens.png)

## 16. Licencia comercial y aviso de instalación

El programa es **AGPL-3.0-only y gratis**. La **licencia comercial** es para quien lo usa en una empresa y no quiere cumplir la AGPL. En **Configuración → 🏢 Licencia** (solo la primera persona registrada del servidor):

1. Elegí si el servidor es **personal** o de **empresa**.
2. **Comprar una licencia comercial** → elegí el plan, la red (**USDT en Solana o en Ethereum**) y tu email.
3. Pagá **exactamente** el monto que muestra (los últimos decimales identifican tu pago) a la dirección indicada; el QR ya lo trae. Verificá el **contrato del token** con el enlace al explorador.
4. La app revisa sola el pago. Cuando se confirma, te llega un mail con el código `TM-XXXX-XXXX-XXXX` y la licencia se activa sola. Si no, pegá el código en **¿Ya tenés un código?**.

El **cartel** de arriba aparece cuando hay más de 3 personas o 2 IAs, o si lo declarás de empresa; se descarta 7 días y nunca bloquea nada.

**Aviso de instalación.** Una vez por día el servidor envía un código al azar, la versión, el sistema operativo y cuántas personas, IAs, proyectos y equipos hay. Nunca nombres, correos, tareas ni contenido; la IP no se guarda. Se apaga con `TM_TELEMETRY=0` o desde la ventana Licencia.

### Planes y precios (por año)

| Plan | Incluye | Precio de lista | Precio fundador |
|---|---|---|---|
| **Gratis** (AGPL) | hasta 3 personas y 2 IAs, sin aviso | USD 0 | — |
| **Equipo** | hasta 10 personas y 10 IAs | **USD 290** | **USD 290 de por vida** (primeros 5) |
| **Empresa** | hasta 50 personas y 50 IAs, con soporte | **USD 990** | — |

- **🎖 Precio fundador:** los **primeros 5 clientes** pagan el precio fundador y lo conservan **de por vida** en las renovaciones. La app te lo muestra sola si todavía quedan cupos.
- **Si crecés:** si con una licencia vigente superás las personas o IAs del plan, la app lo avisa y aparece **⬆ Mejorar el plan**: pagás solo la **diferencia** entre el precio del plan nuevo y lo que ya pagaste, y tu vencimiento no cambia.
- **Términos:** antes de pagar leés y aceptás los **términos de la licencia comercial** (en tu idioma). Guardamos la versión que aceptaste y la fecha. Los términos aclaran, entre otras cosas, que cada comando o permiso que aprobás a una IA es responsabilidad tuya y que el software se entrega sin garantías.
- Los precios se confirman en la pantalla de compra: lo que ves ahí es lo que se cobra.


---

**¿Algo no quedó claro?** Cada opción de **Configuración** tiene un «?» con ayuda. Y si encontrás un problema, abrí un [issue](https://github.com/polblancoo/bitacoraai/issues).

## 17. Novedades de la interfaz (0.7.1)

- **Nombre BitacoraAI.** La app se llama ahora **BitacoraAI** (interfaz, correos y manifiesto). Los programas y servicios mantienen sus nombres por ahora.
- **Guía de inicio.** Un asistente de 5 pasos con ilustraciones: bienvenida, equipo, proyecto, IA (opcional) y tarea de ejemplo. Se abre solo si no tenés equipos; si el servidor está vacío, la pantalla de ingreso ofrece directamente **Crear cuenta**.
- **Pedidos de una IA, más rápidos.** Los comandos que pide la IA aparecen juntos en una lista: **📋** copia, **✕** quita los que no querés y **✅ Permitir los N comandos siempre** los agrega todos a la lista de la IA, le avisa y marca el pedido como Hecho. **⚙ Lista de la IA** abre la lista de permitidos para borrar o editar líneas (se guarda al instante). Un comando puede tener hasta 1000 caracteres.
- **Tarea más cómoda.** **💾 Guardar** y **🗑 Borrar** están también arriba, en la cabecera fija, y la barra de abajo queda pegada al fondo. Con **⤢ Maximizar** leés los comentarios más grandes.
- **Terminal de tarea.** Una sola terminal ocupa toda la pantalla y, si la máquina tiene `herdr`, se abre solo (se apaga con `localStorage autoHerdr = "0"`). `~/carpeta` funciona como carpeta de trabajo.
- **Novedades ampliables.** La bandeja se puede agrandar con **⤢** y al tocar una novedad se abre **su** tarea.
- **Portafolio clicable.** En «Lo último que hicieron las IAs», una fila con **💬 ver** abre la tarea y toda la conversación.
- **Configuración en solapas**, con una ayuda bajo cada línea.
- **Zoom de la letra.** Abajo a la derecha: **A−**, el porcentaje (clic = 100 %) y **A+**; también **Ctrl + Alt + rueda**. Se recuerda en cada dispositivo.
- **Confirmaciones propias.** Borrar, salir de un equipo o desconectar una cuenta pide confirmación en una ventana de la app (Esc cancela), no en el aviso del navegador.

### Perfil de uso (Configuración → Equipo y proyecto)

Elegí para qué usa el equipo BitacoraAI y la interfaz se prepara sola: *Desarrollo de software* (Por hacer / En curso / Hecho, con terminales y carpeta del código), *Contenido y redes* (Borrador / En producción / Publicado, calendario como vista inicial y sin partes técnicas), *Fábrica de apps* (Backlog / En construcción / Entregado, varios proyectos en paralelo), *Oficina y estudios* (administración, contabilidad, abogacía: Pendiente / En trámite / Cerrado, con el calendario de vencimientos) u *Otros / equipo general*. «Crear ejemplo» arma un **equipo de ejemplo aparte** («Ejemplo · …») con proyectos y tareas de muestra: tu equipo real no se toca, volvés a él con «Volver a mi equipo» y «Eliminar ejemplo» borra el equipo de ejemplo entero. Solo cambia nombres y qué se muestra: tus datos no se modifican y podés cambiar de perfil cuando quieras. También se elige en la guía de inicio.

Si el equipo maneja información confidencial (por ejemplo, un estudio jurídico), activá la **revisión humana** del proyecto, no cargues datos que no quieras que vea una IA y recordá que el texto de las tareas llega al modelo que uses; para datos muy sensibles, usá un modelo propio de la empresa.

### GitHub por proyecto (botón 🐙 GitHub, arriba)

1. En **⚙ Configuración → Avisos y conexiones → GitHub** conectá tu cuenta (una vez). El administrador del servidor tiene que haber puesto `TM_GITHUB_CLIENT_ID`.
2. En **⚙ Datos del proyecto** cargá el repositorio (`dueño/repo` o la URL). Después tocá **🐙 GitHub** (arriba, junto a ⚙ Proyecto): ahí listás los issues abiertos y **traés los tildados como tareas**.
3. En **Ajustes de GitHub para este proyecto** hay una casilla por función; cada una se prende o se apaga sola: **revisar cada 5 minutos** (pasa a *Hecha* la tarea cuyo issue se cerró o cuyo Pull Request se integró, y muestra el estado del PR), **pasar comentarios** entre la tarea y el issue (si el repositorio es público, esos comentarios los ve cualquiera), **publicar como issue** las tareas nuevas, **avisar cuando una IA termina** para que vos abras el Pull Request, **estado de las pruebas (CI)** con ✅/❌ junto al botón, **anotar los commits** en Actividad y **webhook** (aviso al instante; el servidor debe ser alcanzable desde internet; el cuadro te muestra la URL y el *secret* para pegar en GitHub).
4. En la ficha de una tarea: **🐙 Publicar como issue**, **Abrir Pull Request** (de la rama `tm/tarea-N` u otra que indiques, siempre lo decidís vos) y **Cerrar issue**.

Los tokens de GitHub y de Google se guardan **cifrados** (AES-256-GCM) en la base; la clave está en `~/.config/tm-server/credenciales.key` (o `TM_KEY_FILE`). Guardala junto con tus copias de seguridad: sin ella hay que volver a conectar las cuentas.
