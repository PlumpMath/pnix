//! Builtin function catalog (data only)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외

use crate::contracts::effect::Effect;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Standard library alias map for builtin name resolution
///
/// Maps stdlib-style names (e.g., "String.concat") to builtin function names (e.g., "concat").
/// This is shared across executor-graph and runtime-legacy to avoid duplication.
pub const STDLIB_ALIAS_MAP: &[(&str, &str)] = &[
  ("String.concat", "concat"),
  ("String.slice", "slice"),
  ("String.length", "stringLength"),
  ("String.split", "split"),
  ("String.join", "join"),
  ("List.map", "map"),
  ("List.filter", "filter"),
  ("List.fold", "fold"),
  ("List.find", "find"),
  ("List.sort", "sort"),
  ("List.reverse", "reverse"),
  ("List.take", "take"),
  ("List.drop", "drop"),
  ("List.zip", "zip"),
  ("List.flatten", "flatten"),
  ("AttrSet.get", "mapGet"),
  ("AttrSet.set", "mapSet"),
  ("AttrSet.keys", "mapKeys"),
  ("AttrSet.values", "mapValues"),
  ("AttrSet.merge", "mapMerge"),
  ("Html.parse", "htmlParse"),
  ("Html.emit", "htmlEmit"),
  ("Xml.parse", "xmlParse"),
  ("Xml.emit", "xmlEmit"),
  ("Process.spawn", "processSpawn"),
  ("Process.ensure", "processEnsure"),
  ("Process.status", "processStatus"),
  ("Process.signal", "processSignal"),
  ("Process.wait", "processWait"),
  ("Process.logsTail", "processLogsTail"),
  ("Process.observeSample", "processObserveSample"),
  ("Process.observeSampleById", "processObserveSampleById"),
  ("Process.terminate", "processTerminate"),
  ("process.spawn", "processSpawn"),
  ("process.ensure", "processEnsure"),
  ("process.status", "processStatus"),
  ("process.signal", "processSignal"),
  ("process.wait", "processWait"),
  ("process.logs.tail", "processLogsTail"),
  ("process.observe.sample", "processObserveSample"),
  ("process.observe.sample.by_id", "processObserveSampleById"),
  ("process.terminate", "processTerminate"),
  ("Runtime.ensure", "processEnsure"),
  ("runtime.ensure", "processEnsure"),
  ("Runtime.call", "runtimeCall"),
  ("runtime.call", "runtimeCall"),
  ("Vm.call", "runtimeCall"),
  ("vm.call", "runtimeCall"),
  ("builtins.Vm.call", "runtimeCall"),
  ("builtins.vm.call", "runtimeCall"),
  ("X3d.xmlToJson", "x3dXmlToJson"),
  ("X3d.schemaNormalize", "x3dSchemaNormalize"),
  ("X3d.schemaValidate", "x3dSchemaValidate"),
  ("X3d.schemaExplain", "x3dSchemaExplain"),
  ("X3d.frpGraph", "x3dFrpGraph"),
  ("X3d.syncPlan", "x3dSyncPlan"),
  ("X3d.x3domFragment", "x3dX3domFragment"),
  ("X3d.x3domHtml", "x3dX3domHtml"),
  ("X3d.x3domPatch", "x3dX3domPatch"),
  ("X3d.renderPacket", "x3dRenderPacket"),
  ("MathML.xmlToJson", "mathmlXmlToJson"),
  ("MathML.schemaNormalize", "mathmlSchemaNormalize"),
  ("MathML.schemaValidate", "mathmlSchemaValidate"),
  ("MathML.schemaExplain", "mathmlSchemaExplain"),
  ("MathML.emit", "mathmlEmit"),
  ("OpenMath.xmlToJson", "openmathXmlToJson"),
  ("OpenMath.schemaNormalize", "openmathSchemaNormalize"),
  ("OpenMath.schemaValidate", "openmathSchemaValidate"),
  ("OpenMath.schemaExplain", "openmathSchemaExplain"),
  ("OpenMath.emit", "openmathEmit"),
  ("Excel.xmlToJson", "excelXmlToJson"),
  ("Excel.emit", "excelEmit"),
  ("Excel.toOds", "excelToOds"),
  ("Excel.fromOds", "odsToExcel"),
  ("ODS.toExcel", "odsToExcel"),
  ("ODS.fromExcel", "excelToOds"),
  ("Excel.formulaToOpenFormula", "excelFormulaToOpenFormula"),
  ("Excel.formulaFromOpenFormula", "openFormulaToExcel"),
  ("ODS.formulaFromExcel", "excelFormulaToOpenFormula"),
  ("ODS.formulaToExcel", "openFormulaToExcel"),
  ("Excel.styleToOds", "excelStyleToOds"),
  ("Excel.styleFromOds", "odsStyleToExcel"),
  ("ODS.styleFromExcel", "excelStyleToOds"),
  ("ODS.styleToExcel", "odsStyleToExcel"),
  ("Excel.advancedToOds", "excelAdvancedToOds"),
  ("Excel.advancedFromOds", "odsAdvancedToExcel"),
  ("ODS.advancedFromExcel", "excelAdvancedToOds"),
  ("ODS.advancedToExcel", "odsAdvancedToExcel"),
  ("Xml.schemaNormalize", "xmlSchemaNormalize"),
  ("Xml.schemaValidate", "xmlSchemaValidate"),
  ("Xml.schemaExplain", "xmlSchemaExplain"),
  ("Svg.schemaNormalize", "svgSchemaNormalize"),
  ("Svg.schemaValidate", "svgSchemaValidate"),
  ("Svg.schemaExplain", "svgSchemaExplain"),
  ("Svg.emit", "svgEmit"),
  ("Svg.renderPacket", "svgRenderPacket"),
  ("Ifcxml.schemaNormalize", "ifcxmlSchemaNormalize"),
  ("Ifcxml.schemaValidate", "ifcxmlSchemaValidate"),
  ("Ifcxml.schemaExplain", "ifcxmlSchemaExplain"),
  ("SBML.schemaNormalize", "sbmlSchemaNormalize"),
  ("SBML.schemaValidate", "sbmlSchemaValidate"),
  ("SBML.schemaExplain", "sbmlSchemaExplain"),
  ("CellML.schemaNormalize", "cellmlSchemaNormalize"),
  ("CellML.schemaValidate", "cellmlSchemaValidate"),
  ("CellML.schemaExplain", "cellmlSchemaExplain"),
  ("NeuroML.schemaNormalize", "neuromlSchemaNormalize"),
  ("NeuroML.schemaValidate", "neuromlSchemaValidate"),
  ("NeuroML.schemaExplain", "neuromlSchemaExplain"),
  ("LEMS.schemaNormalize", "lemsSchemaNormalize"),
  ("LEMS.schemaValidate", "lemsSchemaValidate"),
  ("LEMS.schemaExplain", "lemsSchemaExplain"),
  ("SED-ML.schemaNormalize", "sedmlSchemaNormalize"),
  ("SED-ML.schemaValidate", "sedmlSchemaValidate"),
  ("SED-ML.schemaExplain", "sedmlSchemaExplain"),
  ("OMEX.schemaNormalize", "omexSchemaNormalize"),
  ("OMEX.schemaValidate", "omexSchemaValidate"),
  ("OMEX.schemaExplain", "omexSchemaExplain"),
  ("PharmML.schemaNormalize", "pharmmlSchemaNormalize"),
  ("PharmML.schemaValidate", "pharmmlSchemaValidate"),
  ("PharmML.schemaExplain", "pharmmlSchemaExplain"),
  ("CML.schemaNormalize", "cmlSchemaNormalize"),
  ("CML.schemaValidate", "cmlSchemaValidate"),
  ("CML.schemaExplain", "cmlSchemaExplain"),
  ("PDBML.schemaNormalize", "pdbmlSchemaNormalize"),
  ("PDBML.schemaValidate", "pdbmlSchemaValidate"),
  ("PDBML.schemaExplain", "pdbmlSchemaExplain"),
  ("SBGN-ML.schemaNormalize", "sbgnmlSchemaNormalize"),
  ("SBGN-ML.schemaValidate", "sbgnmlSchemaValidate"),
  ("SBGN-ML.schemaExplain", "sbgnmlSchemaExplain"),
  ("BioPAX.schemaNormalize", "biopaxSchemaNormalize"),
  ("BioPAX.schemaValidate", "biopaxSchemaValidate"),
  ("BioPAX.schemaExplain", "biopaxSchemaExplain"),
  ("VTK.schemaNormalize", "vtkSchemaNormalize"),
  ("VTK.schemaValidate", "vtkSchemaValidate"),
  ("VTK.schemaExplain", "vtkSchemaExplain"),
  ("XDMF.schemaNormalize", "xdmfSchemaNormalize"),
  ("XDMF.schemaValidate", "xdmfSchemaValidate"),
  ("XDMF.schemaExplain", "xdmfSchemaExplain"),
  ("GIFTI.schemaNormalize", "giftiSchemaNormalize"),
  ("GIFTI.schemaValidate", "giftiSchemaValidate"),
  ("GIFTI.schemaExplain", "giftiSchemaExplain"),
  ("schema.validate", "schemaValidate"),
  ("schema.normalize", "schemaNormalize"),
  ("schema.explain", "schemaExplain"),
  ("Ontology.lift", "ontologyLift"),
  ("Ontology.evaluate", "ontologyEvaluate"),
  ("Ontology.select", "ontologySelect"),
  ("Ontology.promote", "ontologyPromote"),
  ("Ontology.query", "ontologyQuery"),
  ("Ontology.emit", "ontologyEmit"),
  ("ontology.lift", "ontologyLift"),
  ("ontology.evaluate", "ontologyEvaluate"),
  ("ontology.select", "ontologySelect"),
  ("ontology.promote", "ontologyPromote"),
  ("ontology.query", "ontologyQuery"),
  ("ontology.emit", "ontologyEmit"),
  ("py.abs", "abs"),
  ("py.add", "add"),
  ("py.and", "and"),
  ("py.cos", "cos"),
  ("py.div", "div"),
  ("py.eq", "eq"),
  ("py.exp", "exp"),
  ("py.gt", "gt"),
  ("py.log2", "log2"),
  ("py.lt", "lt"),
  ("py.mod", "mod"),
  ("py.mul", "mul"),
  ("py.neg", "neg"),
  ("py.sin", "sin"),
  ("py.sqrt", "sqrt"),
  ("py.sub", "sub"),
];

/// Resolve explicit builtin uses forms to builtin catalog keys.
///
/// This intentionally resolves only explicit forms:
/// - `builtins.<name>`
/// - aliases listed in `STDLIB_ALIAS_MAP`
pub fn resolve_builtin_name<'a>(uses: &'a str) -> Option<Cow<'a, str>> {
  let uses = uses.trim();
  if let Some(rest) = uses.strip_prefix("builtins.") {
    if !rest.is_empty() && is_well_formed_builtin_path(rest) {
      return Some(normalize_builtin_name(rest));
    }
    return None;
  }

  for (alias, builtin) in STDLIB_ALIAS_MAP {
    if *alias == uses {
      return Some(Cow::Borrowed(*builtin));
    }
  }

  None
}

fn is_well_formed_builtin_path(path: &str) -> bool {
  let mut saw_segment = false;
  for segment in path.split('.') {
    if segment.is_empty() {
      return false;
    }
    saw_segment = true;
  }
  saw_segment
}

/// Normalize builtin name for runtime/eval dispatch.
///
/// Returns canonical builtin key when possible, otherwise the input form.
pub fn normalize_builtin_name<'a>(name: &'a str) -> Cow<'a, str> {
  let name = name.trim();
  let stripped = name.strip_prefix("builtins.").unwrap_or(name);
  for (alias, builtin) in STDLIB_ALIAS_MAP {
    if *alias == stripped {
      return Cow::Borrowed(*builtin);
    }
  }
  Cow::Borrowed(stripped)
}

/// Resolve any builtin-like use to a spec catalog key if the key exists.
///
/// Accepted forms:
/// - direct key (`add`)
/// - explicit forms via `resolve_builtin_name` (`builtins.add`, `String.length`, ...)
/// - lowering-injected `fx_` forms (`fx_add`)
pub fn resolve_spec_builtin_name<'a>(
  uses: &'a str,
  catalog: &BuiltinCatalog,
) -> Option<Cow<'a, str>> {
  let uses = uses.trim();
  if catalog.contains(uses) {
    return Some(Cow::Borrowed(uses));
  }

  if let Some(explicit) = resolve_builtin_name(uses) {
    if catalog.contains(explicit.as_ref()) {
      return Some(explicit);
    }
    let normalized = normalize_builtin_name(explicit.as_ref());
    if catalog.contains(normalized.as_ref()) {
      return Some(Cow::Owned(normalized.into_owned()));
    }
  }

  if let Some(rest) = uses.strip_prefix("fx_") {
    if catalog.contains(rest) {
      return Some(Cow::Borrowed(rest));
    }
  }

  None
}

/// Whether uses string is an explicit builtin form (`builtins.*` or known alias).
pub fn is_builtin_uses(uses: &str) -> bool {
  resolve_builtin_name(uses).is_some()
}

/// Builtin 함수 선언: 내장 함수의 선언 정보
// LOW: signature 구문 검증 없음
// 잘못된 형식 허용
// 현재는 signature 형식을 검증하지 않아 잘못된 형식 허용 가능
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuiltinDecl {
  /// 함수 이름
  pub name: String,
  /// 함수 시그니처 문자열 (예: "Num → Num → Num")
  pub signature: String,
  /// 효과 타입 (Pure/World)
  pub effect: Effect,
  /// Capability 요구사항 목록 (예: ["Math", "Arithmetic"])
  #[serde(default)]
  pub capabilities: Vec<String>,
  /// 파라미터 개수 (None이면 가변 인자)
  pub arity: Option<usize>,
  /// 함수 설명
  pub description: String,
}

/// Builtin 함수 카탈로그: 등록된 모든 builtin 함수의 카탈로그
///
/// Deterministic ordering을 위해 BTreeMap 사용
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinCatalog {
  /// 등록된 builtin 함수들 (이름 → 선언 매핑)
  pub functions: BTreeMap<String, BuiltinDecl>,
}

impl BuiltinCatalog {
  /// 빈 카탈로그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      functions: BTreeMap::new(),
    }
  }

  /// 기본 builtin 포함 카탈로그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_defaults() -> Self {
    let mut catalog = Self::new();

    // Arithmetic
    catalog.register(BuiltinDecl {
      name: "add".to_string(),
      signature: "Num → Num → Num".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Arithmetic".to_string()],
      arity: Some(2),
      description: "Add two numbers".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sub".to_string(),
      signature: "Num → Num → Num".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Arithmetic".to_string()],
      arity: Some(2),
      description: "Subtract two numbers".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mul".to_string(),
      signature: "Num → Num → Num".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Arithmetic".to_string()],
      arity: Some(2),
      description: "Multiply two numbers".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "div".to_string(),
      signature: "Num → Num → Num".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Arithmetic".to_string()],
      arity: Some(2),
      description: "Divide two numbers".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mod".to_string(),
      signature: "Int → Int → Int".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Arithmetic".to_string()],
      arity: Some(2),
      description: "Modulo operation".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "neg".to_string(),
      signature: "Num → Num".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Arithmetic".to_string()],
      arity: Some(1),
      description: "Negate a number".to_string(),
    });

    // Math functions
    catalog.register(BuiltinDecl {
      name: "sin".to_string(),
      signature: "Float → Float".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Trigonometry".to_string()],
      arity: Some(1),
      description: "Sine function".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "cos".to_string(),
      signature: "Float → Float".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Trigonometry".to_string()],
      arity: Some(1),
      description: "Cosine function".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "tan".to_string(),
      signature: "Float → Float".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string(), "Trigonometry".to_string()],
      arity: Some(1),
      description: "Tangent function".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sqrt".to_string(),
      signature: "Float → Float".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string()],
      arity: Some(1),
      description: "Square root".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "exp".to_string(),
      signature: "Float → Float".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string()],
      arity: Some(1),
      description: "Exponential function".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "log2".to_string(),
      signature: "Float → Float".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string()],
      arity: Some(1),
      description: "Log base 2".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "floor".to_string(),
      signature: "Float → Int".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string()],
      arity: Some(1),
      description: "Floor function".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ceil".to_string(),
      signature: "Float → Int".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string()],
      arity: Some(1),
      description: "Ceiling function".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "abs".to_string(),
      signature: "Num → Num".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string()],
      arity: Some(1),
      description: "Absolute value".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "pow".to_string(),
      signature: "Num → Num → Num".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Math".to_string()],
      arity: Some(2),
      description: "Power operation (base^exponent)".to_string(),
    });

    // Ontology operations
    catalog.register(BuiltinDecl {
      name: "ontologyLift".to_string(),
      signature: "Any → Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ontology".to_string(), "Meaning".to_string()],
      arity: Some(2),
      description: "Lift contextual fact(s) into a new ontology context".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ontologyEvaluate".to_string(),
      signature: "Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ontology".to_string(), "Meaning".to_string()],
      arity: Some(2),
      description: "Evaluate an ontology interpretation against contextual facts".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ontologySelect".to_string(),
      signature: "Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ontology".to_string(), "Meaning".to_string()],
      arity: Some(2),
      description: "Select an ontology interpretation deterministically".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ontologyPromote".to_string(),
      signature: "Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ontology".to_string(), "Meaning".to_string()],
      arity: Some(2),
      description: "Promote an ontology judgement into a lifecycle status. \
         OWNER-LAW (2026-05-11): legacy 2-arity entry; treated as InternalOwnerLaw lane. \
         External callers (web search / API / OCR / human prose / tool result / peer evidence) \
         MUST use ontologyPromoteWithLane to ensure Accept→Candidate downgrade."
        .to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ontologyPromoteWithLane".to_string(),
      signature: "Any → Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ontology".to_string(), "Meaning".to_string()],
      arity: Some(3),
      description: "OWNER-LAW (2026-05-11) canonical: promote a judgement with an explicit \
         EvidenceLane (InternalOwnerLaw / InternalAcceptedMemory / ExternalWebSearch / \
         ExternalApi / TransducerOutput / HumanProvidedProse / ToolExecutionResult / \
         PeerEvidence). External lanes downgrade Accept→Candidate; only internal lanes \
         allow direct Accepted."
        .to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ontologyQuery".to_string(),
      signature: "AttrSet → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ontology".to_string(), "Query".to_string()],
      arity: Some(1),
      description: "Query ontology facts from the semantic store".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ontologyEmit".to_string(),
      signature: "AttrSet → AttrSet".to_string(),
      effect: Effect::World,
      capabilities: vec!["Ontology".to_string(), "Emit".to_string()],
      arity: Some(1),
      description: "Emit a new ontology fact into the semantic store".to_string(),
    });

    // Comparison
    catalog.register(BuiltinDecl {
      name: "lt".to_string(),
      signature: "Ord a ⇒ a → a → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Comparison".to_string()],
      arity: Some(2),
      description: "Less than".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "gt".to_string(),
      signature: "Ord a ⇒ a → a → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Comparison".to_string()],
      arity: Some(2),
      description: "Greater than".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "le".to_string(),
      signature: "Ord a ⇒ a → a → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Comparison".to_string()],
      arity: Some(2),
      description: "Less than or equal".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ge".to_string(),
      signature: "Ord a ⇒ a → a → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Comparison".to_string()],
      arity: Some(2),
      description: "Greater than or equal".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "eq".to_string(),
      signature: "Eq a ⇒ a → a → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Comparison".to_string()],
      arity: Some(2),
      description: "Equality".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ne".to_string(),
      signature: "Eq a ⇒ a → a → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Comparison".to_string()],
      arity: Some(2),
      description: "Inequality".to_string(),
    });

    // Logic
    catalog.register(BuiltinDecl {
      name: "and".to_string(),
      signature: "Bool → Bool → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Logic".to_string()],
      arity: Some(2),
      description: "Logical AND".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "or".to_string(),
      signature: "Bool → Bool → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Logic".to_string()],
      arity: Some(2),
      description: "Logical OR".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "not".to_string(),
      signature: "Bool → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Logic".to_string()],
      arity: Some(1),
      description: "Logical NOT".to_string(),
    });

    // Stdlib: String
    catalog.register(BuiltinDecl {
      name: "concat".to_string(),
      signature: "String → String → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Concatenate two strings".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "slice".to_string(),
      signature: "Int → Int → String → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(3),
      description: "Slice a string by start and length".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "stringLength".to_string(),
      signature: "String → Int".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(1),
      description: "Get string length".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "split".to_string(),
      signature: "String → String → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Split a string by delimiter".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "join".to_string(),
      signature: "String → List → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Join strings with delimiter".to_string(),
    });

    // Stdlib: List
    catalog.register(BuiltinDecl {
      name: "map".to_string(),
      signature: "Any → List → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Map function over list".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "genList".to_string(),
      signature: "(Int → Any) → Int → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Generate a list by applying a function to indices 0..n-1".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "filter".to_string(),
      signature: "Any → List → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Filter list by predicate".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "fold".to_string(),
      signature: "Any → Any → List → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(3),
      description: "Left fold over list".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "foldl'".to_string(),
      signature: "Any → Any → List → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(3),
      description: "Strict left fold over list".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "find".to_string(),
      signature: "Any → List → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Find element in list".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sort".to_string(),
      signature: "(a → a → Int) → List → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Sort list with comparison function".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "reverse".to_string(),
      signature: "List → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(1),
      description: "Reverse list".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "take".to_string(),
      signature: "Int → List → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Take first n elements".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "drop".to_string(),
      signature: "Int → List → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Drop first n elements".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "zip".to_string(),
      signature: "List → List → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Zip two lists".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "flatten".to_string(),
      signature: "List → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(1),
      description: "Flatten list of lists".to_string(),
    });

    // Stdlib: AttrSet
    catalog.register(BuiltinDecl {
      name: "mapGet".to_string(),
      signature: "AttrSet → String → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Get value by key from attrset".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "get".to_string(),
      signature: "AttrSet → String → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Get value by key from attrset".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mapSet".to_string(),
      signature: "AttrSet → String → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(3),
      description: "Set value by key in attrset".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "set".to_string(),
      signature: "AttrSet → String → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(3),
      description: "Set value by key in attrset".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mapKeys".to_string(),
      signature: "AttrSet → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(1),
      description: "Get attrset keys".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "keys".to_string(),
      signature: "AttrSet → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(1),
      description: "Get attrset keys".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mapValues".to_string(),
      signature: "AttrSet → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(1),
      description: "Get attrset values".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "values".to_string(),
      signature: "AttrSet → List".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(1),
      description: "Get attrset values".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mapMerge".to_string(),
      signature: "AttrSet → AttrSet → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Merge attrsets (right wins)".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "merge".to_string(),
      signature: "AttrSet → AttrSet → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(2),
      description: "Merge attrsets (right wins)".to_string(),
    });

    // XML helpers
    catalog.register(BuiltinDecl {
      name: "xmlParse".to_string(),
      signature: "String → XmlAst".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Xml".to_string()],
      arity: Some(1),
      description: "Parse XML into XmlAst".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "xmlEmit".to_string(),
      signature: "XmlAst → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Xml".to_string()],
      arity: Some(1),
      description: "Emit XmlAst as XML string".to_string(),
    });
    // Process helpers (control-plane core)
    catalog.register(BuiltinDecl {
      name: "processSpawn".to_string(),
      signature: "ProcessSpec → ProcessHandle".to_string(),
      effect: Effect::World,
      capabilities: vec!["ProcessSpawn".to_string()],
      arity: Some(1),
      description: "Spawn a process (supervisor-managed)".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "processEnsure".to_string(),
      signature: "ProcessSpec → ProcessHandle".to_string(),
      effect: Effect::World,
      capabilities: vec!["ProcessSpawn".to_string(), "ProcessObserve".to_string()],
      arity: Some(1),
      description: "Ensure a process exists (idempotent reconcile; requires spec.id)".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "runtimeCall".to_string(),
      signature: "String → String → Any → AttrSet → Any".to_string(),
      effect: Effect::World,
      capabilities: vec!["RuntimeCall".to_string()],
      arity: Some(4),
      description: "Call a runtime backend method via supervisor runtime.call adapter; world/default apply_world_edit is canonicalized through the shared freecat VmCallRequest contract".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "processStatus".to_string(),
      signature: "ProcessHandle → ProcessStatus".to_string(),
      effect: Effect::World,
      capabilities: vec!["ProcessObserve".to_string()],
      arity: Some(1),
      description: "Get process status snapshot".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "processSignal".to_string(),
      signature: "ProcessHandle → String → Bool".to_string(),
      effect: Effect::World,
      capabilities: vec!["ProcessSignal".to_string()],
      arity: Some(2),
      description: "Send a signal to process".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "processWait".to_string(),
      signature: "ProcessHandle → Num → ProcessExit".to_string(),
      effect: Effect::World,
      capabilities: vec!["ProcessObserve".to_string()],
      arity: Some(2),
      description: "Wait process exit with timeout_ms".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "processLogsTail".to_string(),
      signature: "ProcessHandle → Num → AttrSet".to_string(),
      effect: Effect::World,
      capabilities: vec!["ProcessObserve".to_string()],
      arity: Some(2),
      description: "Tail captured process logs via supervisor".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "processTerminate".to_string(),
      signature: "ProcessHandle → Num → Bool".to_string(),
      effect: Effect::World,
      capabilities: vec!["ProcessSignal".to_string(), "ProcessObserve".to_string()],
      arity: Some(2),
      description: "Graceful terminate (TERM then KILL) via supervisor".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "processObserveSample".to_string(),
      signature: "ProcessHandle → AttrSet → AttrSet".to_string(),
      effect: Effect::World,
      capabilities: vec!["World".to_string(), "ProcessObserve".to_string()],
      arity: Some(2),
      description: "Sample process observation by handle (cpu/mem/io/threads/fds)".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "processObserveSampleById".to_string(),
      signature: "String → AttrSet → AttrSet".to_string(),
      effect: Effect::World,
      capabilities: vec!["World".to_string(), "ProcessObserve".to_string()],
      arity: Some(2),
      description: "Sample process observation by logical id".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dXmlToJson".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["X3d".to_string(), "Xml".to_string()],
      arity: Some(1),
      description: "Parse X3D XML or XML JSON into XML JSON".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["X3d".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize X3D XML JSON with schema defaults".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["X3d".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate X3D XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["X3d".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain X3D XML JSON validation errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dFrpGraph".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["X3d".to_string(), "Frp".to_string()],
      arity: Some(1),
      description: "Build FRP graph JSON from X3D XML or XML JSON".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dSyncPlan".to_string(),
      signature: "Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["X3d".to_string(), "Patch".to_string(), "Sync".to_string()],
      arity: Some(2),
      description:
        "Annotate stable sync ids and compute attr/subtree patch plan between X3D scenes"
          .to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dX3domFragment".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["X3d".to_string(), "X3dom".to_string(), "Html".to_string()],
      arity: Some(1),
      description:
        "Lower X3D XML or XML JSON into mountable X3DOM fragment with stable data-node-id annotations"
          .to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dX3domHtml".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec![
        "X3d".to_string(),
        "X3dom".to_string(),
        "Html".to_string(),
        "Webview".to_string(),
      ],
      arity: Some(1),
      description:
        "Lower X3D XML or XML JSON into standalone X3DOM HTML document for webview/bootstrap"
          .to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dX3domPatch".to_string(),
      signature: "Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec![
        "X3d".to_string(),
        "X3dom".to_string(),
        "Patch".to_string(),
        "Sync".to_string(),
        "Webview".to_string(),
      ],
      arity: Some(2),
      description:
        "Lower X3D sync-plan output into X3DOM/webview patch payload with HTML fragment/document and minimal ops"
          .to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "x3dRenderPacket".to_string(),
      signature: "Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec![
        "X3d".to_string(),
        "Frp".to_string(),
        "Physics".to_string(),
        "HAnim".to_string(),
        "Symbolic".to_string(),
        "Patch".to_string(),
        "Webview".to_string(),
      ],
      arity: Some(2),
      description:
        "Build an append-only render/runtime packet that carries X3D scene sync, x3dom lowering, FRP graph, physics summary, HAnim skeleton state, and symbolic world-model equations/constraints"
          .to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mathmlXmlToJson".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["MathML".to_string(), "Xml".to_string()],
      arity: Some(1),
      description: "Parse MathML XML or XML JSON into XML JSON".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mathmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["MathML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize MathML XML JSON with schema defaults".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mathmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["MathML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate MathML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mathmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["MathML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain MathML XML JSON validation errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "openmathXmlToJson".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["OpenMath".to_string(), "Xml".to_string()],
      arity: Some(1),
      description: "Parse OpenMath XML or XML JSON into XML JSON".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "openmathSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["OpenMath".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize OpenMath XML JSON with schema defaults".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "openmathSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["OpenMath".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate OpenMath XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "openmathSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["OpenMath".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain OpenMath XML JSON validation errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "xmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Xml".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize generic XML JSON with schema defaults".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "xmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Xml".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate generic XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "xmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Xml".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain generic XML JSON validation errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "svgSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Svg".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize SVG XML JSON with schema defaults".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "svgSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Svg".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate SVG XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "svgSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Svg".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain SVG XML JSON validation errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "svgEmit".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Svg".to_string(), "Xml".to_string()],
      arity: Some(1),
      description: "Emit normalized SVG XML string from SVG XML JSON or XML string".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "svgRenderPacket".to_string(),
      signature: "Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec![
        "Svg".to_string(),
        "Patch".to_string(),
        "Webview".to_string(),
      ],
      arity: Some(2),
      description:
        "Build an append-only render/runtime packet that carries SVG scene sync, HTML lowering, and shared world-memory envelope"
          .to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ifcxmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ifcxml".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize IFCXML XML JSON with schema defaults".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ifcxmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ifcxml".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate IFCXML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "ifcxmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Ifcxml".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain IFCXML XML JSON validation errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sbmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SBML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize SBML XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sbmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SBML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate SBML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sbmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SBML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain SBML XML JSON validation errors".to_string(),
    });
    // CellML
    catalog.register(BuiltinDecl {
      name: "cellmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["CellML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize CellML XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "cellmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["CellML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate CellML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "cellmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["CellML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain CellML XML JSON validation errors".to_string(),
    });
    // NeuroML
    catalog.register(BuiltinDecl {
      name: "neuromlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["NeuroML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize NeuroML XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "neuromlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["NeuroML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate NeuroML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "neuromlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["NeuroML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain NeuroML XML JSON validation errors".to_string(),
    });
    // LEMS
    catalog.register(BuiltinDecl {
      name: "lemsSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["LEMS".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize LEMS XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "lemsSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["LEMS".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate LEMS XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "lemsSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["LEMS".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain LEMS XML JSON validation errors".to_string(),
    });
    // SED-ML
    catalog.register(BuiltinDecl {
      name: "sedmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SED-ML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize SED-ML XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sedmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SED-ML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate SED-ML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sedmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SED-ML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain SED-ML XML JSON validation errors".to_string(),
    });
    // OMEX
    catalog.register(BuiltinDecl {
      name: "omexSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["OMEX".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize OMEX XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "omexSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["OMEX".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate OMEX XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "omexSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["OMEX".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain OMEX XML JSON validation errors".to_string(),
    });
    // PharmML
    catalog.register(BuiltinDecl {
      name: "pharmmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["PharmML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize PharmML XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "pharmmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["PharmML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate PharmML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "pharmmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["PharmML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain PharmML XML JSON validation errors".to_string(),
    });
    // CML
    catalog.register(BuiltinDecl {
      name: "cmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["CML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize CML XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "cmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["CML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate CML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "cmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["CML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain CML XML JSON validation errors".to_string(),
    });
    // PDBML
    catalog.register(BuiltinDecl {
      name: "pdbmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["PDBML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize PDBML XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "pdbmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["PDBML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate PDBML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "pdbmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["PDBML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain PDBML XML JSON validation errors".to_string(),
    });
    // SBGN-ML
    catalog.register(BuiltinDecl {
      name: "sbgnmlSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SBGN-ML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize SBGN-ML XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sbgnmlSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SBGN-ML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate SBGN-ML XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "sbgnmlSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["SBGN-ML".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain SBGN-ML XML JSON validation errors".to_string(),
    });
    // BioPAX
    catalog.register(BuiltinDecl {
      name: "biopaxSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["BioPAX".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize BioPAX XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "biopaxSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["BioPAX".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate BioPAX XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "biopaxSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["BioPAX".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain BioPAX XML JSON validation errors".to_string(),
    });
    // VTK
    catalog.register(BuiltinDecl {
      name: "vtkSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["VTK".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize VTK XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "vtkSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["VTK".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate VTK XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "vtkSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["VTK".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain VTK XML JSON validation errors".to_string(),
    });
    // XDMF
    catalog.register(BuiltinDecl {
      name: "xdmfSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["XDMF".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize XDMF XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "xdmfSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["XDMF".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate XDMF XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "xdmfSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["XDMF".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain XDMF XML JSON validation errors".to_string(),
    });
    // GIFTI
    catalog.register(BuiltinDecl {
      name: "giftiSchemaNormalize".to_string(),
      signature: "Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["GIFTI".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Normalize GIFTI XML JSON using schema".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "giftiSchemaValidate".to_string(),
      signature: "Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["GIFTI".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Validate GIFTI XML JSON and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "giftiSchemaExplain".to_string(),
      signature: "Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["GIFTI".to_string(), "Schema".to_string()],
      arity: Some(1),
      description: "Explain GIFTI XML JSON validation errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "mathmlEmit".to_string(),
      signature: "AttrSet → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["MathML".to_string(), "Xml".to_string()],
      arity: Some(1),
      description: "Emit MathML XML from MathML JSON graph".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "openmathEmit".to_string(),
      signature: "AttrSet → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["OpenMath".to_string(), "Xml".to_string()],
      arity: Some(1),
      description: "Emit OpenMath XML from OpenMath JSON graph".to_string(),
    });

    // HTML helpers
    catalog.register(BuiltinDecl {
      name: "htmlParse".to_string(),
      signature: "String → HtmlAst".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Html".to_string()],
      arity: Some(1),
      description: "Parse HTML into HtmlAst".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "htmlEmit".to_string(),
      signature: "HtmlAst → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Html".to_string()],
      arity: Some(1),
      description: "Emit HtmlAst as HTML string".to_string(),
    });

    // Schema helpers
    catalog.register(BuiltinDecl {
      name: "schemaValidate".to_string(),
      signature: "Any → Any → AttrSet".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Schema".to_string()],
      arity: Some(2),
      description: "Validate value against schema and return ok/errors".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "schemaNormalize".to_string(),
      signature: "Any → Any → Any".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Schema".to_string()],
      arity: Some(2),
      description: "Normalize value against schema defaults".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "schemaExplain".to_string(),
      signature: "Any → Any → String".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Schema".to_string()],
      arity: Some(2),
      description: "Explain schema validation errors as text".to_string(),
    });

    // IO functions
    catalog.register(BuiltinDecl {
      name: "io.readFile".to_string(),
      signature: "String → String".to_string(),
      effect: Effect::World,
      capabilities: vec!["Io".to_string(), "FileSystem".to_string()],
      arity: Some(1),
      description: "Read file contents as string".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "io.writeFile".to_string(),
      signature: "String → String → String".to_string(),
      effect: Effect::World,
      capabilities: vec!["Io".to_string(), "FileSystem".to_string()],
      arity: Some(2),
      description: "Write contents to file path".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "io.print".to_string(),
      signature: "String → String".to_string(),
      effect: Effect::World,
      capabilities: vec!["Io".to_string()],
      arity: Some(1),
      description: "Print string to stdout".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "pathExists".to_string(),
      signature: "String → Bool".to_string(),
      effect: Effect::World,
      capabilities: vec!["Io".to_string(), "FileSystem".to_string()],
      arity: Some(1),
      description: "Check if path exists".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "isStorePath".to_string(),
      signature: "String → Bool".to_string(),
      effect: Effect::Pure,
      capabilities: vec!["Pure".to_string()],
      arity: Some(1),
      description: "Check if string is a Nix store path".to_string(),
    });
    catalog.register(BuiltinDecl {
      name: "isValidPath".to_string(),
      signature: "String → Bool".to_string(),
      effect: Effect::World,
      capabilities: vec!["Io".to_string(), "FileSystem".to_string()],
      arity: Some(1),
      description: "Check if store path is valid and exists".to_string(),
    });

    register_px_declared_builtin_extensions(&mut catalog);

    catalog
  }

  /// Builtin 함수 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register(&mut self, decl: BuiltinDecl) {
    // HIGH: 중복 builtin 등록 감지
    // 동일 이름 builtin이 이미 등록되어 있으면 즉시 실패
    if self.functions.contains_key(&decl.name) {
      panic!("Builtin '{}' is already registered", decl.name);
    }
    // LOW: signature 구문 검증 없음 수정
    // signature가 비어있거나 기본 형식이 아니면 경고
    if decl.signature.is_empty() {
      eprintln!("Warning: Builtin '{}' has empty signature", decl.name);
    } else if !decl.signature.contains("→") && !decl.signature.contains("->") {
      // 기본 형식: "Type → Type" 또는 "Type -> Type"
      eprintln!("Warning: Builtin '{}' signature '{}' may have invalid format (expected 'Type → Type' or 'Type -> Type')", decl.name, decl.signature);
    }
    self.functions.insert(decl.name.clone(), decl);
  }

  /// Builtin 함수 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<&BuiltinDecl> {
    self.functions.get(name)
  }

  /// Builtin 함수 존재 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains(&self, name: &str) -> bool {
    self.functions.contains_key(name)
  }
}

const STDLIB_BUILTIN_CATALOG_EXTENSION_PX: &str =
  include_str!("../../../../stdlib/lib/spec-builtin-catalog.px");

fn register_px_declared_builtin_extensions(catalog: &mut BuiltinCatalog) {
  for decl in px_declared_builtin_extensions() {
    catalog.register(decl);
  }
}

fn px_declared_builtin_extensions() -> Vec<BuiltinDecl> {
  let ast = crate::lang::pnix::parse_expr_to_ast_json(STDLIB_BUILTIN_CATALOG_EXTENSION_PX)
    .expect("stdlib/lib/spec-builtin-catalog.px must parse");
  let root = &ast["root"];
  let rows = attr_set_field(root, "builtinCatalogExtensionRows")
    .unwrap_or_else(|| panic!("spec-builtin-catalog.px missing builtinCatalogExtensionRows"));
  let row_items = rows["items"]
    .as_array()
    .unwrap_or_else(|| panic!("builtinCatalogExtensionRows must be a list"));
  row_items
    .iter()
    .map(builtin_decl_from_ast_row)
    .collect::<Vec<_>>()
}

fn builtin_decl_from_ast_row(row: &Value) -> BuiltinDecl {
  let name = string_field(row, "name");
  BuiltinDecl {
    name,
    signature: string_field(row, "signature"),
    effect: effect_field(row, "effect"),
    capabilities: string_list_field(row, "capabilities"),
    arity: arity_field(row, "arity"),
    description: string_field(row, "description"),
  }
}

fn attr_set_field<'a>(attr_set: &'a Value, field_name: &str) -> Option<&'a Value> {
  if attr_set["kind"].as_str()? != "attr_set" {
    return None;
  }
  attr_set["items"].as_array()?.iter().find_map(|item| {
    if item["kind"].as_str()? != "assign" {
      return None;
    }
    let key_path = item["key_path"].as_array()?;
    if key_path.len() == 1 && key_path[0].as_str()? == field_name {
      Some(&item["value"])
    } else {
      None
    }
  })
}

fn string_field(row: &Value, field_name: &str) -> String {
  let value =
    attr_set_field(row, field_name).unwrap_or_else(|| panic!("builtin row missing `{field_name}`"));
  value["value"]
    .as_str()
    .unwrap_or_else(|| panic!("builtin row `{field_name}` must be a string"))
    .to_string()
}

fn string_list_field(row: &Value, field_name: &str) -> Vec<String> {
  let value =
    attr_set_field(row, field_name).unwrap_or_else(|| panic!("builtin row missing `{field_name}`"));
  value["items"]
    .as_array()
    .unwrap_or_else(|| panic!("builtin row `{field_name}` must be a list"))
    .iter()
    .map(|item| {
      item["value"]
        .as_str()
        .unwrap_or_else(|| panic!("builtin row `{field_name}` entries must be strings"))
        .to_string()
    })
    .collect()
}

fn effect_field(row: &Value, field_name: &str) -> Effect {
  match string_field(row, field_name).as_str() {
    "pure" | "Pure" => Effect::Pure,
    "world" | "World" => Effect::World,
    other => panic!("unsupported builtin effect `{other}`"),
  }
}

fn arity_field(row: &Value, field_name: &str) -> Option<usize> {
  let value =
    attr_set_field(row, field_name).unwrap_or_else(|| panic!("builtin row missing `{field_name}`"));
  match value["kind"].as_str() {
    Some("null") => None,
    Some("int") => {
      let raw = value["value"]
        .as_i64()
        .unwrap_or_else(|| panic!("builtin row `{field_name}` int must be numeric"));
      if raw < 0 {
        panic!("builtin row `{field_name}` must not be negative");
      }
      Some(raw as usize)
    }
    _ => panic!("builtin row `{field_name}` must be null or int"),
  }
}

impl Default for BuiltinCatalog {
  fn default() -> Self {
    Self::with_defaults()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_builtin_catalog_creation() {
    let catalog = BuiltinCatalog::new();
    assert!(catalog.functions.is_empty());
  }

  #[test]
  fn test_builtin_catalog_with_defaults() {
    let catalog = BuiltinCatalog::with_defaults();
    assert!(catalog.contains("add"));
    assert!(catalog.contains("sin"));
    assert!(catalog.contains("eq"));
    assert!(catalog.contains("ontologyLift"));
    assert!(catalog.contains("ontologySelect"));
    assert!(catalog.contains("head"));
    assert!(catalog.contains("getAttr"));
  }

  #[test]
  fn supplemental_builtin_rows_are_px_declared() {
    let rows = px_declared_builtin_extensions();
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(
      names,
      vec!["head", "tail", "length", "elemAt", "elem", "list", "lambda", "getAttr", "hasAttr"]
    );
    assert_eq!(rows[0].arity, Some(1));
    assert_eq!(rows[5].arity, None);
    assert_eq!(rows[8].effect, Effect::Pure);
  }

  #[test]
  fn test_builtin_catalog_get() {
    let catalog = BuiltinCatalog::with_defaults();
    let add = catalog.get("add").unwrap();
    assert_eq!(add.name, "add");
    assert_eq!(add.signature, "Num → Num → Num");
    assert_eq!(add.effect, Effect::Pure);
  }

  #[test]
  fn test_builtin_catalog_serialization() {
    let catalog = BuiltinCatalog::with_defaults();
    let json = serde_json::to_string(&catalog).unwrap();
    let deserialized: BuiltinCatalog = serde_json::from_str(&json).unwrap();
    assert_eq!(catalog.functions.len(), deserialized.functions.len());
  }

  #[test]
  fn test_resolve_builtin_name_explicit_forms() {
    assert_eq!(
      resolve_builtin_name("builtins.concat").as_deref(),
      Some("concat")
    );
    assert_eq!(
      resolve_builtin_name("  builtins.concat  ").as_deref(),
      Some("concat")
    );
    assert_eq!(
      resolve_builtin_name("String.length").as_deref(),
      Some("stringLength")
    );
    assert_eq!(
      resolve_builtin_name("  String.length ").as_deref(),
      Some("stringLength")
    );
    assert_eq!(
      resolve_builtin_name("builtins.String.length").as_deref(),
      Some("stringLength")
    );
    assert_eq!(
      resolve_builtin_name("builtins.Process.spawn").as_deref(),
      Some("processSpawn")
    );
    assert_eq!(
      resolve_builtin_name("ontology.lift").as_deref(),
      Some("ontologyLift")
    );
    assert_eq!(
      resolve_builtin_name("builtins.ontology.lift").as_deref(),
      Some("ontologyLift")
    );
    assert!(resolve_builtin_name("builtins..add").is_none());
    assert!(resolve_builtin_name("builtins.add.").is_none());
    assert!(resolve_builtin_name("builtins.process..spawn").is_none());
    assert!(resolve_builtin_name("py.numpy.add").is_none());
  }

  #[test]
  fn test_normalize_builtin_name() {
    assert_eq!(normalize_builtin_name("builtins.add").as_ref(), "add");
    assert_eq!(normalize_builtin_name("  builtins.add ").as_ref(), "add");
    assert_eq!(
      normalize_builtin_name("String.length").as_ref(),
      "stringLength"
    );
    assert_eq!(
      normalize_builtin_name("unknown.name").as_ref(),
      "unknown.name"
    );
    assert_eq!(
      normalize_builtin_name("ontology.select").as_ref(),
      "ontologySelect"
    );
  }

  #[test]
  fn test_resolve_spec_builtin_name() {
    let catalog = BuiltinCatalog::with_defaults();
    assert_eq!(
      resolve_spec_builtin_name("add", &catalog).as_deref(),
      Some("add")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.add", &catalog).as_deref(),
      Some("add")
    );
    assert_eq!(
      resolve_spec_builtin_name("String.length", &catalog).as_deref(),
      Some("stringLength")
    );
    assert_eq!(
      resolve_spec_builtin_name("fx_add", &catalog).as_deref(),
      Some("add")
    );
    assert_eq!(
      resolve_spec_builtin_name("Process.spawn", &catalog).as_deref(),
      Some("processSpawn")
    );
    assert_eq!(
      resolve_spec_builtin_name("process.spawn", &catalog).as_deref(),
      Some("processSpawn")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.processSpawn", &catalog).as_deref(),
      Some("processSpawn")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.Process.spawn", &catalog).as_deref(),
      Some("processSpawn")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.process.spawn", &catalog).as_deref(),
      Some("processSpawn")
    );
    assert_eq!(
      resolve_spec_builtin_name("Process.logsTail", &catalog).as_deref(),
      Some("processLogsTail")
    );
    assert_eq!(
      resolve_spec_builtin_name("Process.observeSample", &catalog).as_deref(),
      Some("processObserveSample")
    );
    assert_eq!(
      resolve_spec_builtin_name("Process.observeSampleById", &catalog).as_deref(),
      Some("processObserveSampleById")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.process.logs.tail", &catalog).as_deref(),
      Some("processLogsTail")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.process.observe.sample", &catalog).as_deref(),
      Some("processObserveSample")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.process.observe.sample.by_id", &catalog).as_deref(),
      Some("processObserveSampleById")
    );
    assert_eq!(
      resolve_spec_builtin_name("Process.terminate", &catalog).as_deref(),
      Some("processTerminate")
    );
    assert_eq!(
      resolve_spec_builtin_name("Runtime.ensure", &catalog).as_deref(),
      Some("processEnsure")
    );
    assert_eq!(
      resolve_spec_builtin_name("runtime.ensure", &catalog).as_deref(),
      Some("processEnsure")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.runtime.ensure", &catalog).as_deref(),
      Some("processEnsure")
    );
    assert_eq!(
      resolve_spec_builtin_name("Runtime.call", &catalog).as_deref(),
      Some("runtimeCall")
    );
    assert_eq!(
      resolve_spec_builtin_name("runtime.call", &catalog).as_deref(),
      Some("runtimeCall")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.runtime.call", &catalog).as_deref(),
      Some("runtimeCall")
    );
    assert_eq!(
      resolve_spec_builtin_name("Vm.call", &catalog).as_deref(),
      Some("runtimeCall")
    );
    assert_eq!(
      resolve_spec_builtin_name("vm.call", &catalog).as_deref(),
      Some("runtimeCall")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.Vm.call", &catalog).as_deref(),
      Some("runtimeCall")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.vm.call", &catalog).as_deref(),
      Some("runtimeCall")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.process.terminate", &catalog).as_deref(),
      Some("processTerminate")
    );
    assert_eq!(
      resolve_spec_builtin_name("  builtins.process.terminate ", &catalog).as_deref(),
      Some("processTerminate")
    );
    assert_eq!(
      resolve_spec_builtin_name("  add  ", &catalog).as_deref(),
      Some("add")
    );
    assert_eq!(
      resolve_spec_builtin_name("ontology.lift", &catalog).as_deref(),
      Some("ontologyLift")
    );
    assert_eq!(
      resolve_spec_builtin_name("builtins.ontology.lift", &catalog).as_deref(),
      Some("ontologyLift")
    );
    assert_eq!(
      resolve_spec_builtin_name("ontology.select", &catalog).as_deref(),
      Some("ontologySelect")
    );
    assert_eq!(
      resolve_spec_builtin_name("ontology.promote", &catalog).as_deref(),
      Some("ontologyPromote")
    );
    assert!(resolve_spec_builtin_name("builtins..add", &catalog).is_none());
    assert!(resolve_spec_builtin_name("builtins.add.", &catalog).is_none());
    assert!(resolve_spec_builtin_name("builtins.process..spawn", &catalog).is_none());
    assert!(resolve_spec_builtin_name("builtins.not_in_catalog", &catalog).is_none());
  }
}
