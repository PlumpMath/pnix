# 18 · action checkpoint — 한 단계 행동을 accepted / held / rejected로 고정

## 쉽게 말하면 (비유)
**출입 심사대**. 그냥 실행하면 "결과가 나왔다"만 알지만, action checkpoint는 "이 행동을 통과시켜도
되는가, 어떤 권한이 필요한가, 증거 해시는 무엇인가"를 한 장의 판정표로 남긴다.

```py
v = ph.check_action("let a = 1; in a + 2")
v["status"], v["effects"], v["witness_id"]   # accepted, ["pure"], ...
```

## 무엇을
한 pnix action step에 대해 gate + safe_eval + mirror + explain + witness를 재사용해
`accepted | held | rejected` verdict를 만든다. rollback은 파일 백업이 아니라 `before:<hash>` 참조다.

## plain의 한계 (`limit_python.py`)
Python `eval`은 값은 줄 수 있지만, 성공/효과/의미/증거/verdict를 하나의 안정된 레코드로 묶지 않는다.
권한 보류(`held`)와 문법/평가 실패(`rejected`)도 직접 규약을 만들어야 한다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `check_action(src)` → 순수 action은 `accepted`
- impure action은 기본 `held`; `granted=("file-read",)`처럼 권한을 줘야 통과
- parse/eval 실패는 `rejected`
- `verify_action(..., before_snapshot=...)` → hash-only rollback ref + witness

## 어디에 쓰나
- AI coding agent가 제안한 한 단계를 실행 전 semantic checkpoint로 고정
- DSL/설정 변경의 승인 플로우: 통과/보류/거부와 이유를 같은 JSON으로 전달
- 감사 로그: source/IR/value/witness/rollback hash ref를 함께 저장

## 실행
```sh
python pnix-hy/examples/18-action-checkpoint/limit_python.py
python pnix-hy/examples/18-action-checkpoint/pnix_hy_way.py
```
