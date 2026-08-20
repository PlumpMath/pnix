//! Parametric 정책: 파라미터 표현식에서 허용할 함수 호출 정책

use serde::{Deserialize, Serialize};

/// 호출 스펙: 함수 이름 및 인자 개수
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSpec {
  /// 함수 이름
  pub name: String,
  /// 인자 개수 (arity, 함수가 받는 인자 수)
  pub arity: usize,
}

/// 호출 정책: 파라미터 표현식에서 허용할 함수 호출 목록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPolicy {
  /// 허용할 호출 스펙 목록 (화이트리스트)
  pub calls: Vec<CallSpec>,
}

impl CallPolicy {
  pub fn default_allowlist() -> Self {
    Self {
      calls: vec![
        CallSpec {
          name: "sin".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "cos".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "tan".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "sqrt".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "abs".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "exp".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "ln".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "floor".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "ceil".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "pow".to_string(),
          arity: 2,
        },
        CallSpec {
          name: "min".to_string(),
          arity: 2,
        },
        CallSpec {
          name: "max".to_string(),
          arity: 2,
        },
      ],
    }
  }

  pub fn expected_arity(&self, name: &str) -> Option<usize> {
    self.calls.iter().find(|c| c.name == name).map(|c| c.arity)
  }
}
