use std::any::Any;
use std::collections::BTreeMap;

pub const SCHEMA: &str = "pnix.machine.host-outcome.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalError {
    pub phase: String,
    pub class: String,
    pub evidence: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectRequest {
    pub effect: String,
    pub args: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Continuation {
    pub id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceReason {
    pub class: String,
    pub divergence_proven: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MachineOutcome<V> {
    Done(V),
    Failed(EvalError),
    Requested(EffectRequest, Continuation),
    Suspended(Checkpoint, ResourceReason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundaryObservation<V> {
    Done {
        status: &'static str,
        value: V,
    },
    Failed {
        status: &'static str,
        error: EvalError,
    },
    Requested {
        status: &'static str,
        request: EffectRequest,
        continuation: Continuation,
    },
    Suspended {
        status: &'static str,
        checkpoint: Checkpoint,
        reason: ResourceReason,
    },
}

pub fn observe<V: Clone>(outcome: &MachineOutcome<V>) -> BoundaryObservation<V> {
    match outcome {
        MachineOutcome::Done(value) => BoundaryObservation::Done {
            status: "done",
            value: value.clone(),
        },
        MachineOutcome::Failed(error) => BoundaryObservation::Failed {
            status: "failed",
            error: error.clone(),
        },
        MachineOutcome::Requested(request, continuation) => {
            BoundaryObservation::Requested {
                status: "requested",
                request: request.clone(),
                continuation: continuation.clone(),
            }
        }
        MachineOutcome::Suspended(checkpoint, reason) => {
            BoundaryObservation::Suspended {
                status: "suspended",
                checkpoint: checkpoint.clone(),
                reason: reason.clone(),
            }
        }
    }
}

pub fn is_machine_outcome<V: 'static>(value: &dyn Any) -> bool {
    value.is::<MachineOutcome<V>>()
}

pub fn self_check_json() -> &'static str {
    assert_eq!(SCHEMA, "pnix.machine.host-outcome.v1");
    let done = MachineOutcome::Done(String::from("value"));
    let failed: MachineOutcome<String> = MachineOutcome::Failed(EvalError {
        phase: String::from("eval"),
        class: String::from("not-callable"),
        evidence: BTreeMap::new(),
    });
    let requested: MachineOutcome<String> = MachineOutcome::Requested(
        EffectRequest {
            effect: String::from("open"),
            args: BTreeMap::new(),
        },
        Continuation { id: 1 },
    );
    let suspended: MachineOutcome<String> = MachineOutcome::Suspended(
        Checkpoint { id: 2 },
        ResourceReason {
            class: String::from("resource-budget-exhausted"),
            divergence_proven: false,
        },
    );
    let mut guest_shape = BTreeMap::new();
    guest_shape.insert(String::from("outcome_kind"), String::from("done"));

    assert!(matches!(
        observe(&done),
        BoundaryObservation::Done { status: "done", .. }
    ));
    assert!(matches!(
        observe(&failed),
        BoundaryObservation::Failed {
            status: "failed",
            error: EvalError { ref phase, ref class, .. },
        } if phase == "eval" && class == "not-callable"
    ));
    assert!(matches!(
        observe(&requested),
        BoundaryObservation::Requested {
            status: "requested",
            request: EffectRequest { ref effect, .. },
            ..
        } if effect == "open"
    ));
    assert!(matches!(
        observe(&suspended),
        BoundaryObservation::Suspended {
            status: "suspended",
            reason: ResourceReason {
                divergence_proven: false,
                ..
            },
            ..
        }
    ));
    assert!(!is_machine_outcome::<String>(&guest_shape));

    "{\"all_ok\":true,\"done\":\"done\",\"failed_class\":\"not-callable\",\"failed_phase\":\"eval\",\"guest_shape_is_outcome\":false,\"requested\":\"requested\",\"requested_effect\":\"open\",\"schema\":\"pnix.machine.host-outcome.v1\",\"suspended\":\"suspended\",\"suspended_divergence_proven\":false}"
}
