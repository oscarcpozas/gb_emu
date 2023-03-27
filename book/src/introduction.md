# Introduccion

> Lo primero de todo. Si has llegado hasta aquí sin saber cómo, esta es la documentación/anotaciones de mi proyecto 
> público en Github para emular la Gameboy en Rust.

Creo que una buena forma de comprender cómo es el funcionamiento de un ordenador y sus componentes desde un punto de vista
de software/coding es tratando de emular sus comportamientos y relaciones. Es por ello que he elegido una de las consolas 
portátiles más míticas como lo fue la Gameboy de Nintendo. La idea es replicar el comportamiento al 100% de esta consola 
al mismo tiempo que entender cómo funciona un sistema de este tipo.

Elijo la Gameboy porque ya hay mucha documentación y otros proyectos que han cumplido con este mismo objetivo en los que
me puedo apoyar y tratar de no alargar el proceso demasiado tiempo.

## ¿Por qué en Rust?

Es normal que en los proyectos en los que se trata de emular sistemas se utilicen lenguajes considerados low-level 
dado que el rendimiento es un factor clave. Sin embargo, es cierto que lo poco exigente que resulta la Gameboy para 
los ordenadores actuales hace que podamos resolver este problema en casi cualquier lenguaje.

**Entonces, ¿por qué Rust?** Era un lenguaje de bajo nivel con el que quería probar hacer algo que resultara retador. 
Además de que lo aprendido en este proyecto podría ser de utilidad para futuros emuladores que quiera hacer donde si 
deba tener más en cuenta el rendimiento. También es un lenguaje con una comunidad sólida en la que poder apoyarme 
en el camino.

