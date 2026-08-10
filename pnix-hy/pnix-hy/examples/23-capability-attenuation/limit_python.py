"""한계: Python 객체를 넘기면 '전권'을 넘긴 것 — 감쇠도 회수도 불가.

plain Python에서 어떤 능력(파일 핸들·소켓·평가기)을 함수에 넘기면, 받는 쪽은 그 객체의 **모든**
메서드를 쓸 수 있고, 준 쪽은 나중에 그 권한을 **줄이거나(attenuate) 회수(revoke)** 할 수 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

class Resource:
    def read(self):   return "read-ok"
    def write(self, x): return "written"
    def delete(self):  return "DELETED"    # 위험한 권한

def hand_to_plugin(res):
    # 플러그인은 read만 필요하다고 했지만, 넘긴 객체엔 delete까지 들어있다
    return res.delete()                    # 막을 방법이 없음

r = Resource()
print("플러그인이 할 수 있는 것:", [m for m in dir(r) if not m.startswith("_")])
print("의도치 않은 삭제:", hand_to_plugin(r))   # 전권 → delete 실행됨
# 회수? 불가 — r을 이미 넘겼고, 참조를 되돌릴 수 없다.
print("한계: 최소권한(read만)도, 사후 회수도 불가 — 넘기면 전권.")
