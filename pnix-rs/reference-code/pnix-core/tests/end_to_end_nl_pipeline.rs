//! End-to-End 자연어 파이프라인 테스트: 자연어 처리 파이프라인 전체 흐름 테스트
//!
//! 자연어 입력부터 MeaningOp 변환까지의 전체 파이프라인을 테스트합니다.

use pnix_core::effects::{Capability, CapabilitySet, ZoneCapabilities};
use pnix_core::fx::meaning_op::MeaningOpId;
use pnix_core::nlp::{
  ArrowType, NounExtractor, NounTypeMapper, ProofStep, SimpleExtractor, TruthDomain,
  TruthVerification, VerbExtractor, VerbMorphismMapper, VerificationResponse,
};
use pnix_core::runtime::{Process, ProcessId};

#[test]
fn end_to_end_nl_pipeline_smoke() {
  let text = "파일을 저장한다";
  let extractor = SimpleExtractor::default();
  let nouns = extractor.extract_nouns(text);
  let verbs = extractor.extract_verbs(text);
  assert!(!nouns.is_empty());
  assert!(!verbs.is_empty());

  let mut noun_mapper = NounTypeMapper::new();
  let noun = noun_mapper.map_noun(&nouns[0]);
  assert!(noun.is_success());

  let mut verb_mapper = VerbMorphismMapper::new();
  let verb = verb_mapper.map_verb(&verbs[0]);
  let action = verb.morphism().unwrap().arrow_type;
  assert_eq!(action, ArrowType::Create);

  let verification = VerificationResponse::verified(
    TruthDomain::Mathematics,
    vec![ProofStep {
      description: "stub".to_string(),
    }],
    1,
  );
  assert!(matches!(
    verification.result,
    TruthVerification::Verified { .. }
  ));

  let meaning_op = match action {
    ArrowType::Query => MeaningOpId::IoRead,
    ArrowType::Create
    | ArrowType::Delete
    | ArrowType::Update
    | ArrowType::Transform
    | ArrowType::Compose => MeaningOpId::IoWrite,
  };

  let capabilities = match meaning_op {
    MeaningOpId::IoRead => vec![Capability::Read],
    MeaningOpId::IoWrite => vec![Capability::Write],
    _ => vec![],
  };
  let cap_set: CapabilitySet = capabilities.iter().copied().collect();
  let policy = ZoneCapabilities::from_seto_default();
  assert!(policy.allows(meaning_op.zone(), &cap_set));

  let process = Process::new(ProcessId(1), meaning_op.zone(), capabilities, None);
  assert!(process.is_alive());
}
