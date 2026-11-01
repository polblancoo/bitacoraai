#!/bin/sh
# Ejemplo de TM_AGENT_CMD para Claude Code con listas de comandos permitidos y prohibidos.
#
# Uso:  TM_AGENT_CMD=/ruta/a/run-claude.sh tm-agent
# tm-agent lo lanza dentro de la carpeta del proyecto y le pasa la tarea en $TM_TASK_PROMPT.
#
# Listas (una por línea; '#' comenta), en la misma carpeta que este script o en $TM_AGENT_CONF:
#   whitelist.txt  comandos que la IA puede ejecutar sin pedir permiso, ej.  Bash(cargo check:*)
#   blacklist.txt  comandos que nunca puede ejecutar,                  ej.  Bash(git push:*)
# A la lista de permitidos se suma TM_ALLOW: lo que una persona habilita desde la app con «/permitir» en un comentario.
# A la lista de prohibidos se suma TM_DENY: la que se edita en la app (ficha de la IA → 🚫 Comandos prohibidos).
# Si la IA necesita algo que no tiene permitido, no lo rodea: te lo pide con `tm-agent ask` (alarma 🙋 + tarea de pedido).
# TM_ALLOW_ALL=1 («Autorización total» en la ficha de la IA): se permite todo Bash y solo rige lo prohibido (la regla de prohibir siempre gana a la de permitir).
# TM_CLAUDE_MODEL (opcional) = modelo de Claude: opus, sonnet o haiku.
# --output-format json: el agente lee de ahí los tokens y el costo reales. TM_AGENT_MAX_USD (opcional) = tope en dólares por ejecución (usá 1 o más: el contexto de Claude Code solo ya cuesta unos centavos).
D="${TM_AGENT_CONF:-$(dirname "$0")}"
lista() { grep -v '^[[:space:]]*\(#\|$\)' "$D/$1" 2>/dev/null | tr '\n' '\t'; }
IFS='	'
set -f
ALLOW=$(lista whitelist.txt)$(printf '%s' "$TM_ALLOW" | grep -v '^[[:space:]]*$' | sed '/(/!s/.*/Bash(&:*)/' | tr '\n' '\t')
[ "$TM_ALLOW_ALL" = "1" ] && ALLOW="Bash$(printf '\t')$ALLOW"
DENY=$(lista blacklist.txt)$(printf '%s' "$TM_DENY" | grep -v '^[[:space:]]*$' | tr '\n' '\t')
exec claude -p "$TM_TASK_PROMPT

REGLAS: no hagas git commit ni git push. No reinicies ni detengas servicios. Si te falta una decisión o un dato, preguntá con: tm-agent ask \"tu pregunta\". Si un comando te es bloqueado o no tenés permiso, NO lo rodees: avisá con tm-agent ask explicando qué comando necesitás y para qué. Verificá tus cambios antes de terminar." \
  --permission-mode acceptEdits ${TM_CLAUDE_MODEL:+--model=$TM_CLAUDE_MODEL} \
  --output-format json ${TM_AGENT_MAX_USD:+--max-budget-usd=$TM_AGENT_MAX_USD} \
  --allowedTools $ALLOW \
  --disallowedTools $DENY
