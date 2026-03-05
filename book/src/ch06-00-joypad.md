# Joypad: los controles

La Gameboy tiene 8 botones: direccional (arriba, abajo, izquierda, derecha) y de acción (A, B, Select, Start). El juego los lee a través del registro `0xFF00`.

## El problema: 8 botones, 4 bits

El byte que devuelve el registro `0xFF00` solo tiene 4 bits para el estado de los botones (bits 0-3). No caben los 8 de golpe. La solución de Nintendo fue dividirlos en **dos grupos** y seleccionar cuál leer con los bits 4 y 5:

```
Bit 5 = 0 → seleccionar grupo "acción"  (A, B, Select, Start)
Bit 4 = 0 → seleccionar grupo "dirección" (Right, Left, Up, Down)
```

El juego primero escribe en `0xFF00` con el bit que quiere consultar a 0, luego lee el resultado en los bits 0-3.

## Lógica activo-bajo

Los bits de estado de botones son **activo-bajo**: `0` significa pulsado, `1` significa suelto. Es contraintuitivo, pero es como funciona el hardware real.

```
Bits 3-0 para el grupo de dirección:
  Bit 3: Down   (0 = pulsado)
  Bit 2: Up     (0 = pulsado)
  Bit 1: Left   (0 = pulsado)
  Bit 0: Right  (0 = pulsado)
```

Por ejemplo, si el jugador pulsa Left y Up a la vez, los bits 2 y 1 estarán a 0, y el byte leído sería `0b11111001`.

## El registro completo

El byte completo de `0xFF00` cuando se lee:

```
Bit 7: siempre 1
Bit 6: siempre 1
Bit 5: grupo acción  seleccionado (0 = sí)
Bit 4: grupo dirección seleccionado (0 = sí)
Bit 3: Down  / Start  (activo-bajo)
Bit 2: Up    / Select (activo-bajo)
Bit 1: Left  / B      (activo-bajo)
Bit 0: Right / A      (activo-bajo)
```

## Cómo lo implementamos

El Joypad comparte un `HashMap<GameBoyKey, bool>` con la ventana GUI. La GUI actualiza ese mapa cada frame con el estado real del teclado, y el Joypad lo consulta cuando la CPU lee `0xFF00`:

```rust
fn read_keys(&self) -> u8 {
    let keys = self.keys.lock().unwrap();
    let mut nibble = 0x0F; // empezar con todos los bits a 1 (sueltos)

    if self.select & 0x20 == 0 {
        // Grupo acción seleccionado
        if *keys.get(&GameBoyKey::A).unwrap_or(&false) { nibble &= !0x01; }
        if *keys.get(&GameBoyKey::B).unwrap_or(&false) { nibble &= !0x02; }
        // ...
    }

    if self.select & 0x10 == 0 {
        // Grupo dirección seleccionado
        if *keys.get(&GameBoyKey::Right).unwrap_or(&false) { nibble &= !0x01; }
        // ...
    }

    nibble
}
```

El truco de `nibble &= !0x01` simplemente pone el bit 0 a 0 (pulsado) dejando el resto intacto.

## La interrupción de Joypad

Cuando se pulsa un botón por primera vez (flanco de bajada: de suelto a pulsado), el Joypad genera una **INT_JOYPAD**. Esto permite que el juego reaccione rápidamente a las pulsaciones sin tener que estar constantemente leyendo el registro.

La detección del flanco de bajada se hace comparando el estado actual con el anterior:

```rust
pub fn poll_interrupt(&mut self) -> bool {
    let current = self.read_keys();
    // Si un bit pasó de 1 (suelto) a 0 (pulsado) → flanco de bajada
    let newly_pressed = self.prev_keys & !current;
    self.prev_keys = current;
    newly_pressed != 0
}
```

## Mapeo de teclas del teclado

En nuestro emulador, las teclas del teclado se mapean así:

| Tecla teclado | Botón Gameboy |
|---|---|
| Flecha derecha | Derecha |
| Flecha izquierda | Izquierda |
| Flecha arriba | Arriba |
| Flecha abajo | Abajo |
| Z | A |
| X | B |
| Space | Select |
| Enter | Start |
| Escape | Cerrar emulador |
| M | Mutear/desmutear audio |
