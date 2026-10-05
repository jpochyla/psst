# Revisión de seguridad · 2026-10-04

Base revisada: fork `angelopol/xpotify`, commit `3c3621a`, derivado de Psst. Se revisaron el código propio de Rust, las conexiones salientes, OAuth, almacenamiento de credenciales, scripts de compilación, workflows y las dependencias con Cargo Audit/RustSec. La revisión es estática con pruebas y comprobaciones de ejecución; no es una certificación de ausencia absoluta de puertas traseras.

## Hallazgos corregidos

1. **Fuga grave de token a un tercero por HTTP.** `WebApi::get_user_info` enviaba `Authorization: Bearer <token Spotify>` a `http://ip-api.com/json`. Se eliminó esa consulta. País se obtiene de la API oficial de Spotify y zona horaria se obtiene de `TZ` local, con UTC por defecto. Si se utilizó previamente el fork autenticado, revocar el acceso en Spotify es recomendable; no se puede recuperar la confidencialidad de un token ya enviado.
2. **Tokens enviados a CDN de imágenes.** `get_image` pasaba URLs de imágenes a la misma función de peticiones autenticadas. Ahora descarga sin credenciales, restringe fuentes a dominios HTTPS esperados y limita el tamaño a 16 MiB. Las peticiones que contienen tokens solo aceptan los tres hosts oficiales usados por el cliente: `api.spotify.com`, `api-partner.spotify.com`, `spclient.wg.spotify.com`, con HTTPS/443 y sin usuario/contraseña en la URL. Las redirecciones están desactivadas. Se añadió una prueba de regresión de los destinos admitidos/rechazados.
3. **OAuth sin comprobación de `state`.** Se comprueba el valor aleatorio generado por cada flujo, método GET, ruta `/login` y ausencia de parámetros `state` duplicados. Se conservó PKCE. El listener se limita a loopback, tiene límites de tamaño/tiempo y libera el puerto al terminar o expirar. Errores de intercambio de sesión ya no provocan un panic.
4. **Servidor de reproducción no autenticado.** El protocolo cifrado Shannon/Diffie-Hellman no verificaba la firma de la clave pública remota. Ahora comprueba la firma RSA contra la clave pública de Spotify utilizada por librespot antes de transmitir credenciales. SHA-1 se usa exclusivamente para verificar la firma exigida por este protocolo heredado. Se limita el tamaño del paquete de handshake y se rechazan firmas inexistentes o inválidas.
5. **Dependencias con vulnerabilidades conocidas.** Se actualizaron dependencias compatibles, OAuth a v5 con transporte ureq mantenido, caché LRU y TLS. Se reemplazó el cliente Last.fm abandonado para eliminar su TLS antiguo, manteniendo scrobbling opcional mediante HTTPS y firmas requeridas por Last.fm. Cargo.lock fija las versiones y revisiones Git. Los scripts usan `--locked`.
6. **Compilación dependiente de la URL Git.** Se eliminó la interpolación de una URL Git arbitraria dentro del código Rust generado. El cliente se puede compilar desde un archivo de fuentes o un subdirectorio.
7. **Integración Gemini.** El modelo es `gemini-3.5-flash-lite`; la clave se envía en `x-goog-api-key`, no en URLs. Hay timeout y no se siguen redirecciones. Solo salen metadatos musicales. La respuesta se valida contra las canciones originales; no se aceptan IDs inventados y no se omiten canciones silenciosamente. Ningún resultado de IA se ejecuta como código ni como comando del sistema.

8. **Clave privada Diffie-Hellman de solo 32 bits.** El exponente era un `u32` aleatorio, insuficiente para proteger las claves de sesión. Ahora usa 95 bytes aleatorios, conforme al grupo heredado utilizado por Spotify. Se añadió una prueba de entropía/tamaño y acuerdo de secretos. Se actualizó el saludo del protocolo a plataforma nativa, producto cliente y versión usada por librespot, y se prueba el siguiente servidor si el anterior rechaza el handshake.

No se encontró instalación oculta, persistencia, ejecución remota de código ni un mecanismo deliberado de puerta trasera en el código propio revisado. Los puntos 1 y 2 son fugas reales; su intención no puede establecerse a partir del código. La declaración anterior del README de conectar exclusivamente a Spotify era inexacta y se corrigió.

## Límites y riesgos que permanecen

OAuth correction validated with the actual account: Developer Web API authorization uses the configured Client ID and registered `http://127.0.0.1:8888/login`; native playback uses the desktop client with its registered `http://127.0.0.1:8898/login`, streaming-only scope and separate reusable credentials. Both use PKCE/state and bind loopback before opening the browser. Passing Developer-client credentials to desktop Login5 was rejected and has been fixed. Actual audio download, decoding and playback through the default Windows output device now pass. Developer API credentials remain separate.

- `cargo audit` reporta **0 vulnerabilidades de su categoría principal** después de las actualizaciones. Sigue reportando advertencias `unsound` en `im` y `sized-chunks`, dependencias archivadas del framework Druid sin parche publicado. `im` afecta inserción en `OrdSet`; la aplicación usa Vector/HashSet. `sized-chunks` tiene un problema de seguridad de memoria si un destructor provoca panic. No se considera resuelto ni se afirma riesgo cero. También existen advertencias de mantenimiento en componentes del framework. `glib` aparece en el lock multiplataforma, pero no se compila en Windows. Sustituir Druid es trabajo adicional para eliminar esas dependencias.
- Los tokens reutilizables de Spotify siguen almacenándose en el archivo local de configuración de Psst, con protección por permisos del usuario del sistema; no se migraron al Credential Manager de Windows. La clave Gemini está en `.env.local`, ignorada por Git. No compartir esos archivos ni el perfil local del cliente.
- OAuth requires Spotify consent and the Developer callback above. Playback is validated for this account; remote playlist creation remains untested to avoid creating unsolicited playlists.
- La creación remota no es transaccional: una caída de red puede dejar playlists parcialmente creadas. La UI muestra el resultado parcial y bloquea la repetición inmediata. Library saving/removal was tested reversibly, restoring the original state; no remote playlists were created.
- La evaluación no inspeccionó manualmente cada línea de todos los crates transitivos ni constituye un análisis de malware del sistema operativo.

Fuentes del protocolo y avisos: [handshake librespot](https://github.com/librespot-org/librespot/blob/dev/core/src/connection/handshake.rs), [RustSec](https://rustsec.org/), [OAuth2/PKCE](https://docs.rs/oauth2/5.0.0/oauth2/), [modelo Gemini](https://ai.google.dev/gemini-api/docs/models/gemini-3.5-flash-lite).

## Comprobaciones reproducibles

```powershell
cargo test -p psst-core -p psst-gui
cargo build --locked --bin psst-gui
cargo audit
cargo run -p psst-core --features cpal --example verify_connection
```

La comprobación `verify_connection` autentica el servidor Spotify sin enviar credenciales de usuario. Las comprobaciones de Gemini con la clave local devolvieron HTTP 200 tanto al consultar el modelo como al generar una clasificación estructurada mínima.

Lyrics update: an independent unauthenticated HTTP client sends only song title, artist, album and duration to https://lrclib.net. It rejects redirects, limits response bodies to 2 MiB and times out in 12 seconds per request. Spotify tokens never reach LRCLIB or YouTube. Video search constructs an encoded query on a fixed https://www.youtube.com/results URL and opens the system browser; song metadata cannot change its destination.
