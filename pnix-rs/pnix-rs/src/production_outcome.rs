use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::machine_outcome::{EvalError, MachineOutcome, SCHEMA as HOST_OUTCOME_SCHEMA};
use crate::px;

const PROJECTION_SCHEMA: &str = "pnix.production-basic-outcome-projection.v1";

fn failed(phase: &str, class: &str) -> MachineOutcome<px::PxVal> {
    MachineOutcome::Failed(EvalError {
        phase: String::from(phase),
        class: String::from(class),
        evidence: std::collections::BTreeMap::new(),
    })
}

pub fn eval_source_outcome(source: &str) -> MachineOutcome<px::PxVal> {
    match px::px_run_value_outcome(source)
        .and_then(|value| px::px_force_deep_outcome(&value))
    {
        Ok(value) => MachineOutcome::Done(value),
        Err(error) => failed(error.phase_name(), error.class_name()),
    }
}

#[derive(Clone)]
struct Projection {
    outcome_kind: &'static str,
    error_phase: Option<String>,
    error_class: Option<String>,
    value_json: Option<String>,
}

fn project(outcome: &MachineOutcome<px::PxVal>) -> Projection {
    match outcome {
        MachineOutcome::Done(value) => Projection {
            outcome_kind: "done",
            error_phase: None,
            error_class: None,
            value_json: Some(px::px_to_json(value).unwrap_or_else(|_| String::from("null"))),
        },
        MachineOutcome::Failed(error) => Projection {
            outcome_kind: "failed",
            error_phase: Some(error.phase.clone()),
            error_class: Some(error.class.clone()),
            value_json: None,
        },
        _ => panic!("basic projection accepts Done or Failed"),
    }
}

fn json_quote(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn option_json(value: &Option<String>) -> String {
    match value {
        Some(item) => json_quote(item),
        None => String::from("null"),
    }
}

impl Projection {
    fn to_json(&self) -> String {
        format!(
            "{{\"error_class\":{},\"error_phase\":{},\"outcome_kind\":{},\"schema\":{},\"value_json\":{}}}",
            option_json(&self.error_class),
            option_json(&self.error_phase),
            json_quote(self.outcome_kind),
            json_quote(PROJECTION_SCHEMA),
            option_json(&self.value_json),
        )
    }
}

fn expected_projection(kind: &str, phase: &str, class: &str, value: &str) -> Projection {
    Projection {
        outcome_kind: if kind == "done" { "done" } else { "failed" },
        error_phase: if phase.is_empty() { None } else { Some(String::from(phase)) },
        error_class: if class.is_empty() { None } else { Some(String::from(class)) },
        value_json: if value.is_empty() { None } else { Some(String::from(value)) },
    }
}

pub fn report_json(path: &str) -> Result<(String, bool), String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut rows = Vec::new();
    let mut all_match = true;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|error| error.to_string())?;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.splitn(6, '\t').collect();
        if fields.len() != 6 {
            return Err(String::from("invalid basic outcome case row"));
        }
        let projection = project(&eval_source_outcome(fields[5]));
        let expected = expected_projection(fields[1], fields[2], fields[3], fields[4]);
        let matches = projection.to_json() == expected.to_json();
        all_match = all_match && matches;
        rows.push(format!(
            "{{\"case\":{},\"matches_expected\":{},\"projection\":{}}}",
            json_quote(fields[0]),
            if matches { "true" } else { "false" },
            projection.to_json(),
        ));
    }
    Ok((format!(
        "{{\"host\":\"pnix-rs\",\"host_outcome_schema\":{},\"matrix\":[{}],\"model_schema\":\"pnix.machine.eval-outcome-model.v1\",\"schema\":\"pnix.production-basic-outcome-report.v1\",\"status\":{{\"automatic_codegen\":false,\"basic_language_errors_are_held\":false,\"legacy_error_transport_is_semantic_owner\":false,\"production_basic_outcome_convergence_v1\":true,\"production_common_machine_replacement\":false,\"production_requested_integration\":false,\"production_suspension_equivalence\":false}}}}",
        json_quote(HOST_OUTCOME_SCHEMA),
        rows.join(","),
    ), all_match))
}
