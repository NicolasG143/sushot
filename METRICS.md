# Métricas de sushot

Mediciones contra los objetivos de rendimiento de la versión 1. Cada fila indica cómo se midió, para poder repetirla en cada release.

## Última medición

- Fecha: 2026-11-22.
- Commit: ver `git log` de la rama; build `release` con el perfil de `Cargo.toml`.
- Equipo: Windows 11 Pro, 16 núcleos lógicos, un monitor de 1920x1080 al 100 % de escala. No hay mediciones en 2 monitores ni con DPI mixto.

| Métrica | Objetivo | Resultado | Cómo se midió |
|---|---|---|---|
| Binario Windows | < 1 MB | 595 KB | `scripts/measure.ps1` |
| RAM en idle (bytes privados) | < 5 MB | 1,8 MB | `scripts/measure.ps1`, después de 5 s en reposo |
| RAM en idle (working set) | < 5 MB | 10,3 MB | `scripts/measure.ps1`; incluye páginas compartidas de DLL del sistema |
| CPU en idle | ~0 % | 0 % | `scripts/measure.ps1`, 20 s de muestreo |
| Arranque en frío | < 200 ms | 65 ms | `scripts/measure.ps1`, hasta la línea `ready` del log, con configuración existente |
| Atajo → overlay visible | < 100 ms | 82 ms | `overlay_to_clipboard`, de la tecla enviada a la ventana visible, con sondeo cada 20 ms |
| Pantalla completa → portapapeles (1080p) | < 150 ms | ~70 ms | `latency`: captura ~30 ms + PNG, DIBV5 y portapapeles ~35 ms |

### Lectura de los resultados

- El objetivo de RAM en reposo (menos de 5 MB) no define la métrica. Con bytes privados se cumple; con working set no, porque el sistema cuenta las páginas compartidas de `user32`, `shell32` y similares. Hay que decidir cuál es la métrica oficial.
- La medición de arranque excluye el diálogo de primer arranque, que añade DLLs de diálogos comunes (118 ms y 13,1 MB de working set con el diálogo abierto, en una medición informal previa).
- El PNG del portapapeles usa la compresión más rápida del crate `png`: 1080p pasa de 365 ms con la compresión por defecto a 22 ms, y el PNG crece de 1,2 MB a 2,1 MB. El archivo guardado con `Alt` sigue usando la compresión por defecto.

## Prueba de estabilidad

`platform-windows/tests/stability.rs`, ejecutada con `cargo test --release -p sushot-platform-windows --test stability -- --ignored --nocapture`. Captura el escritorio y escribe el portapapeles 1000 veces seguidas, después de 20 capturas de calentamiento.

| Recurso | Antes | Después |
|---|---|---|
| Objetos GDI | 0 | 0 |
| Objetos USER | 2 | 2 |
| Handles del kernel | 105 | 105 |
| Bytes privados | 1,3 MB | 3,0 MB |

La prueba falla si crecen los objetos GDI o USER, si los handles suben más de 2 o si los bytes privados crecen más de 16 MB. La prueba usa la capa de plataforma y no pasa por la bandeja ni por el overlay; dura 68 s.

## Cómo repetir

```powershell
cargo build --release
powershell -File scripts/measure.ps1
cargo test --release -p sushot-platform-windows --test stability -- --ignored --nocapture
cargo test --release -p sushot-platform-windows --test latency -- --ignored --nocapture
```

`overlay_to_clipboard` inyecta mouse y teclado reales: ejecutarlo sin tocar el equipo.
