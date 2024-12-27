# Rust TUI Manager 📝

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

Una aplicación TUI (Terminal User Interface) para gestionar notas, comandos y snippets de código de manera eficiente. Perfecta para desarrolladores que prefieren trabajar en la terminal.

## Características 🌟

### Organización
- 📚 Sistema jerárquico de secciones y detalles
- 📁 Iconos visuales automáticos para secciones
- 💾 Persistencia automática en JSON
- 📅 Timestamps automáticos en entradas

### Búsqueda Avanzada 🔍
- Búsqueda local instantánea
- Integración con crates.io para búsqueda de paquetes
- Integración con cheats.rs para referencias de Rust
- Múltiples fuentes de búsqueda configurables

### Interfaz Intuitiva ⌨️
- Navegación completa con teclado
- Paneles redimensionables con Ctrl + ←/→
- Diseño minimalista y funcional
- Indicadores visuales de foco

## 🎮 Guía Completa de Atajos de Teclado

### 📑 Navegación General
| Tecla | Función |
|-------|---------|
| `Tab` | Cambiar entre paneles (Secciones → Detalles → Búsqueda) |
| `Shift + Tab` | Cambiar entre paneles en reversa |
| `↑/↓` | Navegar en el panel actual |
| `Ctrl + ←/→` | Ajustar tamaño de paneles |

### 📝 Gestión de Contenido
| Tecla | Función |
|-------|---------|
| `a` | Agregar nueva sección o detalle |
| `e` | Editar elemento seleccionado |
| `d` | Eliminar elemento seleccionado |
| `Enter` | Confirmar acción |
| `Esc` | Cancelar/Volver |

### 🔍 Modo Búsqueda (`s` para activar)
| Tecla | Función |
|-------|---------|
| `Tab` | Cambiar fuente de búsqueda (Local → Crates.io → Cheats.sh → Todas) |
| `↑/↓` | Navegar entre resultados |
| `PgUp/PgDn` | Scroll rápido |
| `Enter` | Abrir enlace en navegador (para resultados web) |
| `c` | Copiar enlace o guardar resultado en sección actual |
| `Esc` | Cerrar búsqueda |

### 💾 Otras Funciones
| Tecla | Función |
|-------|---------|
| `h` | Mostrar/Ocultar ayuda |
| `q` | Salir de la aplicación |

### 📝 En Modo Edición
| Tecla | Función |
|-------|---------|
| `Tab` | Cambiar entre campos (título/descripción) |
| `Enter` | Guardar cambios |
| `Esc` | Cancelar edición |
| `Backspace` | Borrar caracteres |

### 🔍 Consejos de Búsqueda
- La búsqueda es en tiempo real mientras escribes
- Los resultados se actualizan automáticamente al cambiar la fuente
- Puedes guardar resultados web en tus secciones locales
- Los enlaces web se pueden abrir directamente en tu navegador

## Instalación 🚀

## Estructura del Proyecto 🏗️

---

## Tecnologías 🛠️

- [Rust](https://www.rust-lang.org/) - Lenguaje de programación
- [Ratatui](https://github.com/ratatui-org/ratatui) - Framework TUI
- [Crossterm](https://github.com/crossterm-rs/crossterm) - Manipulación terminal
- [Serde](https://serde.rs/) - Serialización
- [Tokio](https://tokio.rs/) - Runtime asíncrono

## Roadmap 🗺️

- [ ] Soporte para markdown en descripciones
- [ ] Exportación a diferentes formatos
- [ ] Temas personalizables
- [ ] Sincronización en la nube
- [ ] Más integraciones de búsqueda

## Licencia 📄

Este proyecto está bajo la Licencia MIT - ver el archivo [LICENSE](LICENSE) para más detalles.

## Contribuir 🤝

Las contribuciones son bienvenidas:

1. Fork del proyecto
2. Crear rama (`git checkout -b feature/NuevaCaracteristica`)
3. Commit (`git commit -m 'Agrega nueva característica'`)
4. Push (`git push origin feature/NuevaCaracteristica`)
5. Pull Request

---

Desarrollado con ❤️ usando Rust
