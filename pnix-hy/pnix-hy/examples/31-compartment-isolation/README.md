# 31. 구획 격리 — SES compartment (proposal 0021)

## 무엇을
자기만의 바인딩·모듈 네임스페이스를 갖는 격리된 평가 구획 `Compartment`: 서로 다른 구획은 같은 이름을
써도 섞이지 않고, 순수 intrinsics만 공유하며 back-leak이 없다.

## 왜
Python `eval`/`exec`는 전역을 공유한다 — 한 컨텍스트가 정의한 이름이 다른 컨텍스트로 새거나 덮어쓴다.
신뢰 경계가 다른 코드를 같은 프로세스에서 돌리려면 이름·모듈이 격리되어야 한다(SES).

## 예
```
a = Compartment(); a.bind("x", "10")
b = Compartment(); b.bind("x", "99")
a.eval("x + 1") == 11     # A의 x=10
b.eval("x + 1") == 100    # B의 x=99  — 안 섞임
```

## 무엇을 게이트하나
`binding_isolated` · `module_isolated` · `intrinsics_shared` · `no_backleak` (그리고 `state_persists`,
`module_loader`).

## 한 줄
> 구획마다 이름·모듈을 격리하면 — 같은 이름도 섞이지 않고, 한 구획이 다른 구획을 덮어쓰지 못한다(SES).

## 경계
- effect-class 권한(examples/23)과 결합해 최소권한 격리를 구성. 정본 평가기·4-lane 미러 무접촉.
