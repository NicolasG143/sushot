# klip

Capturador de pantalla ultra liviano para Windows y Linux. Vive en la bandeja del sistema, responde a `Impr Pant` y entrega la captura en el portapapeles o en un archivo PNG. Sin editor, sin cuentas, sin red.

> **Estado:** este README describe la versión 1 terminada. Hoy solo funciona `Ctrl + Impr Pant` (pantalla completa al portapapeles) en Windows. El avance real está en [ROADMAP.md](ROADMAP.md), lo pendiente en [TODO.md](TODO.md) y el detalle técnico en [SPECS.md](SPECS.md). Los valores de rendimiento son objetivos de diseño, no mediciones.

## Uso rápido

1. Instalá klip e inicialo; aparece un icono en la bandeja.
2. Presioná `Impr Pant` y arrastrá para seleccionar una región.
3. Pegá la imagen donde la necesites.

Para guardar en lugar de copiar, sumá `Alt` a cualquier atajo.

## Atajos

`Ctrl` y `Shift` eligen **qué** se captura. `Alt` cambia **adónde** va.

| Atajo | Captura | Salida |
|---|---|---|
| `Impr Pant` | Región | Portapapeles |
| `Ctrl + Impr Pant` | Pantalla completa | Portapapeles |
| `Shift + Impr Pant` | Ventana | Portapapeles |
| `Alt + Impr Pant` | Región | Archivo PNG |
| `Ctrl + Alt + Impr Pant` | Pantalla completa | Archivo PNG |
| `Shift + Alt + Impr Pant` | Ventana | Archivo PNG |

Cualquier otra combinación se ignora y pasa al sistema.

### Durante la selección

| Acción | Resultado |
|---|---|
| Arrastrar con botón izquierdo | Define la región; muestra sus dimensiones (`W × H`) |
| Clic izquierdo sobre una ventana | Captura la ventana resaltada |
| `Esc` o clic derecho | Cancela sin copiar ni guardar |

Detalles de comportamiento:

- **Pantalla completa** captura el monitor donde está el cursor.
- **Ventana** captura lo que se ve en pantalla dentro del área de la ventana, incluido lo que la tape.
- La pantalla se congela antes de mostrar la selección, así que el overlay nunca aparece en la imagen.
- Una región menor a 2×2 px se trata como cancelación.

### Guardado

- Nombre sugerido: `Captura_YYYY-MM-DD_HH-MM-SS.png`.
- El diálogo abre en la última carpeta usada; la primera vez, en Imágenes.
- Cancelar el diálogo descarta la captura: no se copia al portapapeles.

## Instalación

| Plataforma | Canal | Notas |
|---|---|---|
| Windows 10 (1809+) / 11, x64 | Microsoft Store | Paquete MSIX |
| Windows 10 (1809+) / 11, x64 | Descarga directa | Ejecutable firmado |
| Linux x64 | AppImage | Requiere las bibliotecas de X11 del sistema |

Los enlaces de descarga se publican junto con la versión 1.

## Soporte por plataforma

| Plataforma | Soporte | Qué cambia |
|---|---|---|
| Windows 10 / 11 | Completo | — |
| Linux X11 | Completo | — |
| Linux Wayland | Degradado | La selección la gestiona el compositor mediante `xdg-desktop-portal`; el overlay de klip no se usa |

macOS no está soportado.

### Wayland

Los atajos globales dependen del compositor. Donde el portal `GlobalShortcuts` no está disponible, se puede vincular un atajo del sistema a la línea de comandos:

```sh
klip --region          # región al portapapeles
klip --full            # pantalla completa al portapapeles
klip --window          # ventana al portapapeles
klip --region --save   # cualquier modo con diálogo de guardado
```

## Bandeja y configuración

El menú de la bandeja tiene tres entradas: nombre y versión, **Iniciar con el sistema** y **Salir**.

La configuración es un archivo TOML que klip recrea con valores por defecto si falta o está dañado.

| Sistema | Ruta |
|---|---|
| Windows | `%APPDATA%\klip\config.toml` |
| Linux | `$XDG_CONFIG_HOME/klip/config.toml` (`~/.config/klip/`) |

| Campo | Función | Por defecto |
|---|---|---|
| `autostart` | Iniciar klip con el sistema | Desactivado |
| `last_save_dir` | Carpeta inicial del diálogo de guardado | Imágenes del usuario |
| `show_toast` | Notificación al copiar | Desactivado |

Solo corre una instancia a la vez; una segunda termina en silencio.

## Privacidad

klip no tiene telemetría, no abre conexiones de red y no requiere cuenta. Las capturas solo salen del proceso hacia el portapapeles o hacia el archivo que elijas.

## Rendimiento

Objetivos de diseño de la versión 1. Las mediciones de cada release se registran en el repositorio.

| Métrica | Objetivo |
|---|---|
| Binario en Windows | < 1 MB |
| RAM en reposo | < 5 MB |
| CPU en reposo | ~0 % |
| Atajo → overlay visible | < 100 ms |
| Pantalla completa → portapapeles (1080p) | < 150 ms |
| Arranque en frío | < 200 ms |

## Limitaciones conocidas

| Situación | Comportamiento |
|---|---|
| Contenido con DRM u overlays de hardware | Puede salir en negro |
| Juegos en pantalla completa exclusiva | La captura puede salir en negro |
| Ventana elevada (administrador) con el foco | Los atajos pueden no dispararse |
| Pantalla bloqueada o escritorio seguro de UAC | No se captura; falla en silencio |
| Windows 11 con Impr Pant asociada a Recortes | klip tiene prioridad y lo avisa en el primer arranque |
| Wayland | Las capacidades dependen del compositor |

## Qué no hace

klip no incluye editor de anotaciones, captura de video o GIF, captura con scroll, OCR, subida a la nube ni atajos configurables desde una interfaz. Son exclusiones deliberadas, no pendientes.

## Compilar desde el código

Requiere el toolchain estable de Rust. En Windows, el CRT estático ya está configurado en `.cargo/config.toml`.

```sh
cargo build --release
```

Verificaciones:

```sh
cargo test --workspace     # tests unitarios y de captura
cargo deny check           # auditoría de licencias de dependencias
cargo about generate about.hbs -o THIRD_PARTY_LICENSES
```

La prueba de extremo a extremo de Windows no corre por defecto: envía un `Ctrl + Impr Pant` real y sobrescribe el portapapeles.

```sh
cargo test -p klip --test full_screen_to_clipboard -- --ignored
```

## Arquitectura

```
klip/
├─ core/                Modos, resolve(), recorte, encode PNG, config
├─ app/                 Binario: bandeja, overlay, orquestación
├─ platform-windows/    Hook de teclado, límites DWM, DPI, autostart
└─ platform-linux/      Atajos X11, portal Wayland, autostart
```

El `core` no conoce el sistema operativo: cada plataforma implementa el trait `Platform` y `app` orquesta el flujo.

| Hilo | Responsabilidad |
|---|---|
| Principal | Event loop, overlay y bandeja |
| Atajos | Detecta el atajo y envía la acción por canal |
| Encode y guardado | Codifica el PNG y escribe a disco |

## Licencia

Código *source-available*: se puede leer y compilar, no revender. La licencia aplicable está en [`LICENSE`](LICENSE) y las de terceros en `THIRD_PARTY_LICENSES`.

Las contribuciones externas requieren firmar un CLA. El nombre y el logo de klip no están cubiertos por la licencia del código.
