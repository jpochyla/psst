# Roadmap de Xpotify + Splitify

Trabajo en el repositorio original `xpotify`, rama `splitify-integration`. El repositorio web `Splitify` conserva `main`. Actualizado el 5 de octubre de 2026.

| Punto del fork | Implementación y límites |
| --- | --- |
| Resistencia a errores de red | Las lecturas reintentan fallos de conexión y timeout hasta tres intentos por etapa, con espera progresiva. También se reintenta la lectura del cuerpo JSON y la resolución/descarga inicial de audio. No se repiten escrituras ambiguas, saltos de canción ni transferencias. Los 429 tienen espera limitada; una cuota larga se muestra al usuario. |
| Seguir/dejar de seguir playlists | Operaciones del endpoint actual de biblioteca. La biblioteca cambia después de la confirmación de Spotify. |
| Añadir/eliminar canciones | Operaciones del endpoint `/playlists/{id}/items`, con errores visibles y actualización de la playlist abierta. La eliminación por URI elimina sus apariciones según el contrato de Spotify. |
| Reordenar canciones | Menú de cada canción en playlists editables: mover arriba/abajo. Comprueba el ID en su posición original y envía `snapshot_id` para detectar cambios concurrentes. Funciona aunque la vista esté ordenada o filtrada. No hay arrastre de filas. |
| Renombrar playlists | Diálogo existente; confirma el servidor antes de actualizar el nombre. |
| Carpetas de playlists | Carpetas locales persistentes: crear, renombrar, eliminar, asignar playlists y filtrar la biblioteca. No sincronizan con carpetas de Spotify: su API pública no las devuelve ni permite crearlas. |
| Cola de reproducción | Vista completa del orden real, modos de repetición/aleatorio, elementos añadidos manualmente y resumen en el panel derecho. |
| Eventos de salida de audio | En el backend CPAL se comprueba la salida predeterminada cada dos segundos. La desaparición de la salida anterior o un error pausa el audio. Una nueva salida mantiene la reproducción si la anterior sigue disponible; una pausa del usuario se conserva. Se reabre el motor sin reemplazar la cola. La identificación usa los nombres publicados por el backend. |
| Mejor caché | GET de metadatos/biblioteca/búsqueda con TTL de cinco minutos y separación por sesión; álbumes y artistas con TTL de 24 horas. Un fallo temporal permite usar datos vencidos. Los errores 401/403 no se ocultan con datos antiguos. Escrituras atómicas y actualización manual que invalida metadatos sin borrar audio ni el límite de cuota. |
| Uso y fecha de caché | Tamaño total en Preferencias → Caché; fecha UTC de los datos almacenados junto a la navegación y botón Actualizar. |
| Artistas: información y Wikipedia | Biografía, seguidores y estadísticas proporcionadas por Spotify; enlaces originales y búsqueda en Wikipedia desde el menú del artista. No se inventan datos cuando el proveedor falla. |
| Descargar pistas cifradas | El motor original descarga rangos cifrados para reproducir; al completar una pista la almacena en la caché de audio. Ahora la publicación del archivo completo es atómica. No existe exportación a MP3 ni descarga completa de playlists para uso sin conexión. |
| Reportar escuchas a Spotify | **Pendiente.** El motor nativo no implementa el protocolo privado de reporte. Actualizar Connect o descargar audio no garantiza registrar escuchas. La reproducción transferida a un cliente oficial usa ese cliente; no se atribuye ese comportamiento al motor nativo. |
| Paquetes por sistema | Windows: `scripts/Package-Native.ps1`, ZIP con ejecutable, documentación, lanzador y SHA-256, sin credenciales. macOS: script para `.app` con icono verde. Linux: archivo con binario, icono y entrada `.desktop`. Workflow manual disponible; macOS/Linux requieren compilación y validación en sus plataformas. No son paquetes firmados. |
| Dos paneles | Navegación a la izquierda y canción actual/cola a la derecha, separador ajustable y opción Panel para ocultarlo. La biblioteca y los controles globales siguen accesibles. |
| Tema del sistema | Predeterminado System; detección de modo claro/oscuro en Windows y opciones explícitas Light/Dark. La detección automática en otros sistemas necesita validación adicional. |
| Errores con reintento | Las vistas de solicitudes muestran un botón Reintentar; Actualizar y Ctrl+R recargan la ruta activa. Los errores de salida de audio se recuperan al detectar una salida disponible. |
| Resaltado de reproducción | Coincidencia por ID de canción y por origen de álbum/playlist. Selecciona la página de la canción actual y solicita mantenerla visible cuando cambia. |
| Listas grandes | Canciones/episodios: 100 filas por página, conservando posiciones originales y la cola completa. La búsqueda filtra toda la colección antes de paginar. Las respuestas de Spotify se siguen descargando por páginas hasta el límite configurado; no se implementó carga remota infinita. |
| Cuadrículas de álbumes/artistas | Tarjetas en dos columnas, 40 elementos por página en biblioteca, resultados y discografía/relacionados. |
| Menús activos/inactivos | Selección de navegación/biblioteca conservada; controles de paginación y acciones no disponibles deshabilitados. |
| Guardar reproducción | Conserva canción, progreso, origen y cola de hasta 5.000 entradas; restaura en pausa. Persiste el orden exacto del motor, el modo de repetición y las adiciones manuales, incluso duplicadas. Los perfiles anteriores se migran al guardarse. |

## Comprobaciones

```powershell
cargo test --locked -p psst-core --lib -p psst-gui --bin psst-gui
cargo clippy --locked -p psst-core --lib -p psst-gui --bin psst-gui -- -D warnings
powershell -File scripts/Build-Native.ps1
powershell -File scripts/Package-Native.ps1 -SkipBuild
```

Las pruebas cubren los reintentos limitados, escrituras sin repetición, aislamiento de caché, conservación de audio al invalidar metadatos, índices de reordenación, carpetas persistentes, búsqueda entre páginas, posiciones originales, resaltado por origen y política de desconexión de audio. Las pruebas de política no sustituyen una prueba física con auriculares. Las escrituras remotas de playlists no se ejecutan en las pruebas unitarias.

## Referencias y pendientes

- [Spotify: carpetas y playlists](https://developer.spotify.com/documentation/web-api/concepts/playlists).
- [Spotify: reordenar elementos y snapshot](https://developer.spotify.com/documentation/web-api/reference/reorder-or-replace-playlists-items).
- [FOSDEM 2026, desarrollador de librespot: reporte de escuchas, páginas 36–37](https://fosdem.org/2026/events/attachments/RNBQ8U-reverse-engineering-spotify/slides/267362/reverse_e_xy4vd0r.pdf).

El roadmap no se declara completamente cerrado: faltan reporte nativo de escuchas, sincronización de carpetas que Spotify no expone y validación real de paquetes/macOS/Linux y cambios físicos de salida de audio.
