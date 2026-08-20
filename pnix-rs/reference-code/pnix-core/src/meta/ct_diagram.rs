//! CTDiagram: Category Theory 다이어그램
//!
//! pnix-old의 ct_diagram.rs를 pnix-new 패러다임에 맞게 적응.
//!
//! ## 변경점
//!
//! - pnix-old: FrpRuntime/SSA에서 생성
//! - pnix-new: FxCoreModule에서 생성
//!
//! ## 출력 포맷
//!
//! - DOT (GraphViz)
//! - Mermaid (통합)
//! - JSON

use serde::{Deserialize, Serialize};

/// CT Object (객체)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CTObject {
  pub id: usize,
  pub name: String,
  pub ct_type: CTType,
}

/// CT Type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CTType {
  /// 실수
  Real,
  /// 정수
  Int,
  /// 불리언
  Bool,
  /// 문자열
  String,
  /// Unit type
  Unit,
  /// Product type
  Product(Box<CTType>, Box<CTType>),
  /// Named type (DSL에서 선언)
  Named(std::string::String),
  /// 알 수 없음
  #[default]
  Unknown,
}

impl CTType {
  /// 타입 문자열에서 파싱
  pub fn parse(s: &str) -> Self {
    let s = s.trim();
    match s {
      "" | "()" | "Unit" => CTType::Unit,
      "Int" | "Integer" => CTType::Int,
      "Real" | "Float" | "Double" => CTType::Real,
      "Bool" | "Boolean" => CTType::Bool,
      "String" => CTType::String,
      _ => CTType::Named(s.to_string()),
    }
  }

  /// CT 기호로 변환
  pub fn symbol(&self) -> &str {
    match self {
      CTType::Real => "ℝ",
      CTType::Int => "ℤ",
      CTType::Bool => "𝔹",
      CTType::String => "𝕊",
      CTType::Unit => "1",
      CTType::Product(_, _) => "×",
      CTType::Named(_) => "•",
      CTType::Unknown => "?",
    }
  }
}

/// CT Morphism (사상)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CTMorphism {
  pub id: usize,
  pub name: String,
  pub source: usize, // CTObject id
  pub target: usize, // CTObject id
  /// Effect (pure/world)
  pub effect: String,
}

/// CT Diagram
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CTDiagram {
  pub objects: Vec<CTObject>,
  pub morphisms: Vec<CTMorphism>,
}

impl CTDiagram {
  pub fn new() -> Self {
    Self::default()
  }

  /// Object 추가
  pub fn add_object(&mut self, name: impl Into<String>, ct_type: CTType) -> usize {
    let id = self.objects.len();
    self.objects.push(CTObject {
      id,
      name: name.into(),
      ct_type,
    });
    id
  }

  /// Morphism 추가
  pub fn add_morphism(
    &mut self,
    name: impl Into<String>,
    source: usize,
    target: usize,
    effect: impl Into<String>,
  ) -> usize {
    let id = self.morphisms.len();
    self.morphisms.push(CTMorphism {
      id,
      name: name.into(),
      source,
      target,
      effect: effect.into(),
    });
    id
  }

  /// Graphviz DOT 포맷 출력
  ///
  /// 결정성 보장: objects와 morphisms를 정렬하여 출력 순서 일관성 유지
  pub fn to_dot(&self) -> String {
    let mut dot = String::from("digraph CT {\n");
    dot.push_str("  rankdir=LR;\n");
    dot.push_str("  node [shape=ellipse];\n\n");

    // Objects: id 기준 정렬하여 결정성 보장
    let mut sorted_objects: Vec<_> = self.objects.iter().collect();
    sorted_objects.sort_by(|a, b| a.id.cmp(&b.id));

    for obj in sorted_objects {
      let type_str = obj.ct_type.symbol();
      dot.push_str(&format!(
        "  obj{} [label=\"{}: {}\"];\n",
        obj.id, obj.name, type_str
      ));
    }
    dot.push('\n');

    // Morphisms: (source, target, name) 기준 정렬하여 결정성 보장
    let mut sorted_morphisms: Vec<_> = self.morphisms.iter().collect();
    sorted_morphisms.sort_by(|a, b| {
      a.source
        .cmp(&b.source)
        .then_with(|| a.target.cmp(&b.target))
        .then_with(|| a.name.cmp(&b.name))
    });

    for morph in sorted_morphisms {
      let style = if morph.effect == "world" {
        ", color=red"
      } else {
        ""
      };
      dot.push_str(&format!(
        "  obj{} -> obj{} [label=\"{}\"{}];\n",
        morph.source, morph.target, morph.name, style
      ));
    }

    dot.push_str("}\n");
    dot
  }

  /// Mermaid 포맷 출력
  ///
  /// 결정성 보장: objects와 morphisms를 정렬하여 출력 순서 일관성 유지
  pub fn to_mermaid(&self) -> String {
    let mut out = String::from("graph LR\n");

    // Objects as nodes: id 기준 정렬하여 결정성 보장
    let mut sorted_objects: Vec<_> = self.objects.iter().collect();
    sorted_objects.sort_by(|a, b| a.id.cmp(&b.id));

    for obj in sorted_objects {
      let type_str = obj.ct_type.symbol();
      out.push_str(&format!(
        "    {}[\"{}\\n{}\"]",
        escape_mermaid_id(&obj.name),
        obj.name,
        type_str
      ));

      // Style based on type
      match &obj.ct_type {
        CTType::Named(_) => out.push_str(":::namedType"),
        CTType::Unit => out.push_str(":::unitType"),
        _ => {}
      }
      out.push('\n');
    }

    out.push('\n');

    // Morphisms as edges: (source, target, name) 기준 정렬하여 결정성 보장
    let mut sorted_morphisms: Vec<_> = self.morphisms.iter().collect();
    sorted_morphisms.sort_by(|a, b| {
      a.source
        .cmp(&b.source)
        .then_with(|| a.target.cmp(&b.target))
        .then_with(|| a.name.cmp(&b.name))
    });

    for morph in sorted_morphisms {
      if morph.source >= self.objects.len() || morph.target >= self.objects.len() {
        continue;
      }

      let from_name = &self.objects[morph.source].name;
      let to_name = &self.objects[morph.target].name;

      let arrow = if morph.effect == "world" {
        "==>" // World effect: thick arrow
      } else {
        "-->" // Pure: normal arrow
      };

      out.push_str(&format!(
        "    {} {}|{}| {}\n",
        escape_mermaid_id(from_name),
        arrow,
        morph.name,
        escape_mermaid_id(to_name)
      ));
    }

    // Style definitions
    out.push('\n');
    out.push_str("    classDef namedType fill:#e1f5fe,stroke:#01579b\n");
    out.push_str("    classDef unitType fill:#f5f5f5,stroke:#9e9e9e\n");

    out
  }

  /// JSON 출력
  ///
  /// 결정성 보장: objects와 morphisms를 정렬하여 출력 순서 일관성 유지
  pub fn to_json(&self) -> String {
    // 정렬된 복사본 생성하여 직렬화
    let mut sorted_objects: Vec<_> = self.objects.clone();
    sorted_objects.sort_by(|a, b| a.id.cmp(&b.id));

    let mut sorted_morphisms: Vec<_> = self.morphisms.clone();
    sorted_morphisms.sort_by(|a, b| {
      a.source
        .cmp(&b.source)
        .then_with(|| a.target.cmp(&b.target))
        .then_with(|| a.name.cmp(&b.name))
    });

    let sorted_diagram = CTDiagram {
      objects: sorted_objects,
      morphisms: sorted_morphisms,
    };

    serde_json::to_string_pretty(&sorted_diagram).unwrap_or_default()
  }

  /// 통계 정보
  pub fn stats(&self) -> CTStats {
    CTStats {
      num_objects: self.objects.len(),
      num_morphisms: self.morphisms.len(),
    }
  }

  /// Dead signal elimination
  pub fn eliminate_dead_signals(&mut self, exported: &[usize]) {
    if exported.is_empty() {
      return;
    }

    // Find reachable objects by traversing backwards
    let mut reachable = std::collections::HashSet::new();
    let mut worklist: Vec<usize> = exported.to_vec();

    while let Some(obj_id) = worklist.pop() {
      if reachable.contains(&obj_id) {
        continue;
      }
      reachable.insert(obj_id);

      // Add sources of morphisms targeting this object
      for morph in &self.morphisms {
        if morph.target == obj_id && !reachable.contains(&morph.source) {
          worklist.push(morph.source);
        }
      }
    }

    // Build old->new id mapping
    let old_to_new: std::collections::HashMap<usize, usize> = self
      .objects
      .iter()
      .filter(|obj| reachable.contains(&obj.id))
      .enumerate()
      .map(|(new_id, obj)| (obj.id, new_id))
      .collect();

    // Filter and reindex objects
    let new_objects: Vec<CTObject> = self
      .objects
      .iter()
      .filter(|obj| reachable.contains(&obj.id))
      .enumerate()
      .map(|(new_id, obj)| CTObject {
        id: new_id,
        name: obj.name.clone(),
        ct_type: obj.ct_type.clone(),
      })
      .collect();

    // Filter and reindex morphisms
    let new_morphisms: Vec<CTMorphism> = self
      .morphisms
      .iter()
      .filter(|m| reachable.contains(&m.source) && reachable.contains(&m.target))
      .enumerate()
      .map(|(new_id, m)| CTMorphism {
        id: new_id,
        name: m.name.clone(),
        source: *old_to_new.get(&m.source).unwrap_or(&0),
        target: *old_to_new.get(&m.target).unwrap_or(&0),
        effect: m.effect.clone(),
      })
      .collect();

    self.objects = new_objects;
    self.morphisms = new_morphisms;
  }
}

/// CT Diagram 통계
#[derive(Debug, Clone)]
pub struct CTStats {
  pub num_objects: usize,
  pub num_morphisms: usize,
}

/// Mermaid ID 이스케이프
fn escape_mermaid_id(s: &str) -> String {
  s.replace(['.', '-', ' ', ':'], "_")
}

/// FxCoreModule에서 CTDiagram 생성
pub fn build_ct_from_fxcore(module: &crate::core::FxCoreModule) -> CTDiagram {
  let mut diag = CTDiagram::new();
  let mut node_to_obj: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

  // 1. Nodes → Objects
  for node in &module.nodes {
    // Find morphism to get type
    let morphism = module.morphisms.iter().find(|m| m.name == node.uses);

    let ct_type = if let Some(m) = morphism {
      CTType::parse(&m.output)
    } else {
      CTType::Unknown
    };

    let obj_id = diag.add_object(&node.name, ct_type);
    node_to_obj.insert(node.name.clone(), obj_id);
  }

  // 2. Inputs → Objects
  for input in &module.inputs {
    let ct_type = CTType::parse(&input.ty);
    let obj_id = diag.add_object(&input.name, ct_type);
    node_to_obj.insert(input.name.clone(), obj_id);
  }

  // 3. Edges → Morphisms
  for edge in &module.edges {
    let source = if let Some(input_name) = &edge.from_input {
      node_to_obj.get(input_name).copied()
    } else {
      node_to_obj.get(&edge.from).copied()
    };

    let target = node_to_obj.get(&edge.to).copied();

    if let (Some(src), Some(tgt)) = (source, target) {
      // Get effect from target node's morphism
      let target_node = module.nodes.iter().find(|n| n.name == edge.to);
      let effect = if let Some(node) = target_node {
        let morphism = module.morphisms.iter().find(|m| m.name == node.uses);
        morphism
          .map(|m| format!("{:?}", m.effect).to_lowercase())
          .unwrap_or_else(|| "pure".to_string())
      } else {
        "pure".to_string()
      };

      // Edge label: port info or just arrow
      let label = match (&edge.from_port, &edge.to_port) {
        (Some(fp), Some(tp)) => format!("{} → {}", fp, tp),
        (Some(fp), None) => fp.clone(),
        (None, Some(tp)) => tp.clone(),
        (None, None) => "→".to_string(),
      };

      diag.add_morphism(label, src, tgt, effect);
    }
  }

  diag
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ct_diagram_basic() {
    let mut diag = CTDiagram::new();
    let a = diag.add_object("A", CTType::Real);
    let b = diag.add_object("B", CTType::Real);
    diag.add_morphism("f", a, b, "pure");

    assert_eq!(diag.objects.len(), 2);
    assert_eq!(diag.morphisms.len(), 1);
  }

  #[test]
  fn test_ct_diagram_to_dot() {
    let mut diag = CTDiagram::new();
    let a = diag.add_object("X", CTType::Int);
    let b = diag.add_object("Y", CTType::Real);
    diag.add_morphism("convert", a, b, "pure");

    let dot = diag.to_dot();

    assert!(dot.contains("digraph CT"));
    assert!(dot.contains("X: ℤ"));
    assert!(dot.contains("Y: ℝ"));
    assert!(dot.contains("convert"));
  }

  #[test]
  fn test_ct_diagram_to_mermaid() {
    let mut diag = CTDiagram::new();
    let a = diag.add_object("Position", CTType::Named("Position".into()));
    let b = diag.add_object("Velocity", CTType::Named("Velocity".into()));
    diag.add_morphism("differentiate", a, b, "pure");

    let mermaid = diag.to_mermaid();

    assert!(mermaid.contains("graph LR"));
    assert!(mermaid.contains("Position"));
    assert!(mermaid.contains("Velocity"));
    assert!(mermaid.contains("differentiate"));
  }

  #[test]
  fn test_ct_diagram_world_effect() {
    let mut diag = CTDiagram::new();
    let a = diag.add_object("Input", CTType::String);
    let b = diag.add_object("Output", CTType::String);
    diag.add_morphism("io", a, b, "world");

    let dot = diag.to_dot();
    assert!(dot.contains("color=red"));

    let mermaid = diag.to_mermaid();
    assert!(mermaid.contains("==>"));
  }

  #[test]
  fn test_ct_diagram_to_json() {
    let mut diag = CTDiagram::new();
    diag.add_object("test", CTType::Int);
    let json = diag.to_json();

    assert!(json.contains("\"name\": \"test\""));
  }

  #[test]
  fn test_dead_signal_elimination() {
    let mut diag = CTDiagram::new();

    // A → B → C (exported)
    //       ↘ D (dead)
    let a = diag.add_object("A", CTType::Real);
    let b = diag.add_object("B", CTType::Real);
    let c = diag.add_object("C", CTType::Real);
    let d = diag.add_object("D", CTType::Real);

    diag.add_morphism("f", a, b, "pure");
    diag.add_morphism("g", b, c, "pure");
    diag.add_morphism("h", b, d, "pure");

    let before_stats = diag.stats();
    assert_eq!(before_stats.num_objects, 4);
    assert_eq!(before_stats.num_morphisms, 3);

    // Only C is exported → D is dead
    diag.eliminate_dead_signals(&[c]);

    let after_stats = diag.stats();
    assert_eq!(after_stats.num_objects, 3, "D should be eliminated");
    assert_eq!(after_stats.num_morphisms, 2, "h should be eliminated");

    let has_d = diag.objects.iter().any(|o| o.name == "D");
    assert!(!has_d, "D should be eliminated");
  }

  #[test]
  fn test_ct_type_parse() {
    assert!(matches!(CTType::parse("Int"), CTType::Int));
    assert!(matches!(CTType::parse("Real"), CTType::Real));
    assert!(matches!(CTType::parse("Bool"), CTType::Bool));
    assert!(matches!(CTType::parse("()"), CTType::Unit));
    assert!(matches!(CTType::parse("Position"), CTType::Named(_)));
  }

  #[test]
  fn test_ct_type_symbol() {
    assert_eq!(CTType::Int.symbol(), "ℤ");
    assert_eq!(CTType::Real.symbol(), "ℝ");
    assert_eq!(CTType::Bool.symbol(), "𝔹");
    assert_eq!(CTType::Unit.symbol(), "1");
  }
}
