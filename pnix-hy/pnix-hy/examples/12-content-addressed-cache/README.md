# 12 · content-addressed cache — 정본 내용주소 캐시

## 쉽게 말하면 (비유)
도서관 **정본 색인**. 띄어쓰기가 달라도 **같은 뜻이면 같은 서랍**에서 꺼낸다(다시 계산 안 함).
`lru_cache`는 글자가 다르면 다른 서랍이라 또 계산한다.
```py
ph.cached_eval("1 + 2");  ph.cached_eval("1 +  2")["cached"]   # True (같은 정본 = 캐시 적중)
```
직관: **내용주소(정본) 캐시** → 같은 의미의 다른 표현도 재사용.

## 무엇을
소스를 **정본(canonical) 형태**로 정규화한 키로 메모이즈 → 같은 의미의 다른 표현도 재사용.

## plain의 한계 (`limit_python.py`)
`functools.lru_cache`는 인자 동치/해시(=표현)로 캐시한다. `"1 + 2"` vs `"1 +  2"`는 다른 키가
되어 **재계산**된다. 정본 내용주소 캐시가 아니다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `cached_eval(src)` — 정본 키로 메모이즈. 공백/포맷이 달라도 같은 정본이면 `cached=True`로 적중,
  `cache_key`가 동일.

## 어디에 쓰나
- 반복 평가되는 DSL/설정의 **증분 재계산 절감**(표현 차이에 견고)
- 내용주소 빌드/파이프라인: "같은 의미 = 같은 캐시"
- (원리) 정본 IR/폼이 진실의 원천, 생성 아티팩트는 캐시 (섹션 06 참고)

## 실행
```sh
python pnix-hy/examples/12-content-addressed-cache/limit_python.py
python pnix-hy/examples/12-content-addressed-cache/pnix_hy_way.py
```
