use crate::markup::{json_to_value, value_to_json};
use crate::value::Env;
use crate::Value;
use anyhow::{anyhow, Result};
use pnix_core::physics::ConstraintType;
use pnix_core::symbolic::expr::SymExpr;
use pnix_core::symbolic::serialize::{to_json_value as symexpr_to_json_value, type_summary};
use serde_json::{Map, Number, Value as JsonValue};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const DEFAULT_CYCLE_INTERVAL: f64 = 1.0;

pub fn x3d_xml_to_json(input: &Value) -> Result<Value> {
  let json = x3d_input_to_json(input, "builtins.x3dXmlToJson", true)?;
  Ok(json_to_value(&json))
}

pub fn x3d_schema_normalize(input: &Value) -> Result<Value> {
  let mut json = x3d_input_to_json(input, "builtins.x3dSchemaNormalize", false)?;
  xml_x3d_core::x3d_normalize_xml_json_with_defaults(&mut json);
  Ok(json_to_value(&json))
}

pub fn x3d_schema_validate(input: &Value) -> Result<Value> {
  let json = x3d_input_to_json(input, "builtins.x3dSchemaValidate", false)?;
  let errors = collect_x3d_validation_errors(&json)?;
  Ok(x3d_validation_report(errors))
}

pub fn x3d_schema_explain(input: &Value) -> Result<Value> {
  let json = x3d_input_to_json(input, "builtins.x3dSchemaExplain", false)?;
  Ok(Value::String(
    collect_x3d_validation_errors(&json)?.join("\n"),
  ))
}

pub fn x3d_frp_graph(input: &Value) -> Result<Value> {
  let mut json = x3d_input_to_json(input, "builtins.x3dFrpGraph", false)?;
  xml_x3d_core::x3d_normalize_xml_json_with_defaults(&mut json);
  let graph_json = frp_graph_json_from_xml_json(&json);
  Ok(json_to_value(&graph_json))
}

pub fn x3d_sync_plan(previous: &Value, next: &Value) -> Result<Value> {
  let previous_json = if matches!(previous, Value::Null) {
    None
  } else {
    Some(x3d_input_to_json(previous, "builtins.x3dSyncPlan", true)?)
  };
  let next_json = x3d_input_to_json(next, "builtins.x3dSyncPlan", true)?;
  let plan = x3d_sync_plan_json(previous_json.as_ref(), &next_json);
  Ok(json_to_value(&plan))
}

pub fn x3d_x3dom_fragment(input: &Value) -> Result<Value> {
  let mut json = x3d_input_to_json(input, "builtins.x3dX3domFragment", true)?;
  annotate_sync_ids(&mut json, "root", 0);
  Ok(Value::String(x3dom_fragment_from_xml_json(&json)))
}

pub fn x3d_x3dom_html(input: &Value) -> Result<Value> {
  let mut json = x3d_input_to_json(input, "builtins.x3dX3domHtml", true)?;
  annotate_sync_ids(&mut json, "root", 0);
  let fragment = x3dom_fragment_from_xml_json(&json);
  Ok(Value::String(x3dom_document_from_fragment(&fragment)))
}

pub fn x3d_x3dom_patch(previous: &Value, next: &Value) -> Result<Value> {
  let previous_json = if matches!(previous, Value::Null) {
    None
  } else {
    Some(x3d_input_to_json(previous, "builtins.x3dX3domPatch", true)?)
  };
  let next_json = x3d_input_to_json(next, "builtins.x3dX3domPatch", true)?;
  let plan = x3d_sync_plan_json(previous_json.as_ref(), &next_json);
  let lowered = x3dom_patch_payload_from_sync_plan(&plan)?;
  Ok(json_to_value(&lowered))
}

pub fn x3d_render_packet(previous: &Value, next: &Value) -> Result<Value> {
  let previous_json = if matches!(previous, Value::Null) {
    None
  } else {
    Some(x3d_input_to_json(
      previous,
      "builtins.x3dRenderPacket",
      true,
    )?)
  };
  let next_json = x3d_input_to_json(next, "builtins.x3dRenderPacket", true)?;
  let packet = x3d_render_packet_json(previous_json.as_ref(), &next_json)?;
  Ok(json_to_value(&packet))
}

fn x3d_input_to_json(input: &Value, builtin_name: &str, normalize: bool) -> Result<JsonValue> {
  match input {
    Value::String(expr) => {
      let json = if normalize {
        xml_x3d_core::x3d_xml_json_from_expr_normalized(expr, eval_expr_to_json)
      } else {
        xml_x3d_core::x3d_xml_json_from_expr(expr, eval_expr_to_json)
      };
      json.ok_or_else(|| anyhow!("{builtin_name}: expected X3D XML or pnix XML JSON expression"))
    }
    _ => {
      let mut json = value_to_json(input)?;
      if xml_x3d_core::xml_nodes_from_json(&json).is_empty() {
        return Err(anyhow!(
          "{builtin_name}: expected X3D XML string or XML JSON attrset"
        ));
      }
      if normalize {
        xml_x3d_core::x3d_normalize_xml_json_with_defaults(&mut json);
      }
      Ok(json)
    }
  }
}

fn eval_expr_to_json(expr: &str) -> Option<JsonValue> {
  let parsed = pnix_core::lang::pnix::parse_expr(expr).ok()?;
  let value = crate::interpret::eval(&parsed, &Env::new()).ok()?;
  value_to_json(&value).ok()
}

fn collect_x3d_validation_errors(json: &JsonValue) -> Result<Vec<String>> {
  let mut errors = Vec::new();

  if let Err(err) = pnix_xml_core::xml_validate_json(json, None) {
    errors.extend(split_xml_validate_errors(&err));
  }

  errors.extend(
    xml_x3d_core::validate_attrs_from_xml_json(json)
      .into_iter()
      .map(|issue| format!("{}@{}: {}", issue.element, issue.attr, issue.message)),
  );

  errors.extend(
    xml_x3d_core::validate_routes_from_xml_json(json)
      .into_iter()
      .map(|issue| {
        format!(
          "ROUTE {}.{} -> {}.{}: {}",
          issue.from_node, issue.from_field, issue.to_node, issue.to_field, issue.message
        )
      }),
  );

  let mut out = Vec::with_capacity(errors.len());
  let mut seen = HashSet::with_capacity(errors.len());
  for error in errors {
    if seen.insert(error.clone()) {
      out.push(error);
    }
  }
  Ok(out)
}

fn x3d_validation_report(errors: Vec<String>) -> Value {
  let ok = errors.is_empty();
  let mut out = std::collections::BTreeMap::new();
  out.insert("success".to_string(), Value::Bool(ok));
  out.insert("ok".to_string(), Value::Bool(ok));
  let mut error_values = Vec::with_capacity(errors.len());
  for error in errors {
    error_values.push(Value::String(error));
  }
  out.insert("errors".to_string(), Value::List(Arc::new(error_values)));
  Value::AttrSet(Arc::new(out))
}

fn x3d_sync_plan_json(previous: Option<&JsonValue>, next: &JsonValue) -> JsonValue {
  let mut next_scene = next.clone();
  annotate_sync_ids(&mut next_scene, "root", 0);

  let mut ops = Vec::new();
  let mode = match previous {
    Some(previous) => {
      let mut previous_scene = previous.clone();
      annotate_sync_ids(&mut previous_scene, "root", 0);
      diff_sync_nodes(&previous_scene, &next_scene, &mut ops);
      if ops.is_empty() {
        "noop"
      } else {
        "patch"
      }
    }
    None => {
      ops.push(serde_json::json!({
        "op": "mount",
        "node_pnix_address": next_scene
          .as_object()
          .map(pnix_address_of)
          .unwrap_or_else(|| "root".to_string()),
        "node": next_scene.clone(),
      }));
      "replace"
    }
  };

  serde_json::json!({
    "mode": mode,
    "changed": !ops.is_empty(),
    "patch_count": ops.len(),
    "scene": next_scene,
    "ops": ops,
  })
}

fn x3dom_patch_payload_from_sync_plan(plan: &JsonValue) -> Result<JsonValue> {
  let plan_obj = plan
    .as_object()
    .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: sync plan must be attrset"))?;
  let scene = plan_obj
    .get("scene")
    .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: sync plan missing scene"))?;
  let scene_ops = plan_obj
    .get("ops")
    .and_then(JsonValue::as_array)
    .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: sync plan missing ops"))?;

  let fragment = x3dom_fragment_from_xml_json(scene);
  let mut html_ops = Vec::with_capacity(scene_ops.len());
  for op in scene_ops {
    html_ops.push(lower_sync_op_to_x3dom_html(op)?);
  }

  Ok(serde_json::json!({
    "protocol": "pnix.x3dom.patch.v1",
    "engine": "x3dom-ssr",
    "mode": plan_obj.get("mode").cloned().unwrap_or(JsonValue::String("noop".to_string())),
    "changed": plan_obj.get("changed").cloned().unwrap_or(JsonValue::Bool(false)),
    "patch_count": plan_obj.get("patch_count").cloned().unwrap_or(JsonValue::Number(Number::from(0))),
    "scene": scene.clone(),
    "scene_ops": JsonValue::Array(scene_ops.to_vec()),
    "fragment": fragment,
    "html": x3dom_document_from_fragment(&fragment),
    "ops": JsonValue::Array(html_ops),
  }))
}

fn x3d_render_packet_json(previous: Option<&JsonValue>, next: &JsonValue) -> Result<JsonValue> {
  let nodes = xml_x3d_core::xml_nodes_from_json(next);
  let route_graph = xml_x3d_core::x3d_route_graph_from_xml_nodes(&nodes);
  let frp = frp_graph_json_from_xml_json(next);
  let sync = x3d_sync_plan_json(previous, next);
  let x3dom = x3dom_patch_payload_from_sync_plan(&sync)?;

  Ok(serde_json::json!({
    "protocol": "pnix.render.packet.v1",
    "family": "x3d",
    "process_api": true,
    "scene": sync.get("scene").cloned().unwrap_or_else(|| next.clone()),
    "sync": sync,
    "lowerings": {
      "x3dom": x3dom,
      "wgpu": {
        "status": "pending",
        "reason": "direct x3d render packet exists, but wgpu lowerer is not implemented yet",
      },
    },
    "simulation": {
      "frp": frp_summary_json(&frp),
      "physics": physics_summary_json(next, &route_graph),
      "hanim": hanim_summary_json(next, &route_graph),
      "symbolic": symbolic_world_summary_json(next, &route_graph),
    },
    "memory": {
      "protocol": "pnix.world.memory.v1",
      "family": "x3d",
      "scene_kind": "spatial-3d-scene",
      "surfaces": ["x3d", "html", "js", "ai"],
      "replayable": true,
      "apply_contract": "append-only-owner-packet",
    },
    "interfaces": {
      "process_mode": "api-packet",
      "authoring_roles": ["scene", "frp", "physics", "hanim", "symbolic-world"],
      "runtime_roles": ["sync", "x3dom", "replay"],
      "pending_roles": ["wgpu"],
    },
  }))
}

fn split_xml_validate_errors(raw: &str) -> Vec<String> {
  let detail = raw.strip_prefix("xml validate: ").unwrap_or(raw).trim();
  if detail.is_empty() {
    return Vec::new();
  }
  let mut out = Vec::new();
  for line in detail.split("; ") {
    let line = line.trim();
    if !line.is_empty() {
      out.push(line.to_string());
    }
  }
  out
}

fn frp_graph_json_from_xml_json(json: &JsonValue) -> JsonValue {
  let nodes = xml_x3d_core::xml_nodes_from_json(json);
  let graph = xml_x3d_core::x3d_route_graph_from_xml_nodes(&nodes);
  let bindings = xml_x3d_core::x3d_bindings(&graph);
  let mut builder = FrpJsonBuilder::new();
  let mut curve_outputs = HashSet::new();

  builder.add_time("time".to_string());
  builder.add_delta_time("dt".to_string());
  builder.add_constant("const.one".to_string(), 1.0);
  builder.add_constant("const.two".to_string(), 2.0);

  for def in sorted_keys(&graph.time_sensors) {
    let sensor = &graph.time_sensors[&def];
    add_time_sensor_signals(&mut builder, &def, sensor);
  }

  for target in sorted_keys(&bindings.transform) {
    let binding = &bindings.transform[&target];
    add_curve_signals(
      &mut builder,
      &mut curve_outputs,
      &target,
      transform_field_name(binding.field),
      &binding.interpolator,
      binding.time_sensor.as_ref(),
    );
  }

  for target in sorted_keys(&bindings.material) {
    let binding = &bindings.material[&target];
    add_curve_signals(
      &mut builder,
      &mut curve_outputs,
      &target,
      material_field_name(binding.field),
      &binding.interpolator,
      binding.time_sensor.as_ref(),
    );
  }

  for target in sorted_keys(&bindings.light) {
    let binding = &bindings.light[&target];
    add_curve_signals(
      &mut builder,
      &mut curve_outputs,
      &target,
      light_field_name(binding.field),
      &binding.interpolator,
      binding.time_sensor.as_ref(),
    );
  }

  let mut routes = Vec::with_capacity(graph.routes.len());
  let mut event_inputs = HashSet::with_capacity(graph.routes.len());
  let mut route_specs: Vec<_> = graph.routes.iter().collect();
  route_specs.sort_by(|left, right| {
    (
      left.from_node.as_str(),
      left.from_field.as_str(),
      left.to_node.as_str(),
      left.to_field.as_str(),
    )
      .cmp(&(
        right.from_node.as_str(),
        right.from_field.as_str(),
        right.to_node.as_str(),
        right.to_field.as_str(),
      ))
  });

  for route in route_specs {
    routes.push(serde_json::json!({
      "fromNode": route.from_node,
      "fromField": route.from_field,
      "toNode": route.to_node,
      "toField": route.to_field,
    }));

    if graph.interpolators.contains_key(&route.to_node)
      || graph.time_sensors.contains_key(&route.to_node)
    {
      continue;
    }

    let source_base = event_base(&route.from_node, &route.from_field);
    let target_field = xml_x3d_core::normalize_x3d_field_name(&route.to_field);
    let stride = target_field_stride(&target_field);
    let target_base = output_base(&route.to_node, &target_field);
    for suffix in component_suffixes(stride) {
      let source_name = if suffix.is_empty() {
        source_base.clone()
      } else {
        signal_with_suffix(&source_base, suffix)
      };
      if !builder.has_signal(&source_name) && event_inputs.insert(source_name.clone()) {
        builder.add_input(source_name.clone(), 0.0);
      }
      let output_name = if suffix.is_empty() {
        target_base.clone()
      } else {
        signal_with_suffix(&target_base, suffix)
      };
      if !curve_outputs.contains(&output_name) {
        builder.add_derived(
          output_name,
          "id",
          vec![source_name],
          Some(route_meta(route)),
        );
      }
    }
  }

  let mut out = builder.build();
  let object = out.as_object_mut().expect("frp graph root object");
  object.insert("routes".to_string(), JsonValue::Array(routes));
  out
}

fn frp_summary_json(frp: &JsonValue) -> JsonValue {
  let signals = frp
    .get("signals")
    .and_then(JsonValue::as_array)
    .cloned()
    .unwrap_or_default();
  let routes = frp
    .get("routes")
    .and_then(JsonValue::as_array)
    .cloned()
    .unwrap_or_default();
  let external_inputs = frp
    .get("external_inputs")
    .and_then(JsonValue::as_object)
    .cloned()
    .unwrap_or_default();

  serde_json::json!({
    "graph": frp.clone(),
    "signal_count": signals.len(),
    "route_count": routes.len(),
    "external_input_count": external_inputs.len(),
  })
}

fn physics_summary_json(scene: &JsonValue, graph: &xml_x3d_core::X3dRouteGraph) -> JsonValue {
  let detected_tags = collect_x3d_tags(scene, is_x3d_physics_tag);
  serde_json::json!({
    "present": !detected_tags.is_empty() || !graph.solvers.is_empty() || !graph.constraints.is_empty(),
    "detected_tags": detected_tags,
    "solver_defs": sorted_keys(&graph.solvers),
    "constraint_defs": sorted_keys(&graph.constraints),
    "sensor_defs": sorted_keys(&graph.sensors),
    "event_utils": sorted_keys(&graph.event_utils),
  })
}

fn hanim_summary_json(scene: &JsonValue, graph: &xml_x3d_core::X3dRouteGraph) -> JsonValue {
  let hierarchy = pnix_hanim_core::extract_skeleton_hierarchy(scene).ok();
  let valid_hierarchy = pnix_hanim_core::validate_joint_hierarchy(scene).is_ok();
  let non_standard_joint_names =
    pnix_hanim_core::validate_standard_joint_names(scene).unwrap_or_default();
  let animations = pnix_hanim_core::extract_animations(scene).unwrap_or_default();
  let transforms = pnix_hanim_core::calculate_forward_kinematics(scene).unwrap_or_default();

  let hierarchy_joints = hierarchy
    .as_ref()
    .map(|hierarchy| {
      let mut out = Vec::with_capacity(hierarchy.joints.len());
      for joint in &hierarchy.joints {
        out.push(serde_json::json!({
          "name": &joint.name,
          "parent": &joint.parent,
          "children": &joint.children,
        }));
      }
      out
    })
    .unwrap_or_default();

  let animated_joints = {
    let mut names = HashSet::new();
    for animation in &animations {
      names.insert(animation.joint_name.clone());
    }
    let mut names: Vec<_> = names.into_iter().collect();
    names.sort();
    names
  };

  let mut motion_ids = Vec::with_capacity(animations.len());
  for animation in &animations {
    motion_ids.push(JsonValue::String(animation.motion_id.clone()));
  }

  serde_json::json!({
    "present": !graph.humanoids.is_empty() || !graph.joints.is_empty() || !graph.segments.is_empty() || !graph.sites.is_empty(),
    "humanoids": sorted_humanoids_json(&graph.humanoids),
    "joint_defs": sorted_keys(&graph.joints),
    "segment_defs": sorted_keys(&graph.segments),
    "site_defs": sorted_keys(&graph.sites),
    "root_joint": hierarchy.as_ref().and_then(|hierarchy| hierarchy.root_joint.clone()),
    "hierarchy_valid": valid_hierarchy,
    "hierarchy": hierarchy_joints,
    "non_standard_joint_names": non_standard_joint_names,
    "motion_ids": motion_ids,
    "animated_joints": animated_joints,
    "forward_kinematics_joint_count": transforms.len(),
  })
}

fn symbolic_world_summary_json(
  scene: &JsonValue,
  graph: &xml_x3d_core::X3dRouteGraph,
) -> JsonValue {
  let equations = symbolic_equations_json(scene, graph);
  let constraints = symbolic_constraints_json(graph);
  let state_variables = symbolic_state_variables(scene, graph);

  serde_json::json!({
    "present": !equations.is_empty() || !constraints.is_empty(),
    "mode": "symbolic-owner",
    "state_variable_count": state_variables.len(),
    "equation_count": equations.len(),
    "constraint_count": constraints.len(),
    "state_variables": state_variables,
    "equations": equations,
    "constraints": constraints,
    "projection_roles": ["math", "physics", "render", "control"],
  })
}

fn symbolic_state_variables(scene: &JsonValue, graph: &xml_x3d_core::X3dRouteGraph) -> Vec<String> {
  let mut state = HashSet::from([String::from("t"), String::from("dt")]);

  for def in sorted_keys(&graph.time_sensors) {
    state.insert(dotted_name(&def, "fraction_changed"));
  }

  for route in &graph.routes {
    let from_field = xml_x3d_core::normalize_x3d_field_name(&route.from_field);
    let to_field = xml_x3d_core::normalize_x3d_field_name(&route.to_field);
    state.insert(dotted_name(&route.from_node, &from_field));
    state.insert(dotted_name(&route.to_node, &to_field));
  }

  for def in transform_defs_from_scene(scene) {
    state.insert(dotted_name(&def, "translation"));
    state.insert(dotted_name(&def, "rotation"));
    state.insert(dotted_name(&def, "scale"));
  }

  if !graph.constraints.is_empty() || !collect_x3d_tags(scene, is_x3d_physics_tag).is_empty() {
    state.insert("position".to_string());
    state.insert("velocity".to_string());
    state.insert("acceleration".to_string());
    state.insert("position_next".to_string());
    state.insert("velocity_next".to_string());
  }

  let mut state: Vec<_> = state.into_iter().collect();
  state.sort();
  state
}

fn symbolic_equations_json(
  scene: &JsonValue,
  graph: &xml_x3d_core::X3dRouteGraph,
) -> Vec<JsonValue> {
  let mut equations = Vec::new();

  for def in sorted_keys(&graph.time_sensors) {
    let sensor = &graph.time_sensors[&def];
    let expr = SymExpr::mul2(
      SymExpr::var("t"),
      SymExpr::pow(
        SymExpr::constant(sensor.cycle_interval as f64),
        SymExpr::constant(-1.0),
      ),
    );
    equations.push(symbolic_equation_entry(
      dotted_name(&def, "fraction"),
      dotted_name(&def, "fraction_changed"),
      "clock-fraction",
      expr,
      Some(format!(
        "cycleInterval={} looped={}",
        sensor.cycle_interval, sensor.looped
      )),
    ));
  }

  let mut route_specs: Vec<_> = graph.routes.iter().collect();
  route_specs.sort_by(|left, right| {
    (
      left.from_node.as_str(),
      left.from_field.as_str(),
      left.to_node.as_str(),
      left.to_field.as_str(),
    )
      .cmp(&(
        right.from_node.as_str(),
        right.from_field.as_str(),
        right.to_node.as_str(),
        right.to_field.as_str(),
      ))
  });
  for route in route_specs {
    let from_field = xml_x3d_core::normalize_x3d_field_name(&route.from_field);
    let to_field = xml_x3d_core::normalize_x3d_field_name(&route.to_field);
    equations.push(symbolic_equation_entry(
      route_equation_id_parts(
        &route.from_node,
        &route.from_field,
        &route.to_node,
        &route.to_field,
      ),
      dotted_name(&route.to_node, &to_field),
      "route-binding",
      SymExpr::var(dotted_name(&route.from_node, &from_field)),
      None,
    ));
  }

  if !graph.constraints.is_empty()
    || !graph.solvers.is_empty()
    || !collect_x3d_tags(scene, is_x3d_physics_tag).is_empty()
  {
    equations.push(symbolic_equation_entry(
      "physics.integrate.position".to_string(),
      "position_next".to_string(),
      "physics-integration",
      SymExpr::add2(
        SymExpr::var("position"),
        SymExpr::mul2(SymExpr::var("velocity"), SymExpr::var("dt")),
      ),
      Some("explicit Euler integration seam".to_string()),
    ));
    equations.push(symbolic_equation_entry(
      "physics.integrate.velocity".to_string(),
      "velocity_next".to_string(),
      "physics-integration",
      SymExpr::add2(
        SymExpr::var("velocity"),
        SymExpr::mul2(SymExpr::var("acceleration"), SymExpr::var("dt")),
      ),
      Some("explicit Euler integration seam".to_string()),
    ));
  }

  equations
}

fn symbolic_constraints_json(graph: &xml_x3d_core::X3dRouteGraph) -> Vec<JsonValue> {
  let mut defs: Vec<_> = graph.constraints.iter().collect();
  defs.sort_by(|left, right| left.0.cmp(right.0));
  let mut out = Vec::with_capacity(defs.len());
  for (def, tag) in defs {
    let constraint_type = x3d_constraint_type(tag);
    let relation = match constraint_type {
      ConstraintType::Holonomic => "g(q)=0",
      ConstraintType::NonHolonomic => "A(q)qdot=0",
      ConstraintType::Unilateral => "g(q)>=0",
    };
    out.push(serde_json::json!({
      "id": def,
      "tag": tag,
      "constraint_type": serde_json::to_value(constraint_type).unwrap_or(JsonValue::String("holonomic".to_string())),
      "relation": relation,
      "symbolic": symbolic_expr_entry(SymExpr::var(dotted_name("constraint", def))),
    }));
  }
  out
}

fn symbolic_equation_entry(
  id: String,
  lhs: String,
  role: &str,
  expr: SymExpr,
  note: Option<String>,
) -> JsonValue {
  let mut entry = Map::new();
  entry.insert("id".to_string(), JsonValue::String(id));
  entry.insert("lhs".to_string(), JsonValue::String(lhs));
  entry.insert("role".to_string(), JsonValue::String(role.to_string()));
  entry.insert("expr".to_string(), symbolic_expr_entry(expr));
  if let Some(note) = note {
    entry.insert("note".to_string(), JsonValue::String(note));
  }
  JsonValue::Object(entry)
}

fn symbolic_expr_entry(expr: SymExpr) -> JsonValue {
  serde_json::json!({
    "summary": type_summary(&expr),
    "json": symexpr_to_json_value(&expr).unwrap_or(JsonValue::Null),
  })
}

fn x3d_constraint_type(tag: &str) -> ConstraintType {
  match tag {
    "slider-joint" | "motor-joint" => ConstraintType::NonHolonomic,
    "collision-sensor" | "collision-collection" | "collision-space" => ConstraintType::Unilateral,
    _ => ConstraintType::Holonomic,
  }
}

fn transform_defs_from_scene(scene: &JsonValue) -> Vec<String> {
  let mut defs = HashSet::new();
  collect_transform_defs_recursive(scene, &mut defs);
  let mut out = Vec::with_capacity(defs.len());
  for def in defs {
    out.push(def);
  }
  out.sort();
  out
}

fn collect_transform_defs_recursive(node: &JsonValue, defs: &mut HashSet<String>) {
  let Some(obj) = node.as_object() else {
    return;
  };
  if obj.get("kind").and_then(JsonValue::as_str) == Some("element") {
    if let Some(name) = obj.get("name").and_then(JsonValue::as_str) {
      if xml_x3d_core::normalize_x3d_xml_tag(name) == "transform" {
        if let Some(def) = obj
          .get("attrs")
          .and_then(JsonValue::as_object)
          .and_then(|attrs| attrs.get("DEF").or_else(|| attrs.get("def")))
          .and_then(JsonValue::as_str)
        {
          defs.insert(def.to_string());
        }
      }
    }
  }
  if let Some(children) = obj.get("children").and_then(JsonValue::as_array) {
    for child in children {
      collect_transform_defs_recursive(child, defs);
    }
  }
}

fn sorted_humanoids_json(
  humanoids: &HashMap<String, xml_x3d_core::HAnimHumanoidNode>,
) -> Vec<JsonValue> {
  let mut defs: Vec<_> = humanoids.keys().cloned().collect();
  defs.sort();
  let mut out = Vec::with_capacity(defs.len());
  for def in defs {
    let humanoid = &humanoids[&def];
    out.push(serde_json::json!({
      "def": humanoid.def,
      "version": humanoid.version,
    }));
  }
  out
}

fn collect_x3d_tags(node: &JsonValue, predicate: fn(&str) -> bool) -> Vec<String> {
  let mut tags = HashSet::new();
  collect_x3d_tags_recursive(node, predicate, &mut tags);
  let mut out = Vec::with_capacity(tags.len());
  for tag in tags {
    out.push(tag);
  }
  out.sort();
  out
}

fn collect_x3d_tags_recursive(
  node: &JsonValue,
  predicate: fn(&str) -> bool,
  tags: &mut HashSet<String>,
) {
  let Some(obj) = node.as_object() else {
    return;
  };
  if obj.get("kind").and_then(JsonValue::as_str) == Some("element") {
    if let Some(name) = obj.get("name").and_then(JsonValue::as_str) {
      let tag = xml_x3d_core::normalize_x3d_xml_tag(name);
      if predicate(&tag) {
        tags.insert(tag);
      }
    }
  }
  if let Some(children) = obj.get("children").and_then(JsonValue::as_array) {
    for child in children {
      collect_x3d_tags_recursive(child, predicate, tags);
    }
  }
}

fn is_x3d_physics_tag(tag: &str) -> bool {
  matches!(
    tag,
    "rigid-body"
      | "rigid-body-collection"
      | "collision-collection"
      | "collision-sensor"
      | "collision-space"
      | "particle-system"
      | "force-physics-model"
      | "wind-physics-model"
      | "ball-joint"
      | "double-axis-hinge-joint"
      | "motor-joint"
      | "single-axis-hinge-joint"
      | "slider-joint"
      | "universal-joint"
  ) || tag.ends_with("physics-model")
}

fn lower_sync_op_to_x3dom_html(op: &JsonValue) -> Result<JsonValue> {
  let op_obj = op
    .as_object()
    .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: patch op must be attrset"))?;
  let kind = op_obj
    .get("op")
    .and_then(JsonValue::as_str)
    .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: patch op missing op kind"))?;

  match kind {
    "mount" => {
      let node = op_obj
        .get("node")
        .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: mount missing node"))?;
      let target_pnix_address = op_obj
        .get("node_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| node.as_object().map(pnix_address_of))
        .unwrap_or_else(|| "root".to_string());
      Ok(serde_json::json!({
        "op": "mount",
        "target_pnix_address": target_pnix_address,
        "html": x3dom_fragment_from_xml_json(node),
      }))
    }
    "replace" => {
      let node = op_obj
        .get("node")
        .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: replace missing node"))?;
      let target_pnix_address = op_obj
        .get("node_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| node.as_object().map(pnix_address_of))
        .unwrap_or_else(|| "root".to_string());
      Ok(serde_json::json!({
        "op": "replace",
        "node_id": op_obj.get("node_id").cloned().unwrap_or(JsonValue::String("root".to_string())),
        "target_pnix_address": target_pnix_address,
        "html": x3dom_fragment_from_xml_json(node),
      }))
    }
    "replace_children" => {
      let children = op_obj
        .get("children")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: replace_children missing children"))?;
      let target_pnix_address = op_obj
        .get("node_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| {
          op_obj
            .get("node_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("root")
            .to_string()
        });
      let html = x3dom_children_fragment(children);
      Ok(serde_json::json!({
        "op": "replace_children",
        "node_id": op_obj.get("node_id").cloned().unwrap_or(JsonValue::String("root".to_string())),
        "target_pnix_address": target_pnix_address,
        "html": html,
      }))
    }
    "update_attrs" => {
      let attrs = op_obj
        .get("attrs")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| anyhow!("builtins.x3dX3domPatch: update_attrs missing attrs"))?;
      let target_pnix_address = op_obj
        .get("node_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| {
          op_obj
            .get("node_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("root")
            .to_string()
        });
      Ok(serde_json::json!({
        "op": "update_attrs",
        "node_id": op_obj.get("node_id").cloned().unwrap_or(JsonValue::String("root".to_string())),
        "target_pnix_address": target_pnix_address,
        "attrs": x3dom_attrs_json(attrs),
      }))
    }
    _ => Err(anyhow!(
      "builtins.x3dX3domPatch: unsupported sync op {}",
      kind
    )),
  }
}

fn x3dom_attrs_json(attrs: &Map<String, JsonValue>) -> JsonValue {
  let mut out = Map::new();
  let keys = sorted_json_object_keys(attrs);
  for key in keys {
    let Some(value) = attrs.get(&key) else {
      continue;
    };
    if let Some(rendered) = json_attr_string(value) {
      out.insert(key, JsonValue::String(rendered));
    }
  }
  JsonValue::Object(out)
}

fn json_attr_string(value: &JsonValue) -> Option<String> {
  match value {
    JsonValue::Null => None,
    JsonValue::Bool(flag) => Some(if *flag {
      "true".to_string()
    } else {
      "false".to_string()
    }),
    JsonValue::Number(number) => Some(number.to_string()),
    JsonValue::String(text) => Some(text.clone()),
    JsonValue::Array(items) => {
      let mut out = String::new();
      for item in items {
        let Some(part) = json_attr_string(item) else {
          continue;
        };
        if !out.is_empty() {
          out.push(' ');
        }
        out.push_str(&part);
      }
      (!out.is_empty()).then_some(out)
    }
    JsonValue::Object(_) => None,
  }
}

fn x3dom_fragment_from_xml_json(node: &JsonValue) -> String {
  match node.as_object() {
    Some(obj) => match obj.get("kind").and_then(JsonValue::as_str) {
      Some("document") => obj
        .get("children")
        .and_then(JsonValue::as_array)
        .map(|children| x3dom_children_fragment(children))
        .unwrap_or_default(),
      Some("text") => obj
        .get("value")
        .and_then(JsonValue::as_str)
        .map(|text| x3dom_escape_text(text).into_owned())
        .unwrap_or_default(),
      Some("element") => x3dom_element_fragment(obj),
      _ => String::new(),
    },
    None => String::new(),
  }
}

fn x3dom_element_fragment(obj: &Map<String, JsonValue>) -> String {
  let name = obj
    .get("name")
    .and_then(JsonValue::as_str)
    .unwrap_or("Node")
    .to_string();
  let raw_attr_count = obj
    .get("attrs")
    .and_then(JsonValue::as_object)
    .map(Map::len)
    .unwrap_or(0);
  let mut attrs = Vec::with_capacity(raw_attr_count + 4);

  if let Some(sync_id) = obj.get("sync-id").and_then(JsonValue::as_str) {
    attrs.push(("data-node-id".to_string(), sync_id.to_string()));
    attrs.push(("data-sync-id".to_string(), sync_id.to_string()));
  }
  if let Some(pnix_address) = obj.get("pnix-address").and_then(JsonValue::as_str) {
    attrs.push(("data-pnix-address".to_string(), pnix_address.to_string()));
  }
  attrs.push((
    "data-x3d-tag".to_string(),
    xml_x3d_core::normalize_x3d_xml_tag(&name),
  ));

  if let Some(raw_attrs) = obj.get("attrs").and_then(JsonValue::as_object) {
    let keys = sorted_json_object_keys(raw_attrs);
    for key in keys {
      let Some(value) = raw_attrs.get(&key) else {
        continue;
      };
      if let Some(rendered) = json_attr_string(value) {
        attrs.push((key, rendered));
      }
    }
  }

  let children = obj
    .get("children")
    .and_then(JsonValue::as_array)
    .map(|children| x3dom_children_fragment(children))
    .unwrap_or_default();

  emit_x3dom_element(&name, attrs, &children)
}

fn emit_x3dom_element(name: &str, mut attrs: Vec<(String, String)>, children: &str) -> String {
  attrs.sort_by(|left, right| left.0.cmp(&right.0));
  let attr_len: usize = attrs
    .iter()
    .map(|(key, value)| key.len() + value.len() + 4)
    .sum();
  let mut out = String::with_capacity(name.len() * 2 + children.len() + attr_len + 5);
  out.push('<');
  out.push_str(name);
  for (key, value) in attrs {
    out.push(' ');
    out.push_str(&key);
    out.push_str("=\"");
    out.push_str(&x3dom_escape_attr(&value));
    out.push('"');
  }
  out.push('>');
  out.push_str(children);
  out.push_str("</");
  out.push_str(name);
  out.push('>');
  out
}

fn x3dom_children_fragment(children: &[JsonValue]) -> String {
  let mut out = String::new();
  for child in children {
    out.push_str(&x3dom_fragment_from_xml_json(child));
  }
  out
}

fn x3dom_document_from_fragment(fragment: &str) -> String {
  const PREFIX: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>pnix-x3dom-ssr</title></head><body data-pnix-x3d-runtime=\"ssr\"><div id=\"pnix-x3d-root\" data-pnix-x3d-root=\"true\">";
  const SUFFIX: &str = "</div></body></html>";
  let mut out = String::with_capacity(PREFIX.len() + fragment.len() + SUFFIX.len());
  out.push_str(PREFIX);
  out.push_str(fragment);
  out.push_str(SUFFIX);
  out
}

fn x3dom_escape_text(value: &str) -> Cow<'_, str> {
  x3dom_escape(value, false)
}

fn x3dom_escape_attr(value: &str) -> Cow<'_, str> {
  x3dom_escape(value, true)
}

fn x3dom_escape(value: &str, attr: bool) -> Cow<'_, str> {
  let Some((start, first_char)) = value.char_indices().find(|(_, ch)| match ch {
    '&' | '<' | '>' => true,
    '"' if attr => true,
    _ => false,
  }) else {
    return Cow::Borrowed(value);
  };

  let mut out = String::with_capacity(value.len() + 8);
  out.push_str(&value[..start]);
  push_x3dom_escaped_char(&mut out, first_char, attr);
  for ch in value[start + first_char.len_utf8()..].chars() {
    push_x3dom_escaped_char(&mut out, ch, attr);
  }
  Cow::Owned(out)
}

fn push_x3dom_escaped_char(out: &mut String, ch: char, attr: bool) {
  match ch {
    '&' => out.push_str("&amp;"),
    '<' => out.push_str("&lt;"),
    '>' => out.push_str("&gt;"),
    '"' if attr => out.push_str("&quot;"),
    _ => out.push(ch),
  }
}

fn annotate_sync_ids(node: &mut JsonValue, parent_path: &str, sibling_index: usize) {
  match node {
    JsonValue::Object(obj) => {
      let name = obj
        .get("name")
        .and_then(JsonValue::as_str)
        .unwrap_or("node")
        .to_string();
      let sync_id = stable_sync_id(obj, parent_path, sibling_index, &name);
      obj.insert("sync-id".to_string(), JsonValue::String(sync_id.clone()));
      let pnix_address = stable_pnix_address(obj, parent_path, sibling_index, &name);
      obj.insert("pnix-address".to_string(), JsonValue::String(pnix_address));

      if let Some(children) = obj.get_mut("children").and_then(JsonValue::as_array_mut) {
        for (idx, child) in children.iter_mut().enumerate() {
          annotate_sync_ids(child, &sync_id, idx);
        }
      }
    }
    JsonValue::Array(items) => {
      for (idx, item) in items.iter_mut().enumerate() {
        annotate_sync_ids(item, parent_path, idx);
      }
    }
    _ => {}
  }
}

fn stable_sync_id(
  obj: &Map<String, JsonValue>,
  parent_path: &str,
  sibling_index: usize,
  name: &str,
) -> String {
  let attrs = obj.get("attrs").and_then(JsonValue::as_object);
  if let Some(def) = attrs
    .and_then(|attrs| attrs.get("DEF").or_else(|| attrs.get("def")))
    .and_then(JsonValue::as_str)
  {
    let component = signal_name_component(def);
    let mut out = String::with_capacity("def:".len() + component.len());
    out.push_str("def:");
    out.push_str(&component);
    return out;
  }
  if let Some(id) = attrs
    .and_then(|attrs| attrs.get("id"))
    .and_then(JsonValue::as_str)
  {
    let component = signal_name_component(id);
    let mut out = String::with_capacity("id:".len() + component.len());
    out.push_str("id:");
    out.push_str(&component);
    return out;
  }
  let name_component = signal_name_component(name);
  let mut out = String::with_capacity(parent_path.len() + 1 + name_component.len() + 3);
  out.push_str(parent_path);
  out.push('/');
  out.push_str(&name_component);
  out.push('[');
  push_usize_decimal(sibling_index, &mut out);
  out.push(']');
  out
}

fn stable_pnix_address(
  obj: &Map<String, JsonValue>,
  parent_path: &str,
  sibling_index: usize,
  name: &str,
) -> String {
  let attrs = obj.get("attrs").and_then(JsonValue::as_object);
  let name_component = signal_name_component(name);
  if let Some(def) = attrs
    .and_then(|attrs| attrs.get("DEF").or_else(|| attrs.get("def")))
    .and_then(JsonValue::as_str)
  {
    let def_component = signal_name_component(def);
    let mut out =
      String::with_capacity(parent_path.len() + 1 + name_component.len() + 1 + def_component.len());
    out.push_str(parent_path);
    out.push('/');
    out.push_str(&name_component);
    out.push('#');
    out.push_str(&def_component);
    return out;
  }
  if let Some(id) = attrs
    .and_then(|attrs| attrs.get("id"))
    .and_then(JsonValue::as_str)
  {
    let id_component = signal_name_component(id);
    let mut out =
      String::with_capacity(parent_path.len() + 1 + name_component.len() + 1 + id_component.len());
    out.push_str(parent_path);
    out.push('/');
    out.push_str(&name_component);
    out.push('#');
    out.push_str(&id_component);
    return out;
  }
  let mut out = String::with_capacity(parent_path.len() + 1 + name_component.len() + 2);
  out.push_str(parent_path);
  out.push('/');
  out.push_str(&name_component);
  out.push(':');
  push_usize_decimal(sibling_index, &mut out);
  out
}

fn diff_sync_nodes(previous: &JsonValue, next: &JsonValue, ops: &mut Vec<JsonValue>) {
  if previous == next {
    return;
  }

  let Some(previous_obj) = previous.as_object() else {
    ops.push(replace_sync_op(next));
    return;
  };
  let Some(next_obj) = next.as_object() else {
    ops.push(replace_sync_op(next));
    return;
  };

  let previous_id = sync_id_of(previous_obj);
  let next_id = sync_id_of(next_obj);
  let previous_name = previous_obj.get("name").and_then(JsonValue::as_str);
  let next_name = next_obj.get("name").and_then(JsonValue::as_str);
  let previous_kind = previous_obj.get("kind").and_then(JsonValue::as_str);
  let next_kind = next_obj.get("kind").and_then(JsonValue::as_str);

  if previous_id != next_id || previous_name != next_name || previous_kind != next_kind {
    ops.push(replace_sync_op(next));
    return;
  }

  if previous_obj.get("value") != next_obj.get("value") {
    ops.push(replace_sync_op(next));
    return;
  }

  if previous_obj.get("attrs") != next_obj.get("attrs") {
    ops.push(serde_json::json!({
      "op": "update_attrs",
      "node_id": next_id,
      "node_pnix_address": pnix_address_of(next_obj),
      "attrs": next_obj.get("attrs").cloned().unwrap_or(JsonValue::Object(Map::new())),
    }));
  }

  let previous_children = previous_obj
    .get("children")
    .and_then(JsonValue::as_array)
    .cloned()
    .unwrap_or_default();
  let next_children = next_obj
    .get("children")
    .and_then(JsonValue::as_array)
    .cloned()
    .unwrap_or_default();

  if previous_children.len() != next_children.len()
    || child_id_order(&previous_children) != child_id_order(&next_children)
  {
    ops.push(serde_json::json!({
      "op": "replace_children",
      "node_id": next_id,
      "node_pnix_address": pnix_address_of(next_obj),
      "children": next_children,
    }));
    return;
  }

  for (previous_child, next_child) in previous_children.iter().zip(next_children.iter()) {
    diff_sync_nodes(previous_child, next_child, ops);
  }
}

fn replace_sync_op(next: &JsonValue) -> JsonValue {
  serde_json::json!({
    "op": "replace",
    "node_id": next
      .as_object()
      .map(sync_id_of)
      .unwrap_or_else(|| "root".to_string()),
    "node_pnix_address": next
      .as_object()
      .map(pnix_address_of)
      .unwrap_or_else(|| "root".to_string()),
    "node": next,
  })
}

fn sync_id_of(obj: &Map<String, JsonValue>) -> String {
  obj
    .get("sync-id")
    .and_then(JsonValue::as_str)
    .unwrap_or("root")
    .to_string()
}

fn pnix_address_of(obj: &Map<String, JsonValue>) -> String {
  obj
    .get("pnix-address")
    .and_then(JsonValue::as_str)
    .unwrap_or("root")
    .to_string()
}

fn child_id_order(children: &[JsonValue]) -> Vec<String> {
  let mut out = Vec::with_capacity(children.len());
  for child in children {
    out.push(
      child
        .as_object()
        .map(sync_id_of)
        .unwrap_or_else(|| "<leaf>".to_string()),
    );
  }
  out
}

fn add_time_sensor_signals(
  builder: &mut FrpJsonBuilder,
  def: &str,
  sensor: &xml_x3d_core::X3dTimeSensorSpec,
) {
  let base = time_sensor_base(def);
  let cycle = if sensor.cycle_interval > 0.0 {
    sensor.cycle_interval as f64
  } else {
    DEFAULT_CYCLE_INTERVAL
  };
  let cycle_name = signal_with_suffix(&base, "cycle");
  let enabled_name = signal_with_suffix(&base, "enabled");
  let fraction_name = signal_with_suffix(&base, "fraction");
  let event_fraction_name = event_base(def, "fraction_changed");
  let event_active_name = event_base(def, "isActive");

  let mut meta = Map::new();
  meta.insert(
    "node_kind".to_string(),
    JsonValue::String("time-sensor".to_string()),
  );
  meta.insert("def".to_string(), JsonValue::String(def.to_string()));
  meta.insert("enabled".to_string(), JsonValue::Bool(sensor.enabled));
  meta.insert("loop".to_string(), JsonValue::Bool(sensor.looped));
  meta.insert(
    "cycleInterval".to_string(),
    JsonValue::Number(safe_number(cycle)),
  );

  builder.add_constant(cycle_name.clone(), cycle);
  builder.add_constant(enabled_name.clone(), if sensor.enabled { 1.0 } else { 0.0 });
  builder.add_derived(base, "id", vec![], Some(meta));
  builder.add_derived(
    fraction_name.clone(),
    "div",
    vec!["time".to_string(), cycle_name],
    None,
  );
  builder.add_derived(event_fraction_name, "id", vec![fraction_name], None);
  builder.add_derived(
    event_active_name,
    "gate_true",
    vec!["const.one".to_string(), enabled_name],
    None,
  );
}

fn add_curve_signals(
  builder: &mut FrpJsonBuilder,
  curve_outputs: &mut HashSet<String>,
  target: &str,
  field: &str,
  interpolator: &xml_x3d_core::X3dInterpolatorSpec,
  sensor: Option<&xml_x3d_core::X3dTimeSensorSpec>,
) {
  let stride = interpolator_stride(interpolator);
  let fraction_signal = sensor
    .map(|sensor| signal_with_suffix(&time_sensor_base(&sensor.def), "fraction"))
    .unwrap_or_else(|| "time".to_string());
  let base = output_base(target, field);
  let curve = curve_meta(interpolator);

  for (component, suffix) in component_suffixes(stride).iter().enumerate() {
    let name = if suffix.is_empty() {
      base.clone()
    } else {
      signal_with_suffix(&base, suffix)
    };
    let mut extra = Map::new();
    extra.insert("curve".to_string(), curve.clone());
    extra.insert(
      "component".to_string(),
      JsonValue::Number(Number::from(component as u64)),
    );
    builder.add_derived(
      name.clone(),
      "curve_sample",
      vec![fraction_signal.clone()],
      Some(extra),
    );
    curve_outputs.insert(name);
  }
}

fn curve_meta(interpolator: &xml_x3d_core::X3dInterpolatorSpec) -> JsonValue {
  serde_json::json!({
    "keys": interpolator.keys,
    "values": interpolator.values,
    "stride": interpolator_stride(interpolator),
    "interp": interpolator_interp(interpolator.kind),
  })
}

fn route_meta(route: &xml_x3d_core::X3dRouteSpec) -> Map<String, JsonValue> {
  let mut out = Map::new();
  out.insert(
    "route".to_string(),
    serde_json::json!({
      "fromNode": route.from_node,
      "fromField": route.from_field,
      "toNode": route.to_node,
      "toField": route.to_field,
    }),
  );
  out
}

fn interpolator_stride(interpolator: &xml_x3d_core::X3dInterpolatorSpec) -> usize {
  let stride = interpolator.stride();
  if stride > 0 {
    return stride;
  }
  1
}

fn interpolator_interp(kind: xml_x3d_core::X3dInterpolatorKind) -> &'static str {
  match kind {
    xml_x3d_core::X3dInterpolatorKind::Orientation => "slerp",
    _ => "linear",
  }
}

fn transform_field_name(field: xml_x3d_core::X3dTransformField) -> &'static str {
  match field {
    xml_x3d_core::X3dTransformField::Translation => "translation",
    xml_x3d_core::X3dTransformField::Rotation => "rotation",
    xml_x3d_core::X3dTransformField::Scale => "scale",
    xml_x3d_core::X3dTransformField::Center => "center",
    xml_x3d_core::X3dTransformField::ScaleOrientation => "scaleorientation",
  }
}

fn material_field_name(field: xml_x3d_core::X3dMaterialField) -> &'static str {
  match field {
    xml_x3d_core::X3dMaterialField::DiffuseColor => "diffusecolor",
    xml_x3d_core::X3dMaterialField::EmissiveColor => "emissivecolor",
    xml_x3d_core::X3dMaterialField::SpecularColor => "specularcolor",
    xml_x3d_core::X3dMaterialField::Metallic => "metallic",
    xml_x3d_core::X3dMaterialField::Roughness => "roughness",
    xml_x3d_core::X3dMaterialField::Shininess => "shininess",
    xml_x3d_core::X3dMaterialField::Transparency => "transparency",
    xml_x3d_core::X3dMaterialField::Opacity => "opacity",
    xml_x3d_core::X3dMaterialField::AmbientIntensity => "ambientintensity",
  }
}

fn light_field_name(field: xml_x3d_core::X3dLightField) -> &'static str {
  match field {
    xml_x3d_core::X3dLightField::Color => "color",
    xml_x3d_core::X3dLightField::Intensity => "intensity",
    xml_x3d_core::X3dLightField::Attenuation => "attenuation",
    xml_x3d_core::X3dLightField::Location => "location",
    xml_x3d_core::X3dLightField::Direction => "direction",
    xml_x3d_core::X3dLightField::AmbientIntensity => "ambientintensity",
    xml_x3d_core::X3dLightField::Radius => "radius",
    xml_x3d_core::X3dLightField::BeamWidth => "beamwidth",
    xml_x3d_core::X3dLightField::CutOffAngle => "cutoffangle",
  }
}

fn target_field_stride(field: &str) -> usize {
  match field {
    "translation" | "scale" | "center" | "diffusecolor" | "emissivecolor" | "specularcolor"
    | "color" | "attenuation" | "location" | "direction" => 3,
    "rotation" | "scaleorientation" => 4,
    _ => 1,
  }
}

fn component_suffixes(stride: usize) -> &'static [&'static str] {
  match stride {
    2 => &["x", "y"],
    3 => &["x", "y", "z"],
    4 => &["x", "y", "z", "w"],
    _ => &[""],
  }
}

fn output_base(node: &str, field: &str) -> String {
  let node = signal_name_component(node);
  let field = signal_name_component(field);
  let mut out = String::with_capacity("x3d.".len() + node.len() + 1 + field.len());
  out.push_str("x3d.");
  out.push_str(&node);
  out.push('.');
  out.push_str(&field);
  out
}

fn event_base(node: &str, field: &str) -> String {
  let node = signal_name_component(node);
  let field = signal_name_component(field);
  let mut out = String::with_capacity("event.".len() + node.len() + 1 + field.len());
  out.push_str("event.");
  out.push_str(&node);
  out.push('.');
  out.push_str(&field);
  out
}

fn time_sensor_base(def: &str) -> String {
  let def = signal_name_component(def);
  let mut out = String::with_capacity("time_sensor.".len() + def.len());
  out.push_str("time_sensor.");
  out.push_str(&def);
  out
}

fn signal_with_suffix(base: &str, suffix: &str) -> String {
  dotted_name(base, suffix)
}

fn dotted_name(base: &str, suffix: &str) -> String {
  let mut out = String::with_capacity(base.len() + 1 + suffix.len());
  out.push_str(base);
  out.push('.');
  out.push_str(suffix);
  out
}

fn route_equation_id_parts(
  from_node: &str,
  from_field: &str,
  to_node: &str,
  to_field: &str,
) -> String {
  let mut out = String::with_capacity(
    "route:".len()
      + from_node.len()
      + 1
      + from_field.len()
      + "->".len()
      + to_node.len()
      + 1
      + to_field.len(),
  );
  out.push_str("route:");
  out.push_str(from_node);
  out.push(':');
  out.push_str(from_field);
  out.push_str("->");
  out.push_str(to_node);
  out.push(':');
  out.push_str(to_field);
  out
}

fn signal_name_component(raw: &str) -> String {
  let mut out = String::new();
  for ch in raw.chars() {
    if ch.is_ascii_alphanumeric() {
      out.push(ch.to_ascii_lowercase());
    } else {
      out.push('_');
    }
  }
  if out.is_empty() {
    "_".to_string()
  } else {
    out
  }
}

fn push_usize_decimal(value: usize, out: &mut String) {
  if value == 0 {
    out.push('0');
    return;
  }

  let mut n = value;
  let mut buf = [0u8; 20];
  let mut len = 0;
  while n > 0 {
    buf[len] = b'0' + (n % 10) as u8;
    len += 1;
    n /= 10;
  }
  for idx in (0..len).rev() {
    out.push(buf[idx] as char);
  }
}

fn sorted_keys<T>(map: &HashMap<String, T>) -> Vec<String> {
  let mut out = Vec::with_capacity(map.len());
  for key in map.keys() {
    out.push(key.clone());
  }
  out.sort_unstable();
  out
}

fn sorted_json_object_keys(map: &Map<String, JsonValue>) -> Vec<String> {
  let mut out = Vec::with_capacity(map.len());
  for key in map.keys() {
    out.push(key.clone());
  }
  out.sort();
  out
}

fn safe_number(value: f64) -> Number {
  Number::from_f64(value).unwrap_or_else(|| Number::from(0))
}

struct FrpJsonBuilder {
  signals: Vec<JsonValue>,
  names: HashSet<String>,
  external_inputs: Map<String, JsonValue>,
}

impl FrpJsonBuilder {
  fn new() -> Self {
    Self {
      signals: Vec::new(),
      names: HashSet::new(),
      external_inputs: Map::new(),
    }
  }

  fn has_signal(&self, name: &str) -> bool {
    self.names.contains(name)
  }

  fn add_time(&mut self, name: String) {
    self.add_signal(serde_json::json!({
      "name": name,
      "kind": "time",
    }));
  }

  fn add_delta_time(&mut self, name: String) {
    self.add_signal(serde_json::json!({
      "name": name,
      "kind": "delta_time",
    }));
  }

  fn add_constant(&mut self, name: String, value: f64) {
    self.add_signal(serde_json::json!({
      "name": name,
      "kind": "constant",
      "value": value,
    }));
  }

  fn add_input(&mut self, name: String, default: f64) {
    self
      .external_inputs
      .entry(name.clone())
      .or_insert_with(|| JsonValue::Number(safe_number(default)));
    self.add_signal(serde_json::json!({
      "name": name,
      "kind": "input",
      "default": default,
    }));
  }

  fn add_derived(
    &mut self,
    name: String,
    op: &str,
    deps: Vec<String>,
    extra: Option<Map<String, JsonValue>>,
  ) {
    let mut signal = Map::new();
    signal.insert("name".to_string(), JsonValue::String(name));
    signal.insert("kind".to_string(), JsonValue::String("derived".to_string()));
    signal.insert("op".to_string(), JsonValue::String(op.to_string()));
    signal.insert(
      "deps".to_string(),
      JsonValue::Array({
        let mut out = Vec::with_capacity(deps.len());
        for dep in deps {
          out.push(JsonValue::String(dep));
        }
        out
      }),
    );
    if let Some(extra) = extra {
      signal.extend(extra);
    }
    self.add_signal(JsonValue::Object(signal));
  }

  fn add_signal(&mut self, signal: JsonValue) {
    let Some(name) = signal
      .as_object()
      .and_then(|obj| obj.get("name"))
      .and_then(JsonValue::as_str)
    else {
      return;
    };
    if self.names.insert(name.to_string()) {
      self.signals.push(signal);
    }
  }

  fn build(self) -> JsonValue {
    let mut out = Map::new();
    out.insert("signals".to_string(), JsonValue::Array(self.signals));
    out.insert(
      "external_inputs".to_string(),
      JsonValue::Object(self.external_inputs),
    );
    JsonValue::Object(out)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn x3d_json_attr_array_preserves_join_surface() {
    let value = JsonValue::Array(vec![
      JsonValue::String("0".to_string()),
      JsonValue::Number(serde_json::Number::from(10)),
      JsonValue::Null,
      JsonValue::Bool(true),
    ]);
    assert_eq!(json_attr_string(&value).as_deref(), Some("0 10 true"));
  }

  #[test]
  fn x3dom_children_fragment_preserves_concat_surface() {
    let children = vec![
      serde_json::json!({
        "kind": "text",
        "value": "a<&",
      }),
      serde_json::json!({
        "kind": "element",
        "name": "Shape",
        "attrs": {},
        "children": [],
      }),
    ];
    assert_eq!(
      x3dom_children_fragment(&children),
      "a&lt;&amp;<Shape data-x3d-tag=\"shape\"></Shape>"
    );
  }

  #[test]
  fn x3dom_document_from_fragment_preserves_wrapper_surface() {
    assert_eq!(
      x3dom_document_from_fragment("<Shape></Shape>"),
      "<!doctype html><html><head><meta charset=\"utf-8\"><title>pnix-x3dom-ssr</title></head><body data-pnix-x3d-runtime=\"ssr\"><div id=\"pnix-x3d-root\" data-pnix-x3d-root=\"true\"><Shape></Shape></div></body></html>"
    );
  }

  #[test]
  fn stable_sync_ids_preserve_def_id_and_path_surface() {
    let def_node = serde_json::json!({
      "attrs": {
        "DEF": "Arm Joint",
      },
    });
    let id_node = serde_json::json!({
      "attrs": {
        "id": "Node-1",
      },
    });
    let plain_node = serde_json::json!({
      "attrs": {},
    });
    let def_obj = def_node.as_object().expect("def node object");
    let id_obj = id_node.as_object().expect("id node object");
    let plain_obj = plain_node.as_object().expect("plain node object");

    assert_eq!(
      stable_sync_id(def_obj, "root", 2, "Transform"),
      "def:arm_joint"
    );
    assert_eq!(
      stable_pnix_address(def_obj, "root", 2, "Transform"),
      "root/transform#arm_joint"
    );
    assert_eq!(stable_sync_id(id_obj, "root", 3, "Shape"), "id:node_1");
    assert_eq!(
      stable_pnix_address(id_obj, "root", 3, "Shape"),
      "root/shape#node_1"
    );
    assert_eq!(
      stable_sync_id(plain_obj, "root", 12, "IndexedFaceSet"),
      "root/indexedfaceset[12]"
    );
    assert_eq!(
      stable_pnix_address(plain_obj, "root", 12, "IndexedFaceSet"),
      "root/indexedfaceset:12"
    );
  }

  #[test]
  fn push_usize_decimal_preserves_surface() {
    let mut out = String::new();
    push_usize_decimal(0, &mut out);
    out.push(',');
    push_usize_decimal(42, &mut out);
    out.push(',');
    push_usize_decimal(usize::MAX, &mut out);
    assert_eq!(out, format!("0,42,{}", usize::MAX));
  }

  #[test]
  fn x3d_signal_base_builders_preserve_surface() {
    assert_eq!(
      output_base("Main Transform", "set_translation"),
      "x3d.main_transform.set_translation"
    );
    assert_eq!(
      event_base("Clock-1", "fraction_changed"),
      "event.clock_1.fraction_changed"
    );
    assert_eq!(time_sensor_base("Clock-1"), "time_sensor.clock_1");
  }

  #[test]
  fn x3d_signal_suffix_builder_preserves_surface() {
    assert_eq!(
      signal_with_suffix("time_sensor.clock_1", "fraction"),
      "time_sensor.clock_1.fraction"
    );
    assert_eq!(
      signal_with_suffix("x3d.main_transform.set_translation", "x"),
      "x3d.main_transform.set_translation.x"
    );
  }

  #[test]
  fn x3d_dotted_name_preserves_symbolic_surface() {
    assert_eq!(
      dotted_name("Clock-1", "fraction_changed"),
      "Clock-1.fraction_changed"
    );
    assert_eq!(
      dotted_name("constraint", "JointLimit"),
      "constraint.JointLimit"
    );
  }

  #[test]
  fn x3d_route_equation_id_preserves_symbolic_surface() {
    assert_eq!(
      route_equation_id_parts(
        "Touch Sensor",
        "fraction_changed",
        "Main Transform",
        "set_translation"
      ),
      "route:Touch Sensor:fraction_changed->Main Transform:set_translation"
    );
  }

  #[test]
  fn x3d_schema_validate_reports_unknown_attr() {
    let report = x3d_schema_validate(&Value::AttrSet(Arc::new(std::collections::BTreeMap::from(
      [
        ("kind".to_string(), Value::String("element".to_string())),
        ("name".to_string(), Value::String("Transform".to_string())),
        (
          "attrs".to_string(),
          Value::AttrSet(Arc::new(std::collections::BTreeMap::from([(
            "bogus".to_string(),
            Value::String("1".to_string()),
          )]))),
        ),
        ("children".to_string(), Value::List(Arc::new(vec![]))),
      ],
    ))))
    .expect("validate");

    let Value::AttrSet(map) = report else {
      panic!("x3d schema validate must return attrset");
    };
    assert!(matches!(map.get("ok"), Some(Value::Bool(false))));
  }

  #[test]
  fn x3d_frp_graph_contains_signals_for_curve_outputs() {
    let value = x3d_frp_graph(&Value::String(
      "<X3D><Scene><Transform DEF='Mover'/><TimeSensor DEF='Clock' cycleInterval='2' loop='true'/><PositionInterpolator DEF='Move' key='0 1' keyValue='0 0 0  1 0 0'/><ROUTE fromNode='Clock' fromField='fraction_changed' toNode='Move' toField='set_fraction'/><ROUTE fromNode='Move' fromField='value_changed' toNode='Mover' toField='translation'/></Scene></X3D>".to_string(),
    ))
    .expect("frp graph");

    let Value::AttrSet(map) = value else {
      panic!("x3d frp graph must return attrset");
    };
    let Some(Value::List(signals)) = map.get("signals") else {
      panic!("signals missing");
    };
    assert!(signals.iter().any(|signal| {
      matches!(signal, Value::AttrSet(attrs) if matches!(attrs.get("name"), Some(Value::String(name)) if name == "x3d.mover.translation.x"))
    }));
  }

  #[test]
  fn x3d_sync_plan_prefers_attr_update_over_full_replace() {
    let plan = x3d_sync_plan(
      &Value::String(
        "<X3D><Scene><Transform DEF='Mover' translation='0 0 0'/></Scene></X3D>".to_string(),
      ),
      &Value::String(
        "<X3D><Scene><Transform DEF='Mover' translation='1 0 0'/></Scene></X3D>".to_string(),
      ),
    )
    .expect("sync plan");

    let Value::AttrSet(map) = plan else {
      panic!("x3d sync plan must return attrset");
    };
    assert!(matches!(map.get("mode"), Some(Value::String(mode)) if mode == "patch"));
    assert!(matches!(map.get("changed"), Some(Value::Bool(true))));
    let Some(Value::List(ops)) = map.get("ops") else {
      panic!("ops missing");
    };
    assert!(ops.iter().any(|op| {
      matches!(op, Value::AttrSet(attrs) if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "update_attrs"))
    }));
  }

  #[test]
  fn x3d_x3dom_fragment_emits_sync_annotations() {
    let fragment = x3d_x3dom_fragment(&Value::String(
      "<X3D><Scene><Transform DEF='Mover'><Shape><Box size='1 1 1'/></Shape></Transform></Scene></X3D>"
        .to_string(),
    ))
    .expect("fragment");

    let Value::String(html) = fragment else {
      panic!("x3d x3dom fragment must return string");
    };
    assert!(html.contains("data-node-id=\"def:mover\""));
    assert!(html.contains("data-x3d-tag=\"transform\""));
    assert!(html.contains("data-pnix-address=\"root/x3d[0]/scene[0]/transform#mover\""));
    assert!(html.contains("<Transform"));
  }

  #[test]
  fn x3dom_escape_borrows_when_unchanged() {
    assert!(matches!(
      x3dom_escape_text("plain text"),
      Cow::Borrowed("plain text")
    ));
    assert!(matches!(
      x3dom_escape_attr("def:mover"),
      Cow::Borrowed("def:mover")
    ));
  }

  #[test]
  fn x3dom_escape_owns_when_escaping_needed() {
    assert_eq!(x3dom_escape_text("a<&>\"'b").as_ref(), "a&lt;&amp;&gt;\"'b");
    assert_eq!(
      x3dom_escape_attr("a<&>\"'b").as_ref(),
      "a&lt;&amp;&gt;&quot;'b"
    );
  }

  #[test]
  fn x3d_x3dom_patch_lowers_update_attrs_to_html_patch() {
    let patch = x3d_x3dom_patch(
      &Value::String(
        "<X3D><Scene><Transform DEF='Mover' translation='0 0 0'/></Scene></X3D>".to_string(),
      ),
      &Value::String(
        "<X3D><Scene><Transform DEF='Mover' translation='1 0 0'/></Scene></X3D>".to_string(),
      ),
    )
    .expect("patch");

    let Value::AttrSet(map) = patch else {
      panic!("x3d x3dom patch must return attrset");
    };
    assert!(matches!(
      map.get("protocol"),
      Some(Value::String(protocol)) if protocol == "pnix.x3dom.patch.v1"
    ));
    assert!(matches!(
      map.get("fragment"),
      Some(Value::String(fragment)) if fragment.contains("data-node-id=\"def:mover\"")
        && fragment.contains("data-pnix-address=\"root/x3d[0]/scene[0]/transform#mover\"")
    ));
    let Some(Value::List(ops)) = map.get("ops") else {
      panic!("ops missing");
    };
    assert!(
      ops.iter().any(|op| {
        matches!(
          op,
          Value::AttrSet(attrs)
            if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "update_attrs")
              && matches!(attrs.get("target_pnix_address"), Some(Value::String(address)) if address == "root/x3d[0]/scene[0]/transform#mover")
              && matches!(attrs.get("attrs"), Some(Value::AttrSet(rendered)) if matches!(rendered.get("translation"), Some(Value::String(value)) if value == "1.0 0.0 0.0"))
        )
      }),
      "unexpected html ops: {:?}",
      ops
    );
  }

  #[test]
  fn x3d_render_packet_carries_frp_physics_and_hanim_interfaces() {
    let packet = x3d_render_packet(
      &Value::String(
        "<X3D><Scene><Transform DEF='Mover' translation='0 0 0'/><HAnimHumanoid DEF='Avatar' version='2.0'><HAnimJoint DEF='Root' name='humanoid_root'/></HAnimHumanoid></Scene></X3D>".to_string(),
      ),
      &Value::String(
        "<X3D><Scene><Transform DEF='Mover' translation='1 0 0'/><TimeSensor DEF='Clock' cycleInterval='2' loop='true'/><PositionInterpolator DEF='Move' key='0 1' keyValue='0 0 0  1 0 0'/><ROUTE fromNode='Clock' fromField='fraction_changed' toNode='Move' toField='set_fraction'/><ROUTE fromNode='Move' fromField='value_changed' toNode='Mover' toField='translation'/><ParticleSystem DEF='Dust'><ForcePhysicsModel DEF='Gravity' enabled='true'/></ParticleSystem><HAnimHumanoid DEF='Avatar' version='2.0'><HAnimJoint DEF='Root' name='humanoid_root'><HAnimJoint DEF='Hip' name='l_hip_joint'/></HAnimJoint></HAnimHumanoid></Scene></X3D>".to_string(),
      ),
    )
    .expect("render packet");

    let Value::AttrSet(map) = packet else {
      panic!("x3d render packet must return attrset");
    };
    assert!(matches!(map.get("process_api"), Some(Value::Bool(true))));
    let Some(Value::AttrSet(simulation)) = map.get("simulation") else {
      panic!("simulation missing");
    };
    assert!(matches!(
      simulation.get("frp"),
      Some(Value::AttrSet(frp)) if matches!(frp.get("signal_count"), Some(Value::Int(count)) if *count > 0)
    ));
    assert!(matches!(
      simulation.get("physics"),
      Some(Value::AttrSet(physics)) if matches!(physics.get("detected_tags"), Some(Value::List(tags)) if tags.iter().any(|tag| matches!(tag, Value::String(name) if name == "force-physics-model")))
    ));
    assert!(matches!(
      simulation.get("hanim"),
      Some(Value::AttrSet(hanim)) if matches!(hanim.get("root_joint"), Some(Value::String(root)) if root == "humanoid_root")
    ));
    assert!(matches!(
      simulation.get("symbolic"),
      Some(Value::AttrSet(symbolic))
        if matches!(symbolic.get("equation_count"), Some(Value::Int(count)) if *count >= 4)
          && matches!(symbolic.get("constraint_count"), Some(Value::Int(count)) if *count >= 0)
          && matches!(symbolic.get("state_variables"), Some(Value::List(vars)) if vars.iter().any(|var| matches!(var, Value::String(name) if name == "Clock.fraction_changed")))
    ));
    assert!(matches!(
      map.get("memory"),
      Some(Value::AttrSet(memory))
        if matches!(memory.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.world.memory.v1")
          && matches!(memory.get("surfaces"), Some(Value::List(surfaces)) if surfaces.iter().any(|item| matches!(item, Value::String(name) if name == "x3d")))
    ));
    assert!(matches!(
      map.get("lowerings"),
      Some(Value::AttrSet(lowerings))
        if matches!(lowerings.get("x3dom"), Some(Value::AttrSet(x3dom)) if matches!(x3dom.get("fragment"), Some(Value::String(fragment)) if fragment.contains("data-pnix-address=\"root/x3d[0]/scene[0]/transform#mover\"")))
    ));
  }
}
