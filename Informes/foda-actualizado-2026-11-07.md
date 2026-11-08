# FODA de CrewHelm actualizado y tabla de diferencias (2026-11-07)

> Parte del informe del 2026-11-06 (`analisis-crewhelm-competencia-FODA.md`, tarea #105). La competencia no se volvió a investigar en la web: sus datos siguen con la misma salvedad (precios y funciones de terceros sin confirmar en páginas oficiales). Lo que cambió es **CrewHelm**: se cerraron varias brechas que ese informe marcaba.

## 1. Qué cambió desde el informe anterior

| Brecha marcada el 2026-11-06 | Estado hoy |
|---|---|
| Listas de comandos que no son sandbox | **Cerrada en Linux**: cápsula bwrap por persona e IA, red con lista de destinos, copia de trabajo por tarea con rama y push |
| Sin revisión de diffs ni integración | **Cerrada a nivel proyecto**: 🔀 Integrar (ver cambios, integrar, devolver, aviso a quien envió). Falta verlo dentro de la tarea |
| Sin cuotas por persona | **Cerrada**: disco, memoria, CPU y procesos por persona, panel del administrador/CTO, uso por persona |
| Git difícil para no programadores | **Resuelta**: barra de botones con tooltips de git, comando `cw`, PR sin cuenta de GitHub, asistente de repositorio |
| Instalar en la máquina de otra persona | **Parcial**: `install.sh --client` (solo Linux) e invitación lista para pegar |
| Instaladores macOS/Windows, sandbox ahí | Sin hacer |
| Auditoría exportable, corte duro de presupuesto y alertas al 80 % | **Cerradas (2026-11-08)**: CSV/JSON del equipo, corte antes de cada ejecución y aviso al 80 % |
| `tm-mcp`, SSO, Postgres | Sin hacer |

## 2. FODA de CrewHelm (hoy)

**Fortalezas**
- Gobierno de IAs: permisos por comando, aislamiento real (bwrap), cuotas de disco/memoria/CPU, costos y cuotas de gasto, todo en un tablero de equipo.
- La IA pide ayuda a una persona (web, Telegram, email) y las personas trabajan en su propio espacio con git simplificado.
- Una persona no programadora puede enviar trabajo a revisión sin cuenta de GitHub; el owner lo revisa e integra sin salir de la app.
- Autoalojado, un binario + SQLite, sin licencia por usuario; independiente del proveedor de IA.
- Tres clientes, tres idiomas, manual integrado.

**Oportunidades**
- Nicho de «gobierno de IAs para equipos» que ningún competidor revisado junta con tablero.
- Empresas que no pueden usar Devin/Copilot por datos; regulación que pide trazabilidad y supervisión humana.
- Vibe Kanban con futuro incierto y AutoGen en mantenimiento: usuarios buscando alternativa.
- Modelos locales y MCP como canal de descubrimiento.
- Licencia comercial ya definida.

**Debilidades**
- Sin tracción: un autor, sin comunidad ni referencias públicas; repositorio privado (el instalador público hoy da 404).
- Aislamiento solo en Linux con bwrap; en macOS/Windows no hay equivalente. Los límites de memoria/CPU dependen de systemd; la cuota de disco no es un tope duro.
- Sin SSO, sin 2FA, sin revisión de seguridad externa.
- SQLite limita escala y alta disponibilidad.
- Mensaje amplio (tareas, procesos, código, contenido).
- La mayor parte de lo nuevo se probó con repos y usuarios de prueba; falta uso real sostenido de varios miembros.

**Amenazas**
- GitHub Copilot agent (distribución masiva, precio bajo) y Plane AI si agrega permisos y agentes de código.
- Vibe Kanban si suma multiusuario; los CLI (Claude Code, Codex) con subagentes y permisos propios.
- Un incidente de seguridad con IAs ejecutando comandos.
- Cambios de términos de los CLI de terceros.
- La ventana de tiempo: lo que hoy es ventaja de funciones se puede copiar en 6 a 12 meses.

## 3. Tabla de diferencias: funciones a agregar y por qué

| # | Función | Hoy | Quién la tiene | Por qué agregarla | Prioridad |
|---|---|---|---|---|---|
| 1 | ✅ **Auditoría exportable** (quién/qué IA ejecutó qué, aprobaciones, costo; CSV/JSON) | Hecha | Devin y Copilot (empresa) | Es el argumento de venta ante regulación; convierte «gobierno» en algo demostrable | Alta |
| 2 | **`tm-mcp` terminado y publicado** | En desarrollo | Vibe Kanban, Plane, OpenHands | Canal de descubrimiento más barato; deja que otras IAs usen CrewHelm | Alta |
| 3 | ✅ **Presupuesto con corte duro y alertas al 80 %** | Hecho | Ninguno lo junta con tablero | Evita sorpresas de gasto; refuerza «costos por IA» | Alta |
| 4 | **Seguridad: 2FA, `SECURITY.md`, revisión externa** | Pendiente | Todos los SaaS serios | Sin esto no se puede vender a empresas; un incidente sería fatal para un proyecto joven | Alta |
| 5 | **Diffs dentro de la tarea** (hoy solo en 🔀 Integrar) | A nivel proyecto | Vibe Kanban, Devin | La IA entrega por tarea; revisar donde se trabaja ahorra pasos | Media |
| 6 | **Aprobaciones por riesgo** (borrar, desplegar, gastar sobre X) | Solo listas de comandos | Ninguno claro | Acerca el control a lo que una empresa realmente teme | Media |
| 7 | **Cuota de disco dura** (partición o cuota del sistema) | Medición cada 90 s | Los contenedores la traen de fábrica | Cierra la limitación real que quedó | Media |
| 8 | **Modelos locales probados** con receta | Sin probar | OpenHands | «Nada sale de tu infraestructura» abre sector público y salud | Media |
| 9 | **Enrutado y fallback entre IAs** y comparación de costo/rendimiento | Manual | Frameworks (en código) | Ahorra gasto y da datos para elegir IA | Media |
| 10 | **Plantillas de proyecto y flujo**; disparadores externos (webhook, issue, correo, cron) | Mayormente manual | Linear, Plane | Baja la barrera de entrada para equipos no técnicos | Media |
| 11 | **Instalador y cliente de Windows/macOS**, sandbox equivalente | Solo Linux | Todos los competidores de escritorio | Los miembros no técnicos suelen usar Windows; hoy entran por navegador | Media |
| 12 | **Docker como alternativa a bwrap** | Sin hacer | OpenHands, Devin | Aislamiento en máquinas sin bwrap y portabilidad | Media |
| 13 | **SSO (OIDC/SAML)** y roles finos | Usuarios locales | Todos los de empresa | Requisito habitual de compra | Baja |
| 14 | **Postgres opcional** | SQLite | Plane, Linear | Escala y alta disponibilidad, sin quitar la instalación simple | Baja |
| 15 | **Memoria del proyecto** consultable por las IAs y **evaluaciones** de calidad por IA | Historial | Plane AI (Skills) | Mejora resultados y da un argumento medible | Baja |
| 16 | **Hacer público el repositorio y una demo corta** | Privado | Todos | Sin repo público no hay instalador, estrellas ni comunidad: es la brecha comercial más grande | Alta |

## 4. Conclusión

El diferenciador sigue siendo el **gobierno**: tablero de equipo + IA con permisos, aislamiento, cuotas y costos + autoalojado. Lo técnico que faltaba para sostener esa promesa en Linux ya está. Lo que falta para ganar el nicho es menos código y más **credibilidad**: repositorio público, auditoría exportable, seguridad revisada, `tm-mcp` y una demo. La ventana es de meses, no de años.
