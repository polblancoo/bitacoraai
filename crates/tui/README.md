# rust-tui-manager

> 🎛️ **Tablero de comando para tus proyectos**: dirigí varios proyectos a la vez —una fábrica de apps, una planilla, un onboarding, lo que sea— con **personas e IAs en el mismo tablero**. Cada proyecto sabe dónde vive su trabajo (carpeta y repo). Self-hosted.

Cliente de terminal (ratatui) del gestor de tareas colaborativo [Rust TUI Manager](https://github.com/polblancoo/Rust_tuiManager). Necesita un servidor [`rust-tui-manager-server`](https://crates.io/crates/rust-tui-manager-server) en marcha.

> **Versiones 0.2 en adelante**: este crate pasó de ser un gestor local de notas y snippets a ser un cliente de red. El código anterior está en la carpeta `legacy/` del repositorio y en las versiones 0.1.x.

![Tablero en la terminal](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/tui-board.png)

![Detalle de una tarea: subtareas, etiquetas y comentarios](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/tui-detail.png)

> 👉 **[Guía paso a paso: tu primer proyecto](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.md)**, con capturas.

## 🤖 Agentes de IA como integrantes del equipo

Los asistentes de código (Claude Code, Cursor, Windsurf, Zed o cualquier cliente MCP) pueden **conectarse a un equipo humano**: cada IA tiene su usuario, token y rol, crea y completa tareas, **se las asigna a personas** y recibe las que le asignes. Se suman desde la barra lateral (**🤖 IAs → «+»**), el token se muestra una vez y se rota o revoca con un clic. Una IA no puede administrar equipos, invitar ni tocar tareas ajenas. Detalles en el [README principal](https://github.com/polblancoo/Rust_tuiManager#readme); el binario `tm-mcp` está en desarrollo.

## Uso

```sh
cargo install rust-tui-manager
rust-tui-manager http://127.0.0.1:7878
```

En la pantalla de acceso, `F2` alterna entre iniciar sesión y crear cuenta. La sesión se guarda en `~/.config/rust-tui-manager/client.json` (permisos `0600`) y el servidor se recuerda para la próxima vez.

## Atajos

**Tablero**

| Tecla | Acción |
|---|---|
| `h` `l` / `←` `→` | Cambiar de columna |
| `j` `k` / `↑` `↓` | Mover la selección |
| `Enter` | Abrir detalle (subtareas, etiquetas, comentarios) |
| `n` | Nueva tarea |
| `e` | Editar el título |
| `d` | Borrar la tarea |
| `H` / `L` | Mover la tarea de estado |
| `+` / `-` | Subir o bajar la prioridad |
| `a` | Tomar la tarea o soltar la mía |
| `A` | Asignar a un miembro, con sugerencias (admin o dueño; `-` quita) |
| `N` | Bandeja: novedades y mis tareas (se abre sola al iniciar sesión) |
| `Tab` / `Shift+Tab` | Proyecto siguiente o anterior |
| `[` / `]` | Equipo anterior o siguiente |
| `P` / `T` | Nuevo proyecto / nuevo equipo |
| `m` | Agregar miembro con búsqueda: `↑↓` elegir, `Tab` completa, `Ctrl+R` cambia el rol |
| `Ctrl+T` + letra | Prefijo de equipo: `m` miembro · `r` rol · `i` invitar · `j` unirse · `t` equipo · `e` mi email · `v` verificar email · `s` mensaje al equipo · `g` vincular Telegram · `n` bandeja |
| `R` | Cambiar el rol de un miembro: `usuario rol` |
| `I` | Generar un código de invitación (rol miembro, 7 días, 10 usos) |
| `J` | Unirse a un equipo con un código |
| `X` | Cerrar sesión en todos los dispositivos |
| `C` | Menú de configuración: email, Telegram, bandeja, mensajes y equipo, sesión |
| `?` | Ayuda con buscador de atajos |
| `q` | Salir |

**Detalle de una tarea**

| Tecla | Acción |
|---|---|
| `s` | Nueva subtarea |
| `j` `k` | Elegir subtarea |
| `Espacio` / `x` | Marcar o desmarcar la subtarea |
| `d` | Borrar la subtarea |
| `l` | Poner o quitar una etiqueta (se crea si no existe) |
| `c` | Comentar |
| `Esc` | Volver |

La cabecera marca los roles: `★` dueño, `⚙` admin, `👁` lector. El punto verde de la esquina indica conexión en vivo.

Más información, seguridad y configuración del servidor en el [README del proyecto](https://github.com/polblancoo/Rust_tuiManager).

## Licencia

MIT
