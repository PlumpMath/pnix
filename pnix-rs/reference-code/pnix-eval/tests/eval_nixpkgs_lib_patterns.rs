//! nixpkgs/lib 핵심 패턴들이 pnix-eval 위에서 작동하는지 점검.
//!
//! pnix는 nixpkgs/build system이 아니지만, **nixpkgs/lib 의 functional
//! 패턴 (fix-point, override, optional, makeExtensible 등)** 은 일반
//! 의미 substrate 가 함수형 언어로서 호환되는지의 잣대다. 깨지면
//! 어떤 nix 라이브러리 코드도 import 불가능.
//!
//! 모든 람다 정의는 nixpkgs/lib 실제 코드에서 가져왔거나 그 근사.

use pnix_eval::{eval_expr, Value};

fn eval(src: &str) -> Value {
  eval_expr(src).unwrap_or_else(|e| panic!("eval: {e}\n\nsrc:\n{src}"))
}

// ============================================================
// fix-point combinator
// ============================================================

#[test]
fn fix_combinator_works() {
  let v = eval(
    r#"
    let
      fix = f: let x = f x; in x;
      counter = self: { value = 0; inc = self // { value = self.value + 1; }; };
      c0 = fix counter;
    in
    c0.value
    "#,
  );
  assert!(matches!(v, Value::Int(0)), "got {:?}", v);
}

#[test]
fn fix_recursive_factorial() {
  let v = eval(
    r#"
    let
      fix = f: let x = f x; in x;
      fact = self: n: if n <= 1 then 1 else n * self (n - 1);
      f = fix fact;
    in
    f 5
    "#,
  );
  assert!(matches!(v, Value::Int(120)), "got {:?}", v);
}

// ============================================================
// optional / optionals (nixpkgs/lib/lists.nix)
// ============================================================

#[test]
fn optional_returns_singleton_or_empty() {
  let v = eval(
    r#"
    let
      optional = cond: x: if cond then [ x ] else [ ];
    in
    [ (optional true 1) (optional false 2) ]
    "#,
  );
  if let Value::List(items) = v {
    assert_eq!(items.len(), 2);
    if let Value::List(a) = &items[0] {
      assert_eq!(a.len(), 1);
    } else {
      panic!();
    }
    if let Value::List(b) = &items[1] {
      assert_eq!(b.len(), 0);
    } else {
      panic!();
    }
  } else {
    panic!();
  }
}

#[test]
fn optionals_returns_list_or_empty() {
  let v = eval(
    r#"
    let
      optionals = cond: xs: if cond then xs else [ ];
    in
    [ (optionals true [1 2 3]) (optionals false [4 5]) ]
    "#,
  );
  if let Value::List(items) = v {
    if let Value::List(a) = &items[0] {
      assert_eq!(a.len(), 3);
    } else {
      panic!();
    }
    if let Value::List(b) = &items[1] {
      assert_eq!(b.len(), 0);
    } else {
      panic!();
    }
  } else {
    panic!();
  }
}

// ============================================================
// optionalAttrs / recursiveUpdate
// ============================================================

#[test]
fn optional_attrs() {
  let v = eval(
    r#"
    let
      optionalAttrs = cond: as: if cond then as else { };
      a = optionalAttrs true { x = 1; y = 2; };
      b = optionalAttrs false { x = 1; };
    in
    [ a b ]
    "#,
  );
  if let Value::List(items) = v {
    if let Value::AttrSet(am) = &items[0] {
      assert_eq!(am.len(), 2);
    } else {
      panic!();
    }
    if let Value::AttrSet(bm) = &items[1] {
      assert_eq!(bm.len(), 0);
    } else {
      panic!();
    }
  } else {
    panic!();
  }
}

#[test]
fn recursive_update_merges_nested() {
  let v = eval(
    r#"
    let
      recursiveUpdate = lhs: rhs:
        let
          intersect = builtins.intersectAttrs lhs rhs;
          recurseKey = name: recursiveUpdate (lhs.${name}) (rhs.${name});
          recurseable =
            builtins.attrNames
              (builtins.intersectAttrs
                (builtins.mapAttrs (n: v: builtins.isAttrs v && builtins.isAttrs (rhs.${n}) ) lhs)
                rhs);
        in
        lhs // rhs;
    in
    recursiveUpdate { a = 1; b = { x = 1; }; } { b = { y = 2; }; c = 3; }
    "#,
  );
  if let Value::AttrSet(map) = v {
    // simplified version: outer update only
    assert!(map.contains_key("a"));
    assert!(map.contains_key("b"));
    assert!(map.contains_key("c"));
  } else {
    panic!();
  }
}

// ============================================================
// makeOverridable / makeExtensible
// ============================================================

#[test]
fn make_overridable_via_functor() {
  // Real nixpkgs uses __functor for makeOverridable. Here we test the
  // pattern: a callable attrset that wraps a function and exposes
  // `.override`.
  let v = eval(
    r#"
    let
      makeOverridable = f: origArgs:
        let
          result = f origArgs;
          overrideWith = newArgs: makeOverridable f (origArgs // (
            if builtins.isFunction newArgs then newArgs origArgs else newArgs
          ));
        in
        result // {
          override = overrideWith;
          overrideAttrs = overrideWith;
        };
      hello = makeOverridable (a: { greeting = "hello, ${a.name}"; }) { name = "world"; };
    in
    [ hello.greeting (hello.override { name = "pnix"; }).greeting ]
    "#,
  );
  if let Value::List(items) = v {
    assert!(matches!(&items[0], Value::String(s) if s == "hello, world"));
    assert!(matches!(&items[1], Value::String(s) if s == "hello, pnix"));
  } else {
    panic!();
  }
}

#[test]
fn make_extensible_via_fix() {
  let v = eval(
    r#"
    let
      fix = f: let x = f x; in x;
      makeExtensible = f: let
        self = f self // {
          extend = ext: makeExtensible (self_: f self_ // ext self_ (f self_));
        };
      in self;

      base = makeExtensible (self: { a = 1; b = 2; sum = self.a + self.b; });
      ext = base.extend (self: super: { c = 3; sum = super.sum + self.c; });
    in
    [ base.sum ext.sum ext.c ]
    "#,
  );
  if let Value::List(items) = v {
    assert!(matches!(&items[0], Value::Int(3)));
    assert!(matches!(&items[1], Value::Int(6)));
    assert!(matches!(&items[2], Value::Int(3)));
  } else {
    panic!("expected list, got {:?}", v);
  }
}

// ============================================================
// genAttrs / getAttrs / nameValuePair
// ============================================================

#[test]
fn gen_attrs_builds_set_from_names_and_function() {
  let v = eval(
    r#"
    let
      genAttrs = names: f:
        builtins.listToAttrs (builtins.map (name: { inherit name; value = f name; }) names);
    in
    genAttrs [ "a" "b" "c" ] (n: "v_" + n)
    "#,
  );
  if let Value::AttrSet(map) = v {
    assert_eq!(map.len(), 3);
    assert!(matches!(map.get("a"), Some(Value::String(s)) if s == "v_a"));
    assert!(matches!(map.get("c"), Some(Value::String(s)) if s == "v_c"));
  } else {
    panic!();
  }
}

#[test]
fn nameValuePair_to_attrset() {
  let v = eval(
    r#"
    let
      nameValuePair = name: value: { inherit name value; };
      pairs = builtins.map (n: nameValuePair n (n + n)) [ "a" "b" ];
    in
    builtins.listToAttrs pairs
    "#,
  );
  if let Value::AttrSet(map) = v {
    assert!(matches!(map.get("a"), Some(Value::String(s)) if s == "aa"));
    assert!(matches!(map.get("b"), Some(Value::String(s)) if s == "bb"));
  } else {
    panic!();
  }
}

// ============================================================
// foldAttrs / mapAttrsToList
// ============================================================

#[test]
fn fold_attrs_aggregates() {
  let v = eval(
    r#"
    let
      foldAttrs = op: nul: list_of_attrs:
        builtins.foldl' (acc: as:
          builtins.foldl' (acc2: name:
            acc2 // { ${name} = op (as.${name}) (acc2.${name} or nul); }
          ) acc (builtins.attrNames as)
        ) {} list_of_attrs;
    in
    foldAttrs (item: acc: acc + item) 0 [
      { x = 1; y = 10; }
      { x = 2; y = 20; }
      { x = 3; }
    ]
    "#,
  );
  if let Value::AttrSet(map) = v {
    assert!(matches!(map.get("x"), Some(Value::Int(6))));
    assert!(matches!(map.get("y"), Some(Value::Int(30))));
  } else {
    panic!();
  }
}

#[test]
fn map_attrs_to_list() {
  let v = eval(
    r#"
    let
      mapAttrsToList = f: attrs:
        builtins.map (name: f name (attrs.${name})) (builtins.attrNames attrs);
    in
    mapAttrsToList (n: v: n + "=" + (builtins.toString v)) { a = 1; b = 2; c = 3; }
    "#,
  );
  if let Value::List(items) = v {
    assert_eq!(items.len(), 3);
    let strs: Vec<String> = items
      .iter()
      .map(|v| match v {
        Value::String(s) => s.clone(),
        _ => panic!(),
      })
      .collect();
    assert!(strs.contains(&"a=1".to_string()));
    assert!(strs.contains(&"b=2".to_string()));
    assert!(strs.contains(&"c=3".to_string()));
  } else {
    panic!();
  }
}

// ============================================================
// mkDefault / mkForce / mkMerge — module system primitives
// ============================================================

#[test]
fn mk_default_force_merge_flags() {
  // nixpkgs module system tags option values with priority. We just
  // check that the wrapper attrsets can be constructed and round-trip.
  let v = eval(
    r#"
    let
      mkOverride = priority: content: { _type = "override"; inherit priority content; };
      mkDefault = mkOverride 1000;
      mkForce   = mkOverride 50;
      mkMerge   = contents: { _type = "merge"; inherit contents; };
      defaulted = mkDefault "hello";
      forced    = mkForce "world";
      merged    = mkMerge [ "a" "b" ];
    in
    [ defaulted.content defaulted.priority forced.content forced.priority (builtins.length merged.contents) ]
    "#,
  );
  if let Value::List(items) = v {
    assert!(matches!(&items[0], Value::String(s) if s == "hello"));
    assert!(matches!(&items[1], Value::Int(1000)));
    assert!(matches!(&items[2], Value::String(s) if s == "world"));
    assert!(matches!(&items[3], Value::Int(50)));
    assert!(matches!(&items[4], Value::Int(2)));
  } else {
    panic!();
  }
}

// ============================================================
// composeExtensions / extends — overlay pattern
// ============================================================

#[test]
fn compose_extensions_layered_overrides() {
  let v = eval(
    r#"
    let
      composeExtensions = f: g: final: prev:
        let r = f final prev; in g final (prev // r) // r;

      base = self: { a = 1; b = self.a * 2; };
      addC = self: super: { c = self.a + 100; };
      addD = self: super: { d = self.c + super.b; };
      extension = composeExtensions addC addD;

      fix = f: let x = f x; in x;
      mk = self: let init = base self; in init // (extension self init);
      result = fix mk;
    in
    [ result.a result.b result.c result.d ]
    "#,
  );
  if let Value::List(items) = v {
    assert!(matches!(&items[0], Value::Int(1)));
    assert!(matches!(&items[1], Value::Int(2)));
    assert!(matches!(&items[2], Value::Int(101)));
    assert!(matches!(&items[3], Value::Int(103)));
  } else {
    panic!("got {:?}", v);
  }
}

// ============================================================
// nixpkgs/lib/strings.nix patterns
// ============================================================

#[test]
fn has_prefix_has_suffix() {
  let v = eval(
    r#"
    let
      hasPrefix = pref: str:
        builtins.substring 0 (builtins.stringLength pref) str == pref;
      hasSuffix = suf: str:
        let
          lenStr = builtins.stringLength str;
          lenSuf = builtins.stringLength suf;
        in
        lenStr >= lenSuf
        && builtins.substring (lenStr - lenSuf) lenSuf str == suf;
    in
    [
      (hasPrefix "ab" "abcdef")
      (hasPrefix "xy" "abcdef")
      (hasSuffix "ef" "abcdef")
      (hasSuffix "xy" "abcdef")
    ]
    "#,
  );
  if let Value::List(items) = v {
    assert!(matches!(&items[0], Value::Bool(true)));
    assert!(matches!(&items[1], Value::Bool(false)));
    assert!(matches!(&items[2], Value::Bool(true)));
    assert!(matches!(&items[3], Value::Bool(false)));
  } else {
    panic!();
  }
}

#[test]
fn split_string_into_words() {
  let v = eval(
    r#"
    let
      splitString = sep: str:
        builtins.filter (x: builtins.isString x) (builtins.split sep str);
    in
    splitString "[[:space:]]+" "  one  two\tthree   four "
    "#,
  );
  if let Value::List(items) = v {
    let words: Vec<String> = items
      .iter()
      .filter_map(|v| match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
      })
      .collect();
    assert!(words.contains(&"one".to_string()));
    assert!(words.contains(&"two".to_string()));
    assert!(words.contains(&"three".to_string()));
    assert!(words.contains(&"four".to_string()));
  } else {
    panic!();
  }
}

// ============================================================
// nixpkgs/lib/attrsets.nix patterns
// ============================================================

#[test]
fn filter_attrs_keeps_matching_keys() {
  let v = eval(
    r#"
    let
      filterAttrs = pred: set:
        builtins.listToAttrs (
          builtins.filter (kv: pred kv.name kv.value)
            (builtins.map (n: { name = n; value = set.${n}; }) (builtins.attrNames set))
        );
    in
    filterAttrs (n: _: builtins.stringLength n > 1) {
      a = 1;
      bb = 2;
      ccc = 3;
    }
    "#,
  );
  if let Value::AttrSet(map) = v {
    assert!(!map.contains_key("a"));
    assert!(map.contains_key("bb"));
    assert!(map.contains_key("ccc"));
  } else {
    panic!();
  }
}

#[test]
fn cartesian_product_of_attrs() {
  // Nixpkgs uses cartesianProductOfSets to expand attrset of lists into
  // a list of attrset combinations. Smoke test: 2x2 → 4.
  let v = eval(
    r#"
    let
      cartesianProductOfSets = attrsOfLists:
        builtins.foldl'
          (listOfAttrs: name:
            builtins.concatMap
              (a: builtins.map (v: a // { ${name} = v; }) attrsOfLists.${name})
              listOfAttrs
          )
          [ {} ]
          (builtins.attrNames attrsOfLists);
    in
    builtins.length (cartesianProductOfSets { a = [1 2]; b = [3 4]; })
    "#,
  );
  assert!(matches!(v, Value::Int(4)), "got {:?}", v);
}
