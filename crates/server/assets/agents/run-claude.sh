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
# TM_ALLOW_ALL=1 («Autorización total» en la ficha de la IA): se permiten todas las herramientas (Bash, WebSearch, WebFetch, archivos…) y solo rige lo prohibido (la regla de prohibir siempre gana a la de permitir).
# TM_LISTS_FROM_APP=1: las listas se manejan desde la app (ficha de la IA) y whitelist.txt / blacklist.txt se ignoran.
# TM_CLAUDE_MODEL (opcional) = modelo de Claude: opus, sonnet o haiku.
# --output-format json: el agente lee de ahí los tokens y el costo reales. TM_AGENT_MAX_USD (opcional) = tope en dólares por ejecución (usá 1 o más: el contexto de Claude Code solo ya cuesta unos centavos).
D="${TM_AGENT_CONF:-$(dirname "$0")}"
lista() { [ "$TM_LISTS_FROM_APP" = "1" ] && return; grep -v '^[[:space:]]*\(#\|$\)' "$D/$1" 2>/dev/null | tr '\n' '\t'; }
IFS='	'
set -f
ALLOW=$(lista whitelist.txt)$(printf '%s' "$TM_ALLOW" | grep -v '^[[:space:]]*$' | sed '/(/!{/^\(Read\|Edit\|Write\|Glob\|Grep\|WebFetch\|WebSearch\|NotebookEdit\|Skill\|Agent\|Bash\|mcp__[^ ]*\)$/!s/.*/Bash(&:*)/}' | tr '\n' '\t')
[ "$TM_ALLOW_ALL" = "1" ] && ALLOW="$(printf 'Bash\tWebSearch\tWebFetch\tRead\tEdit\tWrite\tGlob\tGrep\tNotebookEdit\tAgent\tTask\tSkill\tTodoWrite\tTaskStop\t')$ALLOW"
DENY=$(lista blacklist.txt)$(printf '%s' "$TM_DENY" | grep -v '^[[:space:]]*$' | tr '\n' '\t')
# Sin autorización total, la IA no commitea ni sube; con autorización total (o si lo sacás de Prohibidos en la app) puede hacerlo
GITRULE="no hagas git commit ni git push. "
[ "$TM_ALLOW_ALL" = "1" ] && GITRULE=""
# Proyecto con «Pull Request»: la IA puede commitear y subir SOLO su rama tm/tarea-ID (main y otras ramas siguen prohibidas)
if [ -n "$TM_PR_BRANCH" ]; then
  GITRULE="trabajá en la rama $TM_PR_BRANCH, hacé commit de tus cambios y subila con git push -u origin $TM_PR_BRANCH; nunca hagas push a main ni a otra rama. "
  DENY=$(printf '%s' "$DENY" | tr '\t' '\n' | grep -vxF -e 'Bash(git push:*)' -e 'Bash(git commit:*)' -e 'Bash(git commit -m:*)' -e 'Bash(git checkout:*)' | tr '\n' '\t')
  ALLOW="$ALLOW$(printf 'Bash(git checkout -b %s:*)\tBash(git add:*)\tBash(git commit:*)\tBash(git push -u origin %s:*)\t' "$TM_PR_BRANCH" "$TM_PR_BRANCH")"
fi
exec claude -p "$TM_TASK_PROMPT

REGLAS: ${GITRULE}No reinicies ni detengas servicios. Si te falta una decisión o un dato, preguntá con: tm-agent ask \"tu pregunta\". Si un comando te es bloqueado o no tenés permiso, NO lo rodees: avisá con tm-agent ask explicando qué comando necesitás y para qué. Verificá tus cambios antes de terminar." \
  --permission-mode acceptEdits ${TM_CLAUDE_MODEL:+--model=$TM_CLAUDE_MODEL} \
  --output-format json ${TM_AGENT_MAX_USD:+--max-budget-usd=$TM_AGENT_MAX_USD} \
  --allowedTools $ALLOW \
  --disallowedTools $DENY
