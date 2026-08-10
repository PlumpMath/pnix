# 11 · self-hosting convergence — 4개 substrate 수렴 (meta-circular의 핵심)

> Hy 1.3.0 proof Python + 저장소 트리 필요 (`nix develop` / `PNIX_HY_PYTHON`).

## 쉽게 말하면 (비유)
같은 문제를 **서로 다른 4명(구현)**이 풀어 답이 모두 같으면 믿을 수 있다. 특히 "그 언어로 자기
자신을 구현한" 런타임까지 같은 답을 내면 = 자기호스팅(meta-circular)의 증거.
```py
ph.pnix_meta_circular_projection("2 * 3 + 4")["lanes"]
# {host_interp:10, host_compiler:10, stage7_runtime:10, stage7_compiler:10}  -> converged
```
직관: **네 경로 수렴** = 구현 교차검증 + 자기호스팅 증명.

## 무엇을
같은 pnix 식을 **네 경로**에서 평가하고 **같은 값으로 수렴**함을 증명한다:
Python 해석기 · Python 컴파일러 · **Hy로 작성된 pnix 평가기(stage7)** · **Hy로 작성된 pnix 컴파일러(stage7)**.

## plain의 한계 (`limit_python.py`)
Python은 값 하나를 낼 뿐, "같은 언어로 자기 자신을 구현한 런타임과도 결과가 같다"는 자기호스팅
수렴을 표준으로 보여주지 못한다(교차검증할 언어-내 구현이 딸려오지 않는다).

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `pnix_meta_circular_projection(src)` → `lanes`(4 substrate의 값) + `converged`(수렴 여부) +
  `substrates`(각 경로 설명). `2 * 3 + 4` → 네 경로 모두 `10`, `converged: True`.
- 이것이 hy-meta(호스트 자기컴파일 증명 레인) 위에 pnix 런타임을 올린 결과의 자기호스팅 증거.
  (`--gate`의 four_lane_mirror 545×4 수렴이 전체 코퍼스에 대한 강한 버전.)

## 어디에 쓰나
- 구현 교차검증: 새 최적화/컴파일 경로가 기존 해석과 **값이 일치**하는지 자동 증명
- 언어/런타임 연구: meta-circular 자기호스팅을 실제로 관찰·회귀검출
- 이식/재작성 시 "다른 백엔드가 같은 의미"를 보장

## 실행
```sh
nix develop
python pnix-hy/examples/11-self-hosting-convergence/limit_python.py
python pnix-hy/examples/11-self-hosting-convergence/pnix_hy_way.py
```
