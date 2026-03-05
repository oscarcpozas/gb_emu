# Sistema de interrupciones

Imagina que estás leyendo un libro (la CPU ejecutando instrucciones normalmente) y de repente suena el teléfono. Paras de leer, atiendes la llamada, y luego vuelves exactamente donde estabas. Eso es una **interrupción**: un mecanismo para que el hardware le avise a la CPU de que ocurrió algo urgente.

## Los dos registros clave

El sistema de interrupciones usa solo dos registros de memoria:

| Registro | Dirección | Nombre | Función |
|---|---|---|---|
| IF | `0xFF0F` | Interrupt Flag | Indica qué interrupciones han ocurrido |
| IE | `0xFFFF` | Interrupt Enable | Indica cuáles están habilitadas |

Y un flag interno de la CPU:

| Flag | Nombre | Función |
|---|---|---|
| IME | Interrupt Master Enable | Interruptor global de interrupciones |

## Las cinco interrupciones

La Gameboy tiene cinco tipos de interrupción, cada una con un bit propio en IF/IE y una dirección fija de salto (el "vector"):

| Bit | Nombre | Quien la genera | Vector |
|---|---|---|---|
| 0 | VBlank | PPU (fin de frame) | `0x0040` |
| 1 | LCD STAT | PPU (varios eventos) | `0x0048` |
| 2 | Timer | Timer (overflow de TIMA) | `0x0050` |
| 3 | Serial | Puerto serie (no implementado) | `0x0058` |
| 4 | Joypad | Botón recién pulsado | `0x0060` |

## Cómo funciona el flujo

```
Hardware (PPU, Timer, Joypad...)
    │
    │  "ocurrió algo"
    ▼
Setea bit en IF (Interrupt Flag)
    │
    │  (en cada step del emulador)
    ▼
Interrupt::dispatch() comprueba:
    IF & IE & 0x1F  ← ¿hay alguna pendiente Y habilitada?
    │
    ├── No → continuar con la CPU normalmente
    │
    └── Sí → ¿CPU.ime == true?
                │
                ├── No → despertar de HALT si estaba en halt, pero no saltar
                │
                └── Sí → despejar el bit en IF
                          │
                          cpu.ime = false  (no más interrupciones anidadas)
                          │
                          push PC en stack
                          │
                          PC = vector de la interrupción
                          │
                          CPU empieza a ejecutar el handler
```

## Un ejemplo: VBlank

```
1. PPU termina scanline 143
2. PPU llama interrupt.request(INT_VBLANK) → IF |= 0x01
3. En el siguiente step():
   - dispatch() ve IF=0x01, IE tiene bit 0, IME=true
   - Limpia IF bit 0
   - Pone IME=false (para evitar interrupciones durante el handler)
   - Guarda el PC actual en el stack
   - Salta a 0x0040 (handler VBlank del juego)
4. El juego actualiza sprites, lógica, etc.
5. Al final del handler, ejecuta RETI
   - RETI restaura IME=true
   - Pop PC del stack → vuelve al punto exacto donde estaba
```

## EI, DI y el delay de un ciclo

La CPU tiene dos instrucciones para controlar el IME:
- `DI` (Disable Interrupts) → `IME = false` inmediatamente
- `EI` (Enable Interrupts) → `IME = true` con **un ciclo de delay**

El delay de EI es importante: significa que la instrucción **después** de EI todavía no puede ser interrumpida. En nuestro código lo implementamos con un flag `ime_pending`:

```rust
// Cuando la CPU ejecuta EI:
self.ime_pending = true;  // no activar aún

// Al final de cada instrucción:
if self.ime_pending {
    self.ime = true;
    self.ime_pending = false;
}
```

`RETI` es diferente: activa IME **inmediatamente** (sin delay), ya que es la instrucción de retorno de una interrupción y necesita reactivar las interrupciones al instante.

## HALT: modo de bajo consumo

La instrucción `HALT` suspende la CPU hasta que llegue una interrupción. Esto se usa mucho en los juegos: en lugar de ejecutar un bucle vacío esperando el VBlank, el código hace `HALT` y la CPU simplemente... espera.

```rust
// Cuando llega una interrupción con cpu.halted == true:
if cpu.halted {
    cpu.halted = false;  // despertar aunque IME sea false
}
```

La CPU siempre despierta del HALT cuando hay una interrupción pendiente habilitada, aunque IME esté a false. La diferencia es que si IME es false, despierta pero no salta al vector; simplemente continúa ejecutando la siguiente instrucción.

## En el código

En `emu.rs`, el dispatch se llama al final de cada step, después de que todos los componentes hayan tenido oportunidad de generar interrupciones:

```rust
fn step(&mut self) -> usize {
    // ... ejecutar CPU, PPU, Timer, APU, Joypad ...

    // Al final, despachar las interrupciones pendientes
    let int_cycles = self.interrupt.borrow_mut().dispatch(&mut self.cpu, &mut self.mmu);

    cycles + int_cycles  // las interrupciones también cuestan ciclos (20)
}
```
