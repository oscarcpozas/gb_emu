# El arranque: de encender la consola a ver la primera imagen

Esta es quizás la parte más importante de entender, porque conecta todos los componentes. Vamos a seguir paso a paso qué ocurre desde que se enciende la Gameboy hasta que aparece la primera imagen en pantalla.

## El ciclo principal

Todo el emulador gira alrededor de un bucle que imita el reloj físico de la consola. La Gameboy corre a **4.194.304 ciclos por segundo**, y queremos renderizar a **60 fotogramas por segundo**. Eso nos da:

```
4.194.304 / 60 ≈ 69.905 ciclos por fotograma
```

El bucle principal en `emu.rs` tiene esta estructura:

```rust
while ventana_abierta {
    let inicio_frame = Instant::now();

    // Ejecutar ~69.905 ciclos de CPU
    emu.run_frame();

    // Dormir el tiempo que falte para completar 1/60 de segundo
    let tiempo_usado = inicio_frame.elapsed();
    if tiempo_usado < duracion_objetivo {
        sleep(duracion_objetivo - tiempo_usado);
    }
}
```

Dentro de `run_frame`, llamamos a `step()` una y otra vez hasta completar los ciclos del frame. Cada `step()` es una **instrucción de CPU** más la actualización de todos los componentes.

## Un `step()` completo

Esto es lo que ocurre en cada paso del emulador:

```
┌─────────────────────────────────────────────────┐
│  1. ¿Hay un DMA pendiente? → Copiar sprites      │
│  2. CPU: fetch + decode + execute (1 instrucción)│
│  3. PPU: avanzar N ciclos (dibujo de pantalla)   │
│  4. Timer: avanzar N ciclos                      │
│  5. APU: avanzar N ciclos (generación de audio)  │
│  6. Joypad: ¿alguna tecla nueva pulsada?         │
│  7. Interrupciones: ¿hay alguna pendiente?       │
└─────────────────────────────────────────────────┘
```

El número de ciclos de cada paso depende de la instrucción que ejecutó la CPU (las hay de 4, 8, 12 o 20 ciclos).

## Fase 1: La Boot ROM (los primeros 256 bytes)

Al encender la Gameboy real, antes de ejecutar nada del cartucho, la consola ejecuta un pequeño programa grabado en su hardware: la **Boot ROM**. Son 256 bytes que hacen varias cosas:

1. **Inicializar la RAM** — pone todo a cero
2. **Copiar el logo de Nintendo** — lo lee del cartucho y lo dibuja en VRAM
3. **Reproducir el sonido de arranque** — el famoso "ding"
4. **Verificar el logo** — compara el logo del cartucho con el que tiene grabado. Si no coincide, se cuelga (protección antipiratería)
5. **Desactivarse** — escribe `1` en `0xFF50` para ceder el control al cartucho

En nuestro emulador, la Boot ROM está incluida como bytes crudos en `io/boot.rs`. El handler de la Boot ROM responde a `0x0000-0x00FF` mientras está activa, y devuelve `PassThrough` en cuanto se desactiva.

```
PC = 0x0000
    │
    ▼
Boot ROM ejecutándose...
    │  (∼32.000 ciclos después)
    ▼
Escribe 0x01 en 0xFF50 → Boot ROM se desactiva
    │
    ▼
PC = 0x0100 → Primera instrucción del cartucho
```

## Fase 2: El cartucho toma el control

A partir de `0x0100`, la CPU empieza a ejecutar el código del juego. Las primeras instrucciones suelen ser un salto (`JP`) a la dirección real de inicio del juego, porque `0x0100-0x014F` está reservado para la cabecera del cartucho (nombre del juego, tipo de MBC, etc.).

## Fase 3: La PPU construye los fotogramas

Mientras la CPU ejecuta código, la PPU trabaja en paralelo (simulado, ya que todo es un bucle secuencial). La PPU divide su trabajo en cuatro **modos** que se van alternando:

```
Scanline 0:
  ├─ OAM scan    (80 ciclos)  → ¿qué sprites aparecen en esta línea?
  ├─ Transfer    (172 ciclos) → dibujar los píxeles de esta línea
  └─ HBlank      (204 ciclos) → pausa horizontal (CPU puede acceder a VRAM)

Scanline 1:
  └─ (igual)
...
Scanline 143:
  └─ (igual)

Scanlines 144-153:
  └─ VBlank      (4560 ciclos) → pausa vertical (frame completado)
```

Cada scanline dura **456 ciclos**. Con 144 líneas visibles + 10 de VBlank = 154 líneas × 456 ciclos = **70.224 ciclos por frame** (muy cercano a nuestro objetivo de 60fps).

Al terminar la scanline 143, la PPU lanza una **interrupción VBlank**. Esto es la señal para que el juego sepa que el frame está listo y puede actualizar lo que necesite antes del siguiente.

## Fase 4: Las interrupciones coordinan todo

Las interrupciones son la forma que tiene el hardware de decirle a la CPU "para lo que estabas haciendo, ocurrió algo importante". Los eventos que generan interrupciones son:

| Interrupción | Quién la genera | Vector (dirección de salto) |
|---|---|---|
| VBlank | PPU (fin de frame) | `0x0040` |
| LCD STAT | PPU (varios eventos) | `0x0048` |
| Timer | Timer (overflow) | `0x0050` |
| Serial | (no implementado) | `0x0058` |
| Joypad | Botón pulsado | `0x0060` |

El juego tiene código en esas direcciones para manejar cada evento. Por ejemplo, Tetris usa la interrupción VBlank para mover las piezas y actualizar la pantalla exactamente 60 veces por segundo.

## El flujo completo, de principio a fin

```
ENCENDIDO
    │
    ▼
CPU empieza en 0x0000 → Boot ROM
    │   (∼32.000 ciclos)
    │   PPU inicializa modo OAM
    │   APU sin sonido aún
    ▼
Boot ROM termina → cartucho en 0x0100
    │
    ▼
CPU ejecuta código del juego
    │
    │  ┌─────────── cada instrucción ───────────────────────┐
    │  │                                                     │
    │  │  CPU fetch → PPU tick → Timer tick → APU tick      │
    │  │  → Joypad poll → Interrupts dispatch               │
    │  │                                                     │
    │  └─────────────────────────────────────────────────────┘
    │
    │  (después de 144 scanlines)
    ▼
PPU lanza INT_VBLANK
    │
    ▼
CPU salta a 0x0040 → handler VBlank del juego
    │   (el juego actualiza sprites, lógica, etc.)
    ▼
CPU vuelve al código normal
    │
    ▼
PPU empieza el siguiente frame
    │
    ▼
  (bucle infinito hasta que cierres la ventana)
```

Así es como la Gameboy —y nuestro emulador— produce los 60 fotogramas por segundo que ves en pantalla.
