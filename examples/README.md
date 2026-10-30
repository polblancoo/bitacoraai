# Ejemplos

## `run-claude.sh`: Claude Code con permisos limitados

Lanza a Claude Code por cada tarea que `tm-agent` le asigna, con una lista de comandos **permitidos** y otra de **prohibidos**.

```sh
mkdir -p ~/.config/tm-agent
cp examples/run-claude.sh examples/whitelist.txt examples/blacklist.txt ~/.config/tm-agent/
chmod +x ~/.config/tm-agent/run-claude.sh

TM_URL=https://tu-servidor TM_TOKEN=tmk_… \
TM_AGENT_CMD=$HOME/.config/tm-agent/run-claude.sh tm-agent
```

- `whitelist.txt` y `blacklist.txt`: una regla por línea, `#` comenta (formato de Claude Code: `Bash(comando:*)`).
- A los prohibidos se suma lo que escribas en la app (**🤖 la IA → 🚫 Comandos prohibidos**), que `tm-agent` pasa en `TM_DENY`.
- Si la IA necesita un comando que no tiene permitido, no lo rodea: te lo pide con `tm-agent ask`. Te llega una alarma 🙋 y una tarea de pedido con el comando para copiar; ver el capítulo 10 de la [guía](../docs/GUIA.md).
- Para otra IA (Gemini, Codex…) copiá el script y cambiá la última línea por el comando de tu cliente; verificá sus opciones de permisos.
