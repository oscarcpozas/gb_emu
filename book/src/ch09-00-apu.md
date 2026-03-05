# APU: el sistema de audio

La **APU** (Audio Processing Unit) es el chip de sonido de la Gameboy. Genera audio mezclando cuatro canales independientes, cada uno con un tipo de onda distinto. Tetris, Pokémon, Zelda... toda esa música icónica sale de esta pequeña bestia de 4 canales.

## La cadena de audio

El audio sigue este camino desde la Gameboy hasta tus altavoces:

```
CPU escribe en registros 0xFF10-0xFF3F
         │
         ▼
APU actualiza el estado de los canales
         │
         ▼  (cada ~380 T-ciclos = 1 muestra a 44.100 Hz)
Mezcla los 4 canales → 1 muestra f32 (-1.0 a 1.0)
         │
         ▼
Ring buffer (4096 muestras de colchón)
         │
         ▼  (hilo del sistema de audio, separado)
cpal → tarjeta de sonido → altavoces
```

El **ring buffer** desacopla el hilo del emulador del hilo de audio del sistema operativo. El emulador empuja muestras al buffer; el SO las consume a su ritmo. Si el SO pide muestras y el buffer está vacío, reproduce silencio en lugar de bloquearse.

## Los cuatro canales

### Canal 1 — Onda cuadrada con sweep de frecuencia (NR10-NR14)

Una onda cuadrada alterna entre nivel alto y bajo a una frecuencia determinada, produciendo un sonido parecido a un pitido. El canal 1 añade además un **sweep de frecuencia**: puede subir o bajar la frecuencia automáticamente con el tiempo, creando efectos de sirena o glissando.

```
Onda cuadrada (duty 50%):
  ┌───┐   ┌───┐   ┌───┐
  │   │   │   │   │   │
──┘   └───┘   └───┘   └──

Onda cuadrada (duty 12.5%):
  ┐       ┐       ┐
  │       │       │
──┘───────┘───────┘───────
```

Hay cuatro **ciclos de trabajo** (duty cycles) disponibles, que determinan qué porcentaje del ciclo está el nivel alto:

| Duty | Patrón | Sonido |
|---|---|---|
| 12.5% | `00000001` | Muy fino, casi un pitido |
| 25%   | `10000001` | Fino |
| 50%   | `10001111` | Cuadrada pura (la más común) |
| 75%   | `01111110` | Parecido al 25% invertido |

**Registros del canal 1:**
- `NR10` (`0xFF10`): período del sweep, dirección (subir/bajar), desplazamiento
- `NR11` (`0xFF11`): ciclo de trabajo + longitud inicial
- `NR12` (`0xFF12`): volumen inicial + dirección de envelope + período
- `NR13` (`0xFF13`): 8 bits bajos de la frecuencia
- `NR14` (`0xFF14`): 3 bits altos de frecuencia + trigger + activar longitud

### Canal 2 — Onda cuadrada simple (NR21-NR24)

Idéntico al canal 1 pero **sin sweep de frecuencia**. Mismos registros, misma lógica, simplemente un pitido que no cambia de nota automáticamente.

### Canal 3 — Onda arbitraria / Wave (NR30-NR34)

En lugar de una onda cuadrada fija, el canal 3 reproduce una forma de onda **definida por el propio juego**. Hay 16 bytes de "Wave RAM" en `0xFF30-0xFF3F` que contienen 32 muestras de 4 bits cada una (0-15). La PPU las recorre en orden a la frecuencia configurada.

```
Wave RAM (0xFF30-0xFF3F):
  Byte 0: 0xAB → muestras: 10 (0xA) y 11 (0xB)
  Byte 1: 0x34 → muestras:  3 (0x3) y  4 (0x4)
  ...

La onda resultante:
 15 │    ╭─╮
 10 │╭─╮╭╯ ╰╮
  5 │╯ ╰╯    ╰─...
  0 └─────────────
```

El nivel de salida se controla con el registro `NR32`: puede reproducir la onda al 100%, al 50%, al 25% o en silencio.

**Registros del canal 3:**
- `NR30` (`0xFF1A`): activar DAC (encender/apagar el canal)
- `NR31` (`0xFF1B`): longitud (0-255)
- `NR32` (`0xFF1C`): nivel de salida
- `NR33` (`0xFF1D`): 8 bits bajos de frecuencia
- `NR34` (`0xFF1E`): 3 bits altos + trigger + activar longitud

### Canal 4 — Ruido / LFSR (NR41-NR44)

El canal de ruido genera sonido aleatorio usando un **LFSR** (Linear Feedback Shift Register): un registro que se va desplazando y cuyo bit de salida se mezcla con otro bit para producir una secuencia pseudo-aleatoria.

Se usa para explosiones, efectos de percusión, estática, etc.

El LFSR puede ser de 15 bits (sonido más ruidoso) o de 7 bits (ruido más "tonal", con un pitido de fondo), controlado por un bit en `NR43`.

```
LFSR de 15 bits:
  bit 14  13  12  ...  1  0
    ←─────────────────────── desplazamiento
    ↑                   │ │
    └── XOR ────────────┘ │
                          └── salida (0 o volumen)
```

**Registros del canal 4:**
- `NR41` (`0xFF20`): longitud
- `NR42` (`0xFF21`): volumen + envelope
- `NR43` (`0xFF22`): reloj, modo LFSR (7 o 15 bits), divisor
- `NR44` (`0xFF23`): trigger + activar longitud

## El envelope de volumen

Los canales 1, 2 y 4 tienen un **envelope** (envolvente) de volumen: el volumen puede subir o bajar automáticamente con el tiempo. Esto permite efectos como el fade-out de notas o el ataque de un sonido.

El envelope tiene:
- **Volumen inicial** (0-15)
- **Dirección**: subir o bajar
- **Período**: cada cuántos pasos del frame sequencer cambiar el volumen

## El frame sequencer

El frame sequencer es un contador interno que dispara distintos eventos a diferentes frecuencias, **todas derivadas del reloj principal**:

```
Frecuencia base: 512 Hz (un tick cada 32.768 T-ciclos)

Paso 0: Length (256 Hz)
Paso 1: —
Paso 2: Length + Sweep (128 Hz)
Paso 3: —
Paso 4: Length (256 Hz)
Paso 5: —
Paso 6: Length + Sweep (128 Hz)
Paso 7: Envelope (64 Hz)
→ volver al paso 0
```

- **Length** (longitud): si un canal tiene la longitud activada, su contador se decrementa. Al llegar a 0, el canal se apaga automáticamente.
- **Sweep**: calcula la nueva frecuencia del canal 1 según la configuración del sweep.
- **Envelope**: ajusta el volumen de los canales 1, 2 y 4.

## Los registros maestros

Tres registros controlan el sistema global:

| Registro | Dirección | Función |
|---|---|---|
| NR50 | `0xFF24` | Volumen maestro izquierdo/derecho (0-7 cada uno) |
| NR51 | `0xFF25` | Panning: qué canales van por izquierda y/o derecha |
| NR52 | `0xFF26` | Bit 7: master enable. Bits 0-3: estado de canales (solo lectura) |

Cuando el bit 7 de NR52 se pone a 0, la APU se apaga completamente y todos los registros se resetean a 0. Solo NR52 puede escribirse en ese estado.

## El filtro high-pass

El hardware original de la Gameboy tiene un condensador a la salida del DAC que elimina el componente de corriente continua (DC) de la señal. Esto es importante porque cuando los canales están en silencio (valor de salida = 0), sin el filtro estaríamos enviando una señal constante a los altavoces, lo que suena como un zumbido constante.

En nuestro emulador lo replicamos con un filtro digital de paso alto:

```rust
// raw = suma de canales / 60.0  (rango 0.0 a 1.0)
let sample = raw - self.hp_cap;
self.hp_cap = self.hp_cap * 0.999 + raw * 0.001;
```

El `hp_cap` va rastreando lentamente el nivel medio de la señal y lo resta. Cuando el audio es constante (silencio), el cap se ajusta a ese nivel y la salida converge a 0. Cuando hay variaciones rápidas (música), el cap no puede seguirlas y pasan a la salida.

## La tecla M: mute

El emulador incluye una función de silencio instantáneo: pulsar **M** alterna el mute. Cuando está muteado, el APU sigue procesando internamente (los canales avanzan, el frame sequencer sigue su ritmo), pero las muestras que se envían al ring buffer son `0.0` en lugar del valor calculado. Esto asegura que al desmutear el sonido retoma exactamente donde estaba, sin glitches.
