# Proposals — 새 기능의 유일한 진입점

`/SCOPE_LOCK.md` §7 기준: pnix-hy / hy-meta는 현재 meta-circular-projection scope 안에서 닫혀
있다. 새 기능은 `todo.md`의 `[ ]`로 바로 시작하지 **않는다** — 여기 `NNNN-<slug>.md`로 시작하고,
proposal이 수락된 뒤에만 `todo.md`로 들어간다.

proposal은 반드시 다음을 밝힌다:
1. **Scope** — 어느 레인인가(hy-meta 호스트 / pnix-hy 런타임 / interop), 그리고 현재
   meta-circular-projection scope 안인가 아니면 새 scope인가.
2. **placeholder / out-of-scope 점검** — 의도적 placeholder(`/SCOPE_LOCK.md` §3)나 out-of-scope
   항목(§5)을 건드리는가? 그렇다면 이를 승인하는 명시적 human decision.
3. **재사용** — 어떤 기존 심볼 위에 얹는가(먼저 검색; 다시 만들지 말 것).
4. **경계 영향** — 공유 ABI envelope(§14 witness 필드 스키마 / §18-19 opaque-ref shape)를
   바꾸는가? 그렇다면 두 레인 + drift-guard를 함께 갱신해야 한다.

> 의도적 placeholder를 미구현으로 재해석해서 구현하지 말 것.
> (No new implementation may reinterpret intentional placeholders as missing work.)
