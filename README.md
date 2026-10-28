# Rust TUI Manager

Gestor de tareas colaborativo, multiusuario y self-hosted, escrito en Rust. Un servidor único con tres clientes posibles:

- **`tm-server`**: API REST + WebSocket (axum + SQLite).
- **`rust-tui-manager`**: cliente de terminal con ratatui (tablero kanban).
- **`apps/desktop`**: cliente gráfico con Tauri 2.

> La versión anterior (gestor local de notas y snippets) está en [`legacy/`](legacy/) y en las versiones 0.1.x del crate.

## Funciones

- Equipos con roles (lector, miembro, admin, dueño) e invitaciones por código.
- Proyectos, tablero kanban, prioridades, fechas límite, asignación, comentarios.
- Etiquetas por equipo y subtareas con progreso.
- Vista de calendario (escritorio) y actualizaciones en tiempo real en todos los clientes.

## Seguridad

Contraseñas con argon2, JWT de 7 días revocables ("cerrar sesión en todos los dispositivos"), bloqueo por cuenta y por IP, límites de tamaño y de cantidad en cada endpoint, CORS restringido, cabeceras de seguridad, token de WebSocket fuera de la URL y TLS opcional.

## Uso

```sh
# servidor
cargo run -p tm-server            # 0.0.0.0:7878, base SQLite en ./tm-server.db

# cliente de terminal (? = ayuda de atajos)
cargo run -p rust-tui-manager -- http://127.0.0.1:7878

# cliente gráfico
cargo run -p tm-desktop
```

Variables del servidor: `TM_ADDR`, `TM_DATABASE`, `TM_JWT_SECRET` (≥ 32 caracteres), `TM_CORS_ORIGINS`, `TM_TLS_CERT` / `TM_TLS_KEY`, `TM_TRUSTED_PROXY=1` (si corre detrás de un proxy inverso propio).

Sin TLS propio, ponelo detrás de un proxy TLS antes de exponerlo a internet.

## Licencia

MIT
