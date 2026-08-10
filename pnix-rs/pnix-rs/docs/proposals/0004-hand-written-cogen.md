# 0004 — 손으로 쓴 cogen (3차 Futamura 사영을 자기적용 없이)

상태: **bounded DONE (2026-07-03)** — 산술 객체언어의 hand-written cogen 구현
(cogen-check 3/3). full 3rd projection(feature-rich specialiser 자기적용)은
연구 지평으로 held. 근거: deep-research finding [3] (Leuschel et al.,
arxiv cs/0208009), docs/research/2026-07-03-metacircular-frontier.md.

## 동기
3차 사영(cogen = poly의 자기적용)은 monovariant 자기적용에서 폴리바리언스가
의미적으로 폭발(m7~m8 실측: fuel 10k→40k에 specs 170→687 무수렴, 1h40m+
미종결). deep-research finding [5]가 확증: fv-제한 등 subject BTA는
Jones-optimality를 못 올리는 강도 천장 — coarsening은 레버가 아니다.

## 접근 (Leuschel: "자기적용 가능 specializer 없이 자기적용의 이득을")
자기적용 대신 **generating-extension generator(cogen)를 손으로 작성**. cogen은
offline BTA의 얇은 확장이라 노력의 대부분은 BTA(m8 존재) 생산. 종결성은 2 의무로
분리(finding [4]): local(무한 unfold 없음) + global(program point당 유한 특화,
bounded polyvariance) via size-change/strong-termination + mgg generalization.

## 모듈/게이트
tower/bta. 게이트 = m5 자기생성 수용 기준(cogen(mix) == cogen, IR 해시 동등)을
손으로 쓴 cogen이 통과 + 종결 인증서(size-change).

## 구현 기록 (2026-07-03, bounded)
runtime/tower/cogen_int.px: 산술 객체언어(num/arg/add/mul)의 generating
extension을 px 함수로 손 작성(prog -> tower-인코딩 residual). cogen-check(3/3):
어떤 프로그램이든 컴파일된 residual이 10-입력 배터리에서 해석과 동등 +
인터프리터-free(dispatch가 생성 시점에 소진) + 프로그램별 상이. 자기적용 없이
cogen의 이득(Leuschel). 
잔여 held: **full 3rd projection** — feature-rich specialiser(mix/poly)의
자기적용은 여전히 연구 지평(m6c~m8 폴리바리언스 폭발 실측; BTA-driven
generalization + size-change 종결성 필요). pnix-hy build_cogen도 완전 자기생성
검증 미답.
