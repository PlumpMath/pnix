"""plain의 한계 — lru_cache는 '표현(문자열)'을 키로 쓴다, '정본 내용'이 아니다.

functools.lru_cache는 인자의 동치(==)/해시로 캐시한다. 같은 '의미'라도 표현이 다르면
(예: "1 + 2" vs "1 +  2") 다른 키가 되어 캐시가 안 맞는다. 정본(canonical) 내용주소 캐시가 아니다.
"""
from functools import lru_cache

calls = []

@lru_cache(maxsize=None)
def evaluate(src: str) -> int:
    calls.append(src)          # 실제 계산이 일어난 횟수를 센다
    return eval(src)

evaluate("1 + 2")
evaluate("1 +  2")            # 같은 의미, 공백만 다름 -> lru_cache는 '다른 키'로 본다
print("계산 실제 실행 횟수:", len(calls), "(정본 캐시면 1이어야 하는데 2다)")
assert len(calls) == 2        # 캐시 미스: 표현이 다르면 다시 계산한다

print("\n결론: 표현 기반 캐시라, 같은 의미의 다른 표현을 재사용하지 못한다(정본 캐시 아님).")
