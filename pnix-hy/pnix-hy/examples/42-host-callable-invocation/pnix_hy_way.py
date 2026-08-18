"""pnix-hy 방식: 호스트 콜러블 호출 (call_host/try_call_host/host_callable_*).

pnix 쪽에서 host 함수/메서드를 부르는 경로들이 전부 effect-class + witness
증거를 남긴다.
"""
import os, sys, math
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph


class Greeter:
    def hello(self, name):
        return f"hello {name}"


def greet(name):
    return f"hi {name}"


def boom():
    raise ValueError("boom")


value, record = ph.call_host(greet, ["world"])
print(f"call_host: value={value!r} effect_class={record.effect_class!r}")
assert value == "hi world" and record.effect_class == "host-call"

value2, record2 = ph.call_host_method(Greeter(), "hello", ["world"])
print(f"call_host_method: value={value2!r}")
assert value2 == "hello world"

ok = ph.try_call_host(greet, ["world"])
err = ph.try_call_host(boom, [])
print(f"try_call_host ok: {ok['success']} value={ok['value']!r}")
print(f"try_call_host err: success={err['success']} error_kind={err['error']['kind']}")
assert ok["success"] and not err["success"]

arity = ph.host_callable_arity(greet)
print(f"host_callable_arity(greet): {arity}")
assert arity == {"name": False}

mod = ph.host_module_to_pnix(math)
print(f"host_module_to_pnix(math): {len(mod)} public callables exposed, e.g. sqrt={'sqrt' in mod}")
assert "sqrt" in mod

eval_value, eval_record = ph.to_host_eval("1 + 2")
print(f"to_host_eval('1 + 2'): value={eval_value} loss_status={eval_record.loss_status!r}")
assert eval_value == 3 and eval_record.loss_status == "lossless"

print("→ call/call-method/try-call/arity/module-expose/eval-and-cross 전부 증거를 남긴다.")
