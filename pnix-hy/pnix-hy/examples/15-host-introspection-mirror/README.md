# 15 · host introspection mirror — 자기구현과의 내성 일치

> Hy 1.3.0 proof Python 필요 (`nix develop` / `PNIX_HY_PYTHON`). 입력은 **Python 소스**.

## 쉽게 말하면 (비유)
**원본과 사본 공증 대조**. 호스트(CPython)가 본 코드 내부(코드객체/바이트코드/AST)와, 자기구현
커널(stage7)이 본 것이 **일치하는지** 대조한다 — 자기구현이 어긋나면 여기서 잡힌다.
```py
hm.introspection_parity("20 + 22")["ready"]   # True (host 관점 == stage7 관점)
```
직관: **host 관점 vs 자기구현 관점 일치** → 자기구현 드리프트 감지.

## 무엇을
Python 코드의 내성(code object/bytecode/AST/symtable/marshal)을 host-direct로 물화하고, **같은
내성을 stage7(자기구현) 커널 안에서도 수행해 일치(parity)**를 확인한다.

## plain의 한계 (`limit_python.py`)
Python 내성은 CPython(호스트) 관점 하나뿐이다. "같은 내성을 자기 언어로 구현한 커널에서도
수행해 일치하는가"를 교차검증할 두 번째 구현이 없어, 자기구현의 드리프트를 감지할 수 없다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `hy_mirror.full_introspection(src)` — host-direct 내성 단면(ast/bytecode/code_object/symtable/marshal).
- `hy_mirror.introspection_parity(src)` — host-direct vs stage7 커널 내성의 **일치 확인**(`ready`).

## 어디에 쓰나
- 자기호스팅 컴파일러/커널이 호스트와 **같은 내성 결과**를 내는지 회귀검출
- 컴파일 파이프라인의 산출물(코드객체/바이트코드)을 두 관점에서 교차검증
- CPython 내부(ast/dis/marshal)를 다루는 도구를 자기구현과 대조

## 실행
```sh
nix develop
python pnix-hy/examples/15-host-introspection-mirror/pnix_hy_way.py
```
