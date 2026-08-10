"""pnix-hy의 방식 — 순수성 정적판정 + 자원한계 + 게이트 = 신뢰 가능한 샌드박스.

pnix는 순수/지연 언어라 부작용이 '설계상' 없고, safe_eval은 스텝/시간/출력 한계를 강제하며
결코 걸리거나 예외로 새어나오지 않는다(항상 구조화된 판정을 반환). 실행 '전에' 순수성을
정적으로 판정할 수도 있다.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


# 1) 순수 계산은 값으로 안전하게 나온다 (부작용 불가능).
ok = ph.safe_eval("1 + 2 * 3")
print("순수 계산:", ok["ok"], ok["value"], f"(steps={ok['steps']})")
assert ok["ok"] and ok["value"] == 7

# 2) 부작용(impure)은 pure_only=True로 '거부'된다 — 실행 전 정적 판정.
impure = ph.safe_eval('builtins.getEnv "HOME"', pure_only=True)
print("impure 거부:", impure["ok"], "| limit:", impure["limit_exceeded"])
assert impure["ok"] is False and impure["limit_exceeded"] == "impure"

# 3) 무한/과도한 계산은 자원 한계로 '멈춘다' (걸리지 않는다).
bounded = ph.safe_eval("let f = x: f x; in f 1", max_steps=50_000)  # 무한 재귀
print("자원 한계:", bounded["ok"], "| limit:", bounded["limit_exceeded"])
assert bounded["ok"] is False  # timeout/max_steps/recursion 중 하나로 안전하게 종료

# 4) 실행 '전에' 순수성을 정적으로 알 수 있다.
purity = ph.static_purity_check('builtins.readFile "/etc/passwd"')
print("정적 순수성:", purity["pure"], "| 부작용 사용:", purity["impure_uses"])
assert purity["pure"] is False

# 5) capability 게이트: 필요한 effect가 허가돼야만 통과.
gate = ph.gate_check('builtins.readFile "/etc/passwd"')
print("게이트(무허가):", gate["allowed"], "| 필요 effect:", gate["required_effects"])
granted = ph.gate_check('builtins.readFile "/etc/passwd"', granted=("file-read",))
print("게이트(file-read 허가):", granted["allowed"])
assert gate["allowed"] is False and granted["allowed"] is True

print("\n결론: 신뢰할 수 없는 pnix 입력을 순수·자원제한·권한제어 하에 안전하게 평가한다.")
