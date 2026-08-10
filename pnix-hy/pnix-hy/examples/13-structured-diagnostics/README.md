# 13 · structured diagnostics — 구조화 위치 진단

## 쉽게 말하면 (비유)
편집기의 **빨간 밑줄 + "몇 줄 몇 칸"**. 긴 스택트레이스(호스트 관점)가 아니라, 문제 위치를
캐럿(`^`)으로 가리키는 **구조화 진단 데이터**를 준다.
```py
ph.diagnose("let a = ")   # {ok:False, line:1, column:.., excerpt:"let a = \n   ^"}
```
직관: 오류를 **데이터로**(위치·단계·캐럿) → DSL UX/툴링에 바로 사용.

## 무엇을
잘못된 소스에 대해 **위치(line/column/offset) · 단계(phase) · 메시지 · 캐럿 excerpt**를 담은
구조화 진단을 **데이터로** 반환(예외로 새지 않음).

## plain의 한계 (`limit_python.py`)
Python `SyntaxError`/traceback은 사람이 읽는 텍스트다. "몇 줄 몇 칸, 어느 단계, 무엇이 문제"를
데이터로 소비하려면 트레이스백을 직접 파싱해야 하고, DSL 사용자용 구조화 진단 표준이 없다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `diagnose(src)` → `{ok, line, column, offset, phase, message, excerpt}` (excerpt는 `^` 캐럿 포함).
  정상 소스는 `ok=True`.

## 어디에 쓰나
- DSL/설정 편집기·폼 검증의 **사용자 친화 오류 표시**(위치·캐럿)
- 오류를 데이터로 다뤄 로깅/집계/자동수정 힌트에 활용
- REPL·서버에서 예외로 죽지 않고 구조화 오류를 반환

## 실행
```sh
python pnix-hy/examples/13-structured-diagnostics/limit_python.py
python pnix-hy/examples/13-structured-diagnostics/pnix_hy_way.py
```
