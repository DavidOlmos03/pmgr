# PMGR - Arquitectura y Documentación Técnica

## Índice

1. [Descripción General](#descripción-general)
2. [Arquitectura del Proyecto](#arquitectura-del-proyecto)
3. [Componentes Principales](#componentes-principales)
4. [Stack Tecnológico](#stack-tecnológico)
5. [Flujo de Datos](#flujo-de-datos)
6. [Decisiones de Diseño](#decisiones-de-diseño)
7. [Convenciones de Código](#convenciones-de-código)

---

## Descripción General

**PMGR** (Package Manager) es una interfaz de usuario de terminal (TUI) moderna para gestionar paquetes en Arch Linux. El proyecto nació como una evolución de una simple función bash (`kif`) que utilizaba `fzf` para la instalación de paquetes, y se ha convertido en una aplicación completa escrita en Rust con funcionalidades avanzadas.

### Objetivos del Proyecto

- **Experiencia de Usuario Superior**: Proporcionar una interfaz intuitiva y visualmente atractiva para la gestión de paquetes
- **Rendimiento**: Aprovechar Rust para operaciones rápidas y eficientes
- **Flexibilidad**: Soportar tanto modo interactivo como comandos directos
- **Extensibilidad**: Arquitectura modular que permite fácil adición de nuevas características

### Características Principales

- Navegación por pestañas (Home, Install, Remove, List)
- Búsqueda fuzzy en tiempo real
- Previsualización de información de paquetes
- Selección múltiple de paquetes
- Temas personalizables
- Soporte para AUR (yay) y repositorios oficiales (pacman)
- Gestión de permisos con polkit para paquetes oficiales

---

## Arquitectura del Proyecto

### Estructura de Directorios

```
pmgr/
├── src/
│   ├── commands/          # Comandos CLI directos
│   │   ├── install.rs     # Comando de instalación
│   │   ├── remove.rs      # Comando de eliminación
│   │   ├── search.rs      # Comando de búsqueda
│   │   ├── list.rs        # Comando de listado
│   │   └── mod.rs         # Módulo de comandos
│   │
│   ├── ui/                # Componentes de interfaz TUI
│   │   ├── app.rs         # Estado principal de la aplicación
│   │   ├── main_menu.rs   # Menú principal y navegación
│   │   ├── selector.rs    # Componente de selección de paquetes
│   │   ├── render.rs      # Funciones de renderizado
│   │   ├── home_state.rs  # Estado de la pantalla de inicio
│   │   ├── help_window.rs # Ventana de ayuda
│   │   ├── update_window.rs # Ventana de actualización del sistema
│   │   ├── theme.rs       # Sistema de temas
│   │   ├── types.rs       # Tipos compartidos de UI
│   │   └── mod.rs         # Módulo UI
│   │
│   ├── package/           # Lógica de gestión de paquetes
│   │   └── mod.rs         # PackageManager y operaciones
│   │
│   ├── config/            # Configuración y persistencia
│   │   ├── settings.rs    # Gestión de configuración
│   │   └── mod.rs         # Módulo de configuración
│   │
│   └── main.rs            # Punto de entrada de la aplicación
│
├── docs/                  # Documentación
│   ├── EXAMPLES.md        # Ejemplos de uso
│   └── ARCHITECTURE.md    # Este archivo
│
├── Cargo.toml             # Configuración de dependencias
└── README.md              # Documentación principal
```

### Separación de Responsabilidades

El proyecto sigue una arquitectura modular clara:

1. **`main.rs`**: Punto de entrada, parseo de CLI y delegación
2. **`commands/`**: Implementación de comandos directos CLI
3. **`ui/`**: Toda la lógica de interfaz de usuario TUI
4. **`package/`**: Abstracción de operaciones con package managers
5. **`config/`**: Gestión de configuración y persistencia

---

## Componentes Principales

### 1. Main Entry Point (`main.rs`)

Responsabilidades:
- Parseo de argumentos CLI usando `clap`
- Decidir entre modo interactivo (TUI) o modo comando directo
- Manejo de errores global

**Flujo de ejecución:**
```
Usuario ejecuta pmgr
    ↓
¿Hay subcomando?
    ├── Sí → Ejecutar comando directo
    │        (install, remove, search, list)
    │
    └── No → Lanzar MainMenu (TUI)
```

### 2. Package Manager (`package/mod.rs`)

**`PackageManager`**: Abstracción sobre pacman/yay

Métodos principales:
- `list_available()`: Lista todos los paquetes disponibles
- `list_installed()`: Lista paquetes instalados
- `get_info()`: Obtiene información detallada de un paquete
- `install()`: Instala paquetes
- `remove()`: Elimina paquetes
- `search()`: Busca paquetes por query
- `is_aur_package()`: Detecta si un paquete es de AUR
- `separate_packages()`: Separa paquetes AUR de oficiales

**Características técnicas:**
- Auto-detección de `yay` o fallback a `pacman`
- Delegación de privilegios mediante polkit para paquetes oficiales
- Handoff a yay/paru para paquetes AUR
- Heredado de stdio para interacción terminal nativa

### 3. UI System (`ui/`)

#### MainMenu (`main_menu.rs`)
Controlador principal de la interfaz TUI:
- Gestión de navegación entre tabs (Home, Install, Remove, List)
- Event loop principal
- Coordinación de estado de la aplicación
- Renderizado condicional basado en estado

#### Selector (`selector.rs`)
Componente reutilizable para selección de paquetes:
- Búsqueda fuzzy usando `fuzzy-matcher`
- Selección múltiple con TAB
- Preview en tiempo real
- Layouts configurables (horizontal/vertical)

#### Theme System (`theme.rs`)
Sistema de temas con paletas de colores completas:
- **Default**: Esquema de colores original
- **Nord**: Basado en Nord Theme
- **Dracula**: Basado en Dracula Theme
- **Dark**: Material-inspired dark
- **White**: Tema claro de alto contraste

Cada tema define:
- Colores primarios, secundarios, éxito, error, advertencia
- Colores de texto (primario, secundario, atenuado)
- Colores de UI (bordes, highlights, fondos)
- Colores especiales (tabs, preview, ASCII art gradient)

#### Render (`render.rs`)
Funciones de renderizado de componentes UI:
- Renderizado de lista de paquetes
- Preview de información de paquetes
- ASCII art del logo
- Información del sistema
- Ventanas de ayuda y actualización

### 4. Commands (`commands/`)

Cada comando es independiente y puede ejecutarse sin TUI:

- **`InstallCommand`**:
  - Modo interactivo → Lanza selector de paquetes disponibles
  - Modo directo → Instala paquetes especificados
  - Separación automática AUR/oficial

- **`RemoveCommand`**:
  - Modo interactivo → Lanza selector de paquetes instalados
  - Modo directo → Elimina paquetes especificados

- **`SearchCommand`**:
  - Búsqueda directa y muestra resultados en terminal

- **`ListCommand`**:
  - Modo simple → Lista en texto plano
  - Modo interactivo → Selector con preview

### 5. Config System (`config/`)

Gestión de configuración persistente:
- Archivo de configuración en `~/.config/pmgr/settings.json`
- Serialización con `serde_json`
- Fallback a valores por defecto si no existe
- Actualmente almacena tema seleccionado
- Preparado para futuras opciones (keybindings, layouts, etc.)

---

## Stack Tecnológico

### Dependencias Principales

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }  # CLI parsing
ratatui = "0.29"                                    # TUI framework
crossterm = "0.28"                                  # Terminal manipulation
fuzzy-matcher = "0.3"                               # Fuzzy search
serde = { version = "1.0", features = ["derive"] }  # Serialization
serde_json = "1.0"                                  # JSON handling
anyhow = "1.0"                                      # Error handling
colored = "2.1"                                     # Terminal colors
dirs = "5.0"                                        # System directories
signal-hook = "0.3"                                 # Signal handling
```

### Por qué Rust?

1. **Rendimiento**: Zero-cost abstractions, velocidad nativa
2. **Seguridad**: Memory safety sin garbage collector
3. **Concurrencia**: Sistema de ownership facilita código concurrente seguro
4. **Ecosistema**: Excelentes crates para TUI (`ratatui`, `crossterm`)
5. **Tooling**: Cargo, rustfmt, clippy proporcionan experiencia de desarrollo superior

### Optimizaciones de Release

```toml
[profile.release]
opt-level = 3      # Máxima optimización
lto = true         # Link-Time Optimization
strip = true       # Strip symbols para binario más pequeño
```

---

## Flujo de Datos

### Modo Interactivo (TUI)

```
Usuario presiona tecla
    ↓
crossterm::event::read()
    ↓
MainMenu::handle_key_event()
    ↓
┌─────────────────────────┐
│ Actualiza AppState      │
│ - Cambia tab            │
│ - Actualiza búsqueda    │
│ - Modifica selección    │
└─────────────────────────┘
    ↓
MainMenu::draw()
    ↓
ratatui::Terminal::draw()
    ↓
Renderiza a terminal
```

### Modo Comando Directo

```
pmgr install firefox
    ↓
Clap parsea argumentos
    ↓
InstallCommand::execute()
    ↓
PackageManager::install(&["firefox"])
    ↓
┌─────────────────────────────┐
│ 1. Detecta si es AUR        │
│ 2. Separa paquetes          │
│ 3. Usa polkit para oficial  │
│ 4. Handoff a yay para AUR   │
└─────────────────────────────┘
    ↓
Ejecuta comando del sistema
    ↓
Usuario interactúa directamente con pacman/yay
```

### Sistema de Selección de Paquetes

```
Selector::new(packages)
    ↓
Usuario escribe en búsqueda
    ↓
fuzzy_matcher filtra lista
    ↓
Resultados ordenados por score
    ↓
Usuario navega con ↑/↓
    ↓
Preview actualizado en tiempo real
    ↓
Usuario presiona TAB → toggle selección
    ↓
Enter → retorna paquetes seleccionados
```

---

## Decisiones de Diseño

### 1. Separación AUR vs Oficial

**Problema**: AUR requiere helpers (yay/paru) mientras paquetes oficiales usan pacman con sudo.

**Solución**:
- `PackageManager::is_aur_package()` detecta paquetes AUR consultando `pacman -Si`
- Si falla (paquete no en repos oficiales) → es AUR
- Paquetes oficiales: usar polkit para elevación de privilegios
- Paquetes AUR: handoff completo a yay/paru

**Beneficios**:
- Usuario no necesita configurar sudo
- Experiencia más segura con polkit
- Delegación apropiada según tipo de paquete

### 2. Arquitectura Modular

**Decisión**: Separar comandos CLI de lógica TUI

**Razones**:
- Permite uso como herramienta CLI tradicional
- Componentes de UI reutilizables (Selector)
- Testing más fácil
- Menor acoplamiento

### 3. Sistema de Temas

**Decisión**: Implementar themes como enums con paletas completas

**Alternativas consideradas**:
- Temas como archivos de configuración JSON
- Temas como plugins dinámicos

**Por qué la decisión actual**:
- Type-safety en compile time
- Performance (no I/O en runtime)
- Simplicidad de implementación
- Fácil adición de nuevos temas (solo un variant)

### 4. Estado de Aplicación

**Decisión**: `AppState` como estructura centralizada

```rust
struct AppState {
    current_tab: Tab,
    packages: Vec<Package>,
    search_query: String,
    selected: Vec<usize>,
    // ...
}
```

**Beneficios**:
- Single source of truth
- Facilita debugging
- Simplifica serialización futura
- Permite "undo" potencial

### 5. Interacción con Package Managers

**Decisión**: Heredar stdio en lugar de capturar output

```rust
cmd.stdin(Stdio::inherit())
   .stdout(Stdio::inherit())
   .stderr(Stdio::inherit());
```

**Razón**:
- Usuario ve output real de pacman/yay
- Prompts interactivos funcionan (confirmaciones, passwords)
- Progreso de descarga visible
- Más transparente y debuggable

---

## Convenciones de Código

### Estilo

- **Formatting**: `rustfmt` estándar
- **Linting**: `clippy` con warnings como errores
- **Nombres**: snake_case para funciones/variables, PascalCase para tipos
- **Documentación**: Doc comments (`///`) para APIs públicas

### Patrones Comunes

#### Result y Error Handling

```rust
use anyhow::{Context, Result};

pub fn operation() -> Result<T> {
    some_fallible_op()
        .context("Descripción del contexto")?
}
```

#### Constructor Pattern

```rust
impl PackageManager {
    pub fn new() -> Self {
        // Auto-detection y setup
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}
```

#### Builder Pattern para UI

```rust
Selector::new(packages)
    .with_title("Install Packages")
    .with_preview(true)
    .run()
```

### Commits

Usar conventional commits:
- `feat:` - Nueva funcionalidad
- `fix:` - Bug fix
- `docs:` - Documentación
- `refactor:` - Refactoring
- `perf:` - Performance
- `test:` - Tests
- `chore:` - Maintenance

---

## Roadmap Futuro

### Features Planeados

1. **Gestión de AUR helpers**
   - Detección automática de yay/paru/otros
   - Configuración de helper preferido

2. **History y Rollback**
   - Log de operaciones
   - Capacidad de deshacer instalaciones/remociones

3. **Grupos de Paquetes**
   - Definir conjuntos de paquetes para instalar juntos
   - Perfiles (dev, gaming, multimedia, etc.)

4. **Busqueda Avanzada**
   - Filtros por repository
   - Búsqueda por dependencias
   - Tags y categorías

5. **Estadísticas**
   - Uso de espacio por paquete
   - Paquetes huérfanos
   - Fecha de instalación

6. **Integración con Sistema**
   - Notificaciones de actualizaciones disponibles
   - Daemon opcional para checks automáticos

### Mejoras Técnicas

- [ ] Tests unitarios y de integración
- [ ] CI/CD con GitHub Actions
- [ ] Publicación en AUR
- [ ] Benchmarks de performance
- [ ] Logging estructurado con `tracing`
- [ ] Configuración de keybindings personalizada

---

## Contribuir

### Setup de Desarrollo

```bash
# Clonar repo
git clone https://github.com/DavidOlmos03/pmgr.git
cd pmgr

# Build debug
cargo build

# Run con logs
RUST_LOG=debug cargo run

# Tests
cargo test

# Linting
cargo clippy -- -D warnings

# Format
cargo fmt
```

### Áreas de Mejora

- **Testing**: Actualmente hay pocas pruebas, necesita más cobertura
- **Documentación**: Algunos módulos necesitan mejores doc comments
- **Performance**: Profiling y optimización de búsqueda fuzzy
- **Accesibilidad**: Mejorar soporte para screen readers
- **i18n**: Internacionalización para múltiples idiomas

---

## Créditos y Referencias

### Inspiración

- **lazygit**: Excelente ejemplo de TUI git client
- **bottom**: Beautiful system monitor TUI
- **yay**: AUR helper con gran UX

### Recursos

- [Ratatui Book](https://ratatui.rs/)
- [Crossterm Docs](https://docs.rs/crossterm/)
- [Arch Wiki - Pacman](https://wiki.archlinux.org/title/Pacman)

---

**Última actualización**: 2026-01-01
**Versión**: 0.1.0
**Autor**: David Olmos
