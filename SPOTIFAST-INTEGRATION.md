# Mejoras basadas en Spotifast

Referencia revisada: el archivo local de Spotifast facilitado por el usuario,
con licencia MIT de Carmine Paolino (2026). Se estudiaron su cliente HTTP,
cola, reproductor y tema. Xpotify conserva su interfaz nativa, el motor Connect
y la integración Splitify; no ejecuta scripts ni instala el ejecutable externo.
La atribución del tema adaptado figura en `LICENSE-Spotifast.md` y se incluye
en los paquetes.

## Cambios de Xpotify 0.4.0

- La cola local recibe el orden efectivo del motor, incluyendo shuffle y avances,
  mediante un canal que conserva solo el estado más reciente. Los mensajes
  atrasados del servidor no sobrescriben esa cola con el orden anterior.
- El panel de cola crea controles únicamente para las filas visibles y un pequeño
  margen. La longitud completa mantiene el desplazamiento; clic y menú contextual
  conservan el índice y la identidad de la canción. Las actualizaciones resuelven
  metadatos con una tabla por identificador, evitando búsquedas repetidas por toda
  la playlist.
- Añadir a la cola activa conserva la canción y el contexto actuales. El motor
  Connect anuncia una ventana de 80 próximas pistas, que se repone conforme
  avanza la playlist. La cola manual llena muestra una explicación.
- Seleccionar una fila activa avanza hasta su identificador de ocurrencia dentro
  de la cola existente y carga el audio una sola vez. Conserva playlist, shuffle
  y orden restante; rechaza una identidad que ya no corresponde a la cola.
- Máximo de dos solicitudes HTTP simultáneas, con separación de 300 ms entre
  despachos. El permiso se conserva hasta leer o descartar el cuerpo. Las lecturas
  equivalentes comparten un bloqueo por clave y reutilizan la respuesta almacenada;
  otras páginas en caché no esperan a una lectura lenta.
- Las lecturas JSON transitorias tienen un presupuesto total de tres intentos,
  incluyendo conexión y cuerpo. Se mantienen `Retry-After`, el cooldown persistente,
  la separación de cuentas y las escrituras sin reintentos ambiguos.
- Búsquedas y apertura de enlaces descartan respuestas de peticiones anteriores,
  también en la secuencia A → B → A.
- La detección de salida de audio pasa de cinco consultas por segundo a una cada
  dos segundos. La interfaz mantiene el seguimiento fluido de reproducción.
- Colores adaptados de Spotifast, separadores de un píxel con área de arrastre
  amplia, menor espacio perdido en paneles y duración visible en la cola.

## Alcance

Se revisó el código incorporado y su ruta de ejecución. Esto no equivale a una
auditoría completa de Spotifast ni de todas las dependencias. No se adoptaron
Client IDs compartidos, rutas alternativas para eludir cuotas ni cambios en las
credenciales. La reducción de consultas no puede cancelar una cuota HTTP 429
que Spotify ya haya aplicado. Historial, Wrapped y reporte nativo de escuchas
siguen limitados. Las comprobaciones locales se documentan en `VALIDATION.md`.
