//! # OWNER-LAW NOTE (2026-05-10)
//!
//! **pnix 는 LLM 없이 작동하는 self-sufficient deterministic AI 지능이다.**
//! 정본 헌법은 `CLAUDE.md` 의 `OWNER-LAW CONSTITUTION` section.
//!
//! 이 file 의 doc/comment 에 "LLM as X" / "LLM 이 ..." / "LLM 모드 / 연동 /
//! 호출 / 정제" 류 framing 이 남아있어도 모두 owner-law 위반으로
//! *superseded* 다. substrate 안에 LLM 자리 *없음*. 외부 LLM output 은
//! untrusted external prose 일 뿐 substrate enumeration category 가
//! 아님. 본문은 legacy framing 의 history retention.
//!
//! 설정 스키마 문서화 모듈
//!
//! pnix-old의 pnix_config/src/schema.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1, C1)
//!
//! 구조 정의 및 텍스트 생성만, I/O 없음
//!
//! ## 기능
//!
//! - ConfigSchema, FieldInfo, SectionInfo 구조 정의
//! - Markdown, JSON Schema, HTML 문서 생성

use serde::{Deserialize, Serialize};

/// 설정 필드 정보: 설정 필드의 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
  /// 필드 이름 (설정 필드의 이름)
  pub name: String,
  /// 필드 타입 (필드의 타입 문자열)
  pub field_type: String,
  /// 설명 (필드의 설명)
  pub description: String,
  /// 기본값 (필드의 기본값, 선택적)
  pub default_value: Option<String>,
  /// 필수 여부 (필드가 필수인지 여부)
  pub required: bool,
  /// 예시 값 (필드의 예시 값, 선택적)
  pub example: Option<String>,
}

/// 설정 스키마: 설정 파일의 스키마 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
  /// 스키마 버전 (스키마 버전 문자열)
  pub version: String,
  /// 최상위 섹션들 (설정 섹션 목록)
  pub sections: Vec<SectionInfo>,
}

/// 섹션 정보: 설정 섹션의 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionInfo {
  /// 섹션 이름 (섹션의 이름)
  pub name: String,
  /// 설명 (섹션의 설명)
  pub description: String,
  /// 필드 목록 (섹션에 포함된 필드 목록)
  pub fields: Vec<FieldInfo>,
}

/// 스키마 문서 생성기
pub struct SchemaDocumenter;

impl SchemaDocumenter {
  /// 설정 스키마 생성
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 구조 생성만, 파일 I/O 없음
  #[allow(clippy::vec_init_then_push)]
  pub fn generate_schema() -> ConfigSchema {
    let mut sections = Vec::new();

    // REPL 섹션
    sections.push(SectionInfo {
      name: "repl".to_string(),
      description: "REPL (Read-Eval-Print Loop) 설정".to_string(),
      fields: vec![
        FieldInfo {
          name: "default_mode".to_string(),
          field_type: "enum (programming, llm, symbolic)".to_string(),
          description: "기본 REPL 모드".to_string(),
          default_value: Some("programming".to_string()),
          required: false,
          example: Some("programming".to_string()),
        },
        FieldInfo {
          name: "prompt_style".to_string(),
          field_type: "enum (minimal, full, custom)".to_string(),
          description: "프롬프트 스타일".to_string(),
          default_value: Some("minimal".to_string()),
          required: false,
          example: Some("minimal".to_string()),
        },
        FieldInfo {
          name: "custom_prompt".to_string(),
          field_type: "string (optional)".to_string(),
          description: "커스텀 프롬프트 (prompt_style이 custom일 때)".to_string(),
          default_value: None,
          required: false,
          example: Some("\"pnix> \"".to_string()),
        },
        FieldInfo {
          name: "history_file".to_string(),
          field_type: "string (optional)".to_string(),
          description: "히스토리 파일 경로".to_string(),
          default_value: None,
          required: false,
          example: Some("~/.pnix_history".to_string()),
        },
        FieldInfo {
          name: "max_history".to_string(),
          field_type: "integer".to_string(),
          description: "최대 히스토리 항목 수".to_string(),
          default_value: Some("1000".to_string()),
          required: false,
          example: Some("1000".to_string()),
        },
        FieldInfo {
          name: "mode_toggle_key".to_string(),
          field_type: "string".to_string(),
          description: "모드 전환 키".to_string(),
          default_value: Some("F2".to_string()),
          required: false,
          example: Some("F2".to_string()),
        },
      ],
    });

    // LLM 섹션
    sections.push(SectionInfo {
      name: "llm".to_string(),
      description: "LLM (Large Language Model) 설정".to_string(),
      fields: vec![
        FieldInfo {
          name: "provider".to_string(),
          field_type: "enum (claude, openai, local, custom)".to_string(),
          description: "LLM 제공자".to_string(),
          default_value: Some("claude".to_string()),
          required: false,
          example: Some("claude".to_string()),
        },
        FieldInfo {
          name: "api_key_env".to_string(),
          field_type: "string".to_string(),
          description: "API 키 환경 변수 이름".to_string(),
          default_value: Some("ANTHROPIC_API_KEY".to_string()),
          required: false,
          example: Some("ANTHROPIC_API_KEY".to_string()),
        },
        FieldInfo {
          name: "model".to_string(),
          field_type: "string".to_string(),
          description: "모델 이름".to_string(),
          default_value: Some("claude-3-5-sonnet-20241022".to_string()),
          required: false,
          example: Some("claude-3-5-sonnet-20241022".to_string()),
        },
        FieldInfo {
          name: "max_tokens".to_string(),
          field_type: "integer".to_string(),
          description: "최대 토큰 수".to_string(),
          default_value: Some("4096".to_string()),
          required: false,
          example: Some("4096".to_string()),
        },
        FieldInfo {
          name: "temperature".to_string(),
          field_type: "float".to_string(),
          description: "Temperature (0.0 ~ 1.0)".to_string(),
          default_value: Some("0.7".to_string()),
          required: false,
          example: Some("0.7".to_string()),
        },
        FieldInfo {
          name: "system_prompt".to_string(),
          field_type: "string (optional)".to_string(),
          description: "시스템 프롬프트".to_string(),
          default_value: None,
          required: false,
          example: Some("\"You are a helpful assistant.\"".to_string()),
        },
        FieldInfo {
          name: "endpoint".to_string(),
          field_type: "string (optional)".to_string(),
          description: "API 엔드포인트 (Custom provider용)".to_string(),
          default_value: None,
          required: false,
          example: Some("\"https://api.example.com\"".to_string()),
        },
      ],
    });

    // Symbolic 섹션
    sections.push(SectionInfo {
      name: "symbolic".to_string(),
      description: "Symbolic 모드 설정".to_string(),
      fields: vec![
        FieldInfo {
          name: "auto_simplify".to_string(),
          field_type: "boolean".to_string(),
          description: "자동 단순화".to_string(),
          default_value: Some("true".to_string()),
          required: false,
          example: Some("true".to_string()),
        },
        FieldInfo {
          name: "show_steps".to_string(),
          field_type: "boolean".to_string(),
          description: "단계별 표시".to_string(),
          default_value: Some("true".to_string()),
          required: false,
          example: Some("true".to_string()),
        },
        FieldInfo {
          name: "latex_output".to_string(),
          field_type: "boolean".to_string(),
          description: "LaTeX 출력".to_string(),
          default_value: Some("false".to_string()),
          required: false,
          example: Some("false".to_string()),
        },
        FieldInfo {
          name: "confidence_threshold".to_string(),
          field_type: "float".to_string(),
          description: "NL 파서 신뢰도 임계값 (0.0 ~ 1.0)".to_string(),
          default_value: Some("0.5".to_string()),
          required: false,
          example: Some("0.5".to_string()),
        },
        FieldInfo {
          name: "default_variable".to_string(),
          field_type: "string".to_string(),
          description: "기본 변수 (미분 등에서 사용)".to_string(),
          default_value: Some("x".to_string()),
          required: false,
          example: Some("x".to_string()),
        },
      ],
    });

    ConfigSchema {
      version: "1.1".to_string(),
      sections,
    }
  }

  /// Markdown 문서 생성
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn generate_markdown(schema: &ConfigSchema) -> String {
    let mut md = String::new();

    md.push_str("# Pnix 설정 스키마\n\n");
    md.push_str(&format!("버전: {}\n\n", schema.version));
    md.push_str("## 개요\n\n");
    md.push_str("Pnix 설정 파일은 YAML 형식으로 작성됩니다.\n\n");

    for section in &schema.sections {
      md.push_str(&format!("## {}\n\n", section.name));
      md.push_str(&format!("{}\n\n", section.description));
      md.push_str("| 필드 | 타입 | 설명 | 기본값 | 필수 |\n");
      md.push_str("|------|------|------|--------|------|\n");

      for field in &section.fields {
        let default = field.default_value.as_deref().unwrap_or("-");
        let required = if field.required { "예" } else { "아니오" };

        md.push_str(&format!(
          "| `{}` | {} | {} | {} | {} |\n",
          field.name, field.field_type, field.description, default, required
        ));
      }

      md.push('\n');

      // 예시 추가
      if let Some(example_field) = section.fields.iter().find(|f| f.example.is_some()) {
        md.push_str("### 예시\n\n");
        md.push_str("```yaml\n");
        md.push_str(&format!("{}:\n", section.name));
        if let Some(example) = &example_field.example {
          md.push_str(&format!("  {}: {}\n", example_field.name, example));
        }
        md.push_str("```\n\n");
      }
    }

    md
  }

  /// JSON Schema 생성
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn generate_json_schema(schema: &ConfigSchema) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"$schema\": \"http://json-schema.org/draft-07/schema#\",\n");
    json.push_str(&format!("  \"version\": \"{}\",\n", schema.version));
    json.push_str("  \"type\": \"object\",\n");
    json.push_str("  \"properties\": {\n");

    for (i, section) in schema.sections.iter().enumerate() {
      json.push_str(&format!("    \"{}\": {{\n", section.name));
      json.push_str("      \"type\": \"object\",\n");
      json.push_str("      \"description\": \"");
      json.push_str(&section.description.replace('"', "\\\""));
      json.push_str("\",\n");
      json.push_str("      \"properties\": {\n");

      for (j, field) in section.fields.iter().enumerate() {
        json.push_str(&format!("        \"{}\": {{\n", field.name));
        json.push_str(&format!(
          "          \"type\": \"{}\",\n",
          Self::json_type(&field.field_type)
        ));
        json.push_str("          \"description\": \"");
        json.push_str(&field.description.replace('"', "\\\""));
        json.push('"');

        if let Some(default) = &field.default_value {
          json.push_str(",\n          \"default\": ");
          if field.field_type.contains("boolean")
            || field.field_type.contains("integer")
            || field.field_type.contains("float")
          {
            json.push_str(default);
          } else {
            json.push_str(&format!("\"{}\"", default));
          }
        }

        if field.required {
          json.push_str(",\n          \"required\": true");
        }

        json.push_str("\n        }");
        if j < section.fields.len() - 1 {
          json.push(',');
        }
        json.push('\n');
      }

      json.push_str("      }\n");
      json.push_str("    }");
      if i < schema.sections.len() - 1 {
        json.push(',');
      }
      json.push('\n');
    }

    json.push_str("  }\n");
    json.push_str("}\n");

    json
  }

  /// JSON 타입 변환
  fn json_type(field_type: &str) -> &str {
    if field_type.contains("string") {
      "string"
    } else if field_type.contains("integer") {
      "integer"
    } else if field_type.contains("float") {
      "number"
    } else if field_type.contains("boolean") {
      "boolean"
    } else {
      "string"
    }
  }

  /// HTML 문서 생성
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn generate_html(schema: &ConfigSchema) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("  <title>Pnix 설정 스키마</title>\n");
    html.push_str("  <style>\n");
    html.push_str("    body { font-family: sans-serif; margin: 20px; }\n");
    html.push_str("    table { border-collapse: collapse; width: 100%; margin: 20px 0; }\n");
    html.push_str("    th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
    html.push_str("    th { background-color: #f2f2f2; }\n");
    html.push_str("    code { background-color: #f4f4f4; padding: 2px 4px; }\n");
    html.push_str("  </style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("  <h1>Pnix 설정 스키마</h1>\n");
    html.push_str(&format!("  <p>버전: {}</p>\n", schema.version));

    for section in &schema.sections {
      html.push_str(&format!("  <h2>{}</h2>\n", section.name));
      html.push_str(&format!("  <p>{}</p>\n", section.description));
      html.push_str("  <table>\n");
      html.push_str("    <thead>\n");
      html.push_str("      <tr>\n");
      html.push_str("        <th>필드</th>\n");
      html.push_str("        <th>타입</th>\n");
      html.push_str("        <th>설명</th>\n");
      html.push_str("        <th>기본값</th>\n");
      html.push_str("        <th>필수</th>\n");
      html.push_str("      </tr>\n");
      html.push_str("    </thead>\n");
      html.push_str("    <tbody>\n");

      for field in &section.fields {
        let default = field.default_value.as_deref().unwrap_or("-");
        let required = if field.required { "예" } else { "아니오" };

        html.push_str("      <tr>\n");
        html.push_str(&format!("        <td><code>{}</code></td>\n", field.name));
        html.push_str(&format!("        <td>{}</td>\n", field.field_type));
        html.push_str(&format!("        <td>{}</td>\n", field.description));
        html.push_str(&format!("        <td>{}</td>\n", default));
        html.push_str(&format!("        <td>{}</td>\n", required));
        html.push_str("      </tr>\n");
      }

      html.push_str("    </tbody>\n");
      html.push_str("  </table>\n");
    }

    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate_schema() {
    let schema = SchemaDocumenter::generate_schema();
    assert_eq!(schema.version, "1.1");
    assert_eq!(schema.sections.len(), 3);

    let repl = &schema.sections[0];
    assert_eq!(repl.name, "repl");
    assert!(!repl.fields.is_empty());
  }

  #[test]
  fn test_generate_markdown() {
    let schema = SchemaDocumenter::generate_schema();
    let md = SchemaDocumenter::generate_markdown(&schema);

    assert!(md.contains("# Pnix 설정 스키마"));
    assert!(md.contains("## repl"));
    assert!(md.contains("## llm"));
    assert!(md.contains("## symbolic"));
  }

  #[test]
  fn test_generate_json_schema() {
    let schema = SchemaDocumenter::generate_schema();
    let json = SchemaDocumenter::generate_json_schema(&schema);

    assert!(json.contains("$schema"));
    assert!(json.contains("\"repl\""));
    assert!(json.contains("\"llm\""));
  }

  #[test]
  fn test_generate_html() {
    let schema = SchemaDocumenter::generate_schema();
    let html = SchemaDocumenter::generate_html(&schema);

    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<h1>Pnix 설정 스키마</h1>"));
  }

  #[test]
  fn test_json_type() {
    assert_eq!(SchemaDocumenter::json_type("string"), "string");
    assert_eq!(SchemaDocumenter::json_type("integer"), "integer");
    assert_eq!(SchemaDocumenter::json_type("float"), "number");
    assert_eq!(SchemaDocumenter::json_type("boolean"), "boolean");
    assert_eq!(SchemaDocumenter::json_type("enum (a, b)"), "string");
  }
}
