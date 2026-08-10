"""한계: naive 부분평가(partial evaluation)가 잔여 프로그램을 부풀리는 방식.

specializer(부분평가기)를 순진하게 짜면 두 가지로 잔여가 커지고 느려진다:
  (1) SHARING 손실 — dynamic 값을 여러 사용처에 그대로 인라인 복제(call-by-need 공유 파기).
      `let y = <무거운 dyn>; in y + y`  →  `(<dyn> + <dyn>)`   (계산 2배)
  (2) 문맥에 갇힌 정적계산 — dynamic `if`/구조가 바깥에 있으면 안쪽 정적 부분이 폴딩되지 못함.
      `(if b then {v=1} else {v=2}).v`  →  `(if b then {v=1} else {v=2}).v`  (attrset 그대로)

이 파일은 "naive라면 이렇게 나빴을 것"을 문자열로 보여주고(우리는 이렇게 만들지 않는다),
pnix_hy_way.py 가 같은 프로그램을 어떻게 작게 만드는지와 대비한다. 딥리서치 #2(2026-07-03) 근거:
call-by-need는 CBV의 sharing 문제를 상속(naive normalization은 최대 33× 느림, Brown&Palsberg POPL'18).
"""
# naive라면 잔여가 이렇게 나왔을 것 (설명용 — 실제 pnix specializer는 이러지 않는다):
NAIVE_SHARING = "(((x * x) + 7) + ((x * x) + 7))"        # y를 2번 인라인 → x*x 2번 계산
NAIVE_ETA = '(if b then { v = 1; } else { v = 2; }).v'   # attrset 통째 잔여 후 select

print("naive PE라면:")
print("  let y = x*x+7; in y+y   ->", NAIVE_SHARING, "   (x*x 2회 = 중복)")
print("  (if b then {v=1} else {v=2}).v ->", NAIVE_ETA, "   (attrset 폴딩 실패)")
print("→ pnix_hy_way.py 가 같은 입력을 어떻게 작게 만드는지 보라.")

assert NAIVE_SHARING.count("x * x") == 2   # 한계를 명시적으로 기록
assert "{" in NAIVE_ETA
