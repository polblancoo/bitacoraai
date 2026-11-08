# Política de seguridad / Security policy

## Reportar una vulnerabilidad / Reporting a vulnerability

**No abras un issue público** con detalles de una vulnerabilidad. Usá el reporte privado de GitHub
(*Security → Report a vulnerability*) o escribí a **CrewHelmSupport@gmail.com**.
Incluí versión, pasos para reproducirla e impacto estimado.

**Please do not open a public issue** for a vulnerability. Use GitHub's private reporting
(*Security → Report a vulnerability*) or write to **CrewHelmSupport@gmail.com**.
Include the version, reproduction steps and estimated impact.

Respondemos en **5 días hábiles** y buscamos publicar una corrección en **90 días** o menos, con crédito si lo querés.
We aim to reply within **5 business days** and to ship a fix within **90 days**, with credit if you wish.

## Versiones con soporte / Supported versions

Solo la última versión publicada recibe correcciones de seguridad. / Only the latest release gets security fixes.

## Qué protege hoy BitacoraAI / What BitacoraAI does today

- Contraseñas con Argon2; sesiones firmadas que se invalidan al cerrar sesión en todos los dispositivos; límite de intentos por IP y por cuenta.
- **Verificación en dos pasos (TOTP)** por persona, con códigos de recuperación de un solo uso y sin reuso de códigos.
- Secretos del servidor (SMTP, Telegram, GitHub, Google) cifrados en reposo (AES-256-GCM).
- Las IAs ejecutan en una cápsula (bubblewrap) con red filtrada por destinos, y con **aprobación por riesgo** (borrar, publicar, desplegar, instalar, sistema).
- Cuotas por persona (disco, memoria, CPU, procesos) y topes de gasto por tarea, proyecto y equipo.
- **Auditoría exportable** (CSV/JSON) con quién hizo qué, permisos dados a las IAs y gasto.
- Carpeta de red (WebDAV) con escritura solo en la carpeta propia, versiones previas y sin ejecutables.
- Cabeceras de seguridad, CSP estricta y subida de archivos validada por contenido.

## Límites conocidos / Known limitations

- El aislamiento de las IAs es solo en **Linux** (bubblewrap); en macOS y Windows no hay equivalente.
- La cuota de disco se mide periódicamente: **no es un tope duro**.
- El 2FA lo puede pedir el administrador por grupo (administradores, CTO, resto), pero no se impone: es un pedido con avisos.
- No hay SSO (OIDC/SAML) ni una revisión de seguridad externa todavía.
- La terminal de texto (`tm`) aún no admite el código de 2FA.
- Base de datos SQLite de un solo nodo.

Recomendamos poner el servidor detrás de HTTPS con un certificado válido, mantenerlo actualizado y activar el 2FA a administradores y CTO.
