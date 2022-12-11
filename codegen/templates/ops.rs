{% macro nop(i) %}
{% endmacro %}

{% macro inc8(i) %}
let v = {{ i.operands[0].name | getter(bits=i.bits) }};
let (v, h, c, z) = alu::add8(v, 1, false);
{{ i.operands[0].name | setter(bits=i.bits) }}v);
{% endmacro %}

{% macro inc16(i) %}
let v = {{ i.operands[0].name | getter(bits=i.bits) }}.wrapping_add(1);
{{ i.operands[0].name | setter(bits=i.bits) }}v);
{% endmacro %}

{% macro dec8(i) %}
let v = {{ i.operands[0].name | getter(bits=i.bits) }};
let (v, h, c, z) = alu::sub8(v, 1, false);
{{ i.operands[0].name | setter(bits=i.bits) }}v);
{% endmacro %}

{% macro dec16(i) %}
let v = {{ i.operands[0].name | getter(bits=i.bits) }}.wrapping_sub(1);
{{ i.operands[0].name | setter(bits=i.bits) }}v);
{% endmacro %}

{% macro ld(i) %}
let v = {{ i.operands[1].name | getter(bits=i.bits) }};
{{ i.operands[0].name | setter(bits=i.bits) }}v);
{% endmacro %}

{% macro ldhl(i) %}
let p = {{ i.operands[0].name | getter(bits=i.bits) }};
let q = {{ i.operands[1].name | getter(bits=i.bits) }};
let (v, h, c, z) = alu::add16e(p, q, false);
cpu.set_hl(v);
{% endmacro %}

{% macro add8(i) %}
let p = {{ i.operands[0].name | getter(bits=i.bits) }};
let q = {{ i.operands[1].name | getter(bits=i.bits) }};
let (v, h, c, z) = alu::add8(p, q, false);
{{ i.operands[0].name | setter(bits=i.bits) }}v);
{% endmacro %}

{% macro add16(i) %}
let p = {{ i.operands[0].name | getter(bits=i.bits) }};
let q = {{ i.operands[1].name | getter(bits=i.bits) }};
let (v, h, c, z) = alu::add16(p, q, false);
{{ i.operands[0].name | setter(bits=i.bits) }}v);
{% endmacro %}

{% macro addsp(i) %}
let p = {{ i.operands[0].name | getter(bits=i.bits) }};
let q = {{ i.operands[1].name | getter(bits=i.bits) }};
let (v, h, c, z) = alu::add16e(p, q, false);
{{ i.operands[0].name | setter(bits=i.bits) }}v);
{% endmacro %}