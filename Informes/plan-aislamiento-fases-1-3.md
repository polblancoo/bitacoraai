# Plan de aislamiento de las IAs (fases 1 a 3)

> Origen: punto 6 de `analisis-crewhelm-competencia-FODA.md` (la mayor debilidad: las listas de comandos no son un sandbox).
> Decisiones tomadas con Pol (2026-11-06): servidor Linux, un solo usuario de servicio sin root, usuarios **internos** de CrewHelm,
> aislamiento por ejecución con `bwrap` (no Docker, no una cuenta de Linux por persona o IA), git como motor invisible.

## Estructura en disco (modo servidor, `TM_WORKSPACE=/home/crewhelm`)
```
/home/crewhelm/
  homeCrewHelm/<persona o IA>/            espacio personal: sesión del CLI, memoria, intermedios
    <proyecto>/<tarea>/                   lo intermedio de una tarea (no entregable)
  proyectos/<proyecto>/
    tareas/<tarea>/                       copia de trabajo aislada de la tarea (rama tm/tarea-N)
```
Sin `TM_WORKSPACE` (instalación actual) todo sigue como antes: la copia de trabajo vive en `<carpeta del proyecto>/.crewhelm/wt/tarea-N`
y lo intermedio en `<carpeta del proyecto>/.crewhelm/<usuario>/<tarea>`.

## Fase 1 — copia de trabajo por tarea y push desde `tm-agent`
- Al empezar una tarea de un proyecto que es un repositorio git, `tm-agent` crea una **copia aislada** (`git clone --local`, objetos con hardlinks: casi no ocupa)
  y una rama `tm/tarea-N`. La IA trabaja ahí; el repositorio del proyecto no se toca mientras trabaja. Dos tareas en paralelo no se pisan.
- La IA **no commitea ni sube**: al terminar, `tm-agent` (fuera de cualquier sandbox y con las credenciales del servidor) hace `git add -A`, `commit`,
  trae la rama al repositorio del proyecto y hace `push` al remoto `origin` si existe (GitHub u otro). Comenta en la tarea qué subió.
- Se usa cuando el proyecto tiene «Pull Request» activado, o cuando la IA tiene aislamiento activado en su ficha.
- Si la tarea tiene tarea madre con rama propia, la copia sale de esa rama (base para «integrar» subtareas).
- Limpieza: copias y espacios de tareas terminadas hace más de 30 días se borran, con aviso en la tarea 7 días antes.

## Fase 2 — sandbox por ejecución (`bwrap`)
Cada ejecución de la IA corre dentro de una cápsula que solo ve:
- el sistema (`/usr`, `/bin`, `/lib*`, `/etc`, `/opt`) en **solo lectura**;
- la copia de trabajo de la tarea y el espacio personal de la IA, **escribibles**;
- las herramientas que se listen (por defecto `~/.cargo`, `~/.rustup` si existen; sin `credentials.toml`) y la sesión del CLI de la IA;
- `/tmp` propio y vacío, un `HOME` propio, procesos propios.
No ve: tu home, el espacio de otras IAs, la base de datos del servidor, `~/.ssh`, tokens.
Si el aislamiento está activado y no hay `bwrap`, la tarea **no corre** (falla cerrado).
Se activa por IA en su ficha («Aislamiento»), o para todas con `TM_SANDBOX=bwrap`.

## Fase 3 — red con lista de destinos
La cápsula no tiene red propia (`--unshare-net`). Sale solo por dos puentes hacia sockets Unix del `tm-agent` (fuera de la cápsula):
- **proxy HTTPS filtrado**: solo `CONNECT` a destinos de la lista (API del modelo, GitHub, registros de paquetes + lo que agregue un admin);
  no permite direcciones privadas; lo bloqueado queda en un registro y se informa en la tarea;
- **puente al servidor CrewHelm** para `tm-agent ask` y `verdict`.
Si una herramienta ignora el proxy, falla sin red (falla cerrado).

## Qué garantiza y qué no
- Garantiza: la IA no lee ni escribe fuera de lo que se le dio, no manda datos a destinos no listados (con HTTPS por proxy), no toca el repositorio central mientras trabaja.
- No garantiza: protección frente a una falla del kernel de Linux; tráfico que no sea HTTPS/proxy-aware sale bloqueado, no filtrado por contenido.
- Lo que sale de la cápsula es solo el diff de la rama (revisable) y `out`.

## Pendiente (siguientes pasos)
- «Integrar» una subtarea en la rama de su tarea madre con un botón y revisión de diff dentro de la tarea.
- Cuotas de disco y límites de CPU/memoria por ejecución (`systemd-run --user`).
- Driver Docker para servidores no Linux.

## Estado (2026-11-06) — fases 1, 2 y 3 implementadas
Código: `crates/agent/src/iso.rs` (copia de trabajo, `bwrap`, proxy y puente), integración en `crates/agent/src/main.rs`;
servidor: campo `isolation` por IA y `net_allow` general (`crates/server/src/bots.rs`); interfaz: botón «🛡 Aislamiento» en la ficha de la IA y destinos de red en
Configuración → «Comandos permitidos y prohibidos para todas las IAs».

**Cómo se activa**
- Por IA: ficha de la IA → «🛡 Aislamiento» → *Copia de trabajo por tarea* o *Sandbox*. Vale desde la próxima tarea.
- Para todas las IAs de un agente: variables `TM_SANDBOX=bwrap` o `TM_WORKTREE=1`.
- Modo servidor: `TM_WORKSPACE=/home/crewhelm` (espacios personales en `homeCrewHelm/<IA>/`, copias en `proyectos/<proyecto>/tareas/<id>`).
  Con `TM_WORKSPACE` la IA usa su propio `HOME`; la primera vez se inicia sesión de su CLI con `TM_IA=<nombre> tm-agent sandbox-shell`.
- Otras variables: `TM_SANDBOX_NET=open|proxy|none`, `TM_SANDBOX_NET_ALLOW=dominio1,dominio2`, `TM_SANDBOX_RW` y `TM_SANDBOX_RO` (rutas extra, separadas por comas).

**Verificado** (pruebas automáticas y una ejecución completa contra un servidor de prueba)
- Copia aislada en rama `tm/tarea-N`, commit a nombre de la IA, rama llevada al repositorio del proyecto y subida al remoto; `main` intacto; segunda vuelta sin cambios no repite.
- La cápsula no ve `~/.ssh` ni el home ni la base del servidor; `/usr` y `/etc` de solo lectura; el proxy deja pasar GitHub y bloquea lo no listado, y lo bloqueado se informa en la tarea.
- El CLI real de Claude (`claude -p`, modelo haiku) funciona dentro de la cápsula, con la red por el proxy.
- Si el aislamiento está pedido y falta `bwrap`, la tarea no corre y vuelve a «Por hacer» con el motivo.

**Límites conocidos**
- Sin `TM_WORKSPACE`, la cápsula monta la sesión real del CLI (`~/.claude`…) y las herramientas (`~/.cargo` sin `credentials.toml`, `~/.nvm`…): la IA puede leerlas. Con `TM_WORKSPACE` cada IA tiene su propia sesión.
- Solo se filtra HTTPS por `CONNECT` al puerto 443; el HTTP plano y SSH quedan bloqueados (no filtrados). Una herramienta que ignore el proxy no tendrá red.
- No hay límite de CPU/memoria ni cuota de disco por ejecución.
- «Integrar» una subtarea en la rama de su tarea madre: la subtarea ya sale de la rama de la madre, falta el botón de integración y la vista de diff.
- La limpieza (aviso a los 23 días, borrado a los 30) cubre las copias y los espacios de las IAs; los espacios de personas no tienen proceso de limpieza todavía.
- Mac y Windows: sin sandbox (haría falta el driver Docker).

## Fase 4 — espacios personales para personas (2026-11-06)
- Con `TM_WORKSPACE`, la terminal web usa `homeCrewHelm/<persona>/` como `HOME`: ahí persisten su `.bashrc`, herdr y lo que guarde. Es solo suyo.
- Owners: shell completa con su espacio como HOME (las herramientas del servicio siguen en el PATH).
- Miembros: pueden abrir la terminal solo si el agente del proyecto tiene `TM_WORKSPACE`; corre en cápsula `bwrap` (su espacio escribible, el proyecto en solo lectura, sin el home real). Sin `bwrap` no abre.
- Por SSH directo a la máquina del proyecto siguen entrando solo los owners.
- Preferencias de terminal (`autoHerdr`) guardadas por cuenta en el servidor (`/api/me/term-prefs`).
- Admin (owner): selector «👁 Ver espacio de…» en Terminales; abre una cápsula con el espacio de esa persona o IA en **solo lectura** (queda registrado en el log del servidor).
- Limpieza: el barrido del agente también recorre `homeCrewHelm/*/<proyecto>/<tarea>` (personas y demás IAs), con el mismo aviso a los 23 días y borrado a los 30.
- **SSH por persona:** cada persona carga su clave pública en su perfil («🔑 Entrar por SSH», hasta 5). Con `TM_SSH_KEYS=1` (y `TM_WORKSPACE`), `tm-agent` escribe cada minuto un bloque propio en `~/.ssh/authorized_keys` del usuario de servicio. Cada clave lleva `restrict,pty,command="tm-agent ssh-session <persona> <nivel>"`: no abre túneles, agente ni X11, y rechaza comandos sueltos (`ssh host comando`).
  Owner → shell con su espacio como HOME; miembro → cápsula `bwrap` (no ve `~/.ssh`, home real ni a otras personas). El nivel sale del rol en los equipos del agente.
  Conectarse: `ssh <usuario-de-servicio>@<servidor>` con la clave privada correspondiente. Las líneas ajenas de `authorized_keys` no se tocan.

### Activación desde la app (sin consola)
La terminal para las personas y la entrada por SSH se prenden en «⚙ Conexión» del proyecto (solo owners), con dos interruptores:
«Permitir la terminal a las personas del equipo» y «También permitir entrar por ssh con su clave».
El agente consulta `/api/agent/term/config` cada 10 s y se activa solo; no hacen falta `TM_AGENT_TERMINAL`, `TM_WORKSPACE` ni `TM_SSH_KEYS` (siguen valiendo como alternativa).
La carpeta de espacios por defecto es `~/crewhelm` del usuario que corre `tm-agent`. Los espacios y copias de trabajo de las IAs usan `TM_WORKSPACE` si está fijado (si no, siguen dentro de la carpeta de cada proyecto).
