use std::process::ExitCode;

pub const ABI_VERSION: &str = "pnix.primitive-abi.v1";
pub const MANIFEST_DIGEST: &str =
    "f133ee0f3a5c6073eabb6855f3abf44bf36366083f26fbe76e9524521a2a5fd6";

const CHECKED_IDS: [&str; 4] = [
    "i64-add-checked",
    "i64-sub-checked",
    "i64-mul-checked",
    "i64-div-checked",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimitiveOutcome {
    Ok(i64),
    Error {
        phase: &'static str,
        class: &'static str,
    },
}

fn failure(phase: &'static str, class: &'static str) -> PrimitiveOutcome {
    PrimitiveOutcome::Error { phase, class }
}

fn contract_failure() -> PrimitiveOutcome {
    failure("primitive-contract", "primitive-contract-violation")
}

fn invoke_impl(
    abi_version: &str,
    manifest_digest: &str,
    primitive_id: &str,
    operands: &[i64],
) -> PrimitiveOutcome {
    if abi_version != ABI_VERSION || manifest_digest != MANIFEST_DIGEST {
        return contract_failure();
    }
    if !CHECKED_IDS.contains(&primitive_id) || operands.len() != 2 {
        return contract_failure();
    }
    match crate::px::px_checked_i64_kernel(
        abi_version,
        manifest_digest,
        primitive_id,
        operands,
    ) {
        Ok(value) => PrimitiveOutcome::Ok(value),
        Err("primitive-contract-violation") => contract_failure(),
        Err(class) => failure("eval", class),
    }
}

pub fn invoke(
    abi_version: &str,
    manifest_digest: &str,
    primitive_id: &str,
    operands: &[i64],
) -> PrimitiveOutcome {
    std::panic::catch_unwind(|| {
        invoke_impl(abi_version, manifest_digest, primitive_id, operands)
    })
    .unwrap_or_else(|_| contract_failure())
}

fn primitive_id_for_operator(operator: &str) -> Option<&'static str> {
    match operator {
        "add" => Some("i64-add-checked"),
        "sub" => Some("i64-sub-checked"),
        "mul" => Some("i64-mul-checked"),
        "div" => Some("i64-div-checked"),
        _ => None,
    }
}

fn normalize_legacy(result: &Result<i64, String>) -> PrimitiveOutcome {
    match result {
        Ok(value) => PrimitiveOutcome::Ok(*value),
        Err(message) if message == "px: division by zero" => {
            failure("eval", "division-by-zero")
        }
        Err(message) if message.starts_with("px: integer overflow") => {
            failure("eval", "integer-overflow")
        }
        Err(_) => contract_failure(),
    }
}

pub fn invoke_shadow(
    operator: &str,
    left: i64,
    right: i64,
    legacy: Result<i64, String>,
) -> PrimitiveOutcome {
    let primitive_id = match primitive_id_for_operator(operator) {
        Some(value) => value,
        None => return contract_failure(),
    };
    let routed = invoke(ABI_VERSION, MANIFEST_DIGEST, primitive_id, &[left, right]);
    if normalize_legacy(&legacy) == routed {
        routed
    } else {
        contract_failure()
    }
}

fn legacy_checked(operator: &str, left: i64, right: i64) -> Result<i64, String> {
    if operator == "div" && right == 0 {
        return Err(String::from("px: division by zero"));
    }
    let value = match operator {
        "add" => left.checked_add(right),
        "sub" => left.checked_sub(right),
        "mul" => left.checked_mul(right),
        "div" => left.checked_div(right),
        _ => return Err(String::from("px: primitive contract violation")),
    };
    value.ok_or_else(|| format!("px: integer overflow in {} {} {}", operator, left, right))
}

fn self_check() -> bool {
    let cases = [
        ("add", 1, 2, PrimitiveOutcome::Ok(3)),
        ("sub", -7, 5, PrimitiveOutcome::Ok(-12)),
        ("mul", -7, -6, PrimitiveOutcome::Ok(42)),
        ("div", -7, 3, PrimitiveOutcome::Ok(-2)),
        ("div", 7, -3, PrimitiveOutcome::Ok(-2)),
        ("add", i64::MAX, 1, failure("eval", "integer-overflow")),
        ("sub", i64::MIN, 1, failure("eval", "integer-overflow")),
        ("mul", i64::MAX, 2, failure("eval", "integer-overflow")),
        ("div", i64::MIN, -1, failure("eval", "integer-overflow")),
        ("div", 1, 0, failure("eval", "division-by-zero")),
    ];
    let matrix_ok = cases.iter().all(|(operator, left, right, expected)| {
        invoke_shadow(operator, *left, *right, legacy_checked(operator, *left, *right))
            == *expected
    });
    let contract = contract_failure();
    matrix_ok
        && invoke("wrong", MANIFEST_DIGEST, "i64-add-checked", &[1, 2]) == contract
        && invoke(ABI_VERSION, "wrong", "i64-add-checked", &[1, 2]) == contract
        && invoke(ABI_VERSION, MANIFEST_DIGEST, "unknown", &[1, 2]) == contract
        && invoke(ABI_VERSION, MANIFEST_DIGEST, "i64-add-checked", &[1]) == contract
}

const REPORT_JSON: &str = r#"{"schema":"pnix.production-primitive-gate.v1","abi_version":"pnix.primitive-abi.v1","manifest_digest":"f133ee0f3a5c6073eabb6855f3abf44bf36366083f26fbe76e9524521a2a5fd6","checked_integer_primitive_ids":["i64-add-checked","i64-sub-checked","i64-mul-checked","i64-div-checked"],"strict_args":{"i64-add-checked":[0,1],"i64-sub-checked":[0,1],"i64-mul-checked":[0,1],"i64-div-checked":[0,1]},"execution_error_classes":{"i64-add-checked":["integer-overflow"],"i64-sub-checked":["integer-overflow"],"i64-mul-checked":["integer-overflow"],"i64-div-checked":["division-by-zero","integer-overflow"]},"validation_error_classes":{"i64-add-checked":["type-error"],"i64-sub-checked":["type-error"],"i64-mul-checked":["type-error"],"i64-div-checked":["type-error"]},"force_order":[0,1],"shadow_mode":true,"matrix":[{"case":"add-positive","kind":"ok","value":3},{"case":"sub-signed","kind":"ok","value":-12},{"case":"mul-signed","kind":"ok","value":42},{"case":"div-negative-left","kind":"ok","value":-2},{"case":"div-negative-right","kind":"ok","value":-2},{"case":"add-overflow","kind":"error","phase":"eval","class":"integer-overflow"},{"case":"sub-overflow","kind":"error","phase":"eval","class":"integer-overflow"},{"case":"mul-overflow","kind":"error","phase":"eval","class":"integer-overflow"},{"case":"div-overflow","kind":"error","phase":"eval","class":"integer-overflow"},{"case":"division-by-zero","kind":"error","phase":"eval","class":"division-by-zero"}],"contract_matrix":[{"case":"wrong-abi","kind":"error","phase":"primitive-contract","class":"primitive-contract-violation"},{"case":"wrong-digest","kind":"error","phase":"primitive-contract","class":"primitive-contract-violation"},{"case":"unknown-id","kind":"error","phase":"primitive-contract","class":"primitive-contract-violation"},{"case":"wrong-arity","kind":"error","phase":"primitive-contract","class":"primitive-contract-violation"}],"status":{"production_checked_i64_manifest_enforced":true,"production_evaluator_manifest_enforced":false,"full_builtin_surface_manifest_enforced":false}}"#;

pub fn cmd_check() -> ExitCode {
    if self_check() {
        println!("{}", REPORT_JSON);
        ExitCode::SUCCESS
    } else {
        eprintln!("primitive-manifest-check: self-check failed");
        ExitCode::FAILURE
    }
}
