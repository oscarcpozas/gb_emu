import { parse as yalmParse } from "https://deno.land/std@0.161.0/encoding/yaml.ts"

function fixJson(yamlFilePath, jsonFilePath) {
    const yaml = yalmParse(Deno.readTextFileSync(yamlFilePath))
    const json = JSON.parse(Deno.readTextFileSync(jsonFilePath))

    let unprefixed_result = []
    let cbprefixed_result = []

    yaml.forEach(function (value) {
        if (value.code <= 255) {
            json.unprefixed.forEach(function (item) {
                if(Number(item.code) === value.code) {
                    unprefixed_result.push({
                        code: item.code,
                        mnemonic: item.mnemonic,
                        bits: value.bits,
                        ...item
                    })
                }
            })
        } else {
            json.cbprefixed.forEach(function (item) {
                if(Number(item.code) + 51968 === value.code) {
                    cbprefixed_result.push({
                        code: item.code,
                        mnemonic: item.mnemonic,
                        bits: value.bits,
                        ...item
                    })
                }
            })
        }
    })

    console.log("Exporting result to ../codegen/res/LR35902_opcodes.patched.json")
    Deno.writeTextFileSync(
        "../codegen/res/LR35902_opcodes.patched.json",
        JSON.stringify({
            unprefixed: unprefixed_result,
            cbprefixed: cbprefixed_result
        }, null, 2)
    )
}

console.log("Starting fix JSON!")
fixJson("./res/inst.patched.yml", "../codegen/res/LR35902_opcodes.json")

/*
{% macro inc16(i) %}
let v = {{ i.operands[0].name | getter(bits=i.bits) }}.wrapping_add(1);
{{ i.operands[0] | setter(bits=i.bits) }}v);
{% endmacro %}

{% macro ld(i) %}
let v = {{ i.operands[1] | getter(bits=i.bits) }};
{{ i.operands[0] | setter(bits=i.bits) }}v);
{% endmacro %}
 */