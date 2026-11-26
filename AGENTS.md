# Convenciones de Xpotify

- Después de compilar y validar, conservar únicamente `Xpotify.exe` en la raíz como artefacto de compilación. Eliminar `target` y las carpetas temporales de compilación/empaquetado (`dist`, `build`). No borrar fuentes, recursos, dependencias versionadas, configuración ni cachés de usuario.
- Usar `scripts/Build-Native.ps1` en Windows. Copiar el ejecutable antes de limpiar y verificar las rutas absolutas antes de eliminar carpetas. No eliminar un `CARGO_TARGET_DIR` externo compartido.
- Mantener las dependencias instaladas fuera de OneDrive, en `D:\DevLibs`, reutilizando los junctions existentes. `vendor/librespot-*` contiene fuentes parcheadas versionadas: no es una instalación de Composer ni un artefacto de compilación.
