<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIDE.md">English</a> · <b>Español</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.pt.md">Português</a></p>

# Guía paso a paso: tu primer proyecto en el tablero de comando

Rust TUI Manager es un **tablero de comando**: un lugar donde ves y dirigís varios proyectos a la vez, con personas e IAs como recursos. Esta guía es para quien va a **dirigir trabajo con un equipo de personas y de IAs**: un CTO, una jefa de operaciones, una dueña de agencia… El proyecto puede ser una **app**, una **planilla de presupuesto**, un **proceso de onboarding**, una campaña, una auditoría: Rust TUI Manager no sabe de apps, sabe de **tareas, responsables y avance**. En 10 minutos tenés el primero armado.

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

---

## 1. Instalar el servidor

El servidor guarda todo (equipos, tareas, usuarios) y sirve la web. Se instala una vez.

1. Descargá `tm-server-linux-x86_64.tar.gz` de la [última versión](https://github.com/polblancoo/Rust_tuiManager/releases/latest) (o `cargo install rust-tui-manager-server`).
2. Descomprimí y ejecutá `./tm-server`.
3. Abrí `http://localhost:7878` en el navegador.

Para que otras personas entren desde otras máquinas, poné el servidor en una dirección que alcancen (idealmente con HTTPS: ver [Configuración del servidor](https://github.com/polblancoo/Rust_tuiManager/blob/main/README.es.md)). Si solo vas a probar, alcanza con `localhost`.

Prefieras terminal o ventana propia, también hay un cliente de terminal y una app de escritorio en la misma página de descargas.

## 2. Crear tu cuenta y tu equipo

1. En la pantalla de inicio elegí **Crear cuenta** (usuario y contraseña de 8 caracteres o más).
2. En la barra lateral, **Equipos → «+»**, poné el nombre de tu organización (por ejemplo «Mi empresa»). Quedás como **Dueño**.

Un **equipo** es el grupo de personas e IAs que comparten proyectos. Podés tener varios.

## 3. Crear los proyectos

En **Proyectos → «+»** creá uno por cada cosa que quieras llevar adelante. En el ejemplo: una app, una planilla y un proceso, cada uno con su color.

![Tablero del proyecto «App móvil de reservas»](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

Cada proyecto tiene su **tablero** (Por hacer · En curso · Hecho). Con **+ Nueva tarea** cargás el trabajo: título, descripción, prioridad, fechas y responsable. Los mismos datos se ven también en **calendario**, **Gantt** y **pizarra**.

![Otro proyecto: una planilla de presupuesto, con responsables humanos y de IA](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-08-otro-proyecto.png)

Cada proyecto guarda **dónde vive su trabajo**: con **⚙ Proyecto** (admin o dueño) definís la carpeta de trabajo y el repositorio, y se muestran arriba del tablero.

![Editar el proyecto: nombre, carpeta de trabajo y repositorio](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-09-proyecto.png)

> 💡 Tip: no hace falta que sea software. Una planilla, un proceso de onboarding o una auditoría se manejan igual: tareas con responsable y fecha.

## 4. Sumar personas

Botón **👥 Equipo** (arriba a la derecha) → **➕ Agregar / invitar**.

- **Ya tiene cuenta**: escribí parte de su nombre, elegí el rol y **Agregar**.
- **Todavía no tiene cuenta**: elegí un rol y **Generar código**. Usá **Copiar invitación** (texto listo para WhatsApp o mail con enlace de descarga, servidor y código) o **Enviar por email**.

![Invitar con un código](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-07-invitar.png)

| Rol | Qué puede hacer |
|---|---|
| **Lector** | Mirar. |
| **Miembro** | Crear, editar y comentar tareas; tomar las libres. |
| **Admin** | Además, asignar tareas a otros, agregar gente y generar invitaciones. |
| **Dueño** | Todo, incluido transferir o eliminar el equipo. |

El panel del equipo muestra a cada integrante con su rol, desde cuándo está, quién lo sumó y cuántas tareas abiertas tiene:

![Panel del equipo con personas e IAs](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-06-panel-equipo.png)

## 5. Sumar IAs

Las IAs (Claude Code, Cursor, Windsurf, Zed, Gemini u otro cliente compatible con [MCP](https://modelcontextprotocol.io)) trabajan **como un integrante más**. Tantas como necesites, de distintos modelos.

1. En la barra lateral, **🤖 IAs → «+»**.
2. Poné un nombre que te diga qué es (`claude-backend`, `gemini-datos`) y su rol: **Miembro** (crea y edita sus tareas) o **Lector** (solo mira).

   ![Nueva IA en el equipo](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-02-nueva-ia.png)

3. Se muestra **su token una sola vez**, con la configuración MCP lista para copiar. Es la «llave» de esa IA.

   ![Token de la IA y configuración MCP](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-token.png)

4. Si lo perdés o lo querés cortar: clic en la IA → **Nuevo token** (el anterior deja de servir) o **Revocar**.

   ![Ficha de una IA](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-ia.png)

**Qué puede y qué no puede hacer una IA**

- ✅ Crear tareas y **asignárselas a personas** del equipo, comentar, completar o borrar **las que creó ella o le asignaron**.
- ❌ Crear o borrar equipos, invitar gente, cambiar roles, o tocar tareas ajenas (salvo comentar).
- Todo lo que hace queda **a su nombre** en la actividad, y se ve con 🤖 en el panel.

> 🚧 **Estado**: las cuentas de IA y sus tokens ya funcionan contra la API del servidor. El binario `tm-mcp` y el *skill* que las conectan directo a tu plataforma de código están en desarrollo (ver el [ROADMAP](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md)).

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

![Gantt del proyecto, con tareas de personas y de IAs](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-04-gantt.png)

![Actividad: el historial del proyecto, con las acciones de las IAs a su nombre](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-05-actividad.png)

Además, **Google Calendar** (tus tareas con fecha aparecen en tu calendario) se conecta desde **Configuración**.

## 8. Delegar a un líder

Hoy, para delegar un proyecto: sumá a la persona con rol **Admin** (puede asignar tareas, agregar gente y generar invitaciones) y asignale las tareas principales. Ella arma su parte y suma a su gente.

> 🚧 **Próximamente**: **líder por proyecto** (con alcance solo en ese proyecto), **ámbito configurable por IA** (qué proyectos toca, si puede invitar a otras IAs), **reportes de avance** y un **panel de portafolio** con todos tus proyectos a la vez. Mirá la [hoja de ruta](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md).

---

**¿Algo no quedó claro?** Cada opción de **Configuración** tiene un «?» con ayuda. Y si encontrás un problema, abrí un [issue](https://github.com/polblancoo/Rust_tuiManager/issues).
