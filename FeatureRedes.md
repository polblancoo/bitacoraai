# Feature: conexión con redes sociales y herramientas de diseño / edición / traducción con IA

Tarea #33 · Evaluación de viabilidad (no se implementó nada) · 2026-10-31

## Conclusión

**Sí es viable**, y encaja bien con lo que ya tiene el producto. Pero conviene separarlo en dos mitades de riesgo muy distinto:

| Mitad | Viabilidad | Riesgo principal |
|---|---|---|
| **A. Herramientas de IA** (diseño, edición, traducción, texto, imagen) | Alta, esfuerzo bajo-medio | Costos de API y manejo de claves |
| **B. Redes sociales** (publicar, programar, leer métricas) | Media: técnicamente posible, pero depende de terceros | Aprobación de apps, límites de API, cambios de política y costos |

Recomendación: empezar por **A + un par de redes fáciles** (ver «Plan por fases»). No arrancar por Instagram/TikTok, que son lo más lento de habilitar.

## Por qué encaja con lo que ya existe

Mirando el código (`crates/server/src`):

- **Las IAs ya son integrantes del equipo** (`bots.rs`, `runner.rs`, `crates/agent`): tienen token, reciben tareas y comentan. Una «IA traductora» o «IA diseñadora» es un bot más con un conector distinto.
- **Programación de tareas** (`schedule.rs`): ya hay tareas recurrentes y «cada día a las 20:00». Es la base de un calendario de publicaciones.
- **Revisión humana del trabajo de las IAs** (`review.rs`, estado «en revisión», aprobar / pedir cambios). Es exactamente el flujo que necesita una agencia: la IA redacta o diseña → una persona aprueba → recién entonces se publica. **Es el diferencial del feature.**
- **Adjuntos** (`extras.rs`, tabla `attachments`): sirven para guardar imágenes, videos y textos generados.
- **Conexiones OAuth por usuario** (`github.rs` con Device Flow, `google.rs` con OAuth, tablas `github_links` y `google_links`): el patrón de «cada usuario conecta su cuenta» ya está resuelto.
- **Avisos** (`notify.rs`, `digest.rs`, `telegram.rs`): resumen diario y alertas ya funcionan.
- **Licencia firmada y módulo `pro`** (`license.rs`, `license_token.rs`, `pro.rs`): permite vender esto como función de pago (`funciones` en el token) sin tocar el núcleo AGPL.

## Qué habría que construir

### 1. Capa de conectores (núcleo común)

Un trait/módulo `connectors` en el servidor con dos familias:

- **`Publisher`** (redes): `publicar(post)`, `programar(post, fecha)`, `metricas(post)`, `borrar(post)`.
- **`Tool`** (IA / apps): `ejecutar(entrada) -> adjunto/texto`, con tipos: traducir, generar imagen, editar imagen, generar copy, subtitular video.

Tablas nuevas (SQLite, como el resto):

- `connections` (equipo, proveedor, cuenta, token cifrado, alcance, vencimiento, creado por).
- `brand_profiles` (por proyecto o cliente: tono, idiomas, hashtags, paleta, reglas) — clave para agencias con varios clientes.
- `posts` (borrador → en revisión → aprobado → programado → publicado / falló; red, texto, adjuntos, fecha, id externo, métricas).
- `connector_usage` (consumo por conector, para límites y costos; ya existe algo parecido en `usage.rs` / `limits.rs`).

### 2. «Se setea al principio»

Un asistente de configuración por equipo o por proyecto («Conectar redes y herramientas»): elegir proveedores, autorizar cuentas, cargar claves de API y definir el perfil de marca. Se puede reutilizar el estilo de Configuración → GitHub / Google ya existente. Cada **cliente** de una agencia sería un proyecto con sus propias conexiones.

### 3. Integración con el flujo actual

- Una tarea puede tener un «entregable»: *post para Instagram + X, versión ES/EN/PT*.
- La IA lo prepara → queda **en revisión** → la persona aprueba → se publica a la hora programada → se comenta el enlace y las métricas en la tarea.
- Aprobación por cliente externo: rol lector con permiso de aprobar (hoy la aprobación es de miembros).

### 4. Exposición a las IAs

Sumar herramientas a `tm-mcp` / `tm-agent` (traducir, generar imagen, programar post) para que el agente las use sin conocer las claves: el servidor ejecuta, la IA solo pide.

## Redes sociales: realidad de cada API (punto crítico)

> Los detalles de cada API cambian seguido (límites, precios, requisitos de revisión). **Verificar la documentación vigente de cada plataforma antes de comprometer plazos o precio.** Lo siguiente es lo que se sabe hasta mi conocimiento y no pude confirmar en línea desde esta tarea.

| Red | Publicar | Dificultad | Notas |
|---|---|---|---|
| **Telegram** | Sí (Bot API) | Muy baja | Ya hay `telegram.rs`. Canales y grupos. |
| **LinkedIn** | Sí (páginas y perfiles) | Media | Requiere app y productos aprobados. |
| **Facebook / Instagram** (Meta Graph API) | Sí (cuentas profesionales / páginas) | Alta | App Review, cuenta Business, permisos y verificación. Es lo más pedido por influencers y también lo más lento de habilitar. |
| **X (Twitter)** | Sí | Media | API de pago con topes por plan; el costo puede pesar. |
| **YouTube** | Sí (subir video, metadatos) | Media | Cuota diaria; las apps sin auditar tienen restricciones. |
| **TikTok** | Parcial (Content Posting API) | Alta | Auditoría de la app; sin ella los posts salen como privados. |
| **Pinterest, Reddit, Mastodon, Bluesky** | Sí | Baja-media | Mastodon y Bluesky son abiertos y fáciles. |
| **WhatsApp Business** | Mensajes, no «publicar» | Media | Plantillas y costo por conversación. |

Puntos a tener en cuenta:

1. **Un solo «app registrada» por plataforma**: como el producto es self-hosted, hay dos caminos:
   - *Cada instalación registra su propia app* (como hoy `TM_GOOGLE_CLIENT_ID`): sin dependencia nuestra, pero mucho trabajo para el cliente (sobre todo Meta).
   - *CrewHelm Cloud con una app central aprobada*: la mejor experiencia, pero hay que pasar la revisión de cada plataforma una vez y mantenerla. Encaja con la línea «CrewHelm Cloud» del ROADMAP.
2. **Alternativa pragmática**: integrar un agregador que ya resolvió las aprobaciones (tipo Buffer, Postiz —open source, self-hostable—, Ayrshare, Late u otros). Se pierde algo de control y se suma un costo, pero se llega a muchas redes en semanas en vez de meses. Evaluar precios y licencias vigentes antes de decidir.
3. **Tokens que vencen**: hay que renovarlos y avisar cuando una cuenta se desconecta (se puede apoyar en `notify.rs`).
4. **Límites de tasa y colas**: publicar con reintentos y registro de errores; no publicar dos veces si falla a mitad (idempotencia con `posts.id_externo`).

## Herramientas de IA / diseño / edición / traducción

Es la mitad más sencilla: son APIs HTTP con clave.

| Necesidad | Opciones típicas |
|---|---|
| Traducción | Modelos de lenguaje (Claude, etc.), DeepL, Google Translate |
| Redacción / copy / ideas | Claude y otros LLM |
| Imagen (generar / editar) | APIs de modelos de imagen, Stability, etc. |
| Diseño de piezas | Canva (Connect API, con requisitos de la plataforma), Figma (API de lectura principalmente) |
| Video / subtítulos | Whisper / servicios de transcripción, editores con API |

Notas:

- **Canva y Figma**: tienen API pero con alcance limitado (no son «generá la pieza completa» por API en todos los casos). Hay que validar qué permite cada una antes de prometerlo.
- **Claves de los clientes (BYOK)**: lo más simple y barato para nosotros es que cada equipo cargue su propia clave. Alternativa: revender consumo desde Cloud (requiere medición y facturación).
- Los agentes `claude -p` ya ejecutados por `tm-agent` pueden llamar a herramientas por MCP; así no hace falta un cliente de cada API dentro del servidor para empezar.

## Riesgos y cosas a decidir

1. **Seguridad de credenciales.** Hoy `github_links.token` se guarda **en texto plano** en SQLite. Para tokens que *publican en nombre de una marca* eso no alcanza: hay que **cifrar en reposo** (clave maestra por instalación, p. ej. AEAD) y no devolver nunca los tokens por la API. Es prerrequisito del feature.
2. **Responsabilidad de lo publicado.** Una IA publicando sola puede causar daño de marca. Por defecto: **nada se publica sin aprobación humana** (reutilizar `review.rs`), con opción de auto-publicar por proyecto para quien lo elija, más un registro de auditoría.
3. **Términos de las plataformas.** Algunas prohíben o limitan contenido generado por IA sin etiquetar, o automatización masiva. Meta, TikTok y YouTube piden marcar contenido sintético en ciertos casos. Hay que revisar y mostrar avisos.
4. **Privacidad / legal.** Se guardan datos de cuentas de terceros: actualizar los términos (`legal/terminos-v1.*.md`) y la política de datos; el aviso de telemetría ya existente aplica.
5. **Costos.** APIs pagas (X, imagen, video) y cuota de las plataformas. Necesita límites por equipo y por conector (extender `limits.rs` / `usage.rs`).
6. **Mantenimiento.** Las APIs sociales cambian y rompen cosas. Es un costo recurrente, no solo de desarrollo inicial. Es el argumento más fuerte a favor de apoyarse en un agregador al principio.
7. **Producto y licencia.** Sugerencia: conectores básicos y 1–2 redes en el núcleo; multi-cliente, aprobación de clientes externos, métricas y auto-publicación como **función `pro`** (`funciones` en la licencia).
8. **Público objetivo.** La TUI no es el lugar natural para un calendario editorial con imágenes. Esto pide **web (y desktop/Tauri)** como interfaz principal; la TUI solo lista y aprueba.

## Plan por fases (estimación orientativa, una persona)

| Fase | Contenido | Esfuerzo |
|---|---|---|
| 0 | Cifrado de credenciales en reposo + tabla `connections` + perfil de marca | 1 semana |
| 1 | Conectores IA: traducción, copy e imagen (BYOK) expuestos por MCP; resultado como adjunto de la tarea | 1–2 semanas |
| 2 | Entidad `post` + calendario en la web + flujo borrador → revisión → aprobado | 2 semanas |
| 3 | Publicación en Telegram, Mastodon/Bluesky y LinkedIn; cola con reintentos y métricas básicas | 2–3 semanas |
| 4 | Meta (Facebook/Instagram), YouTube, X: trámite de aprobación de apps en paralelo desde la fase 1 | 3–6 semanas, **depende de terceros** |
| 5 | Multi-cliente (agencias), aprobación por cliente externo, informes, límites y facturación en Cloud | 3–4 semanas |

Las estimaciones son gruesas; las fases 3–4 dependen sobre todo del tiempo de aprobación de cada plataforma, no del código.

## Qué necesito que decidas

1. ¿Self-hosted con claves propias de cada cliente, o apostar a **CrewHelm Cloud** con app central?
2. ¿Construir los conectores a mano o apoyarnos en un **agregador** (p. ej. Postiz, que es open source) para llegar rápido a más redes?
3. ¿Qué redes son prioridad para el primer cliente real (influencers vs. agencias de marketing)?
4. ¿Va como función `pro` (de pago) o dentro del núcleo?
5. ¿Auto-publicación sin revisión permitida, o siempre con aprobación humana?

## Resumen en una línea

Se puede y tiene mucho sentido porque el tablero, las IAs, la programación y la revisión humana ya están; lo difícil no es el código sino **las aprobaciones de las APIs de las redes, el cuidado de las credenciales y el costo de mantenerlo**. Empezar por herramientas de IA + redes abiertas y dejar Meta/TikTok en paralelo.
