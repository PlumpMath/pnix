//! Recursive-descent parser: tokens -> `Program`. Zero dependencies.
//!
//! Covers the slice subset: `fn` / `struct` / `enum` / inherent `impl` items;
//! `let` / `return` / expression statements; arithmetic / boolean / comparison
//! expressions; `if`, blocks, calls, tuples, struct & enum literals,
//! field/tuple-index/method access, `match`, and `println!`.
//!
//! Struct literals are ambiguous with `if cond { ... }`, so they are disabled in
//! "no-struct" positions (the head expression of `if` / `match`) exactly like
//! rustc. Parentheses and block bodies re-enable them.

use crate::ast::*;
use crate::lexer::Tok;

pub fn parse_program(toks: &[Tok]) -> Result<Program, String> {
    let mut owned = Vec::new();
    for t in toks {
        owned.push(t.clone());
    }
    let mut p = Parser {
        toks: owned,
        pos: 0,
        no_struct: false,
        hoisted_funcs: Vec::new(),
        hoisted_structs: Vec::new(),
        hoisted_enums: Vec::new(),
        hoisted_impls: Vec::new(),
    };
    let mut funcs = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut impls = Vec::new();
    let mut traits = Vec::new();
    let mut aliases = Vec::new();
    let mut globals = Vec::new();
    let mut uses = Vec::new();
    while p.pos < p.toks.len() {
        let derives = p.parse_attrs()?;
        if p.at(&Tok::KwPub) {
            p.pos += 1;
        }
        match p.peek() {
            Some(Tok::KwFn) => funcs.push(p.parse_func()?),
            Some(Tok::KwStruct) => structs.push(p.parse_struct(derives)?),
            Some(Tok::KwEnum) => enums.push(p.parse_enum(derives)?),
            Some(Tok::KwImpl) => impls.push(p.parse_impl()?),
            Some(Tok::KwTrait) => traits.push(p.parse_trait_item()?),
            Some(Tok::KwConst) => {
                // `const fn ...` is a const function; the interpreter treats it
                // as an ordinary function (it does not track const-evaluability).
                if matches!(p.toks.get(p.pos + 1), Some(Tok::KwFn)) {
                    p.pos += 1;
                    funcs.push(p.parse_func()?);
                } else {
                    globals.push(p.parse_global(false)?);
                }
            }
            Some(Tok::KwStatic) => globals.push(p.parse_global(true)?),
            Some(Tok::KwUse) => uses.push(p.parse_use_item()?),
            Some(Tok::KwMod) => p.parse_mod_item()?,
            Some(Tok::Ident(s)) if s == "type" => aliases.push(p.parse_type_alias_item()?),
            other => {
                return Err(format!(
                "parse: expected item (fn/struct/enum/impl/trait/const/static/use/mod/type), found {:?}",
                other
            ))
            }
        }
    }
    // Items declared inside function bodies (`fn main() { struct P {..} .. }`)
    // are hoisted to the program level -- Rust's item hoisting is sound and the
    // interp/typeck use flat global item maps, so scope-nesting adds nothing.
    while let Some(f) = p.hoisted_funcs.pop() {
        funcs.push(f);
    }
    while let Some(s) = p.hoisted_structs.pop() {
        structs.push(s);
    }
    while let Some(e) = p.hoisted_enums.pop() {
        enums.push(e);
    }
    while let Some(i) = p.hoisted_impls.pop() {
        impls.push(i);
    }
    Ok(Program {
        funcs,
        structs,
        enums,
        impls,
        traits,
        aliases,
        globals,
        uses,
    })
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    no_struct: bool,
    hoisted_funcs: Vec<Func>,
    hoisted_structs: Vec<StructDef>,
    hoisted_enums: Vec<EnumDef>,
    hoisted_impls: Vec<ImplBlock>,
}

impl Parser {
    /// Consume a generic-closing `>`. The lexer greedily makes `>=`/`>>=` into
    /// Ge/ShrEq, so at a type-argument close split them: `>=` -> `>`(here) + `=`,
    /// and `>>=` -> `>`(here) + `>=`. This lets `Vec<i64>=v` (no space) parse.
    fn eat_gt(&mut self) -> Result<(), String> {
        let tok = self.toks.get(self.pos).cloned();
        match tok {
            Some(Tok::Gt) => {
                self.pos += 1;
                Ok(())
            }
            Some(Tok::Ge) => {
                self.toks[self.pos] = Tok::Eq;
                Ok(())
            }
            Some(Tok::ShrEq) => {
                self.toks[self.pos] = Tok::Ge;
                Ok(())
            }
            other => Err(format!("parse: expected '>', found {:?}", other)),
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn peek_n(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n)
    }
    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn eat(&mut self, want: &Tok) -> Result<(), String> {
        match self.peek() {
            Some(t) if t == want => {
                self.pos += 1;
                Ok(())
            }
            other => Err(format!(
                "parse: expected {:?}, found {:?} at token {}",
                want, other, self.pos
            )),
        }
    }
    fn at(&self, t: &Tok) -> bool {
        self.peek() == Some(t)
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.bump() {
            Some(Tok::Ident(s)) => Ok(s),
            other => Err(format!(
                "parse: expected identifier, found {:?} at token {}",
                other,
                self.pos.saturating_sub(1)
            )),
        }
    }

    // ---- items -------------------------------------------------------------

    fn skip_attrs(&mut self) -> Result<(), String> {
        self.parse_attrs()?;
        Ok(())
    }

    /// Consume attributes; `#[derive(A, B)]` names are collected and returned,
    /// every other attribute is skipped.
    fn parse_attrs(&mut self) -> Result<Vec<String>, String> {
        let mut derives = Vec::new();
        while self.at(&Tok::Hash) {
            self.pos += 1;
            self.eat(&Tok::LBracket)?;
            let is_derive = match self.peek() {
                Some(Tok::Ident(s)) => s == "derive",
                _ => false,
            };
            if is_derive {
                self.pos += 1;
                self.eat(&Tok::LParen)?;
                while !self.at(&Tok::RParen) {
                    derives.push(self.ident()?);
                    if self.at(&Tok::Comma) {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                self.eat(&Tok::RParen)?;
                self.eat(&Tok::RBracket)?;
            } else {
                let mut depth = 1usize;
                while depth > 0 {
                    match self.bump() {
                        Some(Tok::LBracket) => depth += 1,
                        Some(Tok::RBracket) => depth -= 1,
                        Some(_) => {}
                        None => return Err("parse: unterminated attribute".to_string()),
                    }
                }
            }
        }
        Ok(derives)
    }

    /// Consume a `use ...;` item and return its canonical source text.
    fn parse_use_item(&mut self) -> Result<String, String> {
        self.eat(&Tok::KwUse)?;
        let mut body = String::new();
        while !self.at(&Tok::Semi) {
            let tok = self
                .bump()
                .ok_or_else(|| "parse: unterminated use item".to_string())?;
            match tok {
                Tok::Ident(s) => body.push_str(&s),
                Tok::ColonColon => body.push_str("::"),
                Tok::LBrace => body.push('{'),
                Tok::RBrace => body.push('}'),
                Tok::Comma => body.push_str(", "),
                Tok::Star => body.push('*'),
                Tok::KwAs => body.push_str(" as "),
                other => {
                    return Err(format!("parse: unsupported token in use item: {:?}", other))
                }
            }
        }
        self.eat(&Tok::Semi)?;
        Ok(format!("use {};", body))
    }

    fn parse_type_alias_item(&mut self) -> Result<TypeAlias, String> {
        match self.bump() {
            Some(Tok::Ident(s)) if s == "type" => {}
            other => return Err(format!("parse: expected type alias, found {:?}", other)),
        }
        let name = self.ident()?;
        self.eat(&Tok::Eq)?;
        let ty = self.parse_type()?;
        self.eat(&Tok::Semi)?;
        Ok(TypeAlias { name, ty })
    }

    fn parse_global(&mut self, is_static: bool) -> Result<Global, String> {
        if is_static {
            self.eat(&Tok::KwStatic)?;
        } else {
            self.eat(&Tok::KwConst)?;
        }
        let name = self.ident()?;
        self.eat(&Tok::Colon)?;
        let ty = self.parse_type()?;
        self.eat(&Tok::Eq)?;
        let init = self.parse_expr()?;
        self.eat(&Tok::Semi)?;
        Ok(Global { name, ty, init })
    }

    fn parse_mod_item(&mut self) -> Result<(), String> {
        self.eat(&Tok::KwMod)?;
        self.ident()?;
        if self.at(&Tok::Semi) {
            self.pos += 1;
            return Ok(());
        }
        self.eat(&Tok::LBrace)?;
        let mut depth = 1usize;
        while depth > 0 {
            match self.bump() {
                Some(Tok::LBrace) => depth += 1,
                Some(Tok::RBrace) => depth -= 1,
                Some(_) => {}
                None => return Err("parse: unterminated mod item".to_string()),
            }
        }
        Ok(())
    }

    fn parse_trait_item(&mut self) -> Result<TraitDef, String> {
        self.eat(&Tok::KwTrait)?;
        let name = self.ident()?;
        self.skip_generic_params()?;
        // optional supertrait bounds `: A + B`
        if self.at(&Tok::Colon) {
            while !self.at(&Tok::LBrace) && self.peek().is_some() {
                self.pos += 1;
            }
        }
        self.eat(&Tok::LBrace)?;
        let mut methods = Vec::new();
        let mut decls = Vec::new();
        while !self.at(&Tok::RBrace) {
            self.skip_attrs()?;
            // Associated type declaration `type Item;` / `type Item: Bound;` --
            // ignored by the value interpreter; consume up to its `;`.
            if matches!(self.peek(), Some(Tok::Ident(s)) if s == "type") {
                while !self.at(&Tok::Semi) && self.peek().is_some() {
                    self.pos += 1;
                }
                self.eat(&Tok::Semi)?;
                continue;
            }
            self.eat(&Tok::KwFn)?;
            let mname = self.ident()?;
            let generics = self.parse_generic_params()?;
            self.eat(&Tok::LParen)?;
            let mut receiver = None;
            let mut params = Vec::new();
            if !self.at(&Tok::RParen) {
                receiver = self.parse_receiver()?;
                if receiver.is_some() && self.at(&Tok::Comma) {
                    self.pos += 1;
                }
                while !self.at(&Tok::RParen) {
                    let pname = self.ident()?;
                    self.eat(&Tok::Colon)?;
                    let ty = self.parse_type()?;
                    params.push(Param { name: pname, ty });
                    if self.at(&Tok::Comma) {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            }
            self.eat(&Tok::RParen)?;
            let ret = if self.at(&Tok::Arrow) {
                self.pos += 1;
                self.parse_type()?
            } else {
                Type::Unit
            };
            self.skip_where_clause();
            if self.at(&Tok::Semi) {
                // signature-only declaration -- kept for faithful emission.
                self.pos += 1;
                decls.push(TraitDecl {
                    name: mname,
                    generics,
                    receiver,
                    params,
                    ret,
                });
            } else {
                let body = self.parse_block()?;
                methods.push(Method {
                    generics,
                    name: mname,
                    receiver,
                    params,
                    ret,
                    body,
                });
            }
        }
        self.eat(&Tok::RBrace)?;
        Ok(TraitDef { name, methods, decls })
    }

    fn parse_func(&mut self) -> Result<Func, String> {
        self.eat(&Tok::KwFn)?;
        let name = self.ident()?;
        let generics = self.parse_generic_params()?;
        self.eat(&Tok::LParen)?;
        let mut params = Vec::new();
        while !self.at(&Tok::RParen) {
            let pname = self.ident()?;
            self.eat(&Tok::Colon)?;
            let ty = self.parse_type()?;
            params.push(Param { name: pname, ty });
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.eat(&Tok::RParen)?;
        let ret = if self.at(&Tok::Arrow) {
            self.pos += 1;
            self.parse_type()?
        } else {
            Type::Unit
        };
        self.skip_where_clause();
        let body = self.parse_block()?;
        Ok(Func {
            generics,
            name,
            params,
            ret,
            body,
        })
    }

    fn parse_struct(&mut self, derives: Vec<String>) -> Result<StructDef, String> {
        self.eat(&Tok::KwStruct)?;
        let name = self.ident()?;
        let generics = self.parse_generic_params()?;
        // Unit struct: `struct D;` (no braces) -> no fields, constructed bare.
        if self.at(&Tok::Semi) {
            self.pos += 1;
            return Ok(StructDef {
                name,
                generics,
                fields: Vec::new(),
                derives,
                unit: true,
                tuple: false,
            });
        }
        // Tuple struct: `struct P(T0, T1, ...);` -> positional fields "0","1",...
        if self.at(&Tok::LParen) {
            self.pos += 1;
            let mut fields = Vec::new();
            let mut idx = 0usize;
            while !self.at(&Tok::RParen) {
                if self.at(&Tok::KwPub) {
                    self.pos += 1;
                }
                let ty = self.parse_type()?;
                fields.push((format!("{}", idx), ty));
                idx += 1;
                if self.at(&Tok::Comma) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            self.eat(&Tok::RParen)?;
            self.eat(&Tok::Semi)?;
            return Ok(StructDef {
                name,
                generics,
                fields,
                derives,
                unit: false,
                tuple: true,
            });
        }
        self.eat(&Tok::LBrace)?;
        let mut fields = Vec::new();
        while !self.at(&Tok::RBrace) {
            self.skip_attrs()?;
            if self.at(&Tok::KwPub) {
                self.pos += 1;
            }
            let fname = self.ident()?;
            self.eat(&Tok::Colon)?;
            let ty = self.parse_type()?;
            fields.push((fname, ty));
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.eat(&Tok::RBrace)?;
        Ok(StructDef { name, generics, fields, derives, unit: false, tuple: false })
    }

    fn parse_enum(&mut self, derives: Vec<String>) -> Result<EnumDef, String> {
        self.eat(&Tok::KwEnum)?;
        let name = self.ident()?;
        let generics = self.parse_generic_params()?;
        self.eat(&Tok::LBrace)?;
        let mut variants = Vec::new();
        while !self.at(&Tok::RBrace) {
            self.skip_attrs()?;
            let vname = self.ident()?;
            let mut fields = Vec::new();
            let mut named_fields = Vec::new();
            if self.at(&Tok::LParen) {
                self.pos += 1;
                while !self.at(&Tok::RParen) {
                    fields.push(self.parse_type()?);
                    if self.at(&Tok::Comma) {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                self.eat(&Tok::RParen)?;
            } else if self.at(&Tok::LBrace) {
                self.pos += 1;
                while !self.at(&Tok::RBrace) {
                    let fname = self.ident()?;
                    self.eat(&Tok::Colon)?;
                    let fty = self.parse_type()?;
                    named_fields.push((fname, fty));
                    if self.at(&Tok::Comma) {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                self.eat(&Tok::RBrace)?;
            }
            variants.push(Variant {
                name: vname,
                fields,
                named_fields,
            });
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.eat(&Tok::RBrace)?;
        Ok(EnumDef { name, generics, variants, derives })
    }

    fn parse_impl(&mut self) -> Result<ImplBlock, String> {
        self.eat(&Tok::KwImpl)?;
        let generics = self.parse_generic_params()?;
        let first = self.parse_type()?;
        let (target, trait_name) = if self.at(&Tok::KwFor) {
            self.pos += 1;
            let tgt = self.parse_type()?;
            (tgt, Some(type_head_name(&first)))
        } else {
            (first, None)
        };
        self.eat(&Tok::LBrace)?;
        let mut methods = Vec::new();
        let mut consts = Vec::new();
        while !self.at(&Tok::RBrace) {
            self.skip_attrs()?;
            if self.at(&Tok::KwPub) {
                self.pos += 1;
            }
            // Associated type `type Item = Foo;` in a TRAIT impl -- ignored by the
            // value interpreter, so consume up to its `;`. (Inherent-impl assoc
            // types are unstable/rejected by rustc, so only trait impls skip here.)
            if trait_name.is_some() && matches!(self.peek(), Some(Tok::Ident(s)) if s == "type") {
                while !self.at(&Tok::Semi) && self.peek().is_some() {
                    self.pos += 1;
                }
                self.eat(&Tok::Semi)?;
                continue;
            }
            if self.at(&Tok::KwConst) {
                consts.push(self.parse_global(false)?);
            } else {
                methods.push(self.parse_method()?);
            }
        }
        self.eat(&Tok::RBrace)?;
        Ok(ImplBlock { generics, target, trait_name, methods, consts })
    }

    fn parse_method(&mut self) -> Result<Method, String> {
        self.eat(&Tok::KwFn)?;
        let name = self.ident()?;
        let generics = self.parse_generic_params()?;
        self.eat(&Tok::LParen)?;
        let mut receiver = None;
        let mut params = Vec::new();

        if !self.at(&Tok::RParen) {
            receiver = self.parse_receiver()?;
            if receiver.is_some() && self.at(&Tok::Comma) {
                self.pos += 1;
            }
            while !self.at(&Tok::RParen) {
                let pname = self.ident()?;
                self.eat(&Tok::Colon)?;
                let ty = self.parse_type()?;
                params.push(Param { name: pname, ty });
                if self.at(&Tok::Comma) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }

        self.eat(&Tok::RParen)?;
        let ret = if self.at(&Tok::Arrow) {
            self.pos += 1;
            self.parse_type()?
        } else {
            Type::Unit
        };
        self.skip_where_clause();
        let body = self.parse_block()?;
        Ok(Method {
            generics,
            name,
            receiver,
            params,
            ret,
            body,
        })
    }

    fn parse_receiver(&mut self) -> Result<Option<ReceiverKind>, String> {
        if self.at(&Tok::Amp) {
            self.pos += 1;
            let is_mut = if self.at(&Tok::KwMut) {
                self.pos += 1;
                true
            } else {
                false
            };
            match self.bump() {
                Some(Tok::Ident(s)) if s == "self" => {
                    return Ok(Some(if is_mut {
                        ReceiverKind::RefMut
                    } else {
                        ReceiverKind::Ref
                    }));
                }
                other => return Err(format!("parse: expected self receiver, found {:?}", other)),
            }
        }
        if matches!(self.peek(), Some(Tok::Ident(s)) if s == "self")
            && !matches!(self.peek_n(1), Some(Tok::Colon))
        {
            self.pos += 1;
            return Ok(Some(ReceiverKind::Value));
        }
        Ok(None)
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match self.peek().cloned() {
            Some(Tok::Amp) => {
                self.pos += 1;
                let mut lifetime = None;
                if let Some(Tok::Lifetime(lt)) = self.peek() {
                    lifetime = Some(lt.clone());
                    self.pos += 1;
                }
                let mutable = if self.at(&Tok::KwMut) {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                let inner = self.parse_type()?;
                match lifetime {
                    Some(lt) => Ok(Type::RefLt {
                        lifetime: lt,
                        mutable,
                        inner: Box::new(inner),
                    }),
                    None => Ok(Type::Ref {
                        mutable,
                        inner: Box::new(inner),
                    }),
                }
            }
            Some(Tok::KwImpl) => self.parse_impl_trait_type(),
            Some(Tok::Ident(s)) if s == "i64" => {
                self.pos += 1;
                Ok(Type::I64)
            }
            Some(Tok::Ident(s)) if s == "f64" => {
                self.pos += 1;
                Ok(Type::F64)
            }
            Some(Tok::Ident(s)) if s == "i32" => {
                self.pos += 1;
                Ok(Type::I32)
            }
            Some(Tok::Ident(s)) if s == "u32" => {
                self.pos += 1;
                Ok(Type::U32)
            }
            Some(Tok::Ident(s)) if s == "u64" => {
                self.pos += 1;
                Ok(Type::U64)
            }
            Some(Tok::Ident(s)) if s == "u8" => {
                self.pos += 1;
                Ok(Type::U8)
            }
            Some(Tok::Ident(s)) if s == "usize" => {
                self.pos += 1;
                Ok(Type::Usize)
            }
            Some(Tok::Ident(s)) if s == "char" => {
                self.pos += 1;
                Ok(Type::Char)
            }
            Some(Tok::Ident(s)) if s == "bool" => {
                self.pos += 1;
                Ok(Type::Bool)
            }
            Some(Tok::Ident(s)) => {
                self.pos += 1;
                let mut name = s;
                while self.at(&Tok::ColonColon) {
                    self.pos += 1;
                    name.push_str("::");
                    name.push_str(&self.ident()?);
                }
                let name = canonical_known_path(&name);
                if self.at(&Tok::Lt) {
                    self.pos += 1;
                    let mut args = Vec::new();
                    while !self.at(&Tok::Gt) {
                        if let Some(Tok::Lifetime(lt)) = self.peek() {
                            // E1d: lifetime arguments ride as Named("'a") for
                            // emission; typeck normalization filters them.
                            args.push(Type::Named(format!("'{}", lt)));
                            self.pos += 1;
                        } else {
                            args.push(self.parse_type()?);
                        }
                        if self.at(&Tok::Comma) {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    self.eat_gt()?;
                    Ok(Type::Generic { name, args })
                } else {
                    Ok(Type::Named(name))
                }
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                if self.at(&Tok::RParen) {
                    self.pos += 1;
                    return Ok(Type::Unit);
                }
                let mut tys = Vec::new();
                loop {
                    tys.push(self.parse_type()?);
                    if self.at(&Tok::Comma) {
                        self.pos += 1;
                        if self.at(&Tok::RParen) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                self.eat(&Tok::RParen)?;
                if tys.len() == 1 {
                    Ok(tys.into_iter().next().unwrap())
                } else {
                    Ok(Type::Tuple(tys))
                }
            }
            Some(Tok::LBracket) => {
                self.pos += 1;
                let inner = self.parse_type()?;
                if self.at(&Tok::Semi) {
                    self.pos += 1;
                    let size = match self.peek() {
                        Some(Tok::Int(n)) => {
                            let s = format!("{}", n);
                            self.pos += 1;
                            s
                        }
                        Some(Tok::Ident(name)) => {
                            let s = name.clone();
                            self.pos += 1;
                            s
                        }
                        other => {
                            return Err(format!(
                                "parse: array size must be an int or const name, found {:?}",
                                other
                            ))
                        }
                    };
                    self.eat(&Tok::RBracket)?;
                    return Ok(Type::Array(Box::new(inner), size));
                }
                self.eat(&Tok::RBracket)?;
                Ok(Type::Slice(Box::new(inner)))
            }
            other => Err(format!(
                "parse: expected type, found {:?} at token {}",
                other, self.pos
            )),
        }
    }

    fn parse_impl_trait_type(&mut self) -> Result<Type, String> {
        self.eat(&Tok::KwImpl)?;
        let mut name = self.ident()?;
        if self.at(&Tok::Lt) {
            self.pos += 1;
            // E1e: keep simple trait arguments in the NAME so emission is
            // complete (`impl Into<String>`); complex arguments fall back to
            // the erased form (held).
            let mut arg_texts = Vec::new();
            let mut simple = true;
            while !self.at(&Tok::Gt) {
                if matches!(self.peek(), Some(Tok::Lifetime(_))) {
                    self.pos += 1;
                    simple = false;
                } else {
                    match self.parse_type()? {
                        Type::Named(n) => arg_texts.push(n),
                        Type::I64 => arg_texts.push(String::from("i64")),
                        Type::Bool => arg_texts.push(String::from("bool")),
                        _ => simple = false,
                    }
                }
                if self.at(&Tok::Comma) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            self.eat_gt()?;
            if simple && !arg_texts.is_empty() {
                name = format!("{}<{}>", name, arg_texts.join(", "));
            }
        }
        let mut fn_params = None;
        let mut fn_ret = Type::Unit;
        if self.at(&Tok::LParen) {
            self.pos += 1;
            let mut params = Vec::new();
            while !self.at(&Tok::RParen) {
                params.push(self.parse_type()?);
                if self.at(&Tok::Comma) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            self.eat(&Tok::RParen)?;
            if self.at(&Tok::Arrow) {
                self.pos += 1;
                fn_ret = self.parse_type()?;
            }
            fn_params = Some(params);
        }
        if name == "Fn" {
            if let Some(params) = fn_params {
                return Ok(Type::Closure {
                    params,
                    ret: Box::new(fn_ret),
                });
            }
        }
        Ok(Type::ImplTrait(name))
    }

    fn skip_generic_params(&mut self) -> Result<(), String> {
        self.parse_generic_params().map(|_| ())
    }

    /// Capture generic parameter NAMES (`T`, `'a`) for emission (E1c). Bounds
    /// after `:` are skipped (accepted but not yet emission-preserved —
    /// outside the current corpus).
    fn parse_generic_params(&mut self) -> Result<Vec<String>, String> {
        if !self.at(&Tok::Lt) {
            return Ok(Vec::new());
        }
        self.pos += 1;
        let mut out = Vec::new();
        loop {
            match self.bump() {
                Some(Tok::Ident(s)) => out.push(s),
                Some(Tok::Lifetime(s)) => out.push(format!("'{}", s)),
                Some(Tok::Gt) if out.is_empty() => break,
                other => {
                    return Err(format!(
                        "parse: unsupported generic parameter start {:?}",
                        other
                    ))
                }
            }
            // Optional bounds: skip to `,` or the closing `>` at depth 0.
            if self.at(&Tok::Colon) {
                let mut depth = 0;
                loop {
                    match self.peek() {
                        Some(Tok::Lt) => {
                            depth += 1;
                            self.pos += 1;
                        }
                        Some(Tok::Gt) if depth > 0 => {
                            depth -= 1;
                            self.pos += 1;
                        }
                        Some(Tok::Gt) | Some(Tok::Comma) => break,
                        Some(_) => self.pos += 1,
                        None => {
                            return Err(
                                "parse: unterminated generic parameter list".to_string()
                            )
                        }
                    }
                }
            }
            match self.bump() {
                Some(Tok::Comma) => continue,
                Some(Tok::Gt) => break,
                other => {
                    return Err(format!(
                        "parse: expected , or > in generics, found {:?}",
                        other
                    ))
                }
            }
        }
        Ok(out)
    }

    // ---- statements / blocks ----------------------------------------------

    /// Parse a function-body-local item (fn/struct/enum/impl, optionally
    /// attributed or `pub`) and stash it for hoisting to the program level.
    fn parse_hoisted_item(&mut self) -> Result<(), String> {
        let derives = self.parse_attrs()?;
        if self.at(&Tok::KwPub) {
            self.pos += 1;
        }
        match self.peek() {
            Some(Tok::KwFn) => {
                let f = self.parse_func()?;
                self.hoisted_funcs.push(f);
            }
            Some(Tok::KwStruct) => {
                let s = self.parse_struct(derives)?;
                self.hoisted_structs.push(s);
            }
            Some(Tok::KwEnum) => {
                let e = self.parse_enum(derives)?;
                self.hoisted_enums.push(e);
            }
            Some(Tok::KwImpl) => {
                let i = self.parse_impl()?;
                self.hoisted_impls.push(i);
            }
            other => {
                return Err(format!("parse: expected a local item, found {:?}", other));
            }
        }
        Ok(())
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        self.eat(&Tok::LBrace)?;
        let saved = self.no_struct;
        self.no_struct = false; // struct literals are fine inside a block body
        let mut stmts = Vec::new();
        let mut tail: Option<Box<Expr>> = None;
        while !self.at(&Tok::RBrace) {
            match self.peek() {
                Some(Tok::KwLet) => stmts.push(self.parse_let()?),
                Some(Tok::KwReturn) => stmts.push(self.parse_return()?),
                // Local item declaration (`fn`/`struct`/`enum`/`impl`, optionally
                // attributed) -- parse it and hoist to the program level.
                Some(Tok::KwFn)
                | Some(Tok::KwStruct)
                | Some(Tok::KwEnum)
                | Some(Tok::KwImpl)
                | Some(Tok::Hash) => {
                    self.parse_hoisted_item()?;
                }
                // Function-body `use ...;` is consumed and dropped: the parser
                // canonicalizes std paths to bare names and emission re-qualifies
                // them, so the import carries no information in this subset.
                Some(Tok::KwUse) => {
                    let _ = self.parse_use_item()?;
                }
                _ => {
                    let starts_block = matches!(
                        self.peek(),
                        Some(Tok::KwIf)
                            | Some(Tok::LBrace)
                            | Some(Tok::KwMatch)
                            | Some(Tok::KwWhile)
                            | Some(Tok::KwLoop)
                            | Some(Tok::KwFor)
                            | Some(Tok::Lifetime(_))
                    );
                    let e = if starts_block {
                        self.parse_block_started_expr()?
                    } else {
                        self.parse_expr()?
                    };
                    if self.at(&Tok::Eq) {
                        self.pos += 1;
                        let value = self.parse_expr()?;
                        self.eat(&Tok::Semi)?;
                        stmts.push(Stmt::Assign { target: e, value });
                    } else if let Some(op) = self.compound_assign_op() {
                        self.pos += 1;
                        let rhs = self.parse_expr()?;
                        self.eat(&Tok::Semi)?;
                        let value = Expr::Binary {
                            op,
                            lhs: Box::new(e.clone()),
                            rhs: Box::new(rhs),
                        };
                        stmts.push(Stmt::Assign { target: e, value });
                    } else if self.at(&Tok::Semi) {
                        self.pos += 1;
                        stmts.push(Stmt::Expr(e));
                    } else if self.at(&Tok::RBrace) {
                        tail = Some(Box::new(e));
                        break;
                    } else if starts_block {
                        stmts.push(Stmt::Expr(e));
                    } else {
                        self.no_struct = saved;
                        return Err(format!(
                            "parse: expected ';' or '}}' after expression, found {:?} at token {}",
                            self.peek(),
                            self.pos
                        ));
                    }
                }
            }
        }
        self.eat(&Tok::RBrace)?;
        self.no_struct = saved;
        Ok(Block { stmts, tail })
    }

    fn parse_block_started_expr(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Tok::KwIf) => self.parse_if(),
            Some(Tok::LBrace) => Ok(Expr::Block(self.parse_block()?)),
            Some(Tok::KwMatch) => self.parse_match(),
            Some(Tok::Lifetime(_)) => self.parse_labeled(),
            Some(Tok::KwWhile) => self.parse_while(),
            Some(Tok::KwLoop) => {
                self.pos += 1;
                Ok(Expr::Loop {
                    body: self.parse_block()?,
                })
            }
            Some(Tok::KwFor) => self.parse_for(),
            _ => self.parse_expr(),
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.eat(&Tok::KwLet)?;
        let mutable = if self.at(&Tok::KwMut) {
            self.pos += 1;
            true
        } else {
            false
        };
        let starts_pattern = !matches!(self.peek(), Some(Tok::Ident(_)))
            || matches!(self.peek(), Some(Tok::Ident(name)) if name == "None")
            || matches!(
                self.peek_n(1),
                Some(Tok::LParen) | Some(Tok::ColonColon) | Some(Tok::LBrace) | Some(Tok::At)
            );
        if starts_pattern {
            if mutable {
                return Err("parse: `let mut` pattern bindings are not supported yet".to_string());
            }
            let pat = self.parse_pattern()?;
            // Optional type annotation on a pattern binding, e.g.
            // `let (a, b): (i64, i64) = ...`. LetPat carries no type, so the
            // annotation is parsed and discarded (emit re-prints without it,
            // which round-trips since the pattern is unchanged).
            if self.at(&Tok::Colon) {
                self.pos += 1;
                let _ = self.parse_type()?;
            }
            self.eat(&Tok::Eq)?;
            let init = self.parse_expr()?;
            if self.at(&Tok::KwElse) {
                self.pos += 1;
                let else_blk = self.parse_block()?;
                self.eat(&Tok::Semi)?;
                return Ok(Stmt::LetElse {
                    pat,
                    init,
                    else_blk,
                });
            }
            self.eat(&Tok::Semi)?;
            return Ok(Stmt::LetPat { pat, init });
        }
        let name = self.ident()?;
        let ty = if self.at(&Tok::Colon) {
            self.pos += 1;
            Some(self.parse_type()?)
        } else {
            None
        };
        self.eat(&Tok::Eq)?;
        let init = self.parse_expr()?;
        if self.at(&Tok::KwElse) {
            self.pos += 1;
            let else_blk = self.parse_block()?;
            self.eat(&Tok::Semi)?;
            return Ok(Stmt::LetElse {
                pat: Pattern::Bind(name),
                init,
                else_blk,
            });
        }
        self.eat(&Tok::Semi)?;
        Ok(Stmt::Let {
            name,
            mutable,
            ty,
            init,
        })
    }

    fn compound_assign_op(&self) -> Option<BinOp> {
        match self.peek() {
            Some(Tok::PlusEq) => Some(BinOp::Add),
            Some(Tok::MinusEq) => Some(BinOp::Sub),
            Some(Tok::StarEq) => Some(BinOp::Mul),
            Some(Tok::SlashEq) => Some(BinOp::Div),
            Some(Tok::PercentEq) => Some(BinOp::Rem),
            Some(Tok::AmpEq) => Some(BinOp::BitAnd),
            Some(Tok::PipeEq) => Some(BinOp::BitOr),
            Some(Tok::CaretEq) => Some(BinOp::BitXor),
            Some(Tok::ShlEq) => Some(BinOp::Shl),
            Some(Tok::ShrEq) => Some(BinOp::Shr),
            _ => None,
        }
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.eat(&Tok::KwReturn)?;
        if self.at(&Tok::Semi) {
            self.pos += 1;
            return Ok(Stmt::Return(None));
        }
        let e = self.parse_expr()?;
        if self.at(&Tok::Semi) {
            self.pos += 1;
        }
        Ok(Stmt::Return(Some(e)))
    }

    // ---- expressions -------------------------------------------------------

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_range()?;
        if self.at(&Tok::Eq) {
            self.pos += 1;
            let value = self.parse_assign()?;
            return Ok(Expr::Assign {
                target: Box::new(lhs),
                value: Box::new(value),
            });
        }
        if let Some(op) = self.compound_assign_op() {
            self.pos += 1;
            let rhs = self.parse_assign()?;
            let value = Expr::Binary {
                op,
                lhs: Box::new(lhs.clone()),
                rhs: Box::new(rhs),
            };
            return Ok(Expr::Assign {
                target: Box::new(lhs),
                value: Box::new(value),
            });
        }
        Ok(lhs)
    }

    fn parse_range(&mut self) -> Result<Expr, String> {
        let start = self.parse_or()?;
        let inclusive = if self.at(&Tok::DotDotEq) {
            self.pos += 1;
            Some(true)
        } else if self.at(&Tok::DotDot) {
            self.pos += 1;
            Some(false)
        } else {
            None
        };
        match inclusive {
            Some(inclusive) => {
                let end = self.parse_or()?;
                Ok(Expr::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                    inclusive,
                })
            }
            None => Ok(start),
        }
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_and()?;
        while self.at(&Tok::OrOr) {
            self.pos += 1;
            let rhs = self.parse_and()?;
            lhs = Expr::Binary {
                op: BinOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitor()?;
        while self.at(&Tok::AndAnd) {
            self.pos += 1;
            let rhs = self.parse_bitor()?;
            lhs = Expr::Binary {
                op: BinOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // Bitwise OR `|` binds looser than `^`; the infix position distinguishes it
    // from a closure `|x| ..` (prefix) parsed by the atom parser.
    fn parse_bitor(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitxor()?;
        while self.at(&Tok::Pipe) {
            self.pos += 1;
            let rhs = self.parse_bitxor()?;
            lhs = Expr::Binary {
                op: BinOp::BitOr,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_bitxor(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitand()?;
        while self.at(&Tok::Caret) {
            self.pos += 1;
            let rhs = self.parse_bitand()?;
            lhs = Expr::Binary {
                op: BinOp::BitXor,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // Bitwise AND `&` binds tighter than `^`; the infix position distinguishes
    // it from a reference `&x` (prefix) parsed by the unary/atom parser.
    fn parse_bitand(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_cmp()?;
        while self.at(&Tok::Amp) {
            self.pos += 1;
            let rhs = self.parse_cmp()?;
            lhs = Expr::Binary {
                op: BinOp::BitAnd,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_shift()?;
        let op = match self.peek() {
            Some(Tok::EqEq) => BinOp::Eq,
            Some(Tok::Ne) => BinOp::Ne,
            Some(Tok::Lt) => BinOp::Lt,
            Some(Tok::Le) => BinOp::Le,
            Some(Tok::Gt) => BinOp::Gt,
            Some(Tok::Ge) => BinOp::Ge,
            _ => return Ok(lhs),
        };
        self.pos += 1;
        let rhs = self.parse_shift()?;
        Ok(Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        })
    }

    // Shifts `<<`/`>>` bind tighter than comparison, looser than `+`. `<<` is two
    // `<` tokens and `>>` is two `>` tokens (the lexer never fuses them, so the
    // type parser keeps closing nested generics like `Vec<Vec<i64>>`); detecting
    // a CONSECUTIVE pair here is what distinguishes a shift from a comparison.
    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_add()?;
        loop {
            let op = if self.at(&Tok::Lt) && matches!(self.toks.get(self.pos + 1), Some(Tok::Lt)) {
                BinOp::Shl
            } else if self.at(&Tok::Gt) && matches!(self.toks.get(self.pos + 1), Some(Tok::Gt)) {
                BinOp::Shr
            } else {
                break;
            };
            self.pos += 2;
            let rhs = self.parse_add()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Plus) => BinOp::Add,
                Some(Tok::Minus) => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.parse_mul()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_cast()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Star) => BinOp::Mul,
                Some(Tok::Slash) => BinOp::Div,
                Some(Tok::Percent) => BinOp::Rem,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.parse_cast()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_cast(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_unary()?;
        while self.at(&Tok::KwAs) {
            self.pos += 1;
            let ty = self.parse_type()?;
            e = Expr::Cast {
                expr: Box::new(e),
                ty,
            };
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Tok::Amp) => {
                self.pos += 1;
                let mutable = if self.at(&Tok::KwMut) {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                Ok(Expr::Ref {
                    mutable,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Some(Tok::Minus) => {
                self.pos += 1;
                Ok(Expr::Unary {
                    op: UnOp::Neg,
                    rhs: Box::new(self.parse_unary()?),
                })
            }
            Some(Tok::Bang) => {
                self.pos += 1;
                Ok(Expr::Unary {
                    op: UnOp::Not,
                    rhs: Box::new(self.parse_unary()?),
                })
            }
            Some(Tok::Star) => {
                self.pos += 1;
                Ok(Expr::Unary {
                    op: UnOp::Deref,
                    rhs: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    /// primary followed by `.field` / `.0` / `.method(...)` accessors.
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_primary()?;
        loop {
            if self.at(&Tok::LParen) {
                let args = self.parse_arg_list()?;
                e = Expr::CallExpr {
                    callee: Box::new(e),
                    args,
                };
            } else if self.at(&Tok::Dot) {
                self.pos += 1;
                match self.bump() {
                    Some(Tok::Int(n)) => {
                        if n < 0 {
                            return Err("parse: negative tuple index".to_string());
                        }
                        e = Expr::TupleIndex {
                            base: Box::new(e),
                            index: n as usize,
                        };
                    }
                    Some(Tok::Ident(name)) => {
                        let type_args = self.parse_optional_turbofish()?;
                        if self.at(&Tok::LParen) {
                            let args = self.parse_arg_list()?;
                            e = Expr::MethodCall {
                                receiver: Box::new(e),
                                name,
                                type_args,
                                args,
                            };
                        } else if !type_args.is_empty() {
                            return Err("parse: turbofish requires a call".to_string());
                        } else {
                            e = Expr::Field {
                                base: Box::new(e),
                                name,
                            };
                        }
                    }
                    other => {
                        return Err(format!(
                            "parse: expected field after '.', found {:?}",
                            other
                        ))
                    }
                }
            } else if self.at(&Tok::LBracket) {
                self.pos += 1;
                if self.at(&Tok::DotDot) || self.at(&Tok::DotDotEq) {
                    let inclusive = self.at(&Tok::DotDotEq);
                    self.pos += 1;
                    let end = if self.at(&Tok::RBracket) {
                        None
                    } else {
                        Some(Box::new(self.parse_or()?))
                    };
                    self.eat(&Tok::RBracket)?;
                    e = Expr::Slice {
                        base: Box::new(e),
                        start: None,
                        end,
                        inclusive,
                    };
                } else {
                    let first = self.parse_or()?;
                    if self.at(&Tok::DotDot) || self.at(&Tok::DotDotEq) {
                        let inclusive = self.at(&Tok::DotDotEq);
                        self.pos += 1;
                        let end = if self.at(&Tok::RBracket) {
                            None
                        } else {
                            Some(Box::new(self.parse_or()?))
                        };
                        self.eat(&Tok::RBracket)?;
                        e = Expr::Slice {
                            base: Box::new(e),
                            start: Some(Box::new(first)),
                            end,
                            inclusive,
                        };
                    } else {
                        self.eat(&Tok::RBracket)?;
                        e = Expr::Index {
                            base: Box::new(e),
                            index: Box::new(first),
                        };
                    }
                }
            } else if self.at(&Tok::Question) {
                self.pos += 1;
                e = Expr::Try(Box::new(e));
            } else {
                break;
            }
        }
        Ok(e)
    }

    /// Skip a `where` clause (`where T: Bound, ..`) before a body. The
    /// interpreter does not resolve trait bounds, so the constraint text is
    /// consumed and discarded; bounds never contain `{`, so stop at the body.
    fn skip_where_clause(&mut self) {
        if matches!(self.peek(), Some(Tok::Ident(s)) if s == "where") {
            while !self.at(&Tok::LBrace) && self.peek().is_some() {
                self.pos += 1;
            }
        }
    }

    fn parse_optional_turbofish(&mut self) -> Result<Vec<Type>, String> {
        if !(self.at(&Tok::ColonColon) && matches!(self.toks.get(self.pos + 1), Some(Tok::Lt))) {
            return Ok(Vec::new());
        }
        self.pos += 2; // `::` `<`
        let mut args = Vec::new();
        while !self.at(&Tok::Gt) {
            args.push(self.parse_type()?);
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.eat_gt()?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Tok::Int(n)) => {
                self.pos += 1;
                Ok(Expr::Int(n))
            }
            // `5i64` desugars to `(5 as i64)`: same value, but the receiver type
            // is CONCRETE (method calls allowed; an unsuffixed literal is not).
            Some(Tok::IntSuffixed(n, suffix)) => {
                self.pos += 1;
                let ty = match suffix.as_str() {
                    "i32" => Type::I32,
                    "u32" => Type::U32,
                    "u64" => Type::U64,
                    "u8" => Type::U8,
                    "usize" => Type::Usize,
                    _ => Type::I64,
                };
                Ok(Expr::Cast {
                    expr: Box::new(Expr::Int(n)),
                    ty,
                })
            }
            Some(Tok::IntHex(n, text)) => {
                self.pos += 1;
                Ok(Expr::IntHex(n, text))
            }
            Some(Tok::Float(text)) => {
                self.pos += 1;
                Ok(Expr::Float(text))
            }
            Some(Tok::Char(ch)) => {
                self.pos += 1;
                Ok(Expr::Char(ch))
            }
            Some(Tok::KwTrue) => {
                self.pos += 1;
                Ok(Expr::Bool(true))
            }
            Some(Tok::KwFalse) => {
                self.pos += 1;
                Ok(Expr::Bool(false))
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Expr::Str(s))
            }
            Some(Tok::LBracket) => self.parse_array_lit(),
            Some(Tok::LParen) => self.parse_paren_or_tuple(),
            Some(Tok::LBrace) => Ok(Expr::Block(self.parse_block()?)),
            Some(Tok::OrOr) => self.parse_zero_arg_closure(),
            Some(Tok::Pipe) => self.parse_closure(),
            Some(Tok::KwMove) => {
                // `move` closures: the interpreter captures by value already, so
                // `move` is accepted and has no distinct runtime effect here.
                self.pos += 1;
                match self.peek() {
                    Some(Tok::OrOr) => self.parse_zero_arg_closure(),
                    Some(Tok::Pipe) => self.parse_closure(),
                    other => Err(format!("parse: expected a closure after `move`, found {:?}", other)),
                }
            }
            Some(Tok::KwIf) => self.parse_if(),
            Some(Tok::KwMatch) => self.parse_match(),
            Some(Tok::KwReturn) => {
                self.pos += 1;
                let value = if matches!(
                    self.peek(),
                    Some(Tok::Semi) | Some(Tok::RBrace) | Some(Tok::Comma) | None
                ) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                Ok(Expr::Return(value))
            }
            Some(Tok::Lifetime(_)) => self.parse_labeled(),
            Some(Tok::KwWhile) => self.parse_while(),
            Some(Tok::KwLoop) => {
                self.pos += 1;
                Ok(Expr::Loop {
                    body: self.parse_block()?,
                })
            }
            Some(Tok::KwFor) => self.parse_for(),
            Some(Tok::KwBreak) => {
                self.pos += 1;
                let label = if let Some(Tok::Lifetime(l)) = self.peek() {
                    let l = l.clone();
                    self.pos += 1;
                    Some(l)
                } else {
                    None
                };
                let value = if matches!(
                    self.peek(),
                    Some(Tok::Semi) | Some(Tok::RBrace) | Some(Tok::Comma) | None
                ) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                Ok(Expr::Break { label, value })
            }
            Some(Tok::KwContinue) => {
                self.pos += 1;
                Ok(Expr::Continue)
            }
            Some(Tok::Ident(name)) => self.parse_ident_expr(name),
            other => Err(format!(
                "parse: unexpected token in expression: {:?} at token {}",
                other, self.pos
            )),
        }
    }

    fn parse_paren_or_tuple(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::LParen)?;
        let saved = self.no_struct;
        self.no_struct = false;
        let result = (|| -> Result<Expr, String> {
            if self.at(&Tok::RParen) {
                return Ok(Expr::TupleLit(Vec::new())); // ()
            }
            let first = self.parse_expr()?;
            if self.at(&Tok::Comma) {
                let mut items = vec![first];
                while self.at(&Tok::Comma) {
                    self.pos += 1;
                    if self.at(&Tok::RParen) {
                        break;
                    }
                    items.push(self.parse_expr()?);
                }
                Ok(Expr::TupleLit(items))
            } else {
                Ok(first) // parenthesized
            }
        })();
        self.no_struct = saved;
        let e = result?;
        self.eat(&Tok::RParen)?;
        Ok(e)
    }

    fn parse_vec_macro(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::LBracket)?;
        self.parse_array_items()
    }

    fn parse_array_lit(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::LBracket)?;
        self.parse_array_items()
    }

    fn parse_array_items(&mut self) -> Result<Expr, String> {
        let saved = self.no_struct;
        self.no_struct = false;
        let mut items = Vec::new();
        while !self.at(&Tok::RBracket) {
            let item = self.parse_expr()?;
            if items.is_empty() && self.at(&Tok::Semi) {
                self.pos += 1;
                let count = self.parse_expr()?;
                self.no_struct = saved;
                self.eat(&Tok::RBracket)?;
                return Ok(Expr::VecRepeat {
                    elem: Box::new(item),
                    count: Box::new(count),
                });
            }
            items.push(item);
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.no_struct = saved;
        self.eat(&Tok::RBracket)?;
        Ok(Expr::VecLit(items))
    }

    fn parse_closure(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::Pipe)?;
        let mut params = Vec::new();
        while !self.at(&Tok::Pipe) {
            // The next `|` is the closure parameter delimiter, not an
            // or-pattern separator. Nested tuple/ref/enum patterns are still
            // accepted through `parse_pattern_atom`.
            let pat = self.parse_pattern_atom()?;
            let ty = if self.at(&Tok::Colon) {
                self.pos += 1;
                Some(self.parse_type()?)
            } else {
                None
            };
            params.push(ClosureParam { pat, ty });
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.eat(&Tok::Pipe)?;
        let ret = if self.at(&Tok::Arrow) {
            self.pos += 1;
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = if self.at(&Tok::LBrace) {
            Expr::Block(self.parse_block()?)
        } else {
            self.parse_expr()?
        };
        Ok(Expr::Closure {
            params,
            ret,
            body: Box::new(body),
        })
    }

    fn parse_zero_arg_closure(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::OrOr)?;
        let ret = if self.at(&Tok::Arrow) {
            self.pos += 1;
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = if self.at(&Tok::LBrace) {
            Expr::Block(self.parse_block()?)
        } else {
            self.parse_expr()?
        };
        Ok(Expr::Closure {
            params: Vec::new(),
            ret,
            body: Box::new(body),
        })
    }

    fn parse_ident_expr(&mut self, name: String) -> Result<Expr, String> {
        self.pos += 1; // consume ident
        if name == "println" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_println();
        }
        if name == "print" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_print();
        }
        if name == "eprintln" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_eprintln();
        }
        if name == "format" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_format();
        }
        if name == "write" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_write(false);
        }
        if name == "writeln" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_write(true);
        }
        if name == "matches" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_matches();
        }
        if name == "cfg" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_cfg_macro();
        }
        if name == "vec" && self.at(&Tok::Bang) {
            self.pos += 1;
            return self.parse_vec_macro();
        }
        if name == "assert" && self.at(&Tok::Bang) {
            self.pos += 1;
            let mut args = self.parse_arg_list()?;
            if args.len() != 1 {
                return Err(format!("parse: assert! expects 1 arg, got {}", args.len()));
            }
            return Ok(Expr::Assert {
                cond: Box::new(args.remove(0)),
            });
        }
        if name == "assert_eq" && self.at(&Tok::Bang) {
            self.pos += 1;
            let mut args = self.parse_arg_list()?;
            if args.len() != 2 {
                return Err(format!(
                    "parse: assert_eq! expects 2 args, got {}",
                    args.len()
                ));
            }
            let right = args.remove(1);
            let left = args.remove(0);
            return Ok(Expr::AssertEq {
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if matches!(name.as_str(), "panic" | "unreachable" | "todo") && self.at(&Tok::Bang) {
            self.pos += 1;
            self.skip_macro_parens()?;
            return Ok(Expr::Panic { name });
        }
        // Bare-call turbofish `id::<T>(args)`: `::` followed by `<` is explicit
        // type arguments, not a path segment. Consume and drop them (the
        // interpreter is dynamically typed), then it is a plain call.
        if self.at(&Tok::ColonColon) && matches!(self.toks.get(self.pos + 1), Some(Tok::Lt)) {
            let _ = self.parse_optional_turbofish()?;
            if self.at(&Tok::LParen) {
                let args = self.parse_arg_list()?;
                return Ok(Expr::Call { name, args });
            }
            return Ok(Expr::Var(name));
        }
        if self.at(&Tok::ColonColon) {
            let mut parts = vec![name];
            while self.at(&Tok::ColonColon) {
                self.pos += 1;
                parts.push(self.ident()?);
            }
            let item = parts
                .pop()
                .ok_or_else(|| "parse: empty path expression".to_string())?;
            let head = parts.join("::");
            let canonical_head = canonical_known_path(&head);
            if self.at(&Tok::LParen) {
                let args = self.parse_arg_list()?;
                return Ok(Expr::PathCall {
                    type_name: canonical_head,
                    item,
                    args,
                });
            }
            if parts.len() == 1 && self.at(&Tok::LBrace) && !self.no_struct {
                let fields = self.parse_field_values()?;
                return Ok(Expr::EnumStructLit {
                    enum_name: parts.remove(0),
                    variant: item,
                    fields,
                });
            }
            if canonical_head != head {
                return Ok(Expr::EnumCtor {
                    enum_name: canonical_head,
                    variant: item,
                });
            }
            if parts.len() != 1 {
                return Ok(Expr::Var(format!("{}::{}", head, item)));
            }
            return Ok(Expr::EnumCtor {
                enum_name: parts.remove(0),
                variant: item,
            });
        }
        if self.at(&Tok::LParen) {
            let args = self.parse_arg_list()?;
            return Ok(Expr::Call { name, args });
        }
        if self.at(&Tok::LBrace) && !self.no_struct {
            return self.parse_struct_lit(name);
        }
        Ok(Expr::Var(name))
    }

    fn skip_macro_parens(&mut self) -> Result<(), String> {
        self.eat(&Tok::LParen)?;
        let mut depth = 1usize;
        while depth > 0 {
            match self.bump() {
                Some(Tok::LParen) => depth += 1,
                Some(Tok::RParen) => depth -= 1,
                Some(_) => {}
                None => return Err("parse: unterminated macro call".to_string()),
            }
        }
        Ok(())
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, String> {
        self.eat(&Tok::LParen)?;
        let saved = self.no_struct;
        self.no_struct = false;
        let mut args = Vec::new();
        while !self.at(&Tok::RParen) {
            args.push(self.parse_expr()?);
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.no_struct = saved;
        self.eat(&Tok::RParen)?;
        Ok(args)
    }

    fn parse_struct_lit(&mut self, name: String) -> Result<Expr, String> {
        let fields = self.parse_field_values()?;
        Ok(Expr::StructLit { name, fields })
    }

    fn parse_field_values(&mut self) -> Result<Vec<(String, Expr)>, String> {
        self.eat(&Tok::LBrace)?;
        let saved = self.no_struct;
        self.no_struct = false;
        let mut fields = Vec::new();
        while !self.at(&Tok::RBrace) {
            let fname = self.ident()?;
            let value = if self.at(&Tok::Colon) {
                self.pos += 1;
                self.parse_expr()?
            } else {
                Expr::Var(fname.clone())
            };
            fields.push((fname, value));
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.no_struct = saved;
        self.eat(&Tok::RBrace)?;
        Ok(fields)
    }

    fn parse_if(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::KwIf)?;
        if self.at(&Tok::KwLet) {
            return self.parse_if_let_after_if();
        }
        let saved = self.no_struct;
        self.no_struct = true;
        let cond = self.parse_expr()?;
        self.no_struct = saved;
        let then_blk = self.parse_block()?;
        let else_blk = if self.at(&Tok::KwElse) {
            self.pos += 1;
            if self.at(&Tok::KwIf) {
                let nested = self.parse_if()?;
                Some(Block {
                    stmts: Vec::new(),
                    tail: Some(Box::new(nested)),
                })
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then_blk,
            else_blk,
        })
    }

    fn parse_if_let_after_if(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::KwLet)?;
        let pat = self.parse_pattern()?;
        self.eat(&Tok::Eq)?;
        let saved = self.no_struct;
        self.no_struct = true;
        let scrut = self.parse_expr()?;
        self.no_struct = saved;
        let then_blk = self.parse_block()?;
        let else_body = if self.at(&Tok::KwElse) {
            self.pos += 1;
            if self.at(&Tok::KwIf) {
                Expr::Block(Block {
                    stmts: Vec::new(),
                    tail: Some(Box::new(self.parse_if()?)),
                })
            } else {
                Expr::Block(self.parse_block()?)
            }
        } else {
            Expr::Block(Block {
                stmts: Vec::new(),
                tail: None,
            })
        };
        Ok(Expr::Match {
            scrut: Box::new(scrut),
            arms: vec![
                Arm {
                    pat,
                    guard: None,
                    body: Expr::Block(then_blk),
                },
                Arm {
                    pat: Pattern::Wild,
                    guard: None,
                    body: else_body,
                },
            ],
        })
    }

    /// `'label: <loop-or-block>` -- wraps the loop so `break 'label [value]`
    /// unwinds to here. (`continue 'label` is not modelled.)
    fn parse_labeled(&mut self) -> Result<Expr, String> {
        let label = if let Some(Tok::Lifetime(l)) = self.peek() {
            let l = l.clone();
            self.pos += 1;
            l
        } else {
            return Err("parse: expected a loop label".to_string());
        };
        self.eat(&Tok::Colon)?;
        let body = match self.peek() {
            Some(Tok::KwLoop) => {
                self.pos += 1;
                Expr::Loop {
                    body: self.parse_block()?,
                }
            }
            Some(Tok::KwWhile) => self.parse_while()?,
            Some(Tok::KwFor) => self.parse_for()?,
            Some(Tok::LBrace) => Expr::Block(self.parse_block()?),
            other => {
                return Err(format!(
                    "parse: expected a loop or block after label, found {:?}",
                    other
                ))
            }
        };
        Ok(Expr::Labeled {
            label,
            body: Box::new(body),
        })
    }

    fn parse_while(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::KwWhile)?;
        if self.at(&Tok::KwLet) {
            self.pos += 1;
            let pat = self.parse_pattern()?;
            self.eat(&Tok::Eq)?;
            let saved = self.no_struct;
            self.no_struct = true;
            let expr = self.parse_expr()?;
            self.no_struct = saved;
            let body = self.parse_block()?;
            return Ok(Expr::WhileLet {
                pat,
                expr: Box::new(expr),
                body,
            });
        }
        let saved = self.no_struct;
        self.no_struct = true;
        let cond = self.parse_expr()?;
        self.no_struct = saved;
        let body = self.parse_block()?;
        Ok(Expr::While {
            cond: Box::new(cond),
            body,
        })
    }

    fn parse_for(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::KwFor)?;
        let pat = self.parse_pattern()?;
        self.eat(&Tok::KwIn)?;
        let saved = self.no_struct;
        self.no_struct = true;
        let start = self.parse_or()?;
        let range_op = if self.at(&Tok::DotDotEq) {
            self.pos += 1;
            Some(true)
        } else if self.at(&Tok::DotDot) {
            self.pos += 1;
            Some(false)
        } else {
            None
        };
        if range_op.is_none() {
            self.no_struct = saved;
            let body = self.parse_block()?;
            return Ok(Expr::ForEach {
                pat,
                iter: Box::new(start),
                body,
            });
        }
        let var = match pat {
            Pattern::Bind(name) => name,
            _ => return Err("parse: range `for` currently requires a simple binding".to_string()),
        };
        let inclusive = range_op.unwrap();
        let end = self.parse_or()?;
        self.no_struct = saved;
        let body = self.parse_block()?;
        Ok(Expr::For {
            var,
            start: Box::new(start),
            end: Box::new(end),
            inclusive,
            body,
        })
    }

    fn parse_match(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::KwMatch)?;
        let saved = self.no_struct;
        self.no_struct = true;
        let scrut = self.parse_expr()?;
        self.no_struct = saved;
        self.eat(&Tok::LBrace)?;
        let mut arms = Vec::new();
        while !self.at(&Tok::RBrace) {
            let pat = self.parse_pattern()?;
            let guard = if self.at(&Tok::KwIf) {
                self.pos += 1;
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.eat(&Tok::FatArrow)?;
            let body = if self.at(&Tok::LBrace) {
                Expr::Block(self.parse_block()?)
            } else {
                self.parse_expr()?
            };
            arms.push(Arm { pat, guard, body });
            if self.at(&Tok::Comma) {
                self.pos += 1;
            }
            // a comma is optional after a block-bodied arm; loop continues either way
        }
        self.eat(&Tok::RBrace)?;
        Ok(Expr::Match {
            scrut: Box::new(scrut),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        let first = self.parse_pattern_atom()?;
        if !self.at(&Tok::Pipe) {
            return Ok(first);
        }
        let mut items = vec![first];
        while self.at(&Tok::Pipe) {
            self.pos += 1;
            items.push(self.parse_pattern_atom()?);
        }
        Ok(Pattern::Or(items))
    }

    /// After a (possibly negative) integer pattern `start`, parse an optional
    /// range tail: `..=end` (inclusive), `..end` (half-open, stored as end-1),
    /// or nothing (a plain integer pattern).
    fn parse_int_range_tail(&mut self, start: i64) -> Result<Pattern, String> {
        if self.at(&Tok::DotDotEq) {
            self.pos += 1;
            let end = self.parse_pattern_int_bound()?;
            Ok(Pattern::IntRange { start, end })
        } else if self.at(&Tok::DotDot) {
            self.pos += 1;
            let end = self.parse_pattern_int_bound()?;
            Ok(Pattern::IntRange {
                start,
                end: end - 1,
            })
        } else {
            Ok(Pattern::Int(start))
        }
    }

    /// Parse a possibly-negative integer used as a range-pattern bound.
    fn parse_pattern_int_bound(&mut self) -> Result<i64, String> {
        let neg = if self.at(&Tok::Minus) {
            self.pos += 1;
            true
        } else {
            false
        };
        match self.bump() {
            Some(Tok::Int(n)) => Ok(if neg { -n } else { n }),
            other => Err(format!(
                "parse: expected integer range bound, found {:?}",
                other
            )),
        }
    }

    fn parse_pattern_atom(&mut self) -> Result<Pattern, String> {
        match self.peek().cloned() {
            Some(Tok::Int(n)) => {
                self.pos += 1;
                self.parse_int_range_tail(n)
            }
            Some(Tok::Char(ch)) => {
                self.pos += 1;
                if self.at(&Tok::DotDotEq) {
                    self.pos += 1;
                    match self.bump() {
                        Some(Tok::Char(end)) => Ok(Pattern::CharRange { start: ch, end }),
                        other => Err(format!(
                            "parse: expected char range pattern end, found {:?}",
                            other
                        )),
                    }
                } else {
                    Ok(Pattern::Char(ch))
                }
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Pattern::Str(s))
            }
            Some(Tok::Minus) => {
                self.pos += 1;
                let start = match self.bump() {
                    Some(Tok::Int(n)) => -n,
                    other => {
                        return Err(format!(
                            "parse: expected integer after '-' in pattern, found {:?}",
                            other
                        ))
                    }
                };
                self.parse_int_range_tail(start)
            }
            Some(Tok::KwTrue) => {
                self.pos += 1;
                Ok(Pattern::Bool(true))
            }
            Some(Tok::KwFalse) => {
                self.pos += 1;
                Ok(Pattern::Bool(false))
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                let mut subs = Vec::new();
                let mut had_comma = false;
                while !self.at(&Tok::RParen) {
                    subs.push(self.parse_pattern()?);
                    if self.at(&Tok::Comma) {
                        self.pos += 1;
                        had_comma = true;
                    } else {
                        break;
                    }
                }
                self.eat(&Tok::RParen)?;
                // `(p)` with no comma is grouping (e.g. `(1 | 2)`), not a 1-tuple.
                if subs.len() == 1 && !had_comma {
                    Ok(subs.into_iter().next().unwrap())
                } else {
                    Ok(Pattern::Tuple(subs))
                }
            }
            Some(Tok::LBracket) => {
                // Slice pattern `[a, b, c]` / `[a, rest @ .., z]` / `[a, .., z]`.
                self.pos += 1;
                let mut prefix = Vec::new();
                let mut suffix = Vec::new();
                let mut rest: Option<Option<String>> = None;
                while !self.at(&Tok::RBracket) {
                    if self.at(&Tok::DotDot) {
                        self.pos += 1;
                        rest = Some(None);
                    } else if matches!(self.peek(), Some(Tok::Ident(_)))
                        && matches!(self.peek_n(1), Some(Tok::At))
                        && matches!(self.peek_n(2), Some(Tok::DotDot))
                    {
                        let name = self.ident()?;
                        self.pos += 2; // consume `@` and `..`
                        rest = Some(Some(name));
                    } else {
                        let p = self.parse_pattern()?;
                        if rest.is_none() {
                            prefix.push(p);
                        } else {
                            suffix.push(p);
                        }
                    }
                    if self.at(&Tok::Comma) {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                self.eat(&Tok::RBracket)?;
                Ok(Pattern::Slice {
                    prefix,
                    rest,
                    suffix,
                })
            }
            Some(Tok::Amp) => {
                self.pos += 1;
                let mutable = if self.at(&Tok::KwMut) {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                // parse_pattern_atom (not parse_pattern): a `&`-pattern's sub must
                // not greedily consume a `|`, which in `|&x| ...` is the closure's
                // closing delimiter, not an or-pattern separator.
                let sub = self.parse_pattern_atom()?;
                Ok(Pattern::Ref {
                    mutable,
                    sub: Box::new(sub),
                })
            }
            Some(Tok::AndAnd) => {
                // `&&x` (a double reference pattern, e.g. `filter(|&&x| ...)`) —
                // lexed as one AndAnd token; nest two Ref patterns.
                self.pos += 1;
                let sub = self.parse_pattern_atom()?;
                Ok(Pattern::Ref {
                    mutable: false,
                    sub: Box::new(Pattern::Ref {
                        mutable: false,
                        sub: Box::new(sub),
                    }),
                })
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                if name == "_" {
                    return Ok(Pattern::Wild);
                }
                if name == "ref" {
                    let mutable = if self.at(&Tok::KwMut) {
                        self.pos += 1;
                        true
                    } else {
                        false
                    };
                    let bind = self.ident()?;
                    return Ok(Pattern::BindRef {
                        name: bind,
                        mutable,
                    });
                }
                if self.at(&Tok::At) {
                    self.pos += 1;
                    let sub = self.parse_pattern()?;
                    return Ok(Pattern::BindAt {
                        name,
                        sub: Box::new(sub),
                    });
                }
                if matches!(name.as_str(), "Some" | "Ok" | "Err") && self.at(&Tok::LParen) {
                    self.pos += 1;
                    let mut sub = Vec::new();
                    while !self.at(&Tok::RParen) {
                        sub.push(self.parse_pattern()?);
                        if self.at(&Tok::Comma) {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    self.eat(&Tok::RParen)?;
                    return Ok(Pattern::Enum {
                        enum_name: if name == "Some" {
                            "Option".to_string()
                        } else {
                            "Result".to_string()
                        },
                        variant: name,
                        sub,
                    });
                }
                if name == "None" {
                    return Ok(Pattern::Enum {
                        enum_name: "Option".to_string(),
                        variant: name,
                        sub: Vec::new(),
                    });
                }
                if self.at(&Tok::ColonColon) {
                    self.pos += 1;
                    let variant = self.ident()?;
                    let mut sub = Vec::new();
                    if self.at(&Tok::LParen) {
                        self.pos += 1;
                        while !self.at(&Tok::RParen) {
                            sub.push(self.parse_pattern()?);
                            if self.at(&Tok::Comma) {
                                self.pos += 1;
                            } else {
                                break;
                            }
                        }
                        self.eat(&Tok::RParen)?;
                    }
                    if self.at(&Tok::LBrace) {
                        let (fields, rest) = self.parse_pattern_fields()?;
                        return Ok(Pattern::EnumStruct {
                            enum_name: name,
                            variant,
                            fields,
                            rest,
                        });
                    }
                    return Ok(Pattern::Enum {
                        enum_name: name,
                        variant,
                        sub,
                    });
                }
                if self.at(&Tok::LBrace) {
                    let (fields, rest) = self.parse_pattern_fields()?;
                    return Ok(Pattern::Struct { name, fields, rest });
                }
                Ok(Pattern::Bind(name))
            }
            other => Err(format!(
                "parse: unexpected token in pattern: {:?} at token {}",
                other, self.pos
            )),
        }
    }

    fn parse_pattern_fields(&mut self) -> Result<(Vec<(String, Pattern)>, bool), String> {
        self.eat(&Tok::LBrace)?;
        let mut fields = Vec::new();
        let mut rest = false;
        while !self.at(&Tok::RBrace) {
            if self.at(&Tok::DotDot) {
                self.pos += 1;
                rest = true;
                if self.at(&Tok::Comma) {
                    self.pos += 1;
                }
                break;
            }
            let fname = self.ident()?;
            let pat = if self.at(&Tok::Colon) {
                self.pos += 1;
                self.parse_pattern()?
            } else {
                Pattern::Bind(fname.clone())
            };
            fields.push((fname, pat));
            if self.at(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.eat(&Tok::RBrace)?;
        Ok((fields, rest))
    }

    fn parse_println(&mut self) -> Result<Expr, String> {
        let (fmt, args) = self.parse_format_args("println!")?;
        Ok(Expr::Println { fmt, args })
    }

    fn parse_print(&mut self) -> Result<Expr, String> {
        let (fmt, args) = self.parse_format_args("print!")?;
        Ok(Expr::Print { fmt, args })
    }

    fn parse_eprintln(&mut self) -> Result<Expr, String> {
        let (fmt, args) = self.parse_format_args("eprintln!")?;
        Ok(Expr::Eprintln { fmt, args })
    }

    fn parse_format(&mut self) -> Result<Expr, String> {
        let (fmt, args) = self.parse_format_args("format!")?;
        Ok(Expr::Format { fmt, args })
    }

    fn parse_write(&mut self, newline: bool) -> Result<Expr, String> {
        let macro_name = if newline { "writeln!" } else { "write!" };
        self.eat(&Tok::LParen)?;
        let saved = self.no_struct;
        self.no_struct = false;
        let target = self.parse_expr()?;
        if newline && self.at(&Tok::RParen) {
            self.no_struct = saved;
            self.eat(&Tok::RParen)?;
            return Ok(Expr::Write {
                newline,
                target: Box::new(target),
                fmt: String::new(),
                args: Vec::new(),
            });
        }
        self.eat(&Tok::Comma).map_err(|_| {
            format!(
                "parse: {} requires a destination and format string",
                macro_name
            )
        })?;
        let fmt = match self.bump() {
            Some(Tok::Str(s)) => s,
            other => {
                self.no_struct = saved;
                return Err(format!(
                    "parse: {} expects a format string, found {:?}",
                    macro_name, other
                ));
            }
        };
        let mut args = Vec::new();
        while self.at(&Tok::Comma) {
            self.pos += 1;
            if self.at(&Tok::RParen) {
                break;
            }
            args.push(self.parse_expr()?);
        }
        self.no_struct = saved;
        self.eat(&Tok::RParen)?;
        Ok(Expr::Write {
            newline,
            target: Box::new(target),
            fmt,
            args,
        })
    }

    fn parse_matches(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::LParen)?;
        let saved = self.no_struct;
        self.no_struct = false;
        let expr = self.parse_expr()?;
        self.eat(&Tok::Comma)?;
        let pat = self.parse_pattern()?;
        let guard = if self.at(&Tok::KwIf) {
            self.pos += 1;
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        self.no_struct = saved;
        self.eat(&Tok::RParen)?;
        Ok(Expr::Matches {
            expr: Box::new(expr),
            pat,
            guard,
        })
    }

    fn parse_cfg_macro(&mut self) -> Result<Expr, String> {
        self.eat(&Tok::LParen)?;
        let name = self.ident()?;
        self.eat(&Tok::RParen)?;
        Ok(Expr::Cfg { name })
    }

    fn parse_format_args(&mut self, macro_name: &str) -> Result<(String, Vec<Expr>), String> {
        self.eat(&Tok::LParen)?;
        // `println!()` / `eprintln!()` with no arguments print just a newline.
        // (print!/format! with no args is an error in rustc too, so those keep
        // requiring a format string.)
        if self.at(&Tok::RParen) && (macro_name == "println!" || macro_name == "eprintln!") {
            self.pos += 1;
            return Ok((String::new(), Vec::new()));
        }
        let fmt = match self.bump() {
            Some(Tok::Str(s)) => s,
            other => {
                return Err(format!(
                    "parse: {} expects a format string, found {:?}",
                    macro_name, other
                ))
            }
        };
        let saved = self.no_struct;
        self.no_struct = false;
        let mut positional = Vec::new();
        let mut named = Vec::new();
        while self.at(&Tok::Comma) {
            self.pos += 1;
            if self.at(&Tok::RParen) {
                break;
            }
            match (self.peek(), self.peek_n(1)) {
                (Some(Tok::Ident(name)), Some(Tok::Eq)) => {
                    let name = name.clone();
                    self.pos += 2;
                    let expr = self.parse_expr()?;
                    named.push((name, expr));
                }
                _ => positional.push(self.parse_expr()?),
            }
        }
        self.no_struct = saved;
        self.eat(&Tok::RParen)?;
        self.normalize_format_args(fmt, positional, named, macro_name)
    }

    fn normalize_format_args(
        &self,
        fmt: String,
        positional: Vec<Expr>,
        named: Vec<(String, Expr)>,
        macro_name: &str,
    ) -> Result<(String, Vec<Expr>), String> {
        let chars: Vec<char> = fmt.chars().collect();
        let mut out = String::new();
        let mut args = Vec::new();
        let mut used_pos = vec![false; positional.len()];
        let mut used_named = vec![false; named.len()];
        let mut auto_i = 0usize;
        let mut i = 0usize;
        while i < chars.len() {
            if chars[i] == '{' && chars.get(i + 1) == Some(&'{') {
                out.push('{');
                out.push('{');
                i += 2;
                continue;
            }
            if chars[i] == '}' && chars.get(i + 1) == Some(&'}') {
                out.push('}');
                out.push('}');
                i += 2;
                continue;
            }
            if chars[i] != '{' {
                out.push(chars[i]);
                i += 1;
                continue;
            }

            let mut j = i + 1;
            while j < chars.len() && chars[j] != '}' {
                j += 1;
            }
            if j >= chars.len() {
                return Err(format!(
                    "parse: {} has an unterminated format placeholder",
                    macro_name
                ));
            }
            let inner: String = chars[i + 1..j].iter().collect();
            let (selector, spec) = split_format_selector(&inner);
            if selector.is_empty() {
                let expr = positional.get(auto_i).cloned().ok_or_else(|| {
                    format!(
                        "parse: {} missing positional format arg {}",
                        macro_name, auto_i
                    )
                })?;
                used_pos[auto_i] = true;
                auto_i += 1;
                out.push('{');
                out.push_str(&spec);
                out.push('}');
                args.push(expr);
            } else if selector.chars().all(|c| c.is_ascii_digit()) {
                let idx = parse_usize_digits(&selector).ok_or_else(|| {
                    format!(
                        "parse: {} invalid positional selector {}",
                        macro_name, selector
                    )
                })?;
                let expr = positional.get(idx).cloned().ok_or_else(|| {
                    format!(
                        "parse: {} missing positional format arg {}",
                        macro_name, idx
                    )
                })?;
                used_pos[idx] = true;
                out.push('{');
                out.push_str(&spec);
                out.push('}');
                args.push(expr);
            } else if is_format_ident(&selector) {
                let idx = named
                    .iter()
                    .position(|(name, _)| name == &selector)
                    .ok_or_else(|| {
                        format!(
                            "parse: {} missing named format arg {}",
                            macro_name, selector
                        )
                    })?;
                used_named[idx] = true;
                out.push('{');
                out.push_str(&spec);
                out.push('}');
                args.push(named[idx].1.clone());
            } else {
                return Err(format!(
                    "parse: {} unsupported format selector {:?}",
                    macro_name, selector
                ));
            }
            i = j + 1;
        }
        for (idx, used) in used_pos.iter().enumerate() {
            if !*used {
                return Err(format!(
                    "parse: {} positional arg {} is never used",
                    macro_name, idx
                ));
            }
        }
        for (idx, used) in used_named.iter().enumerate() {
            if !*used {
                return Err(format!(
                    "parse: {} named arg {} is never used",
                    macro_name, named[idx].0
                ));
            }
        }
        Ok((out, args))
    }
}

fn split_format_selector(inner: &str) -> (String, String) {
    let mut selector = String::new();
    let mut spec = String::new();
    let mut in_spec = false;
    for ch in inner.chars() {
        if !in_spec && ch == ':' {
            in_spec = true;
            spec.push(ch);
        } else if in_spec {
            spec.push(ch);
        } else {
            selector.push(ch);
        }
    }
    (selector, spec)
}

fn is_format_ident(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_usize_digits(s: &str) -> Option<usize> {
    let mut n = 0usize;
    for ch in s.chars() {
        if !ch.is_ascii_digit() {
            return None;
        }
        n = n * 10 + (ch as usize - '0' as usize);
    }
    Some(n)
}

fn canonical_known_path(path: &str) -> String {
    match path {
        "std::vec::Vec" => "Vec".to_string(),
        "std::string::String" => "String".to_string(),
        "std::rc::Rc" => "Rc".to_string(),
        "std::cell::RefCell" => "RefCell".to_string(),
        "std::collections::HashMap" => "HashMap".to_string(),
        "std::path::Path" => "Path".to_string(),
        "std::path::PathBuf" => "PathBuf".to_string(),
        "std::process::Command" => "Command".to_string(),
        "std::process::ExitCode" => "ExitCode".to_string(),
        "std::fmt::Formatter" => "fmt::Formatter".to_string(),
        "std::fmt::Result" => "fmt::Result".to_string(),
        "std::fmt::Display" => "fmt::Display".to_string(),
        "std::fs" => "fs".to_string(),
        "std::env" => "env".to_string(),
        _ => path.to_string(),
    }
}

fn type_head_name(ty: &Type) -> String {
    match ty {
        Type::Named(n) => n.clone(),
        Type::Generic { name, .. } => name.clone(),
        _ => String::new(),
    }
}
