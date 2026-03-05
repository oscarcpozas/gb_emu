# Timer y Divider

El timer de la Gameboy es un componente que cuenta ciclos de CPU y genera una interrupción cuando se desborda. Los juegos lo usan para medir tiempos con precisión: animar sprites, controlar la velocidad del juego, generar efectos de sonido, etc.

## Los cuatro registros

| Registro | Dirección | Nombre | Función |
|---|---|---|---|
| DIV | `0xFF04` | Divider | Se incrementa 256 veces por segundo. Escribir cualquier cosa lo resetea a 0. |
| TIMA | `0xFF05` | Timer Counter | El contador principal. Se incrementa a la frecuencia configurada en TAC. |
| TMA | `0xFF06` | Timer Modulo | Valor que se carga en TIMA cuando hay overflow. |
| TAC | `0xFF07` | Timer Control | Bit 2: activar/desactivar. Bits 1-0: frecuencia. |

## Cómo funciona TIMA

TIMA es un contador de 8 bits (0-255). Se va incrementando a la frecuencia que diga TAC. Cuando llega a 255 y se incrementa una vez más, **se desborda** (overflow): se resetea al valor de TMA y genera una **interrupción INT_TIMER** (`0x0050`).

```
TIMA:  0 → 1 → 2 → ... → 254 → 255 → OVERFLOW → TMA → TMA+1 → ...
                                           ↑
                              genera INT_TIMER
                              TIMA se recarga con el valor de TMA
```

El truco de TMA es útil: si el juego quiere un timer que dispare cada 100 pasos, pone TMA = 156 (256 - 100). Así TIMA empezará en 156 y llegará a 255 exactamente 100 incrementos después.

## Frecuencias de TIMA (TAC bits 1-0)

| Bits 1-0 | Frecuencia | Ciclos por incremento |
|---|---|---|
| `00` | 4.096 Hz | cada 1.024 ciclos |
| `01` | 262.144 Hz | cada 16 ciclos |
| `10` | 65.536 Hz | cada 64 ciclos |
| `11` | 16.384 Hz | cada 256 ciclos |

## El Divider interno

Internamente, la Gameboy tiene un contador de 16 bits que se incrementa en cada ciclo de CPU. El registro DIV es simplemente el **byte alto** de ese contador:

```
Contador interno (16 bits): se incrementa cada ciclo
DIV (0xFF04) = contador >> 8  →  se incrementa 4.194.304/256 ≈ 16.384 veces/seg
```

Cuando el juego escribe en `0xFF04` (cualquier valor), el contador interno completo se pone a 0, lo que también pone DIV a 0. Esto es un comportamiento del hardware que algunos juegos explotan para sincronización.

## En el código

```rust
pub fn update(&mut self, cycles: usize) -> u8 {
    let mut interrupt: u8 = 0;

    // DIV: el contador interno sube siempre
    self.counter = self.counter.wrapping_add(cycles as u16);

    // TIMA solo sube si TAC bit 2 está activo
    if self.tac & 0x04 != 0 {
        self.tima_cycles += cycles as u32;
        let threshold = self.tima_period();  // ciclos por incremento según TAC

        while self.tima_cycles >= threshold {
            self.tima_cycles -= threshold;
            let (new_tima, overflowed) = self.tima.overflowing_add(1);
            if overflowed {
                self.tima = self.tma;      // recargar desde TMA
                interrupt = INT_TIMER;     // señalar interrupción
            } else {
                self.tima = new_tima;
            }
        }
    }

    interrupt
}
```

El `update()` devuelve el bitmask de interrupción (`INT_TIMER` o `0`), y `emu.rs` lo pasa al controlador de interrupciones si es distinto de cero.
