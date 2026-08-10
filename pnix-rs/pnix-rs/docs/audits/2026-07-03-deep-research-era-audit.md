# 2026-07-03 — deep-research-era 게이트 adversarial 감사 (#3)

대상: deep-research 로드맵 이후 급히 추가한 13개 신규 게이트(jones/welltyped/
certify/cogen/attest/reflect-tower/verifying-cache/phase/assumption/ir-diff/
attenuate/explain + registry). 주장: 각 게이트가 falsifiable teeth를 갖는가,
overclaiming/vacuous가 없는가.

## 방법
외부 CLI adversarial 프로브 + 코드 경계 검사. 각 게이트의 "이빨"(negative
케이스)이 실제로 무는지 독립 확인.

## 확인된 무결 (teeth 진짜)
- **welltyped**: 플로어 typeck가 subtle ill-typed를 거부 확인 — i64를 bool
  리턴 위치에 두면 "returns Bool but body yields I64"(단순 파스가 아니라 실제
  타입 검사). rustc가 아니라 플로어(rs-meta typeck) 경로임 재확인.
- **jones**: 인터프리터 bloat(안 쓰는 sub/neg 분기)에도 residual `(input*3)+4`
  불변 — 해석 계층 제거 실증.
- **certify**: 틀린 residual((input*3)+5)을 12-입력 배터리가 거부(이빨 있음).
- **cogen**: 컴파일 결과 인터프리터-free(tag/prog 없음), 프로그램별 상이.
- **assumption**: 가정 변경 시 옛 residual 재사용=오답(20), 재특화=정답(24).
- **verifying-cache**: 오염된 store 엔트리(value_sha 변조) 감지.
- **attenuate**: 감쇠는 부분집합이라 재확대 불가(irreversible).
- **phase**: 정적 변수 누출 감지(residual 자유변수 == 동적 변수).

## 발견 및 조치 (2건 — teeth 강화)

**F1 (attest, teeth 격리 부족)**: attest-check가 predicate 불일치만 테스트
하고 subject-위조를 격리하지 않음(validate_typed는 subject==out_hash를
검사하나 게이트가 그 경로를 독립 증명 안 함). **조치**: subject-위조 테스트
추가 — 올바른 predicate + 변조된 subject 조합을 거부(attest-check 4→5).

**F2 (ir-diff, 정직 경계 미명시)**: ir-diff가 알파-불변이 아님(이름 변경을
diff로 봄)이라는 정직 경계가 문서에만 있고 게이트에 없음. changed_between
(알파-불변)과의 상보성이 미검증. **조치**: alpha-rename 경계 테스트 추가 —
ir-diff는 alpha-rename을 diff로 보고(ir-diff-check 4→5), incremental-check가
같은 alpha-rename을 불변으로 봄(상보성 양쪽 게이트로 확인).

## 판정
13개 신규 게이트 전부 실제 teeth 보유. 2건은 vacuous가 아니라 teeth의
"격리/경계 명시" 부족 — 둘 다 테스트 추가로 강화. overclaiming 없음(게이트
이름이 증명 내용과 일치, bounded/held 경계 정직 표시: cogen bounded,
0007c 부분, 0007d finite). 감사 후 재검증: check aggregate 29 reports all_ready.
