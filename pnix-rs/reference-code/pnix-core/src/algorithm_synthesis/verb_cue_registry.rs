//! Verb / metaphor cue registry.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/verb-cue-registry.px`. Maps
//! raw NL patterns (Korean + English) to fired cue strings consumed
//! by `intent_recognition`. Pattern matching is *substring +
//! case-insensitive on input* — callers should lowercase the
//! utterance before invoking `find_fired_cues`.
//!
//! Separation from `intent_recognition`:
//!
//!   `verb_cue_registry`    : pattern → cue   (what fires)
//!   `intent_recognition`   : cue → intent    (what each cue means)
//!
//! Adding a new metaphor / verb cue = one new row in `VERB_CUE_REGISTRY`
//! and one new entry in `intent-recognition.px::intentSignals`. No
//! branch in code.

/// One registry entry — pure data. Mirror of `.px` `verbCueRegistry`
/// rows. The sync test asserts set-equality on `(cue, patterns)` tuples.
#[derive(Debug, Clone, Copy)]
pub struct VerbCueEntry {
  pub cue: &'static str,
  pub patterns: &'static [&'static str],
}

/// The single source of truth (Rust side) for which NL patterns fire
/// which cue. Bilingual KO+EN. Each row's `patterns` slice is walked
/// generically by `find_fired_cues` — no per-cue branch.
pub const VERB_CUE_REGISTRY: &[VerbCueEntry] = &[
  // verb:* — explicit action verbs
  VerbCueEntry {
    cue: "verb:rename",
    patterns: &[
      "rename",
      "리네임",
      "이름 바꾸",
      "이름 바꿔",
      "이름을 바꾸",
      "이름을 바꿔",
      "이름 변경",
      "로 바꾸",
      "로 바꿔",
    ],
  },
  VerbCueEntry {
    cue: "verb:simplify",
    patterns: &["simplify", "단순화", "간소화"],
  },
  VerbCueEntry {
    cue: "verb:reorganize",
    patterns: &["reorganize", "재정렬", "재구성", "정리해", "정리하"],
  },
  VerbCueEntry {
    cue: "verb:extract",
    patterns: &["extract", "추출", "분리해", "빼"],
  },
  VerbCueEntry {
    cue: "verb:inline",
    patterns: &["inline", "인라인", "합쳐", "합치"],
  },
  VerbCueEntry {
    cue: "verb:move",
    patterns: &["move ", "이동", "옮겨", "옮기"],
  },
  VerbCueEntry {
    cue: "verb:fix",
    patterns: &["fix", "고치", "고쳐", "수정"],
  },
  VerbCueEntry {
    cue: "verb:repair",
    patterns: &["repair", "복구"],
  },
  VerbCueEntry {
    cue: "verb:debug",
    patterns: &["debug", "디버그", "디버깅"],
  },
  VerbCueEntry {
    cue: "verb:patch",
    patterns: &["patch", "패치"],
  },
  VerbCueEntry {
    cue: "verb:add",
    patterns: &["add ", " add", "추가", "더하"],
  },
  VerbCueEntry {
    cue: "verb:create",
    patterns: &[
      "create",
      "생성",
      "신규 생성",
      "새로 만들",
      "새 노드",
      "새 엣지",
      "새 extern",
      "new node",
      "new edge",
      "new extern",
    ],
  },
  VerbCueEntry {
    cue: "verb:connect",
    patterns: &[
      "connect",
      "wire",
      "wiring",
      "연결",
      "잇어",
      "잇기",
      "이어 ",
      "와이어",
      "와이어링",
    ],
  },
  VerbCueEntry {
    cue: "verb:implement",
    patterns: &["implement", "구현"],
  },
  VerbCueEntry {
    cue: "verb:introduce",
    patterns: &["introduce", "도입"],
  },
  VerbCueEntry {
    cue: "verb:support",
    patterns: &["support", "지원"],
  },
  VerbCueEntry {
    cue: "verb:remove",
    patterns: &["remove", "제거", "없애"],
  },
  VerbCueEntry {
    cue: "verb:delete",
    patterns: &["delete", "삭제", "지워", "지우"],
  },
  VerbCueEntry {
    cue: "verb:test",
    patterns: &["test", "테스트"],
  },
  VerbCueEntry {
    cue: "verb:cover",
    patterns: &["cover", "커버"],
  },
  VerbCueEntry {
    cue: "verb:optimize",
    patterns: &["optimize", "최적화"],
  },
  VerbCueEntry {
    cue: "verb:accelerate",
    patterns: &["accelerate", "가속"],
  },
  VerbCueEntry {
    cue: "verb:explain",
    patterns: &["explain", "설명"],
  },
  VerbCueEntry {
    cue: "verb:describe",
    patterns: &["describe", "기술해", "기술하"],
  },
  VerbCueEntry {
    cue: "verb:document",
    patterns: &["document", "문서화"],
  },
  VerbCueEntry {
    cue: "verb:comment",
    patterns: &["comment", "주석"],
  },
  // metaphor:* — abstract metaphors
  VerbCueEntry {
    cue: "metaphor:cleanliness",
    patterns: &["깔끔", "clean", "clean up", "tidy"],
  },
  VerbCueEntry {
    cue: "metaphor:elegance",
    patterns: &["elegant", "우아", "아름다"],
  },
  VerbCueEntry {
    cue: "metaphor:tidiness",
    patterns: &["정돈", "단정", "neat"],
  },
  VerbCueEntry {
    cue: "metaphor:wrongness",
    patterns: &["이상", "이상해", "wrong", "잘못", "weird"],
  },
  VerbCueEntry {
    cue: "metaphor:brokenness",
    patterns: &["broken", "안 돼", "동작 안", "작동 안", "안 됨"],
  },
  VerbCueEntry {
    cue: "metaphor:diagnosis",
    patterns: &["왜 안", "왜 이래", "diagnose"],
  },
  VerbCueEntry {
    cue: "metaphor:new-capability",
    patterns: &["새 기능", "feature", "new feature", "새 capability"],
  },
  VerbCueEntry {
    cue: "metaphor:deadweight",
    patterns: &[
      "사용 안",
      "안 쓰",
      "안 쓰이",
      "쓸데없",
      "unused",
      "dead code",
    ],
  },
  VerbCueEntry {
    cue: "metaphor:coverage",
    patterns: &["커버리지", "coverage"],
  },
  VerbCueEntry {
    cue: "metaphor:speed",
    patterns: &["빨라", "빠르", "느려", "느리", "fast", "slow"],
  },
  VerbCueEntry {
    cue: "metaphor:efficiency",
    patterns: &["효율", "efficient"],
  },
  VerbCueEntry {
    cue: "metaphor:understanding",
    patterns: &["이해", "understand"],
  },
  // ─── Python-specific metaphors — substrate-sharing N=5 ─────────
  //
  // Same registry-driven discipline as the general cues above, with
  // Python-idiomatic markers. Each cue maps to an intent in
  // `INTENT_SIGNALS` so downstream synthesis chains can route to
  // existing transforms (or surface a Held when no transform yet
  // exists for the specific Python operation).
  //
  // Bilingual KO+EN coverage, same as the generic cues. Korean
  // markers prioritize the way Korean developers actually phrase
  // Python work ("타입 힌트 붙여줘", "f-string 으로 바꿔", etc.).
  VerbCueEntry {
    cue: "metaphor:python-typing",
    patterns: &[
      "타입 힌트",
      "타입힌트",
      "type hint",
      "type hints",
      "타입 어노테이션",
      "annotation",
      "annotations",
      "타입 지정",
    ],
  },
  VerbCueEntry {
    cue: "metaphor:python-fstring",
    patterns: &[
      "f-string",
      "f string",
      "fstring",
      "에프 스트링",
      "에프스트링",
      "f\"",
      "f'",
    ],
  },
  VerbCueEntry {
    cue: "metaphor:python-decorator",
    patterns: &[
      "decorator",
      "decorators",
      "데코레이터",
      "@property",
      "@dataclass",
      "@staticmethod",
      "@classmethod",
    ],
  },
  VerbCueEntry {
    cue: "metaphor:python-comprehension",
    patterns: &[
      "list comprehension",
      "리스트 컴프리헨션",
      "컴프리헨션",
      "list comp",
      "dict comprehension",
      "set comprehension",
      "내포 표현",
    ],
  },
  VerbCueEntry {
    cue: "metaphor:python-async",
    patterns: &[
      "async ",
      " async",
      "비동기",
      "await ",
      " await",
      "asyncio",
      "코루틴",
      "coroutine",
    ],
  },
  VerbCueEntry {
    cue: "metaphor:python-pytest",
    patterns: &["pytest", "fixture", "fixtures", "픽스처", "테스트 픽스처"],
  },
  VerbCueEntry {
    cue: "metaphor:python-dataclass",
    patterns: &["dataclass", "dataclasses", "데이터클래스", "@dataclass"],
  },
  VerbCueEntry {
    cue: "metaphor:python-pythonic",
    patterns: &[
      "pythonic",
      "파이써닉",
      "python 스타일",
      "python style",
      "더 python 답게",
      "관용적",
    ],
  },
];

/// Walk the registry; fire each cue whose any pattern is a substring
/// of `text`. Generic over the registry — no per-cue branch. Result
/// is deduplicated (same cue won't fire twice even if multiple of its
/// patterns match).
///
/// Case handling: `text` is matched as-is. Callers should lowercase
/// English-mixed input upstream (the bridge does this). Korean is
/// case-less so no normalization needed.
pub fn find_fired_cues(text: &str) -> Vec<String> {
  let mut out: Vec<String> = Vec::new();
  for entry in VERB_CUE_REGISTRY {
    if entry.patterns.iter().any(|p| text.contains(p)) {
      // Dedup — registry rows are unique by `cue` already so this is
      // belt-and-suspenders.
      if !out.iter().any(|s| s == entry.cue) {
        out.push(entry.cue.to_string());
      }
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn registry_has_no_duplicate_cues() {
    let mut seen: Vec<&str> = Vec::new();
    for e in VERB_CUE_REGISTRY {
      assert!(
        !seen.contains(&e.cue),
        "duplicate cue in VERB_CUE_REGISTRY: {}",
        e.cue
      );
      seen.push(e.cue);
      assert!(!e.patterns.is_empty(), "cue {} has no patterns", e.cue);
    }
  }

  #[test]
  fn english_rename_fires_verb_rename() {
    let fired = find_fired_cues("please rename the function frob to bar");
    assert!(fired.contains(&"verb:rename".to_string()), "got {fired:?}");
  }

  #[test]
  fn korean_rename_fires_verb_rename() {
    let fired = find_fired_cues("이 함수 이름 바꿔줘");
    assert!(fired.contains(&"verb:rename".to_string()), "got {fired:?}");
  }

  #[test]
  fn korean_metaphor_cleanliness_fires() {
    let fired = find_fired_cues("이 함수 좀 깔끔하게 만들어줘");
    assert!(
      fired.contains(&"metaphor:cleanliness".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn english_metaphor_speed_fires() {
    let fired = find_fired_cues("this should be faster");
    // "fast" is in patterns for speed (substring of "faster")
    assert!(
      fired.contains(&"metaphor:speed".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn korean_dead_code_fires_deadweight() {
    let fired = find_fired_cues("이 import 안 쓰이는 거 지워줘");
    assert!(
      fired.contains(&"metaphor:deadweight".to_string()),
      "got {fired:?}"
    );
    assert!(fired.contains(&"verb:delete".to_string()), "got {fired:?}");
  }

  #[test]
  fn empty_text_fires_nothing() {
    let fired = find_fired_cues("");
    assert!(fired.is_empty());
  }

  #[test]
  fn unrelated_text_fires_nothing() {
    let fired = find_fired_cues("the quick brown fox jumps over the lazy dog");
    assert!(fired.is_empty(), "got {fired:?}");
  }

  #[test]
  fn mixed_korean_and_english_fires_both_sides() {
    // Operator mid-sentence-switches; should still get verb cues.
    let fired = find_fired_cues("이 함수 좀 refactor 해줘 — 깔끔하게");
    assert!(
      fired.contains(&"metaphor:cleanliness".to_string()),
      "got {fired:?}"
    );
  }

  // ─── verb:create — pnix3d live-coding patterns ─────────────────

  #[test]
  fn korean_create_node_fires_verb_create() {
    let fired = find_fired_cues("새 노드 만들어줘");
    assert!(fired.contains(&"verb:create".to_string()), "got {fired:?}");
  }

  #[test]
  fn korean_생성_fires_verb_create() {
    let fired = find_fired_cues("extern 함수 생성");
    assert!(fired.contains(&"verb:create".to_string()), "got {fired:?}");
  }

  #[test]
  fn english_create_fires_verb_create() {
    let fired = find_fired_cues("create a new node for this scene");
    assert!(fired.contains(&"verb:create".to_string()), "got {fired:?}");
  }

  #[test]
  fn english_make_new_edge_fires_verb_create() {
    // The narrower patterns require an explicit "new <entity>" form to
    // avoid false positives on optimize-intent phrases like "make this
    // code faster". "new edge" is the unambiguous create signal here.
    let fired = find_fired_cues("make a new edge connecting these two nodes");
    assert!(fired.contains(&"verb:create".to_string()), "got {fired:?}");
  }

  // ─── verb:connect — pnix3d wiring patterns ─────────────────────

  #[test]
  fn korean_연결_fires_verb_connect() {
    let fired = find_fired_cues("이 두 노드를 연결해줘");
    assert!(fired.contains(&"verb:connect".to_string()), "got {fired:?}");
  }

  #[test]
  fn korean_와이어링_fires_verb_connect() {
    let fired = find_fired_cues("엣지 와이어링 다시 해줘");
    assert!(fired.contains(&"verb:connect".to_string()), "got {fired:?}");
  }

  #[test]
  fn english_connect_fires_verb_connect() {
    let fired = find_fired_cues("connect the input port to the output");
    assert!(fired.contains(&"verb:connect".to_string()), "got {fired:?}");
  }

  #[test]
  fn english_wire_fires_verb_connect() {
    let fired = find_fired_cues("wire these two modules together");
    assert!(fired.contains(&"verb:connect".to_string()), "got {fired:?}");
  }

  // ─── pnix3d live-coding scenarios end-to-end ───────────────────

  #[test]
  fn pnix3d_node_addition_scenario_fires_create_plus_add() {
    // "이 씬에 새 노드를 추가하고 두 노드 사이를 엣지로 연결해줘"
    let fired = find_fired_cues("이 씬에 새 노드를 추가하고 두 노드 사이를 엣지로 연결해줘");
    assert!(fired.contains(&"verb:add".to_string()), "got {fired:?}");
    assert!(fired.contains(&"verb:connect".to_string()), "got {fired:?}");
  }

  #[test]
  fn pnix3d_create_owner_law_fires_verb_create() {
    // Unambiguous Korean creation: "생성" is a direct creation noun
    // (cf. "만들어" which is overloaded — refactor "깔끔하게 만들어줘" vs
    // create "노드 만들어줘"). For unambiguous create intent we use
    // "생성" / "새로 만들" / "새 <entity>" / "create" / "new <entity>".
    let fired = find_fired_cues("새 owner-law 생성");
    assert!(fired.contains(&"verb:create".to_string()), "got {fired:?}");
  }

  // ─── Python-domain metaphors — substrate-sharing N=5 ────────────

  #[test]
  fn korean_type_hint_fires_python_typing() {
    let fired = find_fired_cues("이 함수에 타입 힌트 붙여줘");
    assert!(
      fired.contains(&"metaphor:python-typing".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn english_type_hints_fires_python_typing() {
    let fired = find_fired_cues("add type hints to this function");
    assert!(
      fired.contains(&"metaphor:python-typing".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn korean_fstring_fires_python_fstring() {
    let fired = find_fired_cues("이 문자열을 f-string으로 바꿔줘");
    assert!(
      fired.contains(&"metaphor:python-fstring".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn korean_decorator_fires_python_decorator() {
    let fired = find_fired_cues("이 메서드에 @property 데코레이터 적용");
    assert!(
      fired.contains(&"metaphor:python-decorator".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn korean_comprehension_fires_python_comprehension() {
    let fired = find_fired_cues("이 for 루프 리스트 컴프리헨션으로 바꿔");
    assert!(
      fired.contains(&"metaphor:python-comprehension".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn english_async_await_fires_python_async() {
    let fired = find_fired_cues("convert this sync function to async await");
    assert!(
      fired.contains(&"metaphor:python-async".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn korean_pytest_fires_python_pytest() {
    let fired = find_fired_cues("pytest fixture 추가해줘");
    assert!(
      fired.contains(&"metaphor:python-pytest".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn korean_dataclass_fires_python_dataclass() {
    let fired = find_fired_cues("이 클래스를 dataclass로 바꿔");
    assert!(
      fired.contains(&"metaphor:python-dataclass".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn korean_pythonic_fires_python_pythonic() {
    let fired = find_fired_cues("이 코드 좀 더 파이써닉하게");
    assert!(
      fired.contains(&"metaphor:python-pythonic".to_string()),
      "got {fired:?}"
    );
  }

  // ─── Python composition end-to-end scenarios ────────────────────

  #[test]
  fn python_add_type_hints_composes_verb_add_plus_metaphor_typing() {
    let fired = find_fired_cues("이 함수에 타입 힌트 추가해줘");
    assert!(fired.contains(&"verb:add".to_string()), "got {fired:?}");
    assert!(
      fired.contains(&"metaphor:python-typing".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn python_pytest_test_composes_verb_test_plus_metaphor_pytest() {
    let fired = find_fired_cues("pytest 로 이 함수 테스트 작성해줘");
    assert!(fired.contains(&"verb:test".to_string()), "got {fired:?}");
    assert!(
      fired.contains(&"metaphor:python-pytest".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn python_fstring_conversion_composes_with_verb_rename() {
    let fired = find_fired_cues("이 .format() 호출 f-string으로 바꿔줘");
    // "바꿔" → verb:rename, "f-string" → metaphor:python-fstring
    assert!(fired.contains(&"verb:rename".to_string()), "got {fired:?}");
    assert!(
      fired.contains(&"metaphor:python-fstring".to_string()),
      "got {fired:?}"
    );
  }

  #[test]
  fn ambiguous_korean_make_does_not_force_create_for_cleanliness_refactor() {
    // Regression: "깔끔하게 만들어줘" is a refactor request, not create.
    // The narrower verb:create patterns must not fire on bare 만들어.
    let fired = find_fired_cues("이 함수 좀 깔끔하게 만들어줘");
    assert!(
      !fired.contains(&"verb:create".to_string()),
      "verb:create should not fire on bare 만들어 (refactor context): got {fired:?}"
    );
    assert!(fired.contains(&"metaphor:cleanliness".to_string()));
  }

  #[test]
  fn english_make_for_optimize_does_not_force_create() {
    // Regression: "make this faster" is optimize intent, not create.
    // The narrower verb:create patterns must not fire on bare make.
    let fired = find_fired_cues("make this code faster please");
    assert!(
      !fired.contains(&"verb:create".to_string()),
      "verb:create should not fire on bare make (optimize context): got {fired:?}"
    );
    assert!(fired.contains(&"metaphor:speed".to_string()));
  }

  #[test]
  fn each_cue_in_registry_has_at_least_one_self_test_pattern() {
    // Self-test: every registry cue should fire on at least one of
    // its own patterns. Catches mistakes like patterns containing
    // chars that prevent substring match.
    for e in VERB_CUE_REGISTRY {
      let p = e.patterns[0];
      let fired = find_fired_cues(p);
      assert!(
        fired.contains(&e.cue.to_string()),
        "cue `{}` did not fire on its own first pattern `{}`",
        e.cue,
        p
      );
    }
  }
}
