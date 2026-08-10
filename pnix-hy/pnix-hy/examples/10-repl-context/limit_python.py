"""plain의 한계 — 반복 CLI 호출은 '상태가 없다(stateless)'.

`python -c "..."` 를 여러 번 부르면 매번 새 프로세스라 이전 바인딩이 사라진다. 매 호출이
인터프리터를 새로 켜고 다시 import 하므로 느리고, 컨텍스트도 유지되지 않는다.
"""
import subprocess
import sys

# 첫 호출에서 a=20을 만들어도, 다음 호출은 그것을 '기억하지 못한다'.
subprocess.run([sys.executable, "-c", "a = 20"])                 # a=20 (이 프로세스에서만)
p = subprocess.run([sys.executable, "-c", "print(a + 22)"],       # 다른 프로세스 -> a가 없다
                   capture_output=True, text=True)
print("두 번째 호출 결과:", (p.stdout or p.stderr).strip().splitlines()[-1])
print("-> 이전 호출의 바인딩이 유지되지 않는다 (stateless), 매번 재기동으로 느리다")

print("\n결론: 반복 CLI 호출은 컨텍스트를 유지하지 못한다. 대화형 탐색엔 warm REPL이 필요하다.")
