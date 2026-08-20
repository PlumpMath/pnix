//! Clean-room XML-family validation/conversion helpers.
//!
//! This module intentionally does not vendor DTD/XSD files, generated schema
//! maps, official examples, conformance suites, or standard prose. It provides
//! a proprietary-safe native baseline:
//!
//! - parse XML strings through `pnix-xml-core`;
//! - validate pnix XML AST structure instead of returning unconditional success;
//! - apply small project-authored family root checks where a wrapper claims a
//!   concrete XML family such as SBML, MathML, COLLADA, or IFCXML;
//! - normalize by returning a validated pnix XML AST.
//!
//! Full domain semantics can be added later as project-owned code. Until then
//! these builtins are honest XML-family validators, not external-schema claims.

use crate::markup::{json_to_value, value_to_json};
use crate::Value;
use anyhow::{anyhow, Result};
use serde_json::{Map as JsonMap, Value as JsonValue};

#[derive(Debug, Clone, Copy)]
struct XmlFamilyProfile {
  id: &'static str,
  root_names: &'static [&'static str],
}

const XML_PROFILE: XmlFamilyProfile = XmlFamilyProfile {
  id: "xml",
  root_names: &[],
};

fn profile_for_builtin(builtin: &str) -> XmlFamilyProfile {
  let lower = builtin.to_ascii_lowercase();
  if lower.contains("cellml") {
    XmlFamilyProfile {
      id: "cellml",
      root_names: &["model"],
    }
  } else if lower.contains("neuroml") {
    XmlFamilyProfile {
      id: "neuroml",
      root_names: &["neuroml", "neuromldocument"],
    }
  } else if lower.contains("pdbml") {
    XmlFamilyProfile {
      id: "pdbml",
      root_names: &["datablock", "pdbx"],
    }
  } else if lower.contains("sbml") {
    XmlFamilyProfile {
      id: "sbml",
      root_names: &["sbml"],
    }
  } else if lower.contains("biopax") {
    XmlFamilyProfile {
      id: "biopax",
      root_names: &["rdf"],
    }
  } else if lower.contains("gifti") {
    XmlFamilyProfile {
      id: "gifti",
      root_names: &["gifti"],
    }
  } else if lower.contains("lems") {
    XmlFamilyProfile {
      id: "lems",
      root_names: &["lems"],
    }
  } else if lower.contains("omex") {
    XmlFamilyProfile {
      id: "omex",
      root_names: &["omexmanifest", "manifest"],
    }
  } else if lower.contains("pharmml") {
    XmlFamilyProfile {
      id: "pharmml",
      root_names: &["pharmml"],
    }
  } else if lower.contains("sbgnml") {
    XmlFamilyProfile {
      id: "sbgnml",
      root_names: &["sbgn"],
    }
  } else if lower.contains("sedml") {
    XmlFamilyProfile {
      id: "sedml",
      root_names: &["sedml"],
    }
  } else if lower.contains("vtk") {
    XmlFamilyProfile {
      id: "vtk",
      root_names: &["vtkfile", "vtk"],
    }
  } else if lower.contains("xdmf") {
    XmlFamilyProfile {
      id: "xdmf",
      root_names: &["xdmf"],
    }
  } else if lower.contains("ifcxml") {
    XmlFamilyProfile {
      id: "ifcxml",
      root_names: &["ifcxml", "iso_10303_28"],
    }
  } else if lower.contains("mathml") {
    XmlFamilyProfile {
      id: "mathml",
      root_names: &["math"],
    }
  } else if lower.contains("openmath") {
    XmlFamilyProfile {
      id: "openmath",
      root_names: &["omobj", "omapp", "ombinding"],
    }
  } else if lower.contains("collada") {
    XmlFamilyProfile {
      id: "collada",
      root_names: &["collada"],
    }
  } else if lower.contains("hanim") {
    XmlFamilyProfile {
      id: "hanim",
      root_names: &[
        "hanimhumanoid",
        "hanimjoint",
        "hanimsegment",
        "hanimsite",
        "x3d",
      ],
    }
  } else if lower.contains("cml") {
    XmlFamilyProfile {
      id: "cml",
      root_names: &["cml", "molecule"],
    }
  } else if lower.contains("program") {
    XmlFamilyProfile {
      id: "program",
      root_names: &["program"],
    }
  } else if lower.contains("excel") {
    XmlFamilyProfile {
      id: "excel-xml",
      root_names: &["workbook", "worksheet"],
    }
  } else if lower.contains("ods") || lower.contains("openformula") {
    XmlFamilyProfile {
      id: "office-xml",
      root_names: &[],
    }
  } else {
    XML_PROFILE
  }
}

pub fn xml_family_normalize(input: &Value, builtin: &str) -> Result<Value> {
  let json = xml_input_to_json(input, builtin)?;
  let errors = xml_validation_errors(&json, profile_for_builtin(builtin));
  if !errors.is_empty() {
    return Err(anyhow!("{builtin}: {}", errors.join("; ")));
  }
  let normalized =
    pnix_xml_core::xml_normalize_json(&json, None).map_err(|err| anyhow!("{builtin}: {err}"))?;
  Ok(json_to_value(&normalized))
}

pub fn xml_family_validate(input: &Value, builtin: &str) -> Result<Value> {
  let profile = profile_for_builtin(builtin);
  let errors = match xml_input_to_json(input, builtin) {
    Ok(json) => xml_validation_errors(&json, profile),
    Err(err) => vec![err.to_string()],
  };
  Ok(json_to_value(&validation_report(profile, errors)))
}

pub fn xml_family_explain(input: &Value, builtin: &str) -> Result<Value> {
  let profile = profile_for_builtin(builtin);
  let errors = match xml_input_to_json(input, builtin) {
    Ok(json) => xml_validation_errors(&json, profile),
    Err(err) => vec![err.to_string()],
  };
  Ok(Value::String(errors.join("\n")))
}

pub fn xml_format_xml_to_json(input: &Value, builtin: &str) -> Result<Value> {
  let json = xml_input_to_json(input, builtin)?;
  Ok(json_to_value(&json))
}

pub fn xml_format_emit(input: &Value, builtin: &str) -> Result<String> {
  let json = xml_input_to_json(input, builtin)?;
  let errors = xml_validation_errors(&json, profile_for_builtin(builtin));
  if !errors.is_empty() {
    return Err(anyhow!("{builtin}: {}", errors.join("; ")));
  }
  crate::markup::xml_emit(&json_to_value(&json)).map_err(|err| anyhow!("{builtin}: {err}"))
}

pub fn xml_format_convert(input: &Value, builtin: &str) -> Result<Value> {
  let json = xml_input_to_json(input, builtin)?;
  let errors = xml_validation_errors(&json, profile_for_builtin(builtin));
  if !errors.is_empty() {
    return Err(anyhow!("{builtin}: {}", errors.join("; ")));
  }
  Ok(json_to_value(&json))
}

fn xml_input_to_json(input: &Value, builtin: &str) -> Result<JsonValue> {
  if let Some(xml) = input.as_str() {
    return pnix_xml_core::xml_json_from_xml_str(xml)
      .map_err(|err| anyhow!("{builtin}: expected well-formed XML string or pnix XML AST: {err}"));
  }

  let json = value_to_json(input)?;
  if !xml_json_has_node(&json) {
    return Err(anyhow!(
      "{builtin}: expected well-formed XML string or pnix XML AST with `kind`/`name`/`children`"
    ));
  }
  Ok(json)
}

fn xml_validation_errors(json: &JsonValue, profile: XmlFamilyProfile) -> Vec<String> {
  let mut errors = Vec::new();
  if let Err(err) = pnix_xml_core::xml_validate_json(json, None) {
    errors.push(strip_xml_validate_prefix(&err).to_string());
  }
  errors.extend(profile_errors(json, profile));
  errors
}

fn validation_report(profile: XmlFamilyProfile, errors: Vec<String>) -> JsonValue {
  let ok = errors.is_empty();
  serde_json::json!({
    "success": ok,
    "ok": ok,
    "errors": errors,
    "profile": profile.id,
    "validator": "pnix-clean-room-xml-core-v1",
    "external_schema_materialization_count": 0,
  })
}

fn strip_xml_validate_prefix(text: &str) -> &str {
  text.strip_prefix("xml validate: ").unwrap_or(text)
}

fn profile_errors(json: &JsonValue, profile: XmlFamilyProfile) -> Vec<String> {
  if profile.root_names.is_empty() {
    return Vec::new();
  }
  let Some(root_name) = xml_root_name(json) else {
    return vec![format!("{}: missing XML root element", profile.id)];
  };
  let normalized = normalize_name(&root_name);
  if profile
    .root_names
    .iter()
    .any(|candidate| normalize_name(candidate) == normalized)
  {
    Vec::new()
  } else {
    vec![format!(
      "{}: expected root element one of [{}], got '{}'",
      profile.id,
      profile.root_names.join(", "),
      root_name
    )]
  }
}

fn xml_json_has_node(json: &JsonValue) -> bool {
  match json {
    JsonValue::Object(obj) => {
      obj.contains_key("name")
        || obj.contains_key("kind")
        || obj
          .get("children")
          .is_some_and(|children| matches!(children, JsonValue::Array(_)))
    }
    JsonValue::Array(items) => !items.is_empty() && items.iter().all(xml_json_has_node),
    _ => false,
  }
}

fn xml_root_name(json: &JsonValue) -> Option<String> {
  match json {
    JsonValue::Object(obj) => {
      if xml_obj_is_element(obj) {
        return obj
          .get("name")
          .and_then(JsonValue::as_str)
          .map(ToString::to_string);
      }
      obj
        .get("children")
        .and_then(JsonValue::as_array)
        .and_then(|children| children.iter().find_map(xml_root_name))
    }
    JsonValue::Array(items) => items.iter().find_map(xml_root_name),
    _ => None,
  }
}

fn xml_obj_is_element(obj: &JsonMap<String, JsonValue>) -> bool {
  obj
    .get("kind")
    .and_then(JsonValue::as_str)
    .map(|kind| kind == "element")
    .unwrap_or_else(|| obj.contains_key("name"))
}

fn normalize_name(name: &str) -> String {
  let local = if let Some((_, local)) = name.rsplit_once(':') {
    local
  } else {
    name
  };
  local
    .chars()
    .filter(|ch| !ch.is_ascii_whitespace() && *ch != '-' && *ch != '_')
    .flat_map(char::to_lowercase)
    .collect()
}
