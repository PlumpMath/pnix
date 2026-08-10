# 08 · Hy reader macro embeds pnix — read-time 임베드

> Hy 1.3.0 proof Python 필요 (`nix develop` 또는 `PNIX_HY_PYTHON`).

## 쉽게 말하면 (비유)
글 속에 **인용부호로 외국어 문장을 넣고, 읽는 순간 즉석 번역**되는 것. Hy 코드 안에
`#px "..."`로 pnix를 넣으면 읽는 시점에 폼으로 승격되고 pnix-hy가 의미를 준다.
```py
ph.hy_reader_embed_pnix('(+ 10 #px "1 + 2")')["embeddings"][0]   # pnix_value=3, hy_form="(+ 1 2)"
```
직관: **언어 안에 언어를 삽입**(read-time) — polyglot.

## 무엇을
Hy의 `#px "..."` **reader macro**로 pnix를 **읽는 시점(read-time)**에 1급 폼으로 임베드하고,
pnix-hy가 그 pnix에 의미(평가/투영)를 준다.

## plain의 한계 (`limit_python.py`)
Python에서 다른 언어 조각은 결국 **문자열**이라 읽는 시점에 폼으로 승격되지 않고, 사용자 정의
reader macro도 없다 → read-time 임베드/투영이 불가능.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `hy_reader_embed_pnix(hy_src)` — Hy 리더에 `#px` 등록 → `#px "1 + 2"` → `(pnix-eval "1 + 2")`;
  pnix-hy가 임베드된 pnix를 평가(→3)하고 Hy 폼(→`(+ 1 2)`)으로 투영. 멀티 임베드도 지원.
- Hy의 리더 기계를 사용(= pnix에 reader macro를 만들지 않음).

## 어디에 쓰나
- 호스트 코드(Hy) 안에 순수 설정/DSL(pnix)을 **인라인**으로 박고 read-time에 검증/투영
- 언어 임베딩(polyglot) 연구: 한 리더에서 다른 언어를 1급 폼으로 승격

## 실행
```sh
nix develop
python pnix-hy/examples/08-hy-reader-embed-pnix/pnix_hy_way.py
```
