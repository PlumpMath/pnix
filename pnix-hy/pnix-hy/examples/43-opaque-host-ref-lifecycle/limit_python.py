"""한계: 객체를 넘기면 전권이다 - 공개 메서드만 허용, 스코프 있는 빌림,
표면 동결 같은 SES식 안전장치가 plain Python에 없다.

객체를 그냥 넘기면 `_private` 속성/메서드까지 전부 접근 가능하다. "이후
표면이 안 바뀐다"는 약속도, "빌린 뒤 돌려준다"는 생명주기도 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))


class Greeter:
    def hello(self, name):
        return f"hello {name}"

    def _secret(self):
        return "should not be reachable"


g = Greeter()
print("plain Python: 객체를 그냥 넘기면 _secret()도 그대로 호출 가능:", g._secret())
print("한계: 공개 메서드만 허용/빌림 스코프/표면 동결 같은 안전장치가 없음.")
