# CPU

La Gameboy lleva un [procesador Sharp LR35902](https://en.wikipedia.org/wiki/Game_Boy), se trata de un híbrido entre el Zilog Z80 y el Intel 8080. Este corre a 
**4.19MHz**, o lo que es lo mismo, puede hacer **4.19 millones de ciclos de reloj por segundo**.

El **Intel 8080** fue un procesador usado por diferentes ordenadores en la época de los 70s 80s, incluyendo el ordenador
personal [Altair 8800](https://en.wikipedia.org/wiki/Altair_8800).

No entraremos en los detalles específicos que diferencian al LR35902 de los otros dos procesadores ya mencionados porque
no nos será necesario tener en cuenta para la emulación.

## Registros

La CPU esta compuesta por 8 registros diferentes, estos se encargan de almancenar pequeñas cantidades de información
que se utilizan para realizar operaciones. Como la CPU de la Gameboy es de 8-bits, esto quiere decir que cada registro
puede almacenar un valor de 0 a 255 (2^8 - 1). O lo que es lo mismo, 8 bits (a.k.a 1 byte).

| 16-bit | Hi | Lo | Name/Function |
| --- | --- | --- | --- |
| AF | A | - | Accumulator & Flags |
| BC | B | C | BC |
| DE | D | E | DE |
| HL | H | L | HL |

Aunque la CPU solo tiene registros de 8-bits, hay instrucciones que permiten acceder a los registros de 16-bits. Por ejemplo,
la instrucción `LD HL, 0x1234` carga el valor `0x1234` en el registro `HL`. Para ello, se utiliza el registro `H` para
almacenar el byte más significativo y el registro `L` para almacenar el byte menos significativo. (No importa si esto no lo entiendes
por ahora, lo veremos más adelante.)

En el código de la CPU, los registros se representan como `u8` ([unsigned 8-bit integer](https://doc.rust-lang.org/std/primitive.u8.html))

```rust
pub struct Cpu {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: u8,
    ...
}
```

_[Referencia al código](https://github.com/oscarcpozas/gb_emu/blob/master/emu/src/cpu.rs#L4)_


## Regsitro F (Flags register)

El registro `F` es un registro especial que se utiliza para almacenar información sobre el resultado de las operaciones
realizadas por la CPU. Los 4 bits menos significativos siempre serán 0. Los 4 bits más significativos se utilizan para
almacenar cuando ciertas cosas han ocurrido. Por el momento no voy a entrar en más detalles sobre esto, pero lo veremos.

| Bit | Name | Explanation |
| --- | --- | --- |
| 7 | z | Zero flag |
| 6 | n | Subtraction flag (BCD) |
| 5 | h | Half Carry flag (BCD) |
| 4 | c | Carry flag |

Y aquí un diagrama de los bits del registro `F`:

```
   ┌-> Carry
 ┌-+> Subtraction
 | |
1111 0000
| |
└-+> Zero
  └-> Half Carry
```