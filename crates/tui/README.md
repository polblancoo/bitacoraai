# rust-tui-manager

Cliente de terminal (ratatui) del gestor de tareas colaborativo [Rust TUI Manager](https://github.com/polblancoo/tui_comandNote). Necesita un servidor [`rust-tui-manager-server`](https://crates.io/crates/rust-tui-manager-server) en marcha.

> **Versiones 0.2 en adelante**: este crate pasó de ser un gestor local de notas y snippets a ser un cliente de red. El código anterior está en la carpeta `legacy/` del repositorio y en las versiones 0.1.x.

![Tablero en la terminal](https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/tui-board.png)

![Detalle de una tarea: subtareas, etiquetas y comentarios](https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/tui-detail.png)

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
| `a` | Asignármela o quitar la asignación |
| `Tab` / `Shift+Tab` | Proyecto siguiente o anterior |
| `[` / `]` | Equipo anterior o siguiente |
| `P` / `T` | Nuevo proyecto / nuevo equipo |
| `m` | Agregar miembro: `usuario [viewer\|member\|admin]` |
| `R` | Cambiar el rol de un miembro: `usuario rol` |
| `I` | Generar un código de invitación (rol miembro, 7 días, 10 usos) |
| `J` | Unirse a un equipo con un código |
| `X` | Cerrar sesión en todos los dispositivos |
| `?` | Ayuda |
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

Más información, seguridad y configuración del servidor en el [README del proyecto](https://github.com/polblancoo/tui_comandNote).

## Licencia

MIT
