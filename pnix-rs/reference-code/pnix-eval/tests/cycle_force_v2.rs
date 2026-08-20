//! Regression cover for the false-positive cycle detection that used
//! to leave nested function-call attrsets unforced (`"<thunk>"` in
//! JSON). Originally surfaced as the puck `pnix3d_scene.px` regression
//! where 3D objects stopped rendering.

use pnix_eval::eval_to_json;

fn assert_no_thunk(src: &str) {
  let json = eval_to_json(src, false).expect("eval ok");
  assert!(
    !json.contains("\"<thunk>\""),
    "deep_force left thunks unforced: {}",
    json
  );
  assert!(
    !json.contains("\"<cycle>\""),
    "spurious cycle marker (no real cycle in this fixture): {}",
    json
  );
}

#[test]
fn nested_function_call_depth3_attrset_only() {
  assert_no_thunk(
    r#"let mk = x: { proto = "v1"; obj = x; }; in { a = { b = { wrap = (mk [ "x" ]); }; }; }"#,
  );
}

#[test]
fn nested_function_call_with_list_intermediate() {
  assert_no_thunk(
    r#"let mk = x: { proto = "v1"; obj = x; }; in { commands = [ { args = { wrap = (mk [ "x" ]); }; } ]; }"#,
  );
}

#[test]
fn pnix3d_world_call_shape() {
  // Mirrors the absorbed PNIX3D scene shape now owned by pnixc-meta
  // puck-pnix3d policies (function call inside commands[0].args.pnix3d).
  assert_no_thunk(
    r#"let
        world = objects: {
          protocol = "pnix3d.world.v1";
          background_color = [ 0.78 0.84 0.87 ];
          objects = objects;
        };
      in {
        commands = [
          { action = "submit"; args = { pnix3d = (world [ "a" "b" "c" ]); }; }
        ];
      }"#,
  );
}

#[test]
fn genuine_self_cycle_still_terminates() {
  // Self-referential attrset must still terminate via "<cycle>"
  // marker (not infinite loop).
  let json = eval_to_json(r#"let as = { x = 1; y = as; }; in as"#, false).expect("eval ok");
  assert!(
    json.contains("\"<cycle>\""),
    "self-cycle should mark cycle, got {}",
    json
  );
}
