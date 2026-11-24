# TreeFeller 🌳

Mod nativo para **Horizon** que tala automáticamente un árbol cuando rompes uno de sus troncos con un hacha.

## Características

- Tala los troncos conectados al tronco que rompes, incluso cuando están conectados en diagonal.
- Retira las hojas naturales asociadas al árbol.
- Realiza la tala de arriba hacia abajo con animación por capas.
- Evita activarse si estás agachado.
- Requiere que tengas un hacha en la mano principal.
- Incluye límites de seguridad para evitar procesar estructuras o grupos de bloques demasiado grandes.

## Requisitos

- **Minecraft:** 26.3.
- **Plataforma:** Horizon.
- **Tipo de mod:** nativo (Rust).
- **VERSION NIGHTLY:** f1bb66f (22 nov, 21:01 UTC update-9 .
## Instalación

1. Abre Horizon Launcher.
2. Selecciona la instancia donde quieres usar TreeFeller.
3. Entra en la sección **Mods**.
4. Selecciona **Add from GitHub**.
5. Introduce el repositorio:

   `https://github.com/moralesveloso1632-hash/treefaller`

6. Confirma la instalación y la compilación.
7. Si Horizon solicita permiso para ejecutar código nativo, revisa el código y autorízalo solo si confías en él.

> **Aviso de seguridad:** los mods nativos pueden ejecutarse con los permisos de tu usuario y acceder a archivos del equipo. Instala únicamente código en el que confíes.

## Cómo usarlo

1. Ten un hacha en la **mano principal**.
2. Acércate a un árbol natural.
3. No estés agachado.
4. Rompe uno de los troncos del árbol.

TreeFeller buscará los troncos conectados y realizará la tala. Para evitar activaciones accidentales, el mod comprueba que haya hojas naturales cerca del árbol.

### Hachas admitidas

- Madera
- Piedra
- Cobre
- Hierro
- Oro
- Diamante
- Netherita

### Desactivar la tala

Si quieres romper bloques normalmente, mantén pulsada la tecla de agacharse mientras rompes el tronco. En ese caso, TreeFeller no tala el resto del árbol.

## Comandos

Puedes activar o desactivar la animación de tala:

| Comando | Función |
|---|---|
| `/treefaller anim on` | Activa la animación por capas. |
| `/treefaller anim off` | Desactiva la animación y realiza la tala de una sola vez. |

La animación está activada de forma predeterminada.

## Límites de seguridad

TreeFeller utiliza límites para reducir el riesgo de afectar construcciones o procesar demasiados bloques:

- Busca troncos dentro de un radio horizontal limitado y una altura máxima.
- Requiere un mínimo de hojas naturales.
- Tiene un máximo de troncos y hojas que puede procesar en una tala.

Por eso, algunos árboles modificados o estructuras de madera pueden no activar la tala automática.

## Versión y compatibilidad

- **Versión actual:** 0.1.0
- **Etiqueta estable:** `v0.1.0`
- **Minecraft:** 26.3
- **Horizon API:** 0.2

La compatibilidad depende de que Horizon mantenga disponibles las API utilizadas por el mod.

## Reportar problemas

Si encuentras un error, abre una incidencia en la pestaña **Issues** de este repositorio e incluye:

- Qué estabas haciendo cuando ocurrió.
- Qué esperabas que sucediera.
- Qué sucedió realmente.
- El registro o informe de error de Horizon, si está disponible.

---

**TreeFeller** — tala árboles de forma sencilla en Horizon. 🌳
