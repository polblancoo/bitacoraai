# Análisis de CrewHelm y la competencia (orquestadores de IA) — FODA

> Tarea #105 · 2026-11-06 · Actualiza `Investigacion/investigacionCrewHelm.md` (2026-11-01), ahora con foco en **orquestadores de IA**, no solo gestores de tareas.
>
> **Fiabilidad de los datos:** lo de CrewHelm sale del README y del repositorio. Lo de la competencia sale de búsquedas web (agregadores y blogs de comparación, que a veces se contradicen en precios) y de la investigación del 2026-11-01 (estrellas verificadas con `gh api`). Lo marcado ❓ es conocimiento previo sin confirmar. **Verificar precios en las páginas oficiales antes de citarlos.**

## 1. Mapa del mercado

El mercado de "orquestar IAs" son en realidad **cuatro categorías distintas**. CrewHelm toca las cuatro por los bordes, pero no es igual a ninguna:

| Categoría | Ejemplos | Qué resuelven |
|---|---|---|
| A. Frameworks para construir agentes (código Python/TS) | LangGraph, CrewAI, AutoGen/AG2 (Microsoft lo pasó a mantenimiento en oct-2025), OpenAI Agents SDK | Programar flujos multiagente. Son **kits para desarrolladores**, no plataformas con interfaz de equipo |
| B. Agentes de código autónomos (SaaS/abiertos) | Devin, OpenHands, GitHub Copilot coding agent | Una IA que toma un ticket y entrega un PR |
| C. Orquestadores de agentes de código locales | Vibe Kanban, claude-squad | Lanzar varios agentes (Claude Code, Codex, Gemini CLI) en paralelo con worktrees |
| D. Gestores de proyectos con IA | Plane (Plane AI), Linear (Linear Agent), Jira/Asana | Tablero de equipo al que se le suma IA |

## 2. FODA de cada competidor

### 2.1 LangGraph / CrewAI / AutoGen (frameworks)
| | |
|---|---|
| **Fortalezas** | Flexibilidad total; comunidades enormes; LangGraph fuerte en flujos con estado; CrewAI fácil de arrancar con "roles" |
| **Oportunidades** | Ser la base de la mayoría de los agentes empresariales; plataformas gestionadas (LangSmith, CrewAI Enterprise) |
| **Debilidades** | No traen interfaz de equipo, permisos, costos por persona ni tablero: hay que construirlo; curva de aprendizaje; LangGraph es el más lento para el primer resultado; AutoGen fragmentado (3 ramas) |
| **Amenazas** | Los proveedores de modelos (OpenAI, Anthropic, Google) ofrecen SDK de agentes propios; el estándar MCP reduce el valor de los frameworks |

**Contra CrewHelm:** no compiten de frente. Son para quien *programa* agentes; CrewHelm es para quien *gestiona* equipos que ya usan agentes. Son más bien un posible **proveedor** (un agente hecho con LangGraph puede correr bajo `tm-agent`).

### 2.2 Devin (Cognition)
| | |
|---|---|
| **Fortalezas** | Máxima autonomía (tarea → PR); integraciones Slack/GitHub/Linear; planes Enterprise con VPC dedicada y SOC 2 Tipo II |
| **Oportunidades** | Presupuestos empresariales de ingeniería; cada vez más ejecución en paralelo |
| **Debilidades** | El más caro (precios dispersos según fuentes: entrada ~US$20 y Teams ~US$500/mes); cerrado y atado a un proveedor/modelo; la IA es un producto, no un miembro de *tu* equipo con *tus* reglas |
| **Amenazas** | Copilot agent y agentes de código abiertos lo comoditizan por precio |

### 2.3 OpenHands (ex OpenDevin)
| | |
|---|---|
| **Fortalezas** | Abierto y autoalojable; auditable; sin costo de plataforma (se paga la API del modelo); la versión Cloud suma Slack/Jira/Linear, multiusuario, RBAC y conversaciones compartidas |
| **Oportunidades** | Ser el "Devin abierto" para empresas que exigen control de datos |
| **Debilidades** | Cloud todavía madurando según comparaciones; es un agente, no un tablero de gestión de proyectos; sin costos/cuotas por proyecto ❓ |
| **Amenazas** | Copilot agent (integrado donde ya están los equipos) y Devin (más pulido) |

### 2.4 GitHub Copilot coding agent
| | |
|---|---|
| **Fortalezas** | Está donde ya vive el código y el equipo (issues, PR, Actions); precio bajo (Pro ~US$10, Pro+ ~US$39; fuentes hablan de créditos por uso desde junio 2026, sin confirmar); distribución enorme |
| **Oportunidades** | Convertirse en el centro de agentes de varias marcas dentro de GitHub ❓ |
| **Debilidades** | Atado a GitHub como fuente de verdad; límites de "premium requests" (300/1.500 al mes, sobrecosto ~US$0,04); no es autoalojado; poco control fino de comandos por agente |
| **Amenazas** | Para él: modelos y CLI de terceros más baratos. Para CrewHelm: **es la mayor amenaza de distribución** |

### 2.5 Vibe Kanban (BloopAI) — el más parecido
| | |
|---|---|
| **Fortalezas** | ~28,5 mil estrellas (Apache-2.0, también Rust); worktrees por tarea, revisión de diffs, PR; soporta Claude Code, Gemini CLI, Codex, Amp; servidor MCP |
| **Oportunidades** | Estándar de facto para "kanban de agentes de código" |
| **Debilidades** | Pensado para un desarrollador o equipo chico; sin roles, permisos por comando, pedidos de ayuda ni costos (❓ a verificar); **riesgo de continuidad** (una fuente dice que la empresa cerró en abril 2026; el repo siguió activo hasta 2026-10-29) |
| **Amenazas** | Que sume multiusuario y permisos; o que quede sin mantenedor |

### 2.6 claude-squad
| | |
|---|---|
| **Fortalezas** | Ligero (TUI + tmux + worktrees); ~8,7 mil estrellas; muy fácil de usar |
| **Oportunidades** | Quienes quieren algo mínimo sin servidor |
| **Debilidades** | Es un gestor de sesiones: sin equipo, usuarios, historial compartido ni tablero; sin commits desde agosto 2026 |
| **Amenazas** | Vibe Kanban y las funciones nativas de los propios CLI (Claude Code ya hace subagentes y worktrees) |

### 2.7 Plane (Plane AI)
| | |
|---|---|
| **Fortalezas** | ~61,8 mil estrellas; producto completo (ciclos, módulos, páginas); empresa detrás; Plane AI (agentes, Autopilot, Skills desde v2.4.0), servidor MCP, claves propias OpenAI/Anthropic |
| **Oportunidades** | Es la alternativa abierta a Jira/Linear con IA integrada: amenaza seria desde los gestores clásicos |
| **Debilidades** | Despliegue pesado (varios servicios); la IA en autoalojado es comercial; no gobierna agentes de código (sin permisos por comando, sin terminal, sin costo por IA) |
| **Amenazas** | Si agrega permisos por agente y ejecución de código, cubre el nicho de CrewHelm con mucho más alcance |

### 2.8 Linear (Linear Agent)
| | |
|---|---|
| **Fortalezas** | Producto muy pulido; el agente hace triage y redacta especificaciones; barato (Free ≤10 usuarios, Standard ~US$8/usuario); lo usan Devin y otros como fuente de tickets |
| **Oportunidades** | Ser el tablero donde "se asigna" a agentes de terceros |
| **Debilidades** | Su agente no escribe código; SaaS cerrado, datos en la nube ajena; costo por usuario |
| **Amenazas** | Competidores abiertos (Plane) y suites que lo igualan |

## 3. FODA de CrewHelm

| | |
|---|---|
| **Fortalezas** | • **IA como miembro del equipo**: rol, token revocable, listas de comandos permitidos/prohibidos, permiso concedido desde la tarea (`/permitir`).<br>• **Pedido de ayuda de la IA a una persona** (`tm-agent ask`) por web, Telegram y email.<br>• **Costos, tokens, horas y cuotas** por proyecto/IA/tarea, con estimación al crear.<br>• **Terminal dentro de la tarea** (SSH o vía agente), con rastro de lo hecho.<br>• **Independiente del proveedor** (Claude, Gemini, Codex, cualquier CLI) y una IA compartida por todo el equipo.<br>• **Autoalojado y liviano**: un binario + SQLite, sin licencia por usuario.<br>• Tres clientes (TUI, escritorio, web), tiempo real, 3 idiomas, manual integrado.<br>• Flujo por etapas con IA revisora (`verdict`), integración GitHub/PR, portfolio multiproyecto.<br>• Usos más allá del código (procesos, contenido, presupuestos) |
| **Oportunidades** | • Nicho vacío: **gobierno de IAs para equipos** (control, auditoría, costos) que nadie junta con tablero.<br>• Empresas con restricciones de datos que no pueden usar Devin/Copilot (sector público, salud, finanzas).<br>• Incertidumbre sobre Vibe Kanban y AutoGen en mantenimiento: usuarios buscando alternativa.<br>• Auge de MCP: terminar `tm-mcp` abre directorios y clientes.<br>• Modelos locales (Ollama/llama.cpp): "nada sale de tu infraestructura".<br>• Regulación (ej. AI Act) que exige trazabilidad y supervisión humana de agentes: justo lo que ofrece CrewHelm.<br>• Licencia comercial ya definida (0.7.0): hay camino de ingresos |
| **Debilidades** | • **Sin tracción**: 0 estrellas, un solo autor, sin comunidad ni ecosistema.<br>• Instalador solo Linux x86_64; Windows/macOS sin probar.<br>• Modelos locales sin probar; `tm-mcp` en desarrollo.<br>• Las listas de comandos las **aplica el agente**, no un sandbox: no es barrera dura en máquinas no controladas.<br>• SQLite: límite de escala/alta disponibilidad.<br>• Sin auditoría de seguridad, certificaciones (SOC 2) ni soporte comercial.<br>• Mensaje demasiado amplio (tareas, procesos, código, contenido).<br>• Sin worktrees/aislamiento por tarea ni revisión de diffs integrada (Vibe Kanban sí) ❓ |
| **Amenazas** | • **GitHub Copilot agent** y Agent-style hubs: distribución masiva y precio bajo.<br>• **Plane AI** agregando permisos y agentes de código.<br>• Vibe Kanban sumando multiusuario/permisos.<br>• Los propios CLI (Claude Code, Codex) incorporando subagentes, permisos y colas.<br>• Un incidente de seguridad (IA ejecutando comandos) mataría la confianza de un proyecto joven.<br>• Dependencia de que los CLI de terceros mantengan su interfaz y términos de uso (p. ej. uso de suscripciones en automatizaciones) |

## 4. Contraste con la competencia

Leyenda: ✅ sí · ◐ parcial · ❌ no · ❓ sin confirmar.

| Capacidad | CrewHelm | Vibe Kanban | claude-squad | Devin | OpenHands | Copilot agent | Plane AI | Linear Agent | LangGraph/CrewAI |
|---|---|---|---|---|---|---|---|---|---|
| Autoalojado | ✅ | ✅ | ✅ | ❌ (VPC en Enterprise) | ✅ | ❌ | ✅ (IA de pago) | ❌ | ✅ (es código) |
| Tablero de equipo humanos+IA | ✅ | ◐ | ❌ | ❌ | ◐ | ◐ (GitHub) | ✅ | ✅ | ❌ |
| Agnóstico de modelo/CLI | ✅ | ✅ | ✅ | ❌ | ✅ | ◐ | ✅ (claves propias) | ◐ | ✅ |
| Permisos por comando por IA | ✅ | ❓ | ◐ | ❌ | ◐ | ❌ | ❌ | ❌ | ❌ |
| IA pide ayuda a una persona | ✅ | ❓ | ❌ | ◐ (Slack) | ❓ | ◐ | ❌ | ❌ | ◐ (human-in-the-loop, en código) |
| Costos y cuotas por proyecto/IA | ✅ | ❌ | ❌ | ◐ (facturación propia) | ❌ | ◐ | ❌ | ❌ | ❌ |
| Terminal en la tarea | ✅ | ◐ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Worktrees/aislamiento y revisión de diffs | ❌ | ✅ | ✅ (worktree) | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Servidor MCP | ◐ (en desarrollo) | ✅ | ❌ | ❓ | ✅ ❓ | ✅ ❓ | ✅ | ✅ ❓ | ◐ |
| Integración GitHub/PR | ✅ | ✅ | ◐ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Multiusuario con roles | ✅ | ◐ | ❌ | ✅ | ✅ (Cloud) | ✅ | ✅ | ✅ | ❌ |
| Despliegue simple | ✅ | ◐ | ✅ | n/a | ◐ | n/a | ◐ | n/a | ❌ |
| Comunidad y madurez | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Certificaciones / soporte | ❌ | ❌ | ❌ | ✅ | ◐ | ✅ | ✅ | ✅ | ◐ |

## 5. Conclusión: nuestro FODA frente al mercado

1. **Posición real.** CrewHelm no gana en autonomía (Devin), en distribución (Copilot), en pulido (Linear) ni en comunidad (Plane, Vibe Kanban). **Gana en la combinación**: tablero de equipo + IA con permisos por comando + pedido de ayuda + costos/cuotas + autoalojado + agnóstico de proveedor. Ningún competidor revisado junta esas cinco cosas.
2. **El diferenciador es "gobierno", no "tareas".** Hay que dejar de venderse como gestor de tareas con IA. El mensaje defendible: *"el panel de control para que un equipo use varias IAs con reglas, permisos, costos y trazabilidad, en tu propio servidor"*.
3. **El mayor riesgo es la ventana de tiempo.** Plane AI, Vibe Kanban y los propios CLI se mueven hacia el mismo terreno. Hoy la ventaja es de funciones; si no hay tracción y credibilidad (seguridad, usuarios reales) en 6 a 12 meses, será copiable.
4. **La mayor debilidad técnica con peso comercial** es que las listas de comandos no son un sandbox. Para vender "control" a empresas, esto hay que cerrarlo: es la brecha entre lo que prometemos y lo que garantizamos.
5. **Frente a los frameworks (LangGraph/CrewAI)** no hay que competir: conviene ser el lugar donde esos agentes se **gobiernan y miden**.

## 6. Funciones recomendadas para anexar (priorizadas)

### Prioridad alta (cierran brechas frente a la competencia y sostienen el mensaje de "control")
1. **Ejecución aislada por tarea**: worktree de git por tarea + contenedor/sandbox opcional (Docker/bubblewrap) que haga cumplir las listas de comandos de verdad. Cierra la mayor debilidad y alcanza a Vibe Kanban/Devin.
2. **Revisión de diffs dentro de la tarea** (archivos cambiados, aprobar/rechazar, abrir PR desde ahí). Hoy hay flujo por etapas y PR, pero conviene ver el cambio sin salir del tablero.
3. **`tm-mcp` terminado** (crear/leer tareas, pedir ayuda, reportar costo) y publicado en directorios MCP: es el canal de descubrimiento más barato.
4. **Registro de auditoría exportable** (quién/qué IA ejecutó qué comando, aprobaciones, costo), en CSV/JSON: argumento de venta ante regulación y auditoría.
5. **Presupuestos con corte automático**: al llegar a la cuota, la IA se pausa y pide ampliación (ya hay cuotas y estimación; falta el freno duro y las alertas al 80 %).

### Prioridad media (diferenciación)
6. **Aprobaciones por riesgo**: política que exija visto bueno humano para acciones sensibles (borrar, desplegar, tocar producción, gastar por encima de X), no solo listas de comandos.
7. **Modelos locales probados** (Ollama/llama.cpp) con una receta y una plantilla lista: "nada sale de tu infraestructura".
8. **Enrutado/fallback entre IAs**: asignar por tipo de tarea (barata para triage, potente para código) y reintentar con otra IA si falla; comparar rendimiento y costo por IA.
9. **Plantillas de proyecto y de flujo** (desarrollo, contenido, presupuestos, auditoría) con IAs y etapas preconfiguradas.
10. **Disparadores externos**: webhook, issue de GitHub, correo o cron que cree tareas y las asigne a una IA (hoy es sobre todo manual).
11. **Ejecución en paralelo con dependencias** entre tareas (una IA espera el resultado de otra).

### Prioridad baja / a medio plazo
12. **Instaladores macOS y Windows**, y pruebas de la terminal por agente en Windows.
13. **Postgres opcional** para escala y alta disponibilidad, manteniendo SQLite por defecto.
14. **SSO (OIDC/SAML)** y roles más finos para empresas.
15. **Memoria/contexto por proyecto** que las IAs consulten (decisiones previas, convenciones), complementa el historial actual.
16. **Evaluaciones simples** de calidad de IA (tasa de tareas aprobadas a la primera, devueltas por la revisora) en el portfolio.
17. **Revisión de seguridad externa** antes de promover uso empresarial; luego `SECURITY.md` y, a futuro, SOC 2 si hay clientes que lo pidan.

## 7. Recomendación estratégica (resumen)
- **Ahora (1-2 meses):** aislamiento por tarea (1), `tm-mcp` (3), diffs en la tarea (2); README con demo corta y mensaje de "gobierno de IAs".
- **Después (3-6 meses):** auditoría exportable (4), presupuestos con corte (5), aprobaciones por riesgo (6), modelos locales (7); revisión de seguridad.
- **Cuidado con:** ampliar al "todo sirve" (procesos, contenido…) antes de ganar el nicho de equipos que usan IAs de código; y con prometer control que el agente no puede garantizar (ver 1).

## 8. Pendiente de verificar
- Funciones de Vibe Kanban sobre permisos, ayuda y costos; si Bloop cerró.
- Precios actuales de Devin, Copilot (créditos por uso) y Linear: las fuentes se contradicen.
- Alcance exacto de MCP en OpenHands, Copilot agent y Linear.
- Si CrewHelm ya tiene aislamiento por worktree (no lo encontré en el README).

## 9. Fuentes
- `Investigacion/investigacionCrewHelm.md` (GitHub API de los repos, 2026-11-01).
- README de CrewHelm.
- Búsquedas web (agregadores, de fiabilidad media): [futureagi — orquestación 2026](https://futureagi.com/blog/best-ai-agent-orchestration-platforms-2026), [n-ix — LangGraph vs CrewAI vs AutoGen](https://www.n-ix.com/langgraph-vs-crewai-vs-autogen/), [firstaimovers — guía CTO](https://radar.firstaimovers.com/langgraph-vs-langchain-crewai-autogen-2026), [toolhalla — Devin vs OpenHands](https://toolhalla.ai/blog/devin-vs-openhands-vs-swe-agent-2026), [aiagentrank — Devin vs Copilot](https://aiagentrank.io/compare/devin-vs-github-copilot), [aiagentrank — Devin vs Linear Agent](https://aiagentrank.io/compare/devin-vs-linear-agent), [aiagentrank — Copilot vs OpenHands](https://aiagentrank.io/compare/github-copilot-vs-openhands).
