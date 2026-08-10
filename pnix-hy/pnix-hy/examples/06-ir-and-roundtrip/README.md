# 06 · IR & roundtrip — 정본 IR과 값-동치

## 쉽게 말하면 (비유)
손그림 설계도는 그리는 사람마다 다르다. **표준 도면(정본 IR)**으로 바꾸면 같은 건물은 같은
도면·같은 지문이 되고, 그 도면대로 지으면 같은 집(값-동치)이 나온다.
```py
ph.eval_ir(ph.lower_to_ir("let a = 1; in a + 2")) == ph.safe_eval("let a = 1; in a + 2")["value"]  # 3
```
직관: IR이 **정본**(해시 안정 · 직접 평가 · 값-동치) → 캐시/재현/이식의 기준점.

## 무엇을
소스를 **위치-무관 정본 IR**로 낮추고(안정 해시), IR을 직접 평가해 소스 평가와 **값이 같음**을 본다.

## plain의 한계 (`limit_python.py`)
Python `ast`는 정규화된 정본 표현이 아니고(위치/속성 포함), 안정적 내용해시가 없으며,
"정규화 표현을 평가한 값 == 소스 평가 값"을 언어가 보장/관찰해 주지 않는다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `ir_of(src)` — 위치-무관 정본 IR + 안정 `ir_sha256`. 포맷이 달라도 같은 정본으로 수렴.
- `lower_to_ir(src)` / `eval_ir(ir)` — IR을 **직접 평가**, 값은 소스 평가와 동일(값-동치).
- (원리) IR이 정본이고, host로 내보낸 코드는 실행 아티팩트/캐시일 뿐.

## 어디에 쓰나
- 내용주소 **캐시/재현**의 기준점(정본 IR 해시)
- 다른 백엔드/언어로의 **이식**(같은 IR → 여러 실행 아티팩트)
- 변환(최적화/특화)이 의미를 보존하는지 값-동치로 검증

## 실행
```sh
python pnix-hy/examples/06-ir-and-roundtrip/limit_python.py
python pnix-hy/examples/06-ir-and-roundtrip/pnix_hy_way.py
```
