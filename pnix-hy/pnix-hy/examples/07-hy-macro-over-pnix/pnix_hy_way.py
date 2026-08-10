"""pnix-hy의 방식 — Hy의 매크로/quasiquote를 'pnix 코드 위에' 적용한다.

pnix는 비동형 그대로 두고(=pnix 매크로를 만들지 않는다), Hy 쪽의 실제 매크로/quasiquote를
pnix에서 투영된 폼에 적용한다. 즉 두 meta-circular를 잇는 언어기능 interop.

* 이 예제는 Hy 1.3.0 proof Python이 필요하다: `nix develop` 안에서, 또는
  PNIX_HY_PYTHON=<hy 있는 python>을 설정하고 실행하라.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


# C1) pnix 식을 Hy 폼으로 투영한 뒤, Hy 매크로 `when`을 그 위에 적용 -> if로 확장.
c1 = ph.hy_macro_over_pnix("1 + 2")  # 기본 래핑: (when True {form})
print("pnix '1 + 2' -> Hy 폼:", c1["projected_hy_form"])       # (+ 1 2)
print("Hy 매크로 적용:", c1["composed_hy"], "| 매크로임:", c1["is_macro"])
print("확장을 pnix로 재합성:", c1["pnix_of_expansion"])        # (if true then (1 + 2) else null)
assert c1["projected_hy_form"] == "(+ 1 2)" and c1["is_macro"] is True

# C2) pnix '값'을 Hy quasiquote 구멍(~a, ~b)에 주입해 Hy 폼을 생성.
c2 = ph.hy_quasiquote_over_pnix("`(sum ~a ~b)", {"a": "1 + 2", "b": "10"})
items = [i.get("name") or i.get("repr") for i in c2["result"]["items"]]
print("quasiquote에 pnix 값 주입:", c2["template"], "->", items)  # ['sum','3','10']
assert items == ["sum", "3", "10"]

# C3) 'quasiquote = 수동 staging, specialize_pnix = 자동 staging' 대응을 실행가능하게 확인.
c3 = ph.quasiquote_specialize_correspondence("`(+ ~x 10)", "x + 10", ("x",))
print("staging 대응:", "holes=", c3["quasiquote_hole_vars"], "dynamic=", c3["pnix_dynamic_vars"],
      "| 일치:", c3["corresponds"])
assert c3["corresponds"] is True

print("\n결론: pnix를 비동형으로 유지하면서 Hy 매크로/quasiquote를 pnix 코드/값에 연결한다.")
