//! 런타임 에러 테스트: 런타임 에러 처리 및 트레이스 테스트
//!
//! 런타임 에러가 올바르게 생성되고 트레이스 정보가 포함되는지 검증합니다.

use pnix_runtime_api::{AdapterErrorKind, ExecutionErrorKind, RuntimeError, RuntimeTraceFrame};

const SSA_TRACE: &[TraceCase] = &[TraceCase {
  node: "node_div",
  morphism: "div",
  message: "division by zero",
}];

const FRP_TRACE: &[TraceCase] = &[
  TraceCase {
    node: "sig_a",
    morphism: "sin",
    message: "nan",
  },
  TraceCase {
    node: "sig_b",
    morphism: "add",
    message: "nan",
  },
];

const CASES: &[Case] = &[
  Case {
    name: "ssa-trace",
    kind: CaseKind::Execution(ExecutionErrorKind::SSA),
    message: Some("division by zero"),
    trace: SSA_TRACE,
    expected: "[E0410] SSA error: division by zero\n  at node node_div, morphism div failed because division by zero",
  },
  Case {
    name: "frp-trace-multi",
    kind: CaseKind::Execution(ExecutionErrorKind::FRP),
    message: Some("signal evaluation failed"),
    trace: FRP_TRACE,
    expected: "[E0411] FRP error: signal evaluation failed\n  at node sig_a, morphism sin failed because nan\n  at node sig_b, morphism add failed because nan",
  },
  Case {
    name: "adapter-error",
    kind: CaseKind::Adapter(AdapterErrorKind::Unsupported),
    message: Some("SignalVar not supported"),
    trace: &[],
    expected: "[E0403] adapter unsupported: SignalVar not supported\n  at node <unknown>, morphism <runtime> failed because SignalVar not supported",
  },
  Case {
    name: "message-error",
    kind: CaseKind::Message,
    message: Some("runtime failed"),
    trace: &[],
    expected: "[E0402] runtime failed\n  at node <unknown>, morphism <runtime> failed because runtime failed",
  },
  Case {
    name: "unimplemented",
    kind: CaseKind::Unimplemented("jit"),
    message: None,
    trace: &[],
    expected: "[E0401] unimplemented: jit\n  at node <unknown>, morphism <runtime> failed because jit",
  },
];

#[derive(Debug)]
enum CaseKind {
  Execution(ExecutionErrorKind),
  Adapter(AdapterErrorKind),
  Message,
  Unimplemented(&'static str),
}

#[derive(Debug)]
struct TraceCase {
  node: &'static str,
  morphism: &'static str,
  message: &'static str,
}

#[derive(Debug)]
struct Case {
  name: &'static str,
  kind: CaseKind,
  message: Option<&'static str>,
  trace: &'static [TraceCase],
  expected: &'static str,
}

#[test]
fn runtime_error_messages_include_codes_and_traces() {
  for case in CASES {
    let err = match case.kind {
      CaseKind::Execution(exec_kind) => {
        let message = case.message.expect("message").to_string();
        let mut err = RuntimeError::execution(exec_kind, message);
        for frame in case.trace {
          err = err.push_trace_frame(RuntimeTraceFrame::new(
            frame.node,
            frame.morphism,
            frame.message,
          ));
        }
        err
      }
      CaseKind::Adapter(kind) => {
        let message = case.message.expect("message").to_string();
        RuntimeError::adapter(kind, message)
      }
      CaseKind::Message => {
        let message = case.message.expect("message").to_string();
        RuntimeError::message(message)
      }
      CaseKind::Unimplemented(area) => RuntimeError::unimplemented(area),
    };

    let rendered = err.to_string();
    assert_eq!(rendered, case.expected, "case {}", case.name);

    let code = err.code();
    assert!(
      rendered.contains(code.0),
      "case {}: missing code {}",
      case.name,
      code
    );
  }
}

#[test]
fn runtime_error_exposes_source_message() {
  let err =
    RuntimeError::execution(ExecutionErrorKind::SSA, "division by zero").with_source("ssa_eval");
  let source = err.source().expect("expected source to be set");
  assert_eq!(source.to_string(), "ssa_eval");
}

#[test]
fn runtime_error_with_source_opt_is_noop_for_none() {
  let err = RuntimeError::message("oops").with_source_opt(None);
  assert!(err.source().is_none());
}
