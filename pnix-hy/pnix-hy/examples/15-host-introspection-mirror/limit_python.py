"""plain의 한계 — 내성(introspection)은 '호스트 전용'이라 교차검증할 자기구현이 없다.

Python은 ast/dis/symtable/marshal로 코드 객체를 내성할 수 있다. 하지만 이는 CPython(호스트)
하나의 관점일 뿐이고, "같은 내성을 '자기 언어로 구현한 커널' 안에서도 수행해 결과가 같은지"
교차검증할 두 번째 구현이 딸려오지 않는다 -> 자기구현의 드리프트를 감지할 수 없다.
"""
code = compile("20 + 22", "<ex>", "exec")
print("co_names:", code.co_names, "| co_consts:", code.co_consts)
print("bytecode 있음:", len(code.co_code) > 0)
print("교차검증할 '자기 언어로 구현한 내성'?: 없음 (호스트 관점 하나뿐)")

print("\n결론: 호스트 내성만 있어, 자기구현 커널과의 내성 일치(parity)를 확인할 수 없다.")
