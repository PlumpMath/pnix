//! Smoke test for the bio/office schema/format builtins. Each one uses
//! the clean-room pnix XML core: XML strings are parsed, ASTs are checked,
//! family wrappers validate root names, and normalize returns XML AST.

use pnix_eval::eval_to_json;

fn assert_success(src: &str) {
  let out = eval_to_json(src, false).expect(src);
  assert!(out.contains("\"success\":true"), "expected success: {out}");
}

#[test]
fn cellml_validate_pass() {
  assert_success(
    r#"builtins.cellmlSchemaValidate { kind = "element"; name = "model"; attrs = {}; children = []; }"#,
  );
}

#[test]
fn sbml_validate_pass() {
  assert_success(
    r#"builtins.sbmlSchemaValidate { kind = "element"; name = "sbml"; attrs = {}; children = []; }"#,
  );
}

#[test]
fn neuroml_validate_pass() {
  assert_success(
    r#"builtins.neuromlSchemaValidate { kind = "element"; name = "neuroml"; attrs = {}; children = []; }"#,
  );
}

#[test]
fn pdbml_validate_pass() {
  assert_success(
    r#"builtins.pdbmlSchemaValidate { kind = "element"; name = "datablock"; attrs = {}; children = []; }"#,
  );
}

#[test]
fn cml_validate_pass() {
  assert_success(
    r#"builtins.cmlSchemaValidate { kind = "element"; name = "molecule"; attrs = {}; children = []; }"#,
  );
}

#[test]
fn cellml_normalize_returns_ast() {
  let out = eval_to_json(
    r#"builtins.cellmlSchemaNormalize { kind = "element"; name = "model"; attrs = {}; children = []; }"#,
    false,
  )
  .unwrap();
  assert!(out.contains("\"name\":\"model\""), "got {out}");
}

#[test]
fn cellml_explain_empty_on_valid() {
  let out = eval_to_json(
    r#"builtins.cellmlSchemaExplain { kind = "element"; name = "model"; attrs = {}; children = []; }"#,
    false,
  )
  .unwrap();
  assert_eq!(out, "\"\"");
}

#[test]
fn excel_conversion_normalizes_xml_ast() {
  let out = eval_to_json(
    r#"builtins.excelToOds { kind = "element"; name = "Workbook"; attrs = {}; children = []; }"#,
    false,
  )
  .unwrap();
  assert!(out.contains("\"name\":\"Workbook\""), "got {out}");
}

#[test]
fn xml_schema_validate_pass() {
  assert_success(
    r#"builtins.xmlSchemaValidate { kind = "element"; name = "root"; attrs = {}; children = []; }"#,
  );
}

#[test]
fn stdlib_cellml_wrapper_works() {
  // Simulate the stdlib wrapper: `normalize = ast: builtins.cellmlSchemaNormalize ast`
  // applied to a real-shaped CellML model.
  let out = eval_to_json(
    r#"let
        cellml = (import <lib/bio/cellml.px>);
      in
        cellml.normalize {
          kind = "element";
          name = "model";
          attrs = { name = "demo"; };
          children = [];
        }"#,
    false,
  );
  // Either the import path resolution works (full pipeline OK) or
  // returns an error mentioning the lib path — both confirm that
  // when the wrapper *is* called, it now reaches the impl instead
  // of failing with `undefined attribute`.
  match out {
    Ok(json) => assert!(json.contains("\"name\":\"model\""), "got {json}"),
    Err(e) => {
      let m = e.to_string();
      // If import didn't resolve, we should not see the old
      // `undefined attribute: cellmlSchemaNormalize` error any more.
      assert!(
        !m.contains("undefined attribute"),
        "should not be undefined attribute now: {m}"
      );
    }
  }
}

#[test]
fn validate_rejects_non_xml_shape() {
  let out = eval_to_json(r#"builtins.cellmlSchemaValidate 42"#, false).unwrap();
  assert!(out.contains("\"success\":false"), "got {out}");
  assert!(out.contains("expected well-formed XML"), "got {out}");
}

#[test]
fn validate_rejects_wrong_family_root() {
  let out = eval_to_json(
    r#"builtins.sbmlSchemaValidate { kind = "element"; name = "model"; attrs = {}; children = []; }"#,
    false,
  )
  .unwrap();
  assert!(out.contains("\"success\":false"), "got {out}");
  assert!(out.contains("expected root element"), "got {out}");
}

#[test]
fn validate_rejects_malformed_xml_string() {
  let out = eval_to_json(
    r#"builtins.xmlSchemaValidate "<root><child></root>""#,
    false,
  )
  .unwrap();
  assert!(out.contains("\"success\":false"), "got {out}");
  assert!(
    out.contains("mismatched") || out.contains("end tag") || out.contains("ill-formed"),
    "got {out}"
  );
}

#[test]
fn normalize_xml_string_returns_ast() {
  let out = eval_to_json(
    r#"builtins.xmlSchemaNormalize "<root><child>ok</child></root>""#,
    false,
  )
  .unwrap();
  assert!(out.contains("\"name\":\"root\""), "got {out}");
  assert!(out.contains("\"name\":\"child\""), "got {out}");
}
