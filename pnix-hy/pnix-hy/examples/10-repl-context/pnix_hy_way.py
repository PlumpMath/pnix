"""pnix-hy의 방식 — context-retaining pnix REPL (warm, 한 프로세스).

pnix REPL은 한 프로세스에서 pnix env를 '누적'한다: `a = 20` 후 `b = a + 22` 는 a를 보고,
`b` 는 42. 인터프리터가 hot 상태로 유지되므로 반복 CLI보다 빠르다. 여기선 헤드리스로(문자열
입력을 흘려서) 컨텍스트 유지를 검증한다. 순수 — Hy 불필요.

대화형으로는:  nix run .#repl-pnix-hy-pnix   또는   pnix-hy-project --repl pnix
"""
import io
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy.repl import run_pnix_repl  # noqa: E402


# 한 세션(한 프로세스)에서 줄들을 흘려보낸다 — 바인딩이 누적된다(context 유지).
script = "a = 20\nb = a + 22\nb\n1 +\nb + 0\n:env\n:quit\n"  # '1 +' 는 일부러 틀린 줄
out = io.StringIO()
run_pnix_repl(io.StringIO(script), out)
transcript = out.getvalue()
print(transcript)

# b = a + 22 = 42 로, 이전 바인딩이 유지됐다. 틀린 줄 이후에도 세션이 살아남았다.
assert transcript.count("42") >= 2      # `b` 와 `b + 0` 둘 다 42
assert "error:" in transcript           # 틀린 줄은 진단되고
assert "a, b" in transcript             # :env 에 누적된 바인딩이 보인다

print("결론: warm REPL이 컨텍스트를 유지하고 오류에도 세션이 살아남는다 (대화형 탐색에 적합).")
