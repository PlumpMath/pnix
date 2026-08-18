"""A small pnix parser/evaluator used by the pnix-hy mirror lane."""

from __future__ import annotations

import sys as _sys
import threading as _threading

from dataclasses import dataclass
from functools import cmp_to_key
import hashlib
import json
import math
import os
import platform
from pathlib import Path
import re
import sys
import tempfile
import time
import tomllib
from typing import Any, Callable
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET


RUNTIME_SCHEMA = "pnix-hy.runtime.v0"
MIRROR_SCHEMA = "pnix-hy.runtime.mirror-trace.v0"


class PnixError(RuntimeError):
    """Raised for pnix parse/eval failures with non-message classification."""

    def __init__(
        self,
        message: str,
        *,
        phase: str = "eval",
        error_class: str = "unsupported-expression",
        evidence: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.phase = phase
        self.error_class = error_class
        self.evidence = dict(evidence or {})


class PnixCatchableError(PnixError):
    """Explicit throw/assert failure caught by Nix builtins.tryEval."""


@dataclass(frozen=True)
class Token:
    kind: str
    value: Any
    pos: int


@dataclass
class Thunk:
    func: Callable[[], Any]
    forced: bool = False
    forcing: bool = False
    value: Any = None

    def force(self) -> Any:
        if self.forced:
            return self.value
        if self.forcing:
            raise PnixError(
                "infinite recursion encountered (recursive value forced itself)",
                error_class="cycle-detected",
            )
        self.forcing = True
        try:
            self.value = self.func()
            self.forced = True
            return self.value
        finally:
            self.forcing = False


@dataclass(frozen=True)
class Closure:
    param: str | None
    body: dict[str, Any]
    env: dict[str, Any]
    pattern: dict[str, Any] | None = None
    ctx: dict[str, Any] | None = None


@dataclass(frozen=True)
class NativeFunc:
    func: Callable[[Any], Any]
    force_arg: bool = True

    def __call__(self, arg: Any) -> Any:
        return self.func(arg)


def normalize_pnix_path_text(text: Any) -> str:
    text = str(text)
    if text.startswith("<") and text.endswith(">"):
        return text
    absolute = text.startswith("/")
    started_with_dot = text == "." or text.startswith("./")
    out: list[str] = []
    for part in text.split("/"):
        if part in {"", "."}:
            continue
        if part == "..":
            if out and out[-1] != "..":
                out.pop()
            elif not absolute:
                out.append(part)
            continue
        out.append(part)
    if absolute:
        return "/" + "/".join(out) if out else "/"
    if not out:
        return "."
    body = "/".join(out)
    if started_with_dot and out[0] != "..":
        return "./" + body
    return body


class PnixPath(str):
    """String-like path value with a distinct pnix type tag."""

    def __new__(cls, text: Any) -> "PnixPath":
        return str.__new__(cls, normalize_pnix_path_text(text))


class PnixString(str):
    """String value carrying pnix string-context provenance."""

    def __new__(cls, text: Any, context: Any | None = None) -> "PnixString":
        obj = str.__new__(cls, str(text))
        obj.context = frozenset(str(item) for item in (context or ()))
        return obj


class AttrSet(dict):
    """Attrset value carrying literal-source positions for unsafeGetAttrPos."""

    def __init__(self, *args: Any, attr_positions: dict[str, int] | None = None, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        self.attr_positions: dict[str, int] = dict(attr_positions or {})


@dataclass
class ConstructValue:
    variant: str
    args: list[Any]


@dataclass
class WithFrame:
    source: dict[str, Any]
    env: dict[str, Any]
    ctx: dict[str, Any]
    cached: dict[str, Any] | None = None


WITH_CHAIN_KEY = "__pnix_hy_with_chain__"


def pnix_error(
    message: str,
    *,
    phase: str = "eval",
    error_class: str = "unsupported-expression",
    evidence: dict[str, Any] | None = None,
) -> None:
    raise PnixError(
        message,
        phase=phase,
        error_class=error_class,
        evidence=evidence,
    )


def pnix_catchable_error(message: str) -> None:
    raise PnixCatchableError(message)


def _is_digit(c: str) -> bool:
    return c.isdigit()


def _float_lexeme_value(lexeme: str) -> float:
    value = float(lexeme)
    mantissa = re.split("[eE]", lexeme, maxsplit=1)[0]
    underflow = value == 0.0 and any(c in "123456789" for c in mantissa)
    subnormal = value != 0.0 and abs(value) < sys.float_info.min
    if not math.isfinite(value) or underflow or subnormal:
        pnix_error(f"invalid float '{lexeme}'")
    return value


def _is_ident_start(c: str) -> bool:
    return c.isalpha() or c == "_"


def _is_ident_char(c: str) -> bool:
    return c.isalnum() or c in "_-'"


def _is_uri_ascii_alpha(c: str) -> bool:
    return ("A" <= c <= "Z") or ("a" <= c <= "z")


def _is_uri_scheme_char(c: str) -> bool:
    return _is_uri_ascii_alpha(c) or ("0" <= c <= "9") or c in "+-."


def _is_uri_body_char(c: str) -> bool:
    return _is_uri_ascii_alpha(c) or ("0" <= c <= "9") or c in "%/?:@&=+$,-_.!~*'"


def _uri_end(source: str, start: int) -> int | None:
    """Match Nix 2.34.7's deprecated URI-literal lexer rule exactly."""
    n = len(source)
    if start >= n or not _is_uri_ascii_alpha(source[start]):
        return None
    i = start + 1
    while i < n and _is_uri_scheme_char(source[i]):
        i += 1
    if i >= n or source[i] != ":":
        return None
    body_start = i + 1
    i = body_start
    while i < n and _is_uri_body_char(source[i]):
        i += 1
    return i if i > body_start else None


def _skip_block_comment(source: str, start: int) -> int:
    end = source.find("*/", start + 2)
    if end < 0:
        pnix_error(f"unterminated block comment starting at byte {start}")
    return end + 2


def _skip_double_string_in_source(source: str, start: int) -> int:
    i = start + 1
    n = len(source)
    while i < n:
        c = source[i]
        if c == "\\":
            i += 2
        elif c == '"':
            return i + 1
        else:
            i += 1
    pnix_error("unterminated string literal")
    raise AssertionError("unreachable")


def _skip_indented_string_in_source(source: str, start: int) -> int:
    i = start + 2
    n = len(source)
    while i < n:
        if source.startswith("'''", i):
            i += 3
            continue
        if source.startswith("''$", i):
            i += 3
            continue
        if source.startswith("''\\", i):
            i += 4 if i + 3 < n else 3
            continue
        if source.startswith("''", i):
            return i + 2
        i += 1
    pnix_error("unterminated indented string literal")
    raise AssertionError("unreachable")


def _scan_interp_body(source: str, start: int) -> tuple[str, int]:
    depth = 1
    i = start
    n = len(source)
    while i < n:
        c = source[i]
        if c == "{":
            depth += 1
            i += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return source[start:i], i + 1
            i += 1
        elif c == '"':
            i = _skip_double_string_in_source(source, i)
        elif c == "'" and i + 1 < n and source[i + 1] == "'":
            i = _skip_indented_string_in_source(source, i)
        elif c == "#":
            newline = source.find("\n", i)
            i = n if newline == -1 else newline
        elif c == "/" and i + 1 < n and source[i + 1] == "*":
            i = _skip_block_comment(source, i)
        else:
            i += 1
    pnix_error("unterminated interpolation ${...} in string literal")
    raise AssertionError("unreachable")


def _read_string_token(source: str, start: int) -> tuple[Token, int]:
    i = start + 1
    n = len(source)
    buf: list[str] = []
    parts: list[dict[str, Any]] = []
    while i < n:
        c = source[i]
        if c == '"':
            if parts:
                if buf:
                    parts.append({"lit": "".join(buf)})
                return Token("string_interp", parts, start), i + 1
            return Token("string", "".join(buf), start), i + 1
        if c == "\\":
            if i + 1 >= n:
                pnix_error("unterminated string escape")
            e = source[i + 1]
            buf.append(
                {
                    "n": "\n",
                    "r": "\r",
                    "t": "\t",
                    '"': '"',
                    "\\": "\\",
                    "$": "$",
                }.get(e, e)
            )
            i += 2
            continue
        if c == "$" and i + 1 < n and source[i + 1] == "{":
            raw, end = _scan_interp_body(source, i + 2)
            if buf:
                parts.append({"lit": "".join(buf)})
                buf = []
            parts.append({"interp": raw})
            i = end
            continue
        buf.append(c)
        i += 1
    pnix_error("unterminated string literal")
    raise AssertionError("unreachable")


def _indented_lines(raw: str) -> list[str]:
    lines = raw.split("\n")
    return lines[:-1] if raw.endswith("\n") else lines


def _common_indentation(raw: str) -> int:
    indents = []
    for line in _indented_lines(raw):
        if all(ch.isspace() for ch in line):
            continue
        indent = 0
        for ch in line:
            if ch not in " \t":
                break
            indent += 1
        indents.append(indent)
    return min(indents) if indents else 0


def _strip_indent_line(line: str, min_indent: int) -> str:
    skipped = 0
    while skipped < min_indent and skipped < len(line) and line[skipped] in " \t":
        skipped += 1
    return line[skipped:]


def _strip_indented_string(raw: str) -> str:
    lines = _indented_lines(raw)
    if not lines:
        return "\n" if raw.endswith("\n") else ""
    min_indent = _common_indentation(raw)
    out = ""
    first = True
    for line in lines:
        if first and line == "":
            first = False
            continue
        first = False
        stripped = _strip_indent_line(line, min_indent)
        if out:
            out += "\n"
        out += stripped
    if raw.endswith("\n"):
        out += "\n"
    return out


def _strip_indented_string_parts(parts: list[dict[str, Any]]) -> list[dict[str, Any]]:
    combined = "".join(part["lit"] if "lit" in part else "${}" for part in parts)
    min_indent = _common_indentation(combined)
    result: list[dict[str, Any]] = []
    is_first = True
    at_line_start = True
    chars_stripped = 0
    for part in parts:
        if "interp" in part:
            is_first = False
            at_line_start = False
            chars_stripped = 0
            result.append(part)
            continue
        stripped = []
        for ch in part["lit"]:
            if ch == "\n":
                if is_first and not stripped:
                    is_first = False
                    at_line_start = True
                    chars_stripped = 0
                    continue
                is_first = False
                stripped.append("\n")
                at_line_start = True
                chars_stripped = 0
            elif at_line_start and ch in " \t" and chars_stripped < min_indent:
                chars_stripped += 1
            else:
                is_first = False
                at_line_start = False
                stripped.append(ch)
        text = "".join(stripped)
        if text or not result:
            result.append({"lit": text})
    return result


def _read_indented_escape(source: str, pos: int) -> tuple[str, int]:
    if pos >= len(source):
        return "\\", pos
    c = source[pos]
    return (
        {
            "n": "\n",
            "r": "\r",
            "t": "\t",
            "\\": "\\",
        }.get(c, "\\" + c),
        pos + 1,
    )


def _read_indented_string_token(source: str, start: int) -> tuple[Token, int]:
    i = start + 2
    n = len(source)
    buf: list[str] = []
    parts: list[dict[str, Any]] = []
    while i < n:
        if source.startswith("'''", i):
            buf.append("''")
            i += 3
            continue
        if source.startswith("''$", i):
            buf.append("$")
            i += 3
            continue
        if source.startswith("''\\", i):
            escaped, i = _read_indented_escape(source, i + 3)
            buf.append(escaped)
            continue
        if source.startswith("''", i):
            if parts:
                if buf:
                    parts.append({"lit": "".join(buf)})
                return Token("string_interp", _strip_indented_string_parts(parts), start), i + 2
            return Token("string", _strip_indented_string("".join(buf)), start), i + 2
        if source[i] == "$" and i + 1 < n and source[i + 1] == "{":
            raw, end = _scan_interp_body(source, i + 2)
            if buf:
                parts.append({"lit": "".join(buf)})
                buf = []
            parts.append({"interp": raw})
            i = end
            continue
        buf.append(source[i])
        i += 1
    pnix_error("unterminated indented string literal")
    raise AssertionError("unreachable")


def _is_path_char(c: str) -> bool:
    return c.isalnum() or c in "/._-+~"


def _read_path_token(source: str, start: int) -> tuple[Token, int]:
    i = start
    n = len(source)
    lit_start = start
    parts: list[dict[str, Any]] = []
    while i < n:
        if source.startswith("${", i):
            if i > lit_start:
                parts.append({"lit": source[lit_start:i]})
            raw, end = _scan_interp_body(source, i + 2)
            parts.append({"interp": raw})
            i = end
            lit_start = i
            continue
        if _is_path_char(source[i]):
            i += 1
            continue
        break
    if i > lit_start:
        parts.append({"lit": source[lit_start:i]})
    if not parts:
        pnix_error(f"empty path at byte {start}")
    if any("interp" in part for part in parts):
        return Token("path_interp", parts, start), i
    return Token("path", "".join(str(part["lit"]) for part in parts), start), i


def _read_search_path_token(source: str, start: int) -> tuple[Token, int]:
    end = source.find(">", start + 1)
    if end < 0:
        pnix_error("unterminated search path")
    body = source[start + 1 : end]
    if not body:
        pnix_error("empty search path")
    return Token("path", "<" + body + ">", start), end + 1


def tokenize(source: str) -> list[Token]:
    tokens: list[Token] = []
    i = 0
    n = len(source)
    keywords = {
        "let",
        "in",
        "rec",
        "if",
        "then",
        "else",
        "true",
        "false",
        "null",
        "import",
        "with",
        "assert",
        "match",
        "inherit",
        "or",
    }
    while i < n:
        c = source[i]
        if c.isspace():
            i += 1
            continue
        if c == "#":
            newline = source.find("\n", i)
            i = n if newline == -1 else newline
            continue
        if c == "/" and i + 1 < n and source[i + 1] == "*":
            i = _skip_block_comment(source, i)
            continue
        if c == '"':
            token, i = _read_string_token(source, i)
            tokens.append(token)
            continue
        if c == "'" and i + 1 < n and source[i + 1] == "'":
            token, i = _read_indented_string_token(source, i)
            tokens.append(token)
            continue
        if _is_digit(c) or (c == "." and i + 1 < n and _is_digit(source[i + 1])):
            leading_dot = c == "."
            j = i + 1
            while j < n and _is_digit(source[j]):
                j += 1
            if (
                not leading_dot
                and j - i > 1
                and source[i] == "0"
                and j + 1 < n
                and source[j] == "."
                and _is_digit(source[j + 1])
            ):
                suffix_end = j + 2
                while suffix_end < n and _is_digit(source[suffix_end]):
                    suffix_end += 1
                if suffix_end < n and source[suffix_end] in "eE":
                    exponent = suffix_end + 1
                    if exponent < n and source[exponent] in "+-":
                        exponent += 1
                    exponent_start = exponent
                    while exponent < n and _is_digit(source[exponent]):
                        exponent += 1
                    if exponent == exponent_start:
                        pnix_error(f"invalid numeric exponent at byte {suffix_end}")
                    suffix_end = exponent
                suffix = source[j:suffix_end]
                tokens.append(
                    Token(
                        "number_application",
                        (int(source[i:j]), _float_lexeme_value(suffix), suffix_end - i),
                        i,
                    )
                )
                i = suffix_end
                continue
            is_float = leading_dot
            if not leading_dot and j < n and source[j] == ".":
                if j + 1 < n and _is_digit(source[j + 1]):
                    if not (j - i > 1 and source[i] == "0"):
                        is_float = True
                        j += 2
                        while j < n and _is_digit(source[j]):
                            j += 1
                elif j + 1 < n and source[j + 1] in "eE":
                    # Nix accepts an empty fractional part when an exponent
                    # follows (`1.e2`), but a bare trailing dot remains the
                    # attribute-selection token.
                    if not (j - i > 1 and source[i] == "0"):
                        is_float = True
                        j += 1
            if is_float and j < n and source[j] in "eE":
                exp = j + 1
                if exp < n and source[exp] in "+-":
                    exp += 1
                exp_digits = exp
                while exp < n and _is_digit(source[exp]):
                    exp += 1
                if exp == exp_digits:
                    pnix_error(f"invalid numeric exponent at byte {j}")
                is_float = True
                j = exp
            lexeme = source[i:j]
            tokens.append(Token("number", _float_lexeme_value(lexeme) if is_float else int(lexeme), i))
            i = j
            continue
        if _is_ident_start(c):
            uri_end = _uri_end(source, i)
            if uri_end is not None:
                tokens.append(Token("uri", source[i:uri_end], i))
                i = uri_end
                continue
            j = i + 1
            while j < n and _is_ident_char(source[j]):
                j += 1
            word = source[i:j]
            tokens.append(Token("kw" if word in keywords else "ident", word, i))
            i = j
            continue
        if c == "." and (
            (i + 1 < n and source[i + 1] == "/")
            or (i + 2 < n and source[i + 1] == "." and source[i + 2] == "/")
        ):
            token, i = _read_path_token(source, i)
            tokens.append(token)
            continue
        if c == "~":
            if i + 1 < n and source[i + 1] == "/":
                token, i = _read_path_token(source, i)
                tokens.append(token)
                continue
            if i + 1 == n or source[i + 1].isspace() or source[i + 1] in '{}[]();"':
                tokens.append(Token("path", "~", i))
                i += 1
                continue
            pnix_error("expected `/` after `~` for home path")
        if c == "/" and i + 1 < n and source[i + 1] != "/" and (_is_path_char(source[i + 1]) or source[i + 1] == "$"):
            token, i = _read_path_token(source, i)
            tokens.append(token)
            continue
        if c == "<" and i + 1 < n and _is_ident_start(source[i + 1]):
            token, i = _read_search_path_token(source, i)
            tokens.append(token)
            continue
        if source.startswith("...", i):
            tokens.append(Token("sym", "...", i))
            i += 3
            continue
        two = source[i : i + 2]
        if two in {"&&", "||", "==", "!=", "<=", ">=", "//", "++", "->", "=>", "${"}:
            tokens.append(Token("sym", two, i))
            i += 2
            continue
        if c in "{}[]();=.:,+-*/%!<>?|@":
            tokens.append(Token("sym", c, i))
            i += 1
            continue
        pnix_error(f"unexpected character `{c}` at byte {i}")
    return tokens


class Parser:
    def __init__(self, tokens: list[Token]) -> None:
        self.tokens = tokens

    def tok(self, pos: int) -> Token:
        if pos < len(self.tokens):
            return self.tokens[pos]
        return Token("eof", "", pos)

    def tok_is(self, pos: int, kind: str, value: Any) -> bool:
        token = self.tok(pos)
        return token.kind == kind and token.value == value

    def expect(self, pos: int, kind: str, value: Any) -> int:
        if self.tok_is(pos, kind, value):
            return pos + 1
        pnix_error(f"expected `{value}` at token {pos}, got {self.tok(pos)!r}")
        raise AssertionError("unreachable")

    def token_end(self, pos: int) -> int:
        token = self.tok(pos)
        if token.kind == "eof":
            return token.pos
        if token.kind == "number":
            return token.pos + len(str(token.value))
        if token.kind == "number_application":
            return token.pos + token.value[2]
        if token.kind in {"ident", "kw", "uri", "path", "path_interp", "sym"}:
            return token.pos + len(str(token.value))
        return token.pos

    def adjacent(self, left_pos: int, right_pos: int) -> bool:
        return left_pos >= 0 and self.token_end(left_pos) == self.tok(right_pos).pos

    def parse_path_segment(self, pos: int) -> tuple[Any, int | None, int]:
        token = self.tok(pos)
        if self.tok_is(pos, "sym", "${"):
            expr, pos = self.parse_expr(pos + 1)
            return {"expr": expr}, None, self.expect(pos, "sym", "}")
        # bare dynamic key `${e} = v;` where the LEXER consumed `${...}` as a
        # single-interp string_interp token (Nix core; audit 2026-07-09 — the
        # harvested evaluator's `acc // { ${p.name} = p.value; }` needs it).
        if (token.kind == "string_interp" and len(token.value) == 1
                and "interp" in token.value[0]):
            return {"expr": parse(token.value[0]["interp"])}, None, pos + 1
        if token.kind == "string_interp":
            # multi-part key `"a${e}"` — dynamic key from interpolated string
            parts = []
            for part in token.value:
                if "lit" in part:
                    parts.append({"lit": part["lit"]})
                else:
                    parts.append({"expr": parse(part["interp"])})
            return {"expr": {"tag": "str_interp", "parts": parts}}, None, pos + 1
        if token.kind not in {"ident", "string", "kw"}:
            pnix_error(f"expected attribute name at token {pos}")
        # A quoted string key is ONE literal attr name even if it contains dots
        # (`{ "x.y" = 1; }` is a single attr "x.y", not nested x.y); only the
        # unquoted `a.b.c` token sequence forms a nested path. Matches Nix/original.
        return str(token.value), token.pos, pos + 1

    def parse_path(self, pos: int) -> tuple[list[Any], list[int | None], int]:
        part, position, pos = self.parse_path_segment(pos)
        parts = [part]
        positions = [position]
        while self.tok_is(pos, "sym", "."):
            part, position, pos = self.parse_path_segment(pos + 1)
            parts.append(part)
            positions.append(position)
        return parts, positions, pos

    def parse_inherit_name(self, pos: int) -> tuple[str | None, int | None, int]:
        token = self.tok(pos)
        if token.kind in {"ident", "string", "kw"}:
            return str(token.value), token.pos, pos + 1
        if self.tok_is(pos, "sym", "${"):
            name_token = self.tok(pos + 1)
            if name_token.kind != "string":
                pnix_error("expected string literal inside `${...}` inherit name")
            return str(name_token.value), name_token.pos, self.expect(pos + 2, "sym", "}")
        return None, None, pos

    def parse_inherit_bindings(self, pos: int) -> tuple[list[dict[str, Any]], int]:
        pos = self.expect(pos, "kw", "inherit")
        scope = None
        if self.tok_is(pos, "sym", "("):
            scope, pos = self.parse_expr(pos + 1)
            pos = self.expect(pos, "sym", ")")
        bindings: list[dict[str, Any]] = []
        while not self.tok_is(pos, "sym", ";"):
            if self.tok(pos).kind == "eof":
                pnix_error("expected `;` before end of inherit clause")
            name, name_pos, pos = self.parse_inherit_name(pos)
            if name is None:
                pnix_error(f"expected inherited name at token {pos}")
            value = (
                {"tag": "select", "base": scope, "attr": name}
                if scope is not None
                else {"tag": "var", "name": name}
            )
            binding = {"path": [name], "path_positions": [int(name_pos or 0)], "value": value}
            if scope is None:
                binding["inherit_plain"] = True
            bindings.append(binding)
        if not bindings:
            pnix_error("inherit requires at least one name")
        return bindings, pos + 1

    def parse_bindings_until(
        self, pos: int, end_kind: str, end_value: Any, mode: str
    ) -> tuple[list[dict[str, Any]], int]:
        bindings: list[dict[str, Any]] = []
        while not self.tok_is(pos, end_kind, end_value):
            if self.tok(pos).kind == "eof":
                pnix_error(f"expected `{end_value}` before end of input")
            if self.tok_is(pos, "kw", "inherit"):
                inherited, pos = self.parse_inherit_bindings(pos)
                bindings.extend(inherited)
                continue
            path, path_positions, pos = self.parse_path(pos)
            if mode == "let" and any(not isinstance(part, str) for part in path):
                pnix_error("dynamic attributes not allowed in let")
            pos = self.expect(pos, "sym", "=")
            value, pos = self.parse_expr(pos)
            pos = self.expect(pos, "sym", ";")
            bindings.append({"path": path, "path_positions": path_positions, "value": value})
        return bindings, pos + 1

    def parse_let(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "kw", "let")
        bindings, pos = self.parse_bindings_until(pos, "kw", "in", "let")
        body, pos = self.parse_expr(pos)
        return {"tag": "let", "bindings": bindings, "body": body}, pos

    def parse_if(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "kw", "if")
        cond, pos = self.parse_expr(pos)
        pos = self.expect(pos, "kw", "then")
        then_expr, pos = self.parse_expr(pos)
        pos = self.expect(pos, "kw", "else")
        else_expr, pos = self.parse_expr(pos)
        return {
            "tag": "if",
            "cond": cond,
            "then": then_expr,
            "else": else_expr,
        }, pos

    def parse_with(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "kw", "with")
        env_expr, pos = self.parse_expr(pos)
        pos = self.expect(pos, "sym", ";")
        body, pos = self.parse_expr(pos)
        return {"tag": "with", "env": env_expr, "body": body}, pos

    def parse_assert(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "kw", "assert")
        cond, pos = self.parse_expr(pos)
        pos = self.expect(pos, "sym", ";")
        body, pos = self.parse_expr(pos)
        return {"tag": "assert", "cond": cond, "body": body}, pos

    def parse_import(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "kw", "import")
        token = self.tok(pos)
        if token.kind != "path":
            pnix_error("import expects a relative path literal like `./file.px`")
        return {"tag": "import", "path": str(token.value)}, pos + 1

    def construct_head(self, token: Token, pos: int) -> bool:
        if token.kind != "ident" or not str(token.value):
            return False
        return str(token.value)[0].isupper() and self.tok_is(pos + 1, "sym", "(") and self.adjacent(pos, pos + 1)

    def parse_construct(self, pos: int) -> tuple[dict[str, Any], int]:
        token = self.tok(pos)
        variant = str(token.value)
        pos = self.expect(pos + 1, "sym", "(")
        args: list[dict[str, Any]] = []
        while not self.tok_is(pos, "sym", ")"):
            if self.tok(pos).kind == "eof":
                pnix_error("expected `)` before end of constructor")
            arg, pos = self.parse_expr(pos)
            args.append(arg)
            if self.tok_is(pos, "sym", ","):
                pos += 1
        return {"tag": "construct", "variant": variant, "args": args}, pos + 1

    def parse_list(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "sym", "[")
        items: list[dict[str, Any]] = []
        while not self.tok_is(pos, "sym", "]"):
            if self.tok(pos).kind == "eof":
                pnix_error("expected `]` before end of input")
            item, pos = self.parse_unary(pos) if self.tok(pos).kind == "sym" and self.tok(pos).value in {"!", "-"} else self.parse_postfix(pos)
            items.append(item)
            if self.tok_is(pos, "sym", ","):
                pos += 1
        return {"tag": "list", "items": items}, pos + 1

    def parse_attrset(self, pos: int, recursive: bool) -> tuple[dict[str, Any], int]:
        if recursive:
            pos = self.expect(self.expect(pos, "kw", "rec"), "sym", "{")
        else:
            pos = self.expect(pos, "sym", "{")
        bindings, pos = self.parse_bindings_until(pos, "sym", "}", "attr")
        return {"tag": "attrset", "recursive": recursive, "bindings": bindings}, pos

    def parse_dynamic_attr_segment(self, pos: int) -> tuple[dict[str, Any], int]:
        token = self.tok(pos)
        if self.tok_is(pos, "sym", "${"):
            expr, pos = self.parse_expr(pos + 1)
            return {"expr": expr}, self.expect(pos, "sym", "}")
        if token.kind in {"ident", "string", "kw"}:
            return {"lit": str(token.value), "quoted": token.kind == "string"}, pos + 1
        # interpolated-string segment `? "${e}"` / `? "a${e}"` (Nix core;
        # the harvested numeric-evaluator uses env ? "${ast.name}").
        if token.kind == "string_interp":
            parts = []
            for part in token.value:
                if "lit" in part:
                    parts.append({"lit": part["lit"]})
                else:
                    parts.append({"expr": parse(part["interp"])})
            if len(parts) == 1 and "expr" in parts[0]:
                return {"expr": parts[0]["expr"]}, pos + 1
            return {"expr": {"tag": "str_interp", "parts": parts}}, pos + 1
        pnix_error(f"expected attribute path segment at token {pos}")
        raise AssertionError("unreachable")

    def parse_attr_segments(self, pos: int) -> tuple[list[dict[str, Any]], int]:
        segment, pos = self.parse_dynamic_attr_segment(pos)
        segments = [segment]
        while self.tok_is(pos, "sym", "."):
            segment, pos = self.parse_dynamic_attr_segment(pos + 1)
            segments.append(segment)
        return segments, pos

    def static_segments(self, segments: list[dict[str, Any]]) -> str | None:
        if any("expr" in segment for segment in segments):
            return None
        return ".".join(str(segment["lit"]) for segment in segments)

    def static_segment_path(self, segments: list[dict[str, Any]]) -> list[str]:
        return [str(segment["lit"]) for segment in segments]

    def parse_pattern_list(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "sym", "[")
        items: list[dict[str, Any]] = []
        rest: str | None = None
        while not self.tok_is(pos, "sym", "]"):
            if self.tok(pos).kind == "eof":
                pnix_error("expected `]` before end of list pattern")
            if self.tok_is(pos, "sym", "..."):
                token = self.tok(pos + 1)
                if token.kind != "ident":
                    pnix_error(f"expected identifier after list pattern `...` at token {pos}")
                rest = str(token.value)
                pos += 2
                if self.tok_is(pos, "sym", ","):
                    pos += 1
                break
            item, pos = self.parse_pattern(pos)
            items.append(item)
            if self.tok_is(pos, "sym", ","):
                pos += 1
        return {"tag": "list", "items": items, "rest": rest}, pos + 1

    def parse_pattern_attrset(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "sym", "{")
        fields: list[dict[str, Any]] = []
        ellipsis = False
        while not self.tok_is(pos, "sym", "}"):
            if self.tok_is(pos, "sym", "..."):
                ellipsis = True
                pos += 1
                if self.tok_is(pos, "sym", ",") or self.tok_is(pos, "sym", ";"):
                    pos += 1
                continue
            token = self.tok(pos)
            if token.kind not in {"ident", "string"}:
                pnix_error(f"expected attribute name in pattern at token {pos}")
            name = str(token.value)
            pos += 1
            field: dict[str, Any]
            if self.tok_is(pos, "sym", "="):
                pattern, pos = self.parse_pattern(pos + 1)
                field = {"name": name, "pattern": pattern}
            elif self.tok_is(pos, "sym", "?"):
                default, pos = self.parse_expr(pos + 1)
                field = {"name": name, "pattern": {"tag": "var", "name": name}, "default": default}
            else:
                pattern = {"tag": "var", "name": name}
                field = {"name": name, "pattern": pattern}
            fields.append(field)
            if self.tok_is(pos, "sym", ",") or self.tok_is(pos, "sym", ";"):
                pos += 1
        return {"tag": "attrset", "fields": fields, "ellipsis": ellipsis}, pos + 1

    def parse_pattern_construct(self, pos: int) -> tuple[dict[str, Any], int]:
        token = self.tok(pos)
        variant = str(token.value)
        pos = self.expect(pos + 1, "sym", "(")
        args: list[dict[str, Any]] = []
        while not self.tok_is(pos, "sym", ")"):
            if self.tok(pos).kind == "eof":
                pnix_error("expected `)` before end of constructor pattern")
            arg, pos = self.parse_pattern(pos)
            args.append(arg)
            if self.tok_is(pos, "sym", ","):
                pos += 1
        return {"tag": "constructor", "variant": variant, "args": args}, pos + 1

    def parse_pattern_atom(self, pos: int) -> tuple[dict[str, Any], int]:
        token = self.tok(pos)
        if token.kind == "number":
            return {"tag": "literal", "value": token.value}, pos + 1
        if token.kind == "string":
            return {"tag": "literal", "value": token.value}, pos + 1
        if token.kind == "ident":
            name = str(token.value)
            if self.tok_is(pos + 1, "sym", "@"):
                pattern, pos = self.parse_pattern(pos + 2)
                return {"tag": "as", "name": name, "pattern": pattern}, pos
            if name == "_":
                return {"tag": "wildcard"}, pos + 1
            if self.construct_head(token, pos):
                return self.parse_pattern_construct(pos)
            return {"tag": "var", "name": name}, pos + 1
        if token.kind == "kw":
            if token.value == "true":
                return {"tag": "literal", "value": True}, pos + 1
            if token.value == "false":
                return {"tag": "literal", "value": False}, pos + 1
            if token.value == "null":
                return {"tag": "literal", "value": None}, pos + 1
        if token.kind == "sym":
            if token.value == "[":
                return self.parse_pattern_list(pos)
            if token.value == "{":
                return self.parse_pattern_attrset(pos)
        pnix_error(f"unexpected match pattern token {token!r} at token {pos}")
        raise AssertionError("unreachable")

    def parse_pattern(self, pos: int) -> tuple[dict[str, Any], int]:
        pattern, pos = self.parse_pattern_atom(pos)
        if self.tok_is(pos, "sym", "@"):
            token = self.tok(pos + 1)
            if token.kind != "ident":
                pnix_error(f"expected identifier after pattern `@` at token {pos}")
            return {"tag": "as", "name": str(token.value), "pattern": pattern}, pos + 2
        return pattern, pos

    def try_parse_lambda_head(self, pos: int) -> tuple[dict[str, Any], int] | None:
        token = self.tok(pos)
        if token.kind == "ident" and self.tok_is(pos + 1, "sym", ":"):
            return {"param": str(token.value)}, pos + 2
        if token.kind == "ident" and self.tok_is(pos + 1, "sym", "@"):
            try:
                pattern, after = self.parse_pattern(pos)
            except PnixError:
                return None
            if self.tok_is(after, "sym", ":"):
                return {"pattern": pattern}, after + 1
            return None
        if token.kind == "sym" and token.value in {"{", "["}:
            try:
                pattern, after = self.parse_pattern(pos)
            except PnixError:
                return None
            if self.tok_is(after, "sym", ":"):
                return {"pattern": pattern}, after + 1
        return None

    def parse_match(self, pos: int) -> tuple[dict[str, Any], int]:
        pos = self.expect(pos, "kw", "match")
        scrutinee, pos = self.parse_expr(pos)
        pos = self.expect(pos, "kw", "with")
        arms: list[dict[str, Any]] = []
        while self.tok_is(pos, "sym", "|"):
            pattern, pos = self.parse_pattern(pos + 1)
            guard = None
            if self.tok_is(pos, "kw", "if"):
                guard, pos = self.parse_expr(pos + 1)
            pos = self.expect(pos, "sym", "=>")
            body, pos = self.parse_expr(pos)
            arm = {"pattern": pattern, "body": body}
            if guard is not None:
                arm["guard"] = guard
            arms.append(arm)
        if not arms:
            pnix_error("match requires at least one arm")
        return {"tag": "match", "scrutinee": scrutinee, "arms": arms}, pos

    def parse_primary(self, pos: int) -> tuple[dict[str, Any], int]:
        token = self.tok(pos)
        if token.kind == "number":
            if type(token.value) is float:
                return {"tag": "float", "value": token.value}, pos + 1
            return {"tag": "int", "value": token.value}, pos + 1
        if token.kind == "number_application":
            func, arg, _length = token.value
            return {
                "tag": "apply",
                "func": {"tag": "int", "value": func},
                "arg": {"tag": "float", "value": arg},
            }, pos + 1
        if token.kind == "path":
            return {"tag": "path", "value": token.value}, pos + 1
        if token.kind == "path_interp":
            parts = []
            for part in token.value:
                if "lit" in part:
                    parts.append({"lit": part["lit"]})
                else:
                    parts.append({"expr": parse(part["interp"])})
            return {"tag": "path_interp", "parts": parts}, pos + 1
        if token.kind == "string":
            return {"tag": "string", "value": token.value}, pos + 1
        if token.kind == "uri":
            return {"tag": "string", "value": token.value}, pos + 1
        if token.kind == "string_interp":
            parts = []
            for part in token.value:
                if "lit" in part:
                    parts.append({"lit": part["lit"]})
                else:
                    parts.append({"expr": parse(part["interp"])})
            return {"tag": "str_interp", "parts": parts}, pos + 1
        if token.kind == "ident":
            if self.construct_head(token, pos):
                return self.parse_construct(pos)
            return {"tag": "var", "name": token.value, "pos": token.pos}, pos + 1
        if token.kind == "kw":
            if token.value == "true":
                return {"tag": "bool", "value": True}, pos + 1
            if token.value == "false":
                return {"tag": "bool", "value": False}, pos + 1
            if token.value == "null":
                return {"tag": "null"}, pos + 1
            if token.value == "let":
                return self.parse_let(pos)
            if token.value == "if":
                return self.parse_if(pos)
            if token.value == "rec":
                return self.parse_attrset(pos, True)
            if token.value == "import":
                return self.parse_import(pos)
            if token.value == "with":
                return self.parse_with(pos)
            if token.value == "assert":
                return self.parse_assert(pos)
            if token.value == "match":
                # SOFT keyword (audit 2026-07-09): `match` is NOT reserved in
                # Nix/clj/rs — portable code may bind a local `match` (the
                # harvested term-dag does). Try the extension syntax; on a
                # parse failure treat it as a plain variable reference.
                try:
                    return self.parse_match(pos)
                except PnixError:
                    return {"tag": "var", "name": "match"}, pos + 1
            pnix_error(f"unexpected keyword `{token.value}` at token {pos}")
        if token.kind == "sym":
            if token.value == "(":
                node, pos = self.parse_expr(pos + 1)
                return node, self.expect(pos, "sym", ")")
            if token.value == "[":
                return self.parse_list(pos)
            if token.value == "{":
                return self.parse_attrset(pos, False)
        pnix_error(f"unexpected token {token!r} at token {pos}")
        raise AssertionError("unreachable")

    def parse_postfix(self, pos: int) -> tuple[dict[str, Any], int]:
        node, pos = self.parse_primary(pos)
        while True:
            if self.tok_is(pos, "sym", "."):
                nxt = self.tok(pos + 1)
                if self.tok_is(pos + 1, "sym", "${"):
                    segments, pos = self.parse_attr_segments(pos + 1)
                    node = {"tag": "dynamic_select", "base": node, "segments": segments}
                    if self.tok_is(pos, "kw", "or"):
                        default, pos = self.parse_expr(pos + 1)
                        node = {
                            "tag": "dynamic_select_default",
                            "base": node["base"],
                            "segments": node["segments"],
                            "default": default,
                        }
                    continue
                if nxt.kind == "string_interp":
                    # `.${…}`-equivalent via an interpolated string selector
                    # (`e."${k}"` — Nix core; harvested numeric-evaluator).
                    parts = []
                    for part in nxt.value:
                        if "lit" in part:
                            parts.append({"lit": part["lit"]})
                        else:
                            parts.append({"expr": parse(part["interp"])})
                    seg = (parts[0] if len(parts) == 1 and "expr" in parts[0]
                           else {"expr": {"tag": "str_interp", "parts": parts}})
                    node = {"tag": "dynamic_select", "base": node, "segments": [seg]}
                    pos += 2
                    if self.tok_is(pos, "kw", "or"):
                        default, pos = self.parse_expr(pos + 1)
                        node = {
                            "tag": "dynamic_select_default",
                            "base": node["base"],
                            "segments": node["segments"],
                            "default": default,
                        }
                    continue
                if nxt.kind not in {"ident", "string", "kw"}:
                    pnix_error(f"expected selector after `.` at token {pos}")
                node = {"tag": "select", "base": node, "attr": str(nxt.value)}
                pos += 2
                if self.tok_is(pos, "kw", "or"):
                    default, pos = self.parse_expr(pos + 1)
                    node = {"tag": "select_default", "base": node["base"], "attr": node["attr"], "default": default}
            elif self.tok_is(pos, "sym", "[") and self.adjacent(pos - 1, pos):
                index, pos = self.parse_expr(pos + 1)
                pos = self.expect(pos, "sym", "]")
                node = {"tag": "index", "base": node, "index": index}
            else:
                return node, pos

    def primary_start(self, token: Token) -> bool:
        if token.kind in {"number", "number_application", "string", "string_interp", "uri", "ident", "path", "path_interp"}:
            return True
        if token.kind == "kw":
            return token.value in {"true", "false", "null", "let", "if", "rec", "import"}
        if token.kind == "sym":
            return token.value in {"(", "[", "{"}
        return False

    def parse_apply(self, pos: int) -> tuple[dict[str, Any], int]:
        node, pos = self.parse_postfix(pos)
        while node["tag"] in {
            "var",
            "select",
            "select_default",
            "dynamic_select",
            "dynamic_select_default",
            "index",
            "apply",
            "lambda",
            "import",
            "with",
            "assert",
            "match",
            "construct",
        } and self.primary_start(self.tok(pos)):
            arg, pos = self.parse_postfix(pos)
            node = {"tag": "apply", "func": node, "arg": arg}
        return node, pos

    def parse_has_attr(self, pos: int) -> tuple[dict[str, Any], int]:
        # Nix precedence: application binds tighter than `?`.
        #
        #   f { } ? a  ==  (f { }) ? a
        #
        # Keeping `?` in parse_postfix instead made the RHS attr test an
        # application argument (`f ({} ? a)`), unlike pnix-meta/clj/rs.
        node, pos = self.parse_apply(pos)
        while self.tok_is(pos, "sym", "?"):
            segments, pos = self.parse_attr_segments(pos + 1)
            static_attr = self.static_segments(segments)
            if static_attr is None:
                node = {"tag": "dynamic_has_attr", "base": node, "segments": segments}
            else:
                node = {
                    "tag": "has_attr",
                    "base": node,
                    "attr": static_attr,
                    "path": self.static_segment_path(segments),
                }
        return node, pos

    def parse_unary(self, pos: int) -> tuple[dict[str, Any], int]:
        token = self.tok(pos)
        if token.kind == "sym" and token.value in {"!", "-"}:
            arg, pos = self.parse_unary(pos + 1)
            return {"tag": "unary", "op": token.value, "arg": arg}, pos
        return self.parse_has_attr(pos)

    def parse_left(
        self, pos: int, sub_parser: Callable[[int], tuple[dict[str, Any], int]], ops: set[str]
    ) -> tuple[dict[str, Any], int]:
        node, pos = sub_parser(pos)
        while self.tok(pos).kind == "sym" and self.tok(pos).value in ops:
            op = self.tok(pos).value
            rhs, pos = sub_parser(pos + 1)
            node = {"tag": "binary", "op": op, "lhs": node, "rhs": rhs}
        return node, pos

    def parse_mul(self, pos: int) -> tuple[dict[str, Any], int]:
        return self.parse_left(pos, self.parse_unary, {"*", "/", "%"})

    def parse_add(self, pos: int) -> tuple[dict[str, Any], int]:
        return self.parse_left(pos, self.parse_mul, {"+", "-", "++"})

    def parse_merge(self, pos: int) -> tuple[dict[str, Any], int]:
        node, pos = self.parse_add(pos)
        if self.tok(pos).kind == "sym" and self.tok(pos).value == "//":
            rhs, pos = self.parse_merge(pos + 1)
            return {"tag": "binary", "op": "//", "lhs": node, "rhs": rhs}, pos
        return node, pos

    def parse_compare(self, pos: int) -> tuple[dict[str, Any], int]:
        return self.parse_left(pos, self.parse_merge, {"<", "<=", ">", ">="})

    def parse_eq(self, pos: int) -> tuple[dict[str, Any], int]:
        return self.parse_left(pos, self.parse_compare, {"==", "!="})

    def parse_and(self, pos: int) -> tuple[dict[str, Any], int]:
        return self.parse_left(pos, self.parse_eq, {"&&"})

    def parse_or(self, pos: int) -> tuple[dict[str, Any], int]:
        return self.parse_left(pos, self.parse_and, {"||"})

    def parse_implication(self, pos: int) -> tuple[dict[str, Any], int]:
        node, pos = self.parse_or(pos)
        if self.tok(pos).kind == "sym" and self.tok(pos).value == "->":
            rhs, pos = self.parse_implication(pos + 1)
            return {"tag": "binary", "op": "->", "lhs": node, "rhs": rhs}, pos
        return node, pos

    def parse_expr(self, pos: int) -> tuple[dict[str, Any], int]:
        lambda_head = self.try_parse_lambda_head(pos)
        if lambda_head is not None:
            body, pos = self.parse_expr(lambda_head[1])
            if "param" in lambda_head[0]:
                return {"tag": "lambda", "param": lambda_head[0]["param"], "body": body}, pos
            return {"tag": "lambda", "param": None, "pattern": lambda_head[0]["pattern"], "body": body}, pos
        return self.parse_implication(pos)


def parse(source: str) -> dict[str, Any]:
    try:
        tokens = tokenize(source)
        parser = Parser(tokens)
        ast, pos = parser.parse_expr(0)
        if pos != len(tokens):
            trailing = tokens[pos:]
            pnix_error(f"unexpected trailing tokens at {pos}: {trailing!r}")
        return ast
    except PnixError as exc:
        # Parsing owns the phase. Diagnostic wording is deliberately irrelevant
        # to the production outcome classifier.
        exc.phase = "parse"
        exc.error_class = "syntax-error"
        raise


_RESERVED_WORDS = {
    "let",
    "in",
    "rec",
    "if",
    "then",
    "else",
    "true",
    "false",
    "null",
    "import",
    "with",
    "assert",
    "match",
    "inherit",
    "or",
}
_IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_'-]*$")


def _ident_token(value: str) -> bool:
    return bool(_IDENT_RE.match(value)) and value not in _RESERVED_WORDS


def emit_string_literal(value: str) -> str:
    out = ['"']
    for c in value:
        out.append(
            {
                "\\": "\\\\",
                '"': '\\"',
                "\n": "\\n",
                "\r": "\\r",
                "\t": "\\t",
                "$": "\\$",
            }.get(c, c)
        )
    out.append('"')
    return "".join(out)


def emit_attr_name(value: str) -> str:
    return value if _ident_token(value) else emit_string_literal(value)


def emit_path_part(part: Any) -> str:
    if isinstance(part, str):
        return emit_attr_name(part)
    return "${" + emit_source(part["expr"]) + "}"


def emit_path(path: list[Any]) -> str:
    return ".".join(emit_path_part(part) for part in path)


def emit_binding(binding: dict[str, Any]) -> str:
    if (
        binding.get("inherit_plain")
        and len(binding["path"]) == 1
        and binding["value"].get("tag") == "var"
        and binding["value"].get("name") == binding["path"][0]
    ):
        return f"inherit {emit_attr_name(binding['path'][0])};"
    return f"{emit_path(binding['path'])} = {emit_source(binding['value'])};"


def emit_attr_segments(segments: list[dict[str, Any]]) -> str:
    parts = []
    for segment in segments:
        if "lit" in segment:
            parts.append(emit_attr_name(str(segment["lit"])))
        else:
            parts.append("${" + emit_source(segment["expr"]) + "}")
    return ".".join(parts)


def emit_path_interp_parts(parts: list[dict[str, Any]]) -> str:
    out = []
    for part in parts:
        if "lit" in part:
            out.append(str(part["lit"]))
        else:
            out.append("${" + emit_source(part["expr"]) + "}")
    return "".join(out)


def emit_has_attr_path(ast: dict[str, Any]) -> str:
    return ".".join(emit_attr_name(part) for part in ast.get("path", [ast["attr"]]))


def emit_pattern(pattern: dict[str, Any]) -> str:
    tag = pattern["tag"]
    if tag == "wildcard":
        return "_"
    if tag == "as":
        return pattern["name"] + "@" + emit_pattern(pattern["pattern"])
    if tag == "var":
        return pattern["name"]
    if tag == "literal":
        value = pattern["value"]
        if value is True:
            return "true"
        if value is False:
            return "false"
        if value is None:
            return "null"
        if type(value) is str:
            return emit_string_literal(value)
        return repr(value)
    if tag == "list":
        items = [emit_pattern(item) for item in pattern["items"]]
        if pattern.get("rest") is not None:
            items.append("..." + pattern["rest"])
        return "[ " + ", ".join(items) + " ]"
    if tag == "attrset":
        fields = []
        for field in pattern["fields"]:
            if "default" in field and field["pattern"] == {"tag": "var", "name": field["name"]}:
                fields.append(emit_attr_name(field["name"]) + " ? " + emit_source(field["default"]))
            else:
                fields.append(emit_attr_name(field["name"]) + " = " + emit_pattern(field["pattern"]))
        if pattern.get("ellipsis"):
            fields.append("...")
        return "{ " + "; ".join(fields) + " }"
    if tag == "constructor":
        return pattern["variant"] + "(" + ", ".join(emit_pattern(arg) for arg in pattern["args"]) + ")"
    pnix_error(f"unsupported pattern tag for emit-source {tag!r}")
    raise AssertionError("unreachable")


def emit_float_source(value: float) -> str:
    if math.isnan(value):
        pnix_error("cannot emit NaN as pnix source")
    if math.isinf(value):
        return "1.0e309" if value > 0 else "(-1.0e309)"
    source = repr(value)
    if "e" in source and "." not in source.split("e", 1)[0]:
        source = source.replace("e", ".0e", 1)
    return source


def emit_list_item_source(ast: dict[str, Any]) -> str:
    source = emit_source(ast)
    return source if ast["tag"] in {"int", "float", "path", "path_interp", "string", "str_interp", "bool", "null", "var", "attrset", "list", "select", "select_default", "dynamic_select", "dynamic_select_default", "index", "construct"} else f"({source})"


def emit_source(ast: dict[str, Any]) -> str:
    tag = ast["tag"]
    if tag == "int":
        return str(ast["value"])
    if tag == "float":
        return emit_float_source(ast["value"])
    if tag == "path":
        return ast["value"]
    if tag == "path_interp":
        return emit_path_interp_parts(ast["parts"])
    if tag == "string":
        return emit_string_literal(ast["value"])
    if tag == "str_interp":
        out = ['"']
        for part in ast["parts"]:
            if "lit" in part:
                out.append(emit_string_literal(part["lit"])[1:-1])
            else:
                out.append("${")
                out.append(emit_source(part["expr"]))
                out.append("}")
        out.append('"')
        return "".join(out)
    if tag == "bool":
        return "true" if ast["value"] else "false"
    if tag == "null":
        return "null"
    if tag == "var":
        return ast["name"]
    if tag == "import":
        return "import " + ast["path"]
    if tag == "construct":
        return ast["variant"] + "(" + ", ".join(emit_source(arg) for arg in ast["args"]) + ")"
    if tag == "list":
        return "[ " + " ".join(emit_list_item_source(item) for item in ast["items"]) + " ]"
    if tag == "attrset":
        prefix = "rec " if ast["recursive"] else ""
        return prefix + "{ " + " ".join(emit_binding(b) for b in ast["bindings"]) + " }"
    if tag == "let":
        return (
            "let "
            + " ".join(emit_binding(b) for b in ast["bindings"])
            + " in "
            + emit_source(ast["body"])
        )
    if tag == "lambda":
        if ast.get("pattern") is not None:
            return emit_pattern(ast["pattern"]) + ": " + emit_source(ast["body"])
        return ast["param"] + ": " + emit_source(ast["body"])
    if tag == "apply":
        return "(" + emit_source(ast["func"]) + ") (" + emit_source(ast["arg"]) + ")"
    if tag == "if":
        return (
            "if ("
            + emit_source(ast["cond"])
            + ") then ("
            + emit_source(ast["then"])
            + ") else ("
            + emit_source(ast["else"])
            + ")"
        )
    if tag == "with":
        return "with (" + emit_source(ast["env"]) + "); " + emit_source(ast["body"])
    if tag == "assert":
        return "assert (" + emit_source(ast["cond"]) + "); " + emit_source(ast["body"])
    if tag == "select":
        return "(" + emit_source(ast["base"]) + ")." + emit_attr_name(ast["attr"])
    if tag == "select_default":
        return (
            "("
            + emit_source(ast["base"])
            + ")."
            + emit_attr_name(ast["attr"])
            + " or ("
            + emit_source(ast["default"])
            + ")"
        )
    if tag == "dynamic_select":
        return "(" + emit_source(ast["base"]) + ")." + emit_attr_segments(ast["segments"])
    if tag == "dynamic_select_default":
        return (
            "("
            + emit_source(ast["base"])
            + ")."
            + emit_attr_segments(ast["segments"])
            + " or ("
            + emit_source(ast["default"])
            + ")"
        )
    if tag == "has_attr":
        return "(" + emit_source(ast["base"]) + ") ? " + emit_has_attr_path(ast)
    if tag == "dynamic_has_attr":
        return "(" + emit_source(ast["base"]) + ") ? " + emit_attr_segments(ast["segments"])
    if tag == "index":
        return "(" + emit_source(ast["base"]) + ")[" + emit_source(ast["index"]) + "]"
    if tag == "match":
        arms = [
            "| "
            + emit_pattern(arm["pattern"])
            + (" if " + emit_source(arm["guard"]) if "guard" in arm else "")
            + " => "
            + emit_source(arm["body"])
            for arm in ast["arms"]
        ]
        return "match (" + emit_source(ast["scrutinee"]) + ") with " + " ".join(arms)
    if tag == "unary":
        return ast["op"] + "(" + emit_source(ast["arg"]) + ")"
    if tag == "binary":
        return (
            "("
            + emit_source(ast["lhs"])
            + ") "
            + ast["op"]
            + " ("
            + emit_source(ast["rhs"])
            + ")"
        )
    pnix_error(f"unsupported AST tag for emit-source {tag!r}")
    raise AssertionError("unreachable")


def force_value(value: Any) -> Any:
    if isinstance(value, Thunk):
        return value.force()
    return value


def is_closure(value: Any) -> bool:
    return isinstance(value, Closure)


def is_native(value: Any) -> bool:
    return isinstance(value, NativeFunc) or (callable(value) and not is_closure(value))


def attrset_value(value: Any, label: str) -> dict[str, Any]:
    value = force_value(value)
    if isinstance(value, dict):
        return value
    pnix_error(f"{label} must be an attrset")
    raise AssertionError("unreachable")


def list_value(value: Any, label: str) -> list[Any]:
    value = force_value(value)
    if isinstance(value, list):
        return value
    pnix_error(f"{label} must be a list")
    raise AssertionError("unreachable")


def bool_value(
    value: Any,
    label: str,
    *,
    error_class: str = "type-error",
) -> bool:
    value = force_value(value)
    if type(value) is bool:
        return value
    pnix_error(
        f"{label}: expected bool, got {_type_of(value)}",
        error_class=error_class,
    )
    raise AssertionError("unreachable")


def number_value(value: Any, label: str) -> int | float:
    value = force_value(value)
    if type(value) is int or type(value) is float:
        return value
    pnix_error(f"{label} must be a number", error_class="type-error")
    raise AssertionError("unreachable")


def integer_value(value: Any, label: str) -> int:
    value = force_value(value)
    if type(value) is int:
        return value
    pnix_error(f"{label} must be an integer")
    raise AssertionError("unreachable")


def nonnegative_count_value(value: Any, label: str) -> int:
    count = integer_value(value, label)
    if count < 0:
        pnix_error(f"{label}: negative count")
    return count


def is_string_value(value: Any) -> bool:
    return isinstance(value, str) and not isinstance(value, PnixPath)


def context_of_string(value: Any) -> set[str]:
    value = force_value(value)
    if isinstance(value, PnixString):
        return set(value.context)
    return set()


def make_context_string(text: Any, context: Any | None = None) -> str:
    ctx = {str(item) for item in (context or ())}
    return PnixString(str(text), ctx) if ctx else str(text)


def string_text_context(value: Any, label: str) -> tuple[str, set[str]]:
    value = force_value(value)
    if is_string_value(value):
        return str(value), context_of_string(value)
    pnix_error(f"{label} ({_type_of(value)}) must be a string")
    raise AssertionError("unreachable")


def string_value(value: Any, label: str) -> str:
    value = force_value(value)
    if is_string_value(value):
        return str(value)
    pnix_error(f"{label} ({_type_of(value)}) must be a string")
    raise AssertionError("unreachable")


def px_str_bytes(s: str) -> bytes:
    # RAW-BYTE track (2026-07-11): strings may smuggle raw bytes via
    # surrogateescape (the Nix-permitted invalid-UTF-8 intermediates).
    return s.encode("utf-8", errors="surrogateescape")


def px_revalidate(s: str) -> str:
    # a concat that recombines to valid UTF-8 returns to a clean str
    # (oracle: substring 0 1 "가" + substring 1 2 "가" == "가").
    data = px_str_bytes(s)
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return s


def string_byte_length(value: Any, label: str) -> int:
    return len(px_str_bytes(string_value(value, label)))


def length_value(value: Any) -> int:
    value = force_value(value)
    if isinstance(value, list):
        return len(value)
    if is_string_value(value):
        return len(str(value).encode("utf-8"))
    pnix_error(f"builtins.length: expected list or string, got {_type_of(value)}")
    raise AssertionError("unreachable")


def substring_value(start: Any, length: Any, text: Any) -> str:
    start_i = number_value(start, "builtins.substring start")
    length_i = number_value(length, "builtins.substring length")
    if type(start_i) is float or type(length_i) is float:
        pnix_error("builtins.substring start and length must be integers")
    if start_i < 0:
        pnix_error(f"builtins.substring: negative start position {start_i} not allowed")
    source, context = string_text_context(text, "builtins.substring string")
    data = px_str_bytes(source)
    start_b = int(start_i)
    if start_b >= len(data):
        return make_context_string("", context)
    raw_end = len(data) if length_i < 0 else min(start_b + int(length_i), len(data))
    end_b = max(raw_end, start_b)
    # RAW-BYTE track: slice at ANY offset; off-boundary cuts smuggle their
    # raw bytes via surrogateescape (revalidated on later concat).
    return make_context_string(
        data[start_b:end_b].decode("utf-8", errors="surrogateescape"), context)


def replace_strings_value(froms: Any, tos: Any, text: Any) -> str:
    from_items = force_value(froms)
    if not isinstance(from_items, list):
        pnix_error(f"builtins.replaceStrings: 'from' must be list, got {_type_of(from_items)}")
    to_items = force_value(tos)
    if not isinstance(to_items, list):
        pnix_error(f"builtins.replaceStrings: 'to' must be list, got {_type_of(to_items)}")
    if len(from_items) != len(to_items):
        pnix_error("builtins.replaceStrings: `from` and `to` lists must have equal length")
    text_f = force_value(text)
    if not is_string_value(text_f):
        pnix_error(f"builtins.replaceStrings: third argument must be string, got {_type_of(text_f)}")
    haystack, context = string_text_context(text_f, "builtins.replaceStrings string")
    if not from_items:
        return make_context_string(haystack, context)
    patterns: list[str] = []
    for item in from_items:
        item_f = force_value(item)
        if not is_string_value(item_f):
            pnix_error(f"builtins.replaceStrings: 'from' element must be string, got {_type_of(item_f)}")
        patterns.append(str(item_f))
    replacements: list[str | None] = [None] * len(to_items)
    out: list[str] = []
    i = 0
    while i <= len(haystack):
        matched: tuple[int, int] | None = None
        tail = haystack[i:]
        for idx, pattern in enumerate(patterns):
            if tail.startswith(pattern):
                matched = (idx, len(pattern))
                break
        if matched is None:
            if i < len(haystack):
                out.append(haystack[i])
            i += 1
            continue
        idx, pattern_len = matched
        if replacements[idx] is None:
            replacement, replacement_context = string_text_context(to_items[idx], "builtins.replaceStrings to element")
            replacements[idx] = replacement
            context.update(replacement_context)
        out.append(replacements[idx] or "")
        if pattern_len == 0:
            if i < len(haystack):
                out.append(haystack[i])
            i += 1
        else:
            i += pattern_len
    return make_context_string("".join(out), context)


def list_to_attrs_value(xs: Any) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for item in list_value(xs, "builtins.listToAttrs list"):
        entry = attrset_value(item, "builtins.listToAttrs element")
        name = string_value(entry.get("name"), "builtins.listToAttrs name")
        if "value" not in entry:
            pnix_error("builtins.listToAttrs element is missing `value`")
        if name not in out:
            out[name] = entry["value"]
    return out


def has_attr_value(name: Any, m: Any) -> bool:
    name = force_value(name)
    if not is_string_value(name):
        pnix_error(f"builtins.hasAttr: first argument must be string, got {_type_of(name)}")
    m = force_value(m)
    if not isinstance(m, dict):
        pnix_error(f"builtins.hasAttr: second argument must be attrset, got {_type_of(m)}")
    return str(name) in m


def get_attr_value(name: Any, m: Any) -> Any:
    key = str(force_value(name))
    m = attrset_arg_value(m, "getAttr")
    if key not in m:
        pnix_error(f"builtins.getAttr: attribute '{key}' missing")
    return force_value(m[key])


def remove_attrs_value(attrs: Any, names: Any) -> dict[str, Any]:
    source = force_value(attrs)
    if not isinstance(source, dict):
        pnix_error(f"builtins.removeAttrs: first argument must be attrset, got {_type_of(source)}")
    names = force_value(names)
    if not isinstance(names, list):
        pnix_error(f"builtins.removeAttrs: second argument must be list of strings, got {_type_of(names)}")
    remove: set[str] = set()
    for index, name in enumerate(names):
        name = force_value(name)
        if not is_string_value(name):
            pnix_error(f"builtins.removeAttrs: name-list element at index {index} is not a string, got {_type_of(name)}")
        remove.add(str(name))
    return {key: value for key, value in source.items() if key not in remove}


def attr_by_path_value(path: Any, default: Any, attrs: Any) -> Any:
    cur: Any = attrset_value(attrs, "builtins.attrByPath attrs")
    for part in list_value(path, "builtins.attrByPath path"):
        key = string_value(part, "builtins.attrByPath path element")
        if not isinstance(cur, dict) or key not in cur:
            return default
        cur = force_value(cur[key])
    return cur


def has_attr_path_value(attrs: dict[str, Any], path: list[str]) -> bool:
    cur: Any = attrs
    last = len(path) - 1
    for index, part in enumerate(path):
        if not isinstance(cur, dict) or part not in cur:
            return False
        if index == last:
            return True
        cur = force_value(cur[part])
    return True


def get_attr_from_path_value(path: Any, attrs: Any) -> Any:
    """builtins/lib.getAttrFromPath: walk path; error if missing."""
    cur: Any = attrset_value(attrs, "getAttrFromPath attrs")
    parts = [string_value(part, "getAttrFromPath path element") for part in list_value(path, "getAttrFromPath path")]
    for index, key in enumerate(parts):
        if not isinstance(cur, dict) or key not in cur:
            dotted = ".".join(parts)
            pnix_error(f"cannot find attribute `{dotted}`")
        cur = force_value(cur[key])
    return cur


def has_attr_by_path_value(path: Any, attrs: Any) -> bool:
    parts = [string_value(part, "hasAttrByPath path element") for part in list_value(path, "hasAttrByPath path")]
    return has_attr_path_value(attrset_value(attrs, "hasAttrByPath attrs"), parts)


def get_attr_from_path_or_value(attrs: Any, path: Any, default: Any) -> Any:
    """lib.getAttrFromPathOr attrs path default (README order)."""
    return attr_by_path_value(path, default, attrs)


def to_xml_escape_attr(text: str) -> str:
    return (
        text.replace("&", "&amp;")
        .replace("'", "&apos;")
        .replace('"', "&quot;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
    )


def to_xml_node(value: Any, indent: int = 2) -> list[str]:
    """Nix-ish XML body nodes (without the outer <?xml> / <expr> wrapper)."""
    pad = " " * indent
    value = force_value(value)
    if value is None:
        return [f"{pad}<null />"]
    if type(value) is bool:
        return [f"{pad}<bool value=\"{'true' if value else 'false'}\" />"]
    if type(value) is int:
        return [f"{pad}<int>{value}</int>"]
    if type(value) is float:
        return [f"{pad}<float>{value}</float>"]
    if isinstance(value, PnixPath):
        return [f'{pad}<path value="{to_xml_escape_attr(str(value))}" />']
    if is_string_value(value):
        return [f'{pad}<string value="{to_xml_escape_attr(str(value))}" />']
    if isinstance(value, list):
        lines = [f"{pad}<list>"]
        for item in value:
            lines.extend(to_xml_node(item, indent + 2))
        lines.append(f"{pad}</list>")
        return lines
    if isinstance(value, dict):
        lines = [f"{pad}<attrs>"]
        for key in sorted(value.keys()):
            lines.append(f'{pad}  <attr name="{to_xml_escape_attr(str(key))}">')
            lines.extend(to_xml_node(value[key], indent + 4))
            lines.append(f"{pad}  </attr>")
        lines.append(f"{pad}</attrs>")
        return lines
    if isinstance(value, (Closure, NativeFunc)) or callable(value):
        return [f"{pad}<unevaluated />"]
    return [f'{pad}<string value="{to_xml_escape_attr(str(value))}" />']


def to_xml_value(value: Any) -> str:
    body = "\n".join(to_xml_node(value, 2))
    return (
        "<?xml version='1.0' encoding='utf-8'?>\n"
        "<expr>\n"
        f"{body}\n"
        "</expr>\n"
    )


def list_last_value(xs: Any) -> Any:
    items = list_value(xs, "lib.last list")
    if not items:
        pnix_error("lib.last: list is empty")
    return force_value(items[-1])


def list_init_value(xs: Any) -> list[Any]:
    items = list_value(xs, "lib.init list")
    if not items:
        pnix_error("lib.init: list is empty")
    return items[:-1]


def remove_prefix_value(prefix: Any, text: Any) -> str:
    p = string_value(prefix, "lib.removePrefix prefix")
    s = string_value(text, "lib.removePrefix string")
    if s.startswith(p):
        return s[len(p) :]
    return s


def remove_suffix_value(suffix: Any, text: Any) -> str:
    su = string_value(suffix, "lib.removeSuffix suffix")
    s = string_value(text, "lib.removeSuffix string")
    if su and s.endswith(su):
        return s[: -len(su)]
    return s


def split_string_value(sep: Any, text: Any) -> list[str]:
    separator = string_value(sep, "lib.splitString separator")
    s = string_value(text, "lib.splitString string")
    if separator == "":
        return list(s)
    return s.split(separator)


def bool_to_string_value(value: Any) -> str:
    return "true" if bool_value(value, "lib.boolToString") else "false"


def unique_list_value(xs: Any) -> list[Any]:
    out: list[Any] = []
    for item in list_value(xs, "lib.unique list"):
        forced = force_value(item)
        if not any(eq_value(forced, existing, 1) for existing in out):
            out.append(forced)
    return out


def intersect_lists_value(a: Any, b: Any) -> list[Any]:
    left = [force_value(x) for x in list_value(a, "lib.intersectLists first")]
    right = [force_value(x) for x in list_value(b, "lib.intersectLists second")]
    out: list[Any] = []
    for item in left:
        if any(eq_value(item, r, 1) for r in right) and not any(eq_value(item, o, 1) for o in out):
            out.append(item)
    return out


def subtract_lists_value(e: Any, l: Any) -> list[Any]:
    """nixpkgs: subtractLists e l = filter (x: !elem x e) l"""
    excluded = [force_value(x) for x in list_value(e, "lib.subtractLists first")]
    source = [force_value(x) for x in list_value(l, "lib.subtractLists second")]
    return [item for item in source if not any(eq_value(item, ex, 1) for ex in excluded)]


def range_value(first: Any, last: Any) -> list[int]:
    start = integer_value(first, "lib.range first")
    end = integer_value(last, "lib.range last")
    if end < start:
        return []
    return list(range(start, end + 1))


def sum_list_value(xs: Any) -> int | float:
    total: int | float = 0
    for item in list_value(xs, "lib.sum list"):
        total = total + number_value(item, "lib.sum element")
    return total


def product_list_value(xs: Any) -> int | float:
    total: int | float = 1
    for item in list_value(xs, "lib.product list"):
        total = total * number_value(item, "lib.product element")
    return total


def recursive_update_value(lhs: Any, rhs: Any) -> dict[str, Any]:
    left = attrset_value(lhs, "lib.recursiveUpdate lhs")
    right = attrset_value(rhs, "lib.recursiveUpdate rhs")
    out = dict(left)
    for key, r_val in right.items():
        if key in out:
            l_forced = force_value(out[key])
            r_forced = force_value(r_val)
            if isinstance(l_forced, dict) and isinstance(r_forced, dict):
                out[key] = recursive_update_value(l_forced, r_forced)
            else:
                out[key] = r_val
        else:
            out[key] = r_val
    return out


def update_many_attrs_value(updates: Any, original: Any) -> dict[str, Any]:
    """Fold // over updates starting from original: original // u0 // u1 ..."""
    acc = dict(attrset_value(original, "lib.updateManyAttrs original"))
    for item in list_value(updates, "lib.updateManyAttrs updates"):
        acc = merge_attrsets(acc, item)
    return acc


def zip_lists_with_value(func: Any, xl: Any, yl: Any, ctx: dict[str, Any]) -> list[Any]:
    left = list_value(xl, "lib.zipListsWith first")
    right = list_value(yl, "lib.zipListsWith second")
    n = min(len(left), len(right))
    out: list[Any] = []
    for i in range(n):
        a = left[i]
        b = right[i]
        step = apply_pnix(func, Thunk(lambda a=a: force_value(a)), ctx)
        out.append(apply_pnix(step, Thunk(lambda b=b: force_value(b)), ctx))
    return out


def zip_lists_lib_value(xl: Any, yl: Any) -> list[Any]:
    left = list_value(xl, "lib.zipLists first")
    right = list_value(yl, "lib.zipLists second")
    return [{"fst": force_value(a), "snd": force_value(b)} for a, b in zip(left, right)]


def get_name_value(x: Any) -> str:
    value = force_value(x)
    if is_string_value(value):
        return str(parse_drv_name_value(value)["name"])
    if isinstance(value, dict):
        if "pname" in value:
            return string_value(value["pname"], "lib.getName pname")
        if "name" in value:
            return str(parse_drv_name_value(value["name"])["name"])
        pnix_error("lib.getName: attrset missing name/pname")
    pnix_error(f"lib.getName: expected string or attrset, got {_type_of(value)}")
    raise AssertionError("unreachable")


def get_version_value(x: Any) -> str:
    value = force_value(x)
    if is_string_value(value):
        return str(parse_drv_name_value(value)["version"])
    if isinstance(value, dict):
        if "version" in value:
            return string_value(value["version"], "lib.getVersion version")
        if "name" in value:
            return str(parse_drv_name_value(value["name"])["version"])
        pnix_error("lib.getVersion: attrset missing version/name")
    pnix_error(f"lib.getVersion: expected string or attrset, got {_type_of(value)}")
    raise AssertionError("unreachable")


def filter_attrs_recursive_value(pred: Any, attrs: Any, ctx: dict[str, Any]) -> dict[str, Any]:
    source = attrset_value(attrs, "lib.filterAttrsRecursive set")
    out: dict[str, Any] = {}
    for key, item in source.items():
        keep = bool_value(
            apply_pnix(
                apply_pnix(pred, Thunk(lambda key=key: key), ctx),
                Thunk(lambda item=item: force_value(item)),
                ctx,
            ),
            "lib.filterAttrsRecursive predicate",
        )
        if not keep:
            continue
        forced = force_value(item)
        if isinstance(forced, dict):
            out[key] = filter_attrs_recursive_value(pred, forced, ctx)
        else:
            out[key] = item
    return out


def map_attrs_recursive_value(func: Any, attrs: Any, ctx: dict[str, Any], path: list[str] | None = None) -> dict[str, Any]:
    path = path or []
    source = attrset_value(attrs, "lib.mapAttrsRecursive set")
    out: dict[str, Any] = {}
    for key, item in source.items():
        child_path = path + [key]
        forced = force_value(item)
        if isinstance(forced, dict):
            out[key] = map_attrs_recursive_value(func, forced, ctx, child_path)
        else:
            out[key] = Thunk(
                lambda child_path=child_path, item=item: apply_pnix(
                    apply_pnix(func, Thunk(lambda child_path=child_path: list(child_path)), ctx),
                    Thunk(lambda item=item: force_value(item)),
                    ctx,
                )
            )
    return out


def map_attrs_to_list_value(func: Any, attrs: Any, ctx: dict[str, Any]) -> list[Any]:
    source = attrset_value(attrs, "lib.mapAttrsToList set")
    out: list[Any] = []
    for key in sorted(source.keys()):
        value = source[key]
        step = apply_pnix(func, Thunk(lambda key=key: key), ctx)
        out.append(apply_pnix(step, Thunk(lambda value=value: force_value(value)), ctx))
    return out


def zip_attrs_value(sets: Any) -> dict[str, Any]:
    maps = [attrset_value(item, "lib.zipAttrs element") for item in list_value(sets, "lib.zipAttrs list")]
    keys = sorted({key for m in maps for key in m})
    return {key: [m[key] for m in maps if key in m] for key in keys}


def fix_value(func: Any, ctx: dict[str, Any]) -> Any:
    holder: dict[str, Any] = {}
    self_thunk = Thunk(lambda: holder["value"])
    holder["value"] = apply_pnix(func, self_thunk, ctx)
    return holder["value"]


def assert_msg_value(cond: Any, msg: Any) -> bool:
    if bool_value(cond, "lib.assertMsg condition"):
        return True
    pnix_catchable_error(string_value(msg, "lib.assertMsg message"))
    raise AssertionError("unreachable")


def pipe_value(value: Any, fns: Any, ctx: dict[str, Any]) -> Any:
    acc = value
    for fn in list_value(fns, "lib.pipe functions"):
        acc = apply_pnix(fn, Thunk(lambda acc=acc: force_value(acc)), ctx)
    return acc


def concat_map_strings_sep_value(sep: Any, func: Any, xs: Any, ctx: dict[str, Any]) -> str:
    separator = string_value(sep, "lib.concatMapStringsSep separator")
    parts: list[str] = []
    for item in list_value(xs, "lib.concatMapStringsSep list"):
        mapped = apply_pnix(func, Thunk(lambda item=item: force_value(item)), ctx)
        parts.append(value_to_string(mapped, ctx))
    return separator.join(parts)


def _store_cache_dir() -> Path:
    store_dir = Path(tempfile.gettempdir()) / "pnix-nix-store"
    store_dir.mkdir(parents=True, exist_ok=True)
    return store_dir


def _fetch_url_download(url: str, name: str | None = None) -> PnixPath:
    file_name = name or url.rstrip("/").rsplit("/", 1)[-1] or "download"
    digest = hashlib.sha256(url.encode("utf-8")).hexdigest()[:32]
    out = _store_cache_dir() / f"{digest}-{safe_store_name(file_name)}"
    if out.exists():
        return PnixPath(str(out.resolve()))
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "pnix-hy-fetchurl/1.0"})
        with urllib.request.urlopen(req, timeout=60) as response:
            data = response.read()
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        pnix_error(f"builtins.fetchurl: failed to download `{url}`: {exc}")
        raise AssertionError("unreachable")
    out.write_bytes(data)
    return PnixPath(str(out.resolve()))


def fetchurl_value(arg: Any) -> PnixPath:
    value = force_value(arg)
    if is_string_value(value) or isinstance(value, PnixPath):
        return _fetch_url_download(str(value))
    if isinstance(value, dict):
        if "url" not in value:
            pnix_error("builtins.fetchurl: attrset must have `url`")
        url = string_value(value["url"], "builtins.fetchurl url")
        name = string_value(value["name"], "builtins.fetchurl name") if "name" in value else None
        path = _fetch_url_download(url, name)
        if "sha256" in value:
            expected = string_value(value["sha256"], "builtins.fetchurl sha256")
            actual_hex = hashlib.sha256(Path(str(path)).read_bytes()).hexdigest()
            # Accept hex or any non-empty token (Nix base32); only enforce exact match for hex.
            if len(expected) == 64 and all(c in "0123456789abcdef" for c in expected.lower()):
                if actual_hex.lower() != expected.lower():
                    pnix_error(
                        f"builtins.fetchurl: hash mismatch for `{url}`: expected {expected}, got {actual_hex}"
                    )
        return path
    pnix_error(f"builtins.fetchurl: expected string or attrset, got {_type_of(value)}")
    raise AssertionError("unreachable")


def fetch_tarball_value(arg: Any) -> PnixPath:
    value = force_value(arg)
    if is_string_value(value) or isinstance(value, PnixPath):
        url = str(value)
        name = None
        sha256 = None
    elif isinstance(value, dict):
        if "url" not in value:
            pnix_error("builtins.fetchTarball: attrset must have `url`")
        url = string_value(value["url"], "builtins.fetchTarball url")
        name = string_value(value["name"], "builtins.fetchTarball name") if "name" in value else None
        sha256 = string_value(value["sha256"], "builtins.fetchTarball sha256") if "sha256" in value else None
    else:
        pnix_error(f"builtins.fetchTarball: expected string or attrset, got {_type_of(value)}")
        raise AssertionError("unreachable")

    archive_path = _fetch_url_download(url, name or "tarball")
    if sha256 is not None:
        actual_hex = hashlib.sha256(Path(str(archive_path)).read_bytes()).hexdigest()
        if len(sha256) == 64 and all(c in "0123456789abcdef" for c in sha256.lower()):
            if actual_hex.lower() != sha256.lower():
                pnix_error(
                    f"builtins.fetchTarball: hash mismatch for `{url}`: expected {sha256}, got {actual_hex}"
                )

    import tarfile
    import zipfile

    dest = _store_cache_dir() / (Path(str(archive_path)).name + "-unpacked")
    if dest.exists():
        return PnixPath(str(dest.resolve()))
    dest.mkdir(parents=True, exist_ok=True)
    archive_file = Path(str(archive_path))
    try:
        if tarfile.is_tarfile(archive_file):
            with tarfile.open(archive_file) as tf:
                tf.extractall(dest)
        elif zipfile.is_zipfile(archive_file):
            with zipfile.ZipFile(archive_file) as zf:
                zf.extractall(dest)
        else:
            # Non-archive payload: leave as single-file tree for stubs / dmg / etc.
            target = dest / archive_file.name
            if not target.exists():
                target.write_bytes(archive_file.read_bytes())
    except (tarfile.TarError, zipfile.BadZipFile, OSError) as exc:
        pnix_error(f"builtins.fetchTarball: failed to unpack `{url}`: {exc}")
        raise AssertionError("unreachable")
    return PnixPath(str(dest.resolve()))


def fetch_git_value(arg: Any) -> dict[str, Any]:
    """Stub/real: clone shallow when git is available; otherwise return metadata path."""
    import subprocess

    value = force_value(arg)
    if is_string_value(value) or isinstance(value, PnixPath):
        url = str(value)
        rev = None
        ref = None
        name = None
    elif isinstance(value, dict):
        if "url" not in value:
            pnix_error("builtins.fetchGit: attrset must have `url`")
        url = string_value(value["url"], "builtins.fetchGit url")
        rev = string_value(value["rev"], "builtins.fetchGit rev") if "rev" in value else None
        ref = string_value(value["ref"], "builtins.fetchGit ref") if "ref" in value else None
        name = string_value(value["name"], "builtins.fetchGit name") if "name" in value else None
    else:
        pnix_error(f"builtins.fetchGit: expected string or attrset, got {_type_of(value)}")
        raise AssertionError("unreachable")

    label = name or url.rstrip("/").rsplit("/", 1)[-1].removesuffix(".git") or "source"
    key = f"{url}|{rev or ''}|{ref or ''}"
    digest = hashlib.sha256(key.encode("utf-8")).hexdigest()[:32]
    dest = _store_cache_dir() / f"{digest}-{safe_store_name(label)}"
    out_rev = rev or "0000000000000000000000000000000000000000"
    if not dest.exists():
        try:
            cmd = ["git", "clone", "--depth", "1"]
            if ref:
                cmd.extend(["--branch", ref])
            cmd.extend([url, str(dest)])
            subprocess.run(cmd, check=True, capture_output=True, timeout=120)
            if rev:
                subprocess.run(
                    ["git", "-C", str(dest), "checkout", rev],
                    check=False,
                    capture_output=True,
                    timeout=60,
                )
            got = subprocess.run(
                ["git", "-C", str(dest), "rev-parse", "HEAD"],
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if got.returncode == 0 and got.stdout.strip():
                out_rev = got.stdout.strip()
        except (FileNotFoundError, subprocess.SubprocessError, OSError):
            dest.mkdir(parents=True, exist_ok=True)
            (dest / ".pnix-fetchGit-stub").write_text(
                f"url={url}\nrev={rev or ''}\nref={ref or ''}\n",
                encoding="utf-8",
            )
    return {
        "outPath": PnixPath(str(dest.resolve())),
        "rev": out_rev,
        "shortRev": out_rev[:7],
        "revCount": 0,
        "lastModified": 0,
        "submodules": False,
    }


def current_system_value() -> str:
    machine = platform.machine() or "unknown"
    if sys.platform == "darwin":
        return f"{machine}-darwin"
    if sys.platform.startswith("linux"):
        return f"{machine}-linux"
    return f"{machine}-{sys.platform}"


def resolve_fs_path(value: Any, ctx: dict[str, Any], label: str) -> Path:
    value = force_value(value)
    if isinstance(value, PnixPath):
        text = str(value)
    elif isinstance(value, str):
        text = value
    else:
        pnix_error(f"{label}: expected string or path (expected path or string)")
    if text == "":
        pnix_error(f"{label}: empty string is not a valid path")
    path = Path(normalize_pnix_path_text(text)).expanduser()
    if not path.is_absolute():
        path = Path(str(ctx.get("base_dir", Path.cwd()))) / path
    return path.resolve()


def file_type_value(path: Path) -> str:
    if path.is_symlink():
        return "symlink"
    if path.is_dir():
        return "directory"
    if path.is_file():
        return "regular"
    return "unknown"


def read_file_type_value(path: Path) -> str:
    if not path.exists() and not path.is_symlink():
        pnix_error(f"builtins.readFileType: failed to get metadata for `{path}`: No such file or directory")
    return file_type_value(path)


def read_dir_value(value: Any, ctx: dict[str, Any]) -> dict[str, Any]:
    path = resolve_fs_path(value, ctx, "builtins.readDir")
    try:
        return {entry.name: file_type_value(entry) for entry in sorted(path.iterdir(), key=lambda p: p.name)}
    except OSError as exc:
        pnix_error(f"builtins.readDir: failed to read `{path}`: {exc}")
    raise AssertionError("unreachable")


def read_file_value(value: Any, ctx: dict[str, Any]) -> str:
    path = resolve_fs_path(value, ctx, "builtins.readFile")
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        pnix_error(f"builtins.readFile: failed to read `{path}`: {exc}")
    except UnicodeDecodeError:
        pnix_error(f"builtins.readFile: `{path}` is not a valid UTF-8 text file")
    raise AssertionError("unreachable")


def plain_string_value(value: Any, label: str) -> str:
    value = force_value(value)
    if is_string_value(value):
        return str(value)
    pnix_error(f"{label} ({_type_of(value)}) must be string")
    raise AssertionError("unreachable")


def expected_string_value(value: Any, label: str) -> str:
    value = force_value(value)
    if is_string_value(value):
        return str(value)
    pnix_error(f"{label}: expected string, got {_type_of(value)}")
    raise AssertionError("unreachable")


def plain_string_text_context(value: Any, label: str) -> tuple[str, set[str]]:
    value = force_value(value)
    if is_string_value(value):
        return str(value), context_of_string(value)
    pnix_error(f"{label} ({_type_of(value)}) must be string")
    raise AssertionError("unreachable")


def safe_store_name(name: str) -> str:
    safe = "".join(ch if ch.isalnum() or ch in ".-_+" else "_" for ch in name)
    return safe or "unnamed"


def to_file_value(name: Any, contents: Any) -> PnixPath:
    name_value = force_value(name)
    if not is_string_value(name_value):
        pnix_error(f"builtins.toFile: first argument must be string, got {_type_of(name_value)}")
    file_name = str(name_value)
    content_value = force_value(contents)
    if not is_string_value(content_value):
        pnix_error(f"builtins.toFile: second argument must be string, got {_type_of(content_value)}")
    text, context = string_text_context(content_value, "builtins.toFile contents")
    if context:
        pnix_error("builtins.toFile: contents must not have string context; use builtins.unsafeDiscardStringContext to discard it")
    digest = hashlib.sha256(text.encode("utf-8")).hexdigest()[:32]
    store_dir = Path(tempfile.gettempdir()) / "pnix-nix-store"
    store_dir.mkdir(parents=True, exist_ok=True)
    out = store_dir / f"{digest}-{safe_store_name(file_name)}"
    out.write_text(text, encoding="utf-8")
    return PnixPath(str(out.resolve()))


def hash_bytes_value(algo: str, data: bytes, label: str, allow_legacy: bool) -> str:
    if allow_legacy and algo == "md5":
        return hashlib.md5(data, usedforsecurity=False).hexdigest()
    if allow_legacy and algo == "sha1":
        return hashlib.sha1(data, usedforsecurity=False).hexdigest()
    if algo == "sha256":
        return hashlib.sha256(data).hexdigest()
    if algo == "sha512":
        return hashlib.sha512(data).hexdigest()
    if algo in {"md5", "sha1"}:
        pnix_error(
            f"{label}: algorithm '{algo}' is not supported (`{algo}`); cryptographically broken; "
            "use 'sha256' or 'sha512'"
        )
    supported = "'md5', 'sha1', 'sha256', 'sha512'" if allow_legacy else "'sha256', 'sha512'"
    pnix_error(f"{label}: unsupported algorithm '{algo}' (`{algo}`); supported: {supported}")
    raise AssertionError("unreachable")


def hash_string_algorithm_value(algo: Any) -> str:
    algorithm, context = plain_string_text_context(algo, "builtins.hashString algo")
    if context:
        pnix_error(
            f"builtins.hashString algo: the string '{algorithm}' is not allowed to refer to a store path"
        )
    if algorithm not in {"md5", "sha1", "sha256", "sha512"}:
        hash_bytes_value(algorithm, b"", "builtins.hashString", True)
    return algorithm


def hash_string_value(algorithm: str, data: Any) -> str:
    text, _context = plain_string_text_context(data, "builtins.hashString data")
    return hash_bytes_value(algorithm, px_str_bytes(text), "builtins.hashString", True)


def hash_string_function(algo: Any) -> NativeFunc:
    algorithm = hash_string_algorithm_value(algo)
    return NativeFunc(lambda data: hash_string_value(algorithm, data), force_arg=False)


def hash_file_value(algo: Any, value: Any, ctx: dict[str, Any]) -> str:
    algorithm = plain_string_value(algo, "builtins.hashFile algo")
    path_arg = force_value(value)
    path_context = {str(path_arg)} if isinstance(path_arg, PnixPath) else context_of_string(path_arg)
    path = resolve_fs_path(path_arg, ctx, "builtins.hashFile")
    try:
        data = path.read_bytes()
    except OSError as exc:
        pnix_error(f"builtins.hashFile: failed to read `{path}`: {exc}")
    return make_context_string(hash_bytes_value(algorithm, data, "builtins.hashFile", False), path_context)


def path_text_value(value: Any, ctx: dict[str, Any], label: str) -> str:
    return str(resolve_fs_path(value, ctx, label))


def to_path_string_value(value: Any) -> str:
    """Nix toPath: lexical absolute-path normalization, returning a string."""
    text, context = string_text_context(value, "builtins.toPath string")
    if not text.startswith("/"):
        pnix_error(f"string '{text}' doesn't represent an absolute path")
    return make_context_string(normalize_pnix_path_text(text), context)


def base_name_value(value: Any) -> str:
    value = force_value(value)
    if isinstance(value, PnixPath):
        text = str(value)
        context: set[str] = set()
    else:
        text, context = string_text_context(value, "builtins.baseNameOf path")
    if text == "" or text == "/":
        return make_context_string("", context)
    if text.endswith("/"):
        text = text[:-1]
    return make_context_string(text.rsplit("/", 1)[-1], context)


def dir_of_value(value: Any) -> str:
    value = force_value(value)
    if isinstance(value, PnixPath):
        text = str(value)
        if text == "/":
            return PnixPath("/")
        if text.endswith("/") and text != "":
            text = text[:-1]
        if "/" not in text:
            return PnixPath(".")
        head = text.rsplit("/", 1)[0]
        return PnixPath("/" if head == "" else head)
    else:
        text, context = string_text_context(value, "builtins.dirOf path")
    if text == "/":
        return make_context_string("/", context)
    if text.endswith("/") and text != "":
        text = text[:-1]
    if "/" not in text:
        return make_context_string(".", context)
    head = text.rsplit("/", 1)[0]
    return make_context_string("/" if head == "" else head, context)


def get_env_value(name: Any) -> str:
    key = string_arg_value(name, "getEnv")
    return os.environ.get(key, "") if key.startswith("PNIX_") else ""


VALUES_EQUAL_MAX_DEPTH = 64


def split_version_components(text: str) -> list[str]:
    result: list[str] = []
    component_start: int | None = None
    last_was_digit: bool | None = None
    for idx, ch in enumerate(text):
        is_digit = ch.isascii() and ch.isdigit()
        is_sep = ch in ".-"
        if is_sep:
            if component_start is not None:
                result.append(text[component_start:idx])
                component_start = None
            last_was_digit = None
        elif last_was_digit is not None and last_was_digit != is_digit:
            if component_start is not None:
                result.append(text[component_start:idx])
            component_start = idx
            last_was_digit = is_digit
        else:
            if component_start is None:
                component_start = idx
            last_was_digit = is_digit
    if component_start is not None:
        result.append(text[component_start:])
    return result


def split_version_value(value: Any) -> list[Any]:
    text, context = string_text_context(value, "builtins.splitVersion string")
    return [make_context_string(part, context) for part in split_version_components(text)]


def parse_drv_name_value(value: Any) -> dict[str, Any]:
    text, context = string_text_context(value, "builtins.parseDrvName string")
    split_idx: int | None = None
    for idx in range(0, max(0, len(text) - 1)):
        if text[idx] == "-" and text[idx + 1].isascii() and text[idx + 1].isdigit():
            split_idx = idx
            break
    if split_idx is None:
        return {"name": make_context_string(text, context), "version": make_context_string("", context)}
    return {
        "name": make_context_string(text[:split_idx], context),
        "version": make_context_string(text[split_idx + 1 :], context),
    }


def compare_versions_string(value: Any) -> str:
    value = force_value(value)
    if is_string_value(value):
        return str(value)
    pnix_error("builtins.compareVersions: expected two strings")
    raise AssertionError("unreachable")


def compare_version_component(lhs: str, rhs: str) -> int:
    left_num = int(lhs) if lhs.isdigit() else None
    right_num = int(rhs) if rhs.isdigit() else None
    if left_num is not None and right_num is not None:
        return -1 if left_num < right_num else (1 if left_num > right_num else 0)
    if left_num is not None:
        return 1
    if right_num is not None:
        return -1
    if lhs == rhs:
        return 0
    if lhs == "":
        return 1 if rhs == "pre" else -1
    if rhs == "":
        return -1 if lhs == "pre" else 1
    if lhs == "pre":
        return -1
    if rhs == "pre":
        return 1
    return -1 if lhs < rhs else 1


def compare_versions_value(lhs: Any, rhs: Any) -> int:
    left = split_version_components(compare_versions_string(lhs))
    right = split_version_components(compare_versions_string(rhs))
    for idx in range(max(len(left), len(right))):
        cmp = compare_version_component(
            left[idx] if idx < len(left) else "",
            right[idx] if idx < len(right) else "",
        )
        if cmp != 0:
            return cmp
    return 0


def source_file_label(ctx: dict[str, Any]) -> str:
    source = str(ctx.get("source_path", "<pnix-px>"))
    marker = "/fixtures/"
    if marker in source:
        return "fixtures/" + source.split(marker, 1)[1]
    return source


def source_position_value(ctx: dict[str, Any], pos: int) -> dict[str, Any]:
    source = str(ctx.get("source_text", ""))
    clamped = max(0, min(int(pos), len(source)))
    line = source.count("\n", 0, clamped) + 1
    line_start = source.rfind("\n", 0, clamped)
    column = clamped + 1 if line_start < 0 else clamped - line_start
    return {"file": source_file_label(ctx), "line": line, "column": column}


POSIX_REGEX_CLASS_REPLACEMENTS = {
    "[:alnum:]": "A-Za-z0-9",
    "[:alpha:]": "A-Za-z",
    "[:blank:]": r"\x09\x20",
    "[:cntrl:]": r"\x00-\x1f\x7f",
    "[:digit:]": "0-9",
    "[:graph:]": r"\x21-\x7e",
    "[:lower:]": "a-z",
    "[:print:]": r"\x20-\x7e",
    "[:punct:]": r"\x21-\x2f\x3a-\x40\x5b-\x60\x7b-\x7e",
    "[:space:]": r"\x09-\x0d\x20",
    "[:upper:]": "A-Z",
    "[:xdigit:]": "A-Fa-f0-9",
}


def translate_regex_pattern(pattern: str) -> str:
    source = str(pattern)
    out: list[str] = []
    i = 0
    in_bracket = False
    bracket_has_member = False
    while i < len(source):
        c = source[i]
        if c == "\\" and i + 1 < len(source):
            out.append(source[i : i + 2])
            if in_bracket:
                bracket_has_member = True
            i += 2
            continue
        if not in_bracket:
            out.append(c)
            if c == "[":
                in_bracket = True
                bracket_has_member = False
            i += 1
            continue
        if c == "[" and source.startswith("[:", i):
            end = source.find(":]", i + 2)
            if end < 0:
                raise re.error("unterminated POSIX character class")
            marker = source[i : end + 2]
            replacement = POSIX_REGEX_CLASS_REPLACEMENTS.get(marker)
            if replacement is None:
                raise re.error(f"unknown POSIX character class '{source[i + 2 : end]}'")
            out.append(replacement)
            bracket_has_member = True
            i = end + 2
            continue
        out.append(c)
        if c == "]" and bracket_has_member:
            in_bracket = False
        elif not (c == "^" and not bracket_has_member):
            bracket_has_member = True
        i += 1
    return "".join(out)


def invalid_regex_message(exc: Exception) -> str:
    detail = str(exc)
    if "unterminated" in detail and "unclosed" not in detail:
        detail = f"{detail} (unclosed)"
    return f"invalid regex: {detail}"


def compile_regex_pattern(pattern: str, label: str) -> re.Pattern[str]:
    try:
        return re.compile(translate_regex_pattern(pattern))
    except re.error as exc:
        pnix_error(f"{label}: {invalid_regex_message(exc)}")
    raise AssertionError("unreachable")


def regex_match_value(pattern: Any, value: Any) -> list[Any] | None:
    compiled = compile_regex_pattern(string_value(pattern, "builtins.match regex"), "builtins.match")
    text, context = string_text_context(value, "builtins.match string")
    match = compiled.fullmatch(text)
    if match is None:
        return None
    return [make_context_string(group, context) if group is not None else None for group in match.groups()]


def regex_split_value(pattern: Any, value: Any) -> list[Any]:
    text_pattern = string_value(pattern, "builtins.split regex")
    if text_pattern == "":
        pnix_error("builtins.split: regex pattern cannot be empty")
    compiled = compile_regex_pattern(text_pattern, "builtins.split")
    text, context = string_text_context(value, "builtins.split string")
    out: list[Any] = []
    last_end = 0
    for match in compiled.finditer(text):
        out.append(make_context_string(text[last_end : match.start()], context))
        out.append([make_context_string(group, context) if group is not None else None for group in match.groups()])
        last_end = match.end()
    out.append(make_context_string(text[last_end:], context))
    return out


def from_json_value(value: Any) -> Any:
    def reject_constant(token: str) -> None:
        pnix_error(f"builtins.fromJSON: invalid JSON numeric constant {token}")

    def parse_int_token(token: str) -> int | float:
        if token == "-0":
            return -0.0
        parsed = int(token)
        if not (I64_MIN <= parsed <= I64_MAX):
            pnix_error(f"builtins.fromJSON: integer literal too large for i64: {token}")
        return parsed

    text = string_value(value, "builtins.fromJSON string")
    try:
        return json.loads(text, parse_int=parse_int_token, parse_constant=reject_constant)
    except json.JSONDecodeError as exc:
        pnix_error(f"builtins.fromJSON: parse error: {exc}")
    raise AssertionError("unreachable")


def toml_to_pnix_value(value: Any) -> Any:
    if isinstance(value, dict):
        return {str(key): toml_to_pnix_value(item) for key, item in value.items()}
    if isinstance(value, list):
        return [toml_to_pnix_value(item) for item in value]
    if type(value) in (str, int, float, bool):
        return value
    return str(value)


def from_toml_value(value: Any) -> Any:
    text = expected_string_value(value, "builtins.fromTOML")
    try:
        return toml_to_pnix_value(tomllib.loads(text))
    except tomllib.TOMLDecodeError as exc:
        pnix_error(f"builtins.fromTOML: parse error: {exc}")
    raise AssertionError("unreachable")


def markup_escape(value: str, *, attr: bool = False) -> str:
    out = value.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    return out.replace('"', "&quot;") if attr else out


def markup_scalar_string(value: Any, label: str) -> str:
    value = force_value(value)
    if value is None:
        return ""
    if type(value) is bool:
        return "true" if value else "false"
    if type(value) is int or type(value) is float:
        return str(value)
    if isinstance(value, PnixPath):
        return str(value)
    if isinstance(value, str):
        return value
    pnix_error(f"{label} must be string-compatible")
    raise AssertionError("unreachable")


def element_to_markup_node(element: ET.Element) -> dict[str, Any]:
    children: list[Any] = []
    if element.text:
        children.append({"kind": "text", "value": element.text})
    for child in list(element):
        children.append(element_to_markup_node(child))
        if child.tail:
            children.append({"kind": "text", "value": child.tail})
    return {
        "kind": "element",
        "name": str(element.tag),
        "attrs": {str(key): str(value) for key, value in sorted(element.attrib.items())},
        "children": children,
    }


def xml_parse_value(value: Any) -> dict[str, Any]:
    text = string_arg_value(value, "xmlParse")
    try:
        return element_to_markup_node(ET.fromstring(text))
    except ET.ParseError as exc:
        pnix_error(f"builtins.xmlParse: parse error: {exc}")
    raise AssertionError("unreachable")


def html_parse_value(value: Any) -> dict[str, Any]:
    text = string_arg_value(value, "htmlParse")
    try:
        wrapper = ET.fromstring("<pnix-hy-document>" + text + "</pnix-hy-document>")
    except ET.ParseError as exc:
        pnix_error(f"builtins.htmlParse: parse error: {exc}")
    children: list[Any] = []
    if wrapper.text:
        children.append({"kind": "text", "value": wrapper.text})
    for child in list(wrapper):
        children.append(element_to_markup_node(child))
        if child.tail:
            children.append({"kind": "text", "value": child.tail})
    return {"kind": "document", "children": children}


def markup_children(map_value: dict[str, Any], label: str) -> list[Any]:
    if "children" not in map_value:
        return []
    return list_value(map_value["children"], label)


def markup_attrs(value: Any, label: str, *, html: bool = False) -> list[tuple[str, str]]:
    value = force_value(value)
    if value is None:
        return []
    out: list[tuple[str, str]] = []
    if isinstance(value, dict):
        for key, item in value.items():
            name = str(key).lower() if html else str(key)
            out.append((name, markup_scalar_string(item, label)))
    elif isinstance(value, list):
        for item in value:
            attr = attrset_value(item, label)
            if "name" not in attr:
                pnix_error(f"{label}: attr missing name")
            if "value" not in attr:
                pnix_error(f"{label}: attr missing value")
            name = string_value(attr["name"], f"{label} name")
            out.append((name.lower() if html else name, markup_scalar_string(attr["value"], label)))
    else:
        pnix_error(f"{label}: attrs must be attrset or list")
    return sorted(out)


HTML_VOID_ELEMENTS = {
    "area",
    "base",
    "br",
    "col",
    "embed",
    "hr",
    "img",
    "input",
    "link",
    "meta",
    "param",
    "source",
    "track",
    "wbr",
}


def markup_emit_node(value: Any, *, html: bool) -> str:
    node = attrset_value(value, "builtins.htmlEmit node" if html else "builtins.xmlEmit node")
    kind = string_value(node.get("kind"), "markup kind")
    if kind == "document":
        return "".join(markup_emit_node(child, html=html) for child in markup_children(node, "markup document children"))
    if kind == "text":
        return markup_escape(markup_scalar_string(node.get("value", node.get("text", "")), "markup text"))
    if kind == "comment":
        return "<!--" + markup_scalar_string(node.get("value", ""), "markup comment") + "-->"
    if kind == "cdata" and not html:
        return "<![CDATA[" + markup_scalar_string(node.get("value", node.get("text", "")), "markup cdata") + "]]>"
    if kind != "element" and "name" not in node:
        pnix_error(f"markup.emit: unknown kind `{kind}`")
    name = string_value(node.get("name"), "markup element name")
    if html:
        name = name.lower()
    attrs = markup_attrs(node.get("attrs", {}), "markup attrs", html=html)
    attr_text = "".join(" " + key + '="' + markup_escape(value, attr=True) + '"' for key, value in attrs)
    children = markup_children(node, "markup element children")
    if html:
        if name in HTML_VOID_ELEMENTS:
            return "<" + name + attr_text + ">"
        return "<" + name + attr_text + ">" + "".join(markup_emit_node(child, html=True) for child in children) + "</" + name + ">"
    if not children:
        return "<" + name + attr_text + "/>"
    return "<" + name + attr_text + ">" + "".join(markup_emit_node(child, html=False) for child in children) + "</" + name + ">"


def markup_emit_value(value: Any, *, html: bool) -> str:
    forced = force_value(value)
    if isinstance(forced, list):
        text = "".join(markup_emit_node(item, html=html) for item in forced)
    else:
        text = markup_emit_node(forced, html=html)
    return make_context_string(text, collect_string_context(forced))


def schema_root_value(schema: Any) -> dict[str, Any]:
    source = attrset_value(schema, "schema")
    if "kind" in source:
        return source
    if "root" in source:
        return attrset_value(source["root"], "schema.root")
    pnix_error("schema.validate: schema must define kind or root")
    raise AssertionError("unreachable")


def schema_kind(schema: dict[str, Any]) -> str:
    return string_value(schema.get("kind"), "schema kind")


def schema_optional_names(schema: dict[str, Any]) -> set[str]:
    if "optional" not in schema:
        return set()
    return {string_value(item, "schema optional item") for item in list_value(schema["optional"], "schema optional")}


def schema_type_name(value: Any) -> str:
    value = force_value(value)
    if value is None:
        return "null"
    if type(value) is bool:
        return "bool"
    if type(value) is int:
        return "int"
    if type(value) is float:
        return "float"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "list"
    if isinstance(value, dict):
        return "set"
    return type(value).__name__


def predicate_bool_value(value: Any, label: str, index: int | None = None) -> bool:
    forced = force_value(value)
    if type(forced) is bool:
        return forced
    suffix = "" if index is None else f" at index {index}"
    pnix_error(f"{label} must return bool, got {schema_type_name(forced)}{suffix}")
    raise AssertionError("unreachable")


def schema_error(path: list[str], code: str, message: str) -> dict[str, Any]:
    return {"path": path, "code": code, "message": message}


def schema_validate_errors(schema: Any, value: Any, path: list[str] | None = None) -> list[dict[str, Any]]:
    path = path or ["root"]
    schema_map = schema_root_value(schema)
    kind = schema_kind(schema_map)
    value = force_value(value)
    if kind == "any":
        return []
    if kind == "string":
        if not isinstance(value, str):
            return [schema_error(path, "type", f"expected string, got {schema_type_name(value)}")]
        if "minLength" in schema_map and len(value) < number_value(schema_map["minLength"], "schema minLength"):
            return [schema_error(path, "constraint", f"expected min length {schema_map['minLength']}")]
        return []
    if kind == "bool":
        return [] if type(value) is bool else [schema_error(path, "type", f"expected bool, got {schema_type_name(value)}")]
    if kind == "int":
        return [] if type(value) is int else [schema_error(path, "type", f"expected int, got {schema_type_name(value)}")]
    if kind in {"float", "number"}:
        return [] if type(value) in (int, float) else [schema_error(path, "type", f"expected number, got {schema_type_name(value)}")]
    if kind == "list":
        if not isinstance(value, list):
            return [schema_error(path, "type", f"expected list, got {schema_type_name(value)}")]
        if "elem" not in schema_map:
            return []
        errors: list[dict[str, Any]] = []
        for index, item in enumerate(value):
            errors.extend(schema_validate_errors(schema_map["elem"], item, path + [str(index)]))
        return errors
    if kind in {"attrs", "map"}:
        if not isinstance(value, dict):
            return [schema_error(path, "type", f"expected set, got {schema_type_name(value)}")]
        return []
    if kind == "record":
        if not isinstance(value, dict):
            return [schema_error(path, "type", f"expected record, got {schema_type_name(value)}")]
        fields = attrset_value(schema_map.get("fields"), "schema record fields")
        optional = schema_optional_names(schema_map)
        errors = []
        for field, field_schema in fields.items():
            field_map = attrset_value(field_schema, "schema field")
            if field in value:
                errors.extend(schema_validate_errors(field_schema, value[field], path + [field]))
            elif field not in optional and "default" not in field_map:
                errors.append(schema_error(path + [field], "missing", f"missing required field {field}"))
        return errors
    return [schema_error(path, "schema", f"unsupported schema kind {kind}")]


def schema_normalize_value(schema: Any, value: Any) -> Any:
    schema_map = schema_root_value(schema)
    value = force_value(value)
    if schema_kind(schema_map) != "record":
        return value
    source = attrset_value(value, "schemaNormalize value")
    fields = attrset_value(schema_map.get("fields"), "schema record fields")
    out = dict(source)
    for field, field_schema in fields.items():
        field_map = attrset_value(field_schema, "schema field")
        if field in out:
            out[field] = schema_normalize_value(field_schema, out[field])
        elif "default" in field_map:
            out[field] = schema_normalize_value(field_schema, field_map["default"])
    return out


def schema_validate_value(schema: Any, value: Any) -> dict[str, Any]:
    errors = schema_validate_errors(schema, value)
    ok = len(errors) == 0
    return {"success": ok, "ok": ok, "errors": errors}


def schema_explain_value(schema: Any, value: Any) -> str:
    errors = schema_validate_errors(schema, value)
    return "\n".join(".".join(error["path"]) + ": " + error["code"] + ": " + error["message"] for error in errors)


def function_args_value(value: Any) -> dict[str, Any]:
    value = force_value(value)
    if not isinstance(value, Closure):
        if callable(value):
            return {}
        pnix_error(f"builtins.functionArgs: expected function, got {_type_of(value)}")
    pattern = value.pattern
    while isinstance(pattern, dict) and pattern.get("tag") == "as":
        pattern = pattern.get("pattern")
    if not isinstance(pattern, dict) or pattern.get("tag") != "attrset":
        return {}
    return {field["name"]: "default" in field for field in pattern.get("fields", [])}


def zip_attrs_with_value(func: Any, xs: Any, ctx: dict[str, Any]) -> dict[str, Any]:
    maps = [attrset_arg_value(item, "zipAttrsWith", "list element must be attrset") for item in list_arg_value(xs, "zipAttrsWith", "second argument")]
    keys = sorted({key for m in maps for key in m})
    out: dict[str, Any] = {}
    for key in keys:
        values = [m[key] for m in maps if key in m]
        out[key] = Thunk(
            lambda key=key, values=values: apply_pnix(
                apply_pnix(func, Thunk(lambda key=key: key), ctx),
                Thunk(lambda values=values: values),
                ctx,
            )
        )
    return out


def list_take_value(count: Any, xs: Any) -> list[Any]:
    n = nonnegative_count_value(count, "builtins.take count")
    return list_value(xs, "builtins.take list")[:n]


def list_drop_value(count: Any, xs: Any) -> list[Any]:
    n = nonnegative_count_value(count, "builtins.drop count")
    return list_value(xs, "builtins.drop list")[n:]


def list_elem_at_value(xs: Any, index: Any) -> Any:
    items = list_value(xs, "builtins.elemAt list")
    i = integer_value(index, "builtins.elemAt index")
    if i < 0:
        pnix_error("builtins.elemAt: negative index")
    if i >= len(items):
        pnix_error("builtins.elemAt: index out of bounds")
    return force_value(items[i])


def list_head_value(xs: Any) -> Any:
    items = list_value(xs, "builtins.head list")
    if not items:
        pnix_error("builtins.head: list is empty")
    return force_value(items[0])


def list_tail_value(xs: Any) -> list[Any]:
    items = list_value(xs, "builtins.tail list")
    if not items:
        pnix_error("builtins.tail: list is empty")
    return items[1:]


GENLIST_MAX_COUNT = 16 * 1024 * 1024


def gen_list_value(func: Any, count: Any, ctx: dict[str, Any]) -> list[Any]:
    n = nonnegative_count_value(count, "builtins.genList length")
    if n > GENLIST_MAX_COUNT:
        pnix_error(f"builtins.genList count {n} exceeds maximum {GENLIST_MAX_COUNT}")
    return [Thunk(lambda i=i: apply_pnix(func, Thunk(lambda i=i: i), ctx)) for i in range(n)]


def map_list_value(func: Any, xs: Any, ctx: dict[str, Any]) -> list[Any]:
    return [
        Thunk(lambda item=item: apply_pnix(func, Thunk(lambda item=item: force_value(item)), ctx))
        for item in list_value(xs, "builtins.map list")
    ]


def list_arg_value(value: Any, builtin: str, position: str) -> list[Any]:
    value = force_value(value)
    if not isinstance(value, list):
        pnix_error(f"builtins.{builtin}: {position} must be list, got {_type_of(value)}")
    return value


def attrset_arg_value(value: Any, builtin: str, phrase: str = "expected attrset") -> dict[str, Any]:
    value = force_value(value)
    if not isinstance(value, dict):
        pnix_error(f"builtins.{builtin}: {phrase}, got {_type_of(value)}")
    return value


def string_arg_value(value: Any, builtin: str) -> str:
    value = force_value(value)
    if not is_string_value(value):
        pnix_error(f"builtins.{builtin}: expected string, got {_type_of(value)}")
    return str(value)


def concat_lists_value(xss: Any) -> list[Any]:
    xss = force_value(xss)
    if not isinstance(xss, list):
        pnix_error(f"builtins.concatLists: argument must be list, got {_type_of(xss)}")
    out: list[Any] = []
    for index, xs in enumerate(xss):
        xs = force_value(xs)
        if not isinstance(xs, list):
            pnix_error(f"builtins.concatLists: element at index {index} is not a list, got {_type_of(xs)}")
        out.extend(xs)
    return out


def cat_attrs_value(name: Any, xs: Any) -> list[Any]:
    key = string_value(name, "builtins.catAttrs name")
    out: list[Any] = []
    for item in list_value(xs, "builtins.catAttrs list"):
        attrs = attrset_value(item, "builtins.catAttrs element")
        if key in attrs:
            out.append(attrs[key])
    return out


def attr_values_value(attrs: Any, label: str) -> list[Any]:
    attrs = force_value(attrs)
    if not isinstance(attrs, dict):
        pnix_error(f"{label}: expected attrset, got {_type_of(attrs)}")
    return [attrs[key] for key in sorted(attrs)]


def map_attrs_value(func: Any, attrs: Any, ctx: dict[str, Any]) -> dict[str, Any]:
    source = attrset_value(attrs, "builtins.mapAttrs set")
    return {
        key: Thunk(
            lambda key=key, value=value: apply_pnix(
                apply_pnix(func, Thunk(lambda key=key: key), ctx),
                Thunk(lambda value=value: force_value(value)),
                ctx,
            )
        )
        for key, value in source.items()
    }


def filter_list_value(func: Any, xs: Any, ctx: dict[str, Any]) -> list[Any]:
    out: list[Any] = []
    for index, item in enumerate(list_arg_value(xs, "filter", "second argument")):
        keep = predicate_bool_value(
            apply_pnix(func, Thunk(lambda item=item: force_value(item)), ctx),
            "builtins.filter predicate",
            index,
        )
        if keep:
            out.append(force_value(item))
    return out


def any_list_value(func: Any, xs: Any, ctx: dict[str, Any]) -> bool:
    for index, item in enumerate(list_arg_value(xs, "any", "second argument")):
        if predicate_bool_value(
            apply_pnix(func, Thunk(lambda item=item: force_value(item)), ctx),
            "builtins.any predicate",
            index,
        ):
            return True
    return False


def all_list_value(func: Any, xs: Any, ctx: dict[str, Any]) -> bool:
    for index, item in enumerate(list_arg_value(xs, "all", "second argument")):
        if not predicate_bool_value(
            apply_pnix(func, Thunk(lambda item=item: force_value(item)), ctx),
            "builtins.all predicate",
            index,
        ):
            return False
    return True


def zip_lists_value(lhs: Any, rhs: Any) -> list[Any]:
    left = list_value(lhs, "builtins.zip lhs")
    right = list_value(rhs, "builtins.zip rhs")
    return [[force_value(a), force_value(b)] for a, b in zip(left, right)]


def flatten_value(value: Any) -> list[Any]:
    out: list[Any] = []
    for item in list_value(value, "builtins.flatten list"):
        forced = force_value(item)
        if isinstance(forced, list):
            out.extend(flatten_value(forced))
        else:
            out.append(forced)
    return out


def find_value(needle: Any, xs: Any) -> Any:
    for item in list_value(xs, "builtins.find list"):
        forced = force_value(item)
        if eq_value(needle, forced, 1):
            return forced
    return None


def get_value(attrs: Any, name: Any) -> Any:
    source = attrset_value(attrs, "builtins.get attrs")
    key = string_value(name, "builtins.get name")
    if key not in source:
        return None
    return force_value(source[key])


def get_attrs_value(names: Any, attrs: Any) -> dict[str, Any]:
    source = attrset_value(attrs, "builtins.getAttrs attrs")
    out: dict[str, Any] = {}
    for name in list_value(names, "builtins.getAttrs names"):
        key = string_value(name, "builtins.getAttrs name")
        if key not in source:
            pnix_error(f"builtins.getAttrs: attribute '{key}' missing in set")
        out[key] = source[key]
    return out


def generic_closure_value(arg: Any, ctx: dict[str, Any]) -> list[Any]:
    attrs = attrset_value(arg, "builtins.genericClosure argument")
    if "startSet" not in attrs:
        pnix_error("builtins.genericClosure: argument missing required attribute 'startSet'")
    if "operator" not in attrs:
        pnix_error("builtins.genericClosure: argument missing required attribute 'operator'")
    work = list(list_value(attrs["startSet"], "builtins.genericClosure startSet"))
    operator = attrs["operator"]
    seen: set[str] = set()
    out: list[Any] = []
    steps = 0
    while work:
        steps += 1
        if steps > 10_000:
            pnix_error("builtins.genericClosure: maximum depth 10000 exceeded")
        if len(work) > 100_000:
            pnix_error("builtins.genericClosure: work list size 100000 exceeded")
        item = force_value(work.pop())
        item_attrs = attrset_value(item, "builtins.genericClosure item")
        if "key" not in item_attrs:
            pnix_error("builtins.genericClosure: item missing 'key' attribute")
        key_sig = term_hash(force_value(item_attrs["key"]))
        if key_sig in seen:
            continue
        seen.add(key_sig)
        out.append(item)
        next_items = list_value(
            apply_pnix(operator, Thunk(lambda item=item: item), ctx),
            "builtins.genericClosure operator result",
        )
        work.extend(reversed(next_items))
    return out


def set_value(attrs: Any, name: Any, value: Any) -> dict[str, Any]:
    out = dict(attrset_value(attrs, "builtins.set attrs"))
    out[string_value(name, "builtins.set name")] = value
    return out


def concat_strings_value(xs: Any) -> str:
    out: list[str] = []
    context: set[str] = set()
    for idx, item in enumerate(list_arg_value(xs, "concatStrings", "argument")):
        text, ctx = string_text_context(item, f"builtins.concatStrings element at index {idx} is not a string")
        out.append(text)
        context.update(ctx)
    return make_context_string(px_revalidate("".join(out)), context)


def concat_strings_sep_value(sep: Any, xs: Any) -> str:
    sep_forced = force_value(sep)
    if not is_string_value(sep_forced):
        pnix_error(f"builtins.concatStringsSep: separator must be string, got {_type_of(sep_forced)}")
    sep_text, sep_context = string_text_context(sep_forced, "builtins.concatStringsSep separator")
    out: list[str] = []
    context: set[str] = set()
    items = list_value(xs, "builtins.concatStringsSep list")
    for idx, item in enumerate(items):
        if idx > 0:
            out.append(sep_text)
            context.update(sep_context)
        text, ctx = string_text_context(item, f"builtins.concatStringsSep element at index {idx} is not a string")
        out.append(text)
        context.update(ctx)
    # RAW-BYTE aware: per-byte fragments (substring cuts) reassemble; revalidate
    # smuggled surrogate bytes back to a clean str when the concat is valid UTF-8.
    return make_context_string(px_revalidate("".join(out)), context)


def validate_context_attrset(value: Any, label: str) -> dict[str, Any]:
    source = attrset_value(value, label)
    for key, spec_value in source.items():
        spec = force_value(spec_value)
        if not isinstance(spec, dict):
            pnix_error(f"builtins.appendContext: context value for '{key}' must be an attrset, got {_type_of(spec)}")
        if "path" in spec:
            path_flag = force_value(spec["path"])
            if type(path_flag) is not bool:
                pnix_error(f"builtins.appendContext: '{key}'.path must be bool, got {_type_of(path_flag)}")
        if "allOutputs" in spec:
            all_flag = force_value(spec["allOutputs"])
            if type(all_flag) is not bool:
                pnix_error(f"builtins.appendContext: '{key}'.allOutputs must be bool, got {_type_of(all_flag)}")
        if "outputs" in spec:
            outs = force_value(spec["outputs"])
            if not isinstance(outs, list):
                pnix_error(f"builtins.appendContext: '{key}'.outputs must be list of strings, got {_type_of(outs)}")
            for index, item in enumerate(outs):
                item_f = force_value(item)
                if not is_string_value(item_f):
                    pnix_error(f"builtins.appendContext: '{key}'.outputs element at index {index} is not a string, got {_type_of(item_f)}")
    return source


def append_context_value(value: Any, extra: Any) -> str:
    text, context = string_text_context(value, "builtins.appendContext string")
    context.update(str(key) for key in validate_context_attrset(extra, "builtins.appendContext context"))
    return make_context_string(text, context)


def context_string_value(value: Any, label: str) -> str:
    forced = force_value(value)
    if not is_string_value(forced):
        pnix_error(f"builtins.{label}: expected string, got {_type_of(forced)}")
    text, context = string_text_context(forced, f"builtins.{label} string")
    if label == "addDrvOutputDependencies":
        context.add("!out!" + text)
    elif label == "unsafeDiscardOutputDependency":
        context = {item for item in context if not (item.startswith("!out!") or item.startswith("!") or item.startswith("="))}
    elif label == "unsafeAddOutputDependency":
        context.update("!out!" + item for item in list(context) if not item.startswith("!") and not item.startswith("="))
    return make_context_string(text, context)


def unsafe_add_output_name_value(name: Any, value: Any) -> str:
    name_forced = force_value(name)
    if not is_string_value(name_forced):
        pnix_error(f"builtins.unsafeAddOutputName: first arg must be string, got {_type_of(name_forced)}")
    output_name = str(name_forced)
    value_forced = force_value(value)
    if not is_string_value(value_forced):
        pnix_error(f"builtins.unsafeAddOutputName: second arg must be string, got {_type_of(value_forced)}")
    text, context = string_text_context(value_forced, "builtins.unsafeAddOutputName string")
    context.update(f"!{output_name}!{item}" for item in list(context) if not item.startswith("!") and not item.startswith("="))
    return make_context_string(text, context)


def get_context_value(value: Any) -> dict[str, Any]:
    _, context = string_text_context(value, "builtins.getContext string")
    return {item: {"path": True} for item in sorted(context)}


def has_context_value(value: Any) -> bool:
    _, context = string_text_context(value, "builtins.hasContext string")
    return bool(context)


def unsafe_discard_string_context_value(value: Any) -> str:
    return string_value(value, "builtins.unsafeDiscardStringContext string")


def derivation_value(attrs: Any, label: str) -> dict[str, Any]:
    source = attrset_value(attrs, f"builtins.{label} attrs")
    out = dict(source)
    name = string_value(source["name"], f"builtins.{label} name") if "name" in source else "unnamed"
    placeholder = f"/pnix-placeholder/derivation/{name}"
    path_value = make_context_string(placeholder, {"!out!" + name})
    out.setdefault("outPath", path_value)
    out.setdefault("drvPath", path_value)
    out.setdefault("type", "derivation")
    return out


def try_eval_value(thunk: Any) -> dict[str, Any]:
    try:
        return {"success": True, "value": force_value(thunk)}
    except PnixCatchableError:
        return {"success": False, "value": False}


def builtins_mod_value(lhs: Any, rhs: Any) -> Any:
    left = number_value(lhs, "builtins.mod")
    right = number_value(rhs, "builtins.mod")
    if right == 0:
        pnix_error("builtins.mod: division by zero", error_class="division-by-zero")
    if type(left) is float or type(right) is float:
        return math.fmod(left, right)
    if left == I64_MIN and right == -1:
        pnix_error("integer overflow in `%`", error_class="integer-overflow")
    q = abs(left) // abs(right)
    if (left < 0) != (right < 0):
        q = -q
    return check_i64(left - q * right, "%")


def bit_op_value(name: str, lhs: Any, rhs: Any, op: Callable[[int, int], int]) -> int:
    lhs = force_value(lhs)
    if type(lhs) is not int:
        pnix_error(f"builtins.{name}: first arg must be int, got {_type_of(lhs)}")
    rhs = force_value(rhs)
    if type(rhs) is not int:
        pnix_error(f"builtins.{name}: second arg must be int, got {_type_of(rhs)}")
    return op(lhs, rhs)


def add_error_context_value(message: Any, value: Any) -> Any:
    m = force_value(message)
    if not is_string_value(m):
        pnix_error(f"builtins.addErrorContext: context must be string, got {_type_of(m)}")
    return value


def unsafe_get_attr_pos_value(name: Any, attrs: Any, ctx: dict[str, Any]) -> Any:
    attr_name = string_value(name, "builtins.unsafeGetAttrPos name")
    source = attrset_value(attrs, "builtins.unsafeGetAttrPos attrs")
    if attr_name not in source:
        return None
    pos = getattr(source, "attr_positions", {}).get(attr_name)
    if pos is None:
        return None
    return source_position_value(ctx, pos)


def pow_value(lhs: Any, rhs: Any) -> int | float:
    left = force_value(lhs)
    right = force_value(rhs)
    if type(left) is int and type(right) is int and right >= 0:
        exact = left ** right
        if I64_MIN <= exact <= I64_MAX:
            return exact
    return math.pow(
        number_value(left, "builtins.pow base"),
        number_value(right, "builtins.pow exponent"),
    )


def numeric_math1(value: Any, name: str, func: Callable[[float], float]) -> float:
    return func(float(number_value(value, f"builtins.{name} argument")))


def checked_float_to_i64(value: Any, label: str, func: Callable[[float], float]) -> int:
    value = number_value(value, f"{label} argument")
    if type(value) is int:
        as_float = float(value)
        if as_float >= float(2**63) or as_float < float(-(2**63)):
            pnix_error(f"{label}: integer outside i64 range after f64 conversion")
        if int(as_float) != value:
            pnix_error(f"{label}: integer loses precision when converted to f64")
        value = as_float
    if math.isnan(value):
        pnix_error(f"{label}: NaN outside i64 range")
    if math.isinf(value):
        pnix_error(f"{label}: {'+inf' if value > 0 else '-inf'} outside i64 range")
    rounded = func(value)
    if rounded >= 2**63 or rounded < -(2**63):
        pnix_error(f"{label}: value outside i64 range")
    return check_i64(int(rounded), label)


def unary_neg_value(value: Any) -> int | float:
    value = number_value(value, "argument of unary -")
    if type(value) is float and value == 0.0:
        return 0.0
    return check_i64(-value, "-")


def realize_value(value: Any) -> Any:
    value = force_value(value)
    if is_closure(value):
        return "#<pnix-hy-closure>"
    if is_native(value):
        return "#<pnix-hy-native>"
    if isinstance(value, PnixPath):
        return str(value)
    if isinstance(value, PnixString):
        return str(value)
    if isinstance(value, ConstructValue):
        return {
            "variant": value.variant,
            "args": [realize_value(arg) for arg in value.args],
        }
    if isinstance(value, dict):
        return {key: realize_value(value[key]) for key in sorted(value)}
    if isinstance(value, list):
        return [realize_value(item) for item in value]
    return value


def json_ready_value(value: Any, seen: set[int] | None = None) -> Any:
    if seen is None:
        seen = set()
    value = force_value(value)
    if is_closure(value) or is_native(value):
        pnix_error("cannot serialize function as JSON")
    if type(value) is float and not math.isfinite(value):
        kind = "NaN" if math.isnan(value) else ("+inf" if value > 0 else "-inf")
        pnix_error(f"cannot serialize float {kind} as JSON")
    if isinstance(value, PnixPath):
        return str(value)
    if isinstance(value, PnixString):
        return str(value)
    if isinstance(value, ConstructValue):
        value_id = id(value)
        if value_id in seen:
            pnix_error("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        try:
            return {
                "variant": value.variant,
                "args": [json_ready_value(arg, seen) for arg in value.args],
            }
        finally:
            seen.discard(value_id)
    if isinstance(value, dict):
        value_id = id(value)
        if value_id in seen:
            pnix_error("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        try:
            return {key: json_ready_value(value[key], seen) for key in sorted(value)}
        finally:
            seen.discard(value_id)
    if isinstance(value, list):
        value_id = id(value)
        if value_id in seen:
            pnix_error("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        try:
            return [json_ready_value(item, seen) for item in value]
        finally:
            seen.discard(value_id)
    return value


def collect_string_context(value: Any, seen: set[int] | None = None) -> set[str]:
    if seen is None:
        seen = set()
    value = force_value(value)
    if isinstance(value, PnixString):
        return set(value.context)
    if isinstance(value, PnixPath):
        return {str(value)}
    if isinstance(value, ConstructValue):
        value_id = id(value)
        if value_id in seen:
            pnix_error("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        out: set[str] = set()
        try:
            for arg in value.args:
                out.update(collect_string_context(arg, seen))
            return out
        finally:
            seen.discard(value_id)
    if isinstance(value, dict):
        value_id = id(value)
        if value_id in seen:
            pnix_error("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        out: set[str] = set()
        try:
            for item in value.values():
                out.update(collect_string_context(item, seen))
            return out
        finally:
            seen.discard(value_id)
    if isinstance(value, list):
        value_id = id(value)
        if value_id in seen:
            pnix_error("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        out: set[str] = set()
        try:
            for item in value:
                out.update(collect_string_context(item, seen))
            return out
        finally:
            seen.discard(value_id)
    return set()


def to_json_string_value(value: Any) -> str:
    ready = json_ready_value(value)
    return make_context_string(
        json.dumps(ready, ensure_ascii=False, sort_keys=True, separators=(",", ":")),
        collect_string_context(value),
    )


def deep_force_value(value: Any, seen: set[int] | None = None, label: str = "builtins.deepSeq") -> Any:
    if seen is None:
        seen = set()
    value = force_value(value)
    if isinstance(value, ConstructValue):
        value_id = id(value)
        if value_id in seen:
            pnix_error(f"{label}: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        try:
            for arg in value.args:
                deep_force_value(arg, seen, label)
        finally:
            seen.discard(value_id)
    elif isinstance(value, dict):
        value_id = id(value)
        if value_id in seen:
            pnix_error(f"{label}: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        try:
            for item in value.values():
                deep_force_value(item, seen, label)
        finally:
            seen.discard(value_id)
    elif isinstance(value, list):
        value_id = id(value)
        if value_id in seen:
            pnix_error(f"{label}: infinite recursion encountered (cyclic value)")
        seen.add(value_id)
        try:
            for item in value:
                deep_force_value(item, seen, label)
        finally:
            seen.discard(value_id)
    return value


def set_path(
    target: dict[str, Any],
    path: list[str],
    value: Any,
    context: str = "attr",
    positions: list[int | None] | None = None,
) -> dict[str, Any]:
    if len(path) == 1:
        if path[0] in target:
            if context == "let":
                pnix_error(f"let: '{path[0]}' bound more than once")
            existing = force_value(target[path[0]])
            new_value = force_value(value)
            if isinstance(existing, dict) and isinstance(new_value, dict):
                target[path[0]] = merge_defined_attrsets(existing, new_value)
                return target
            pnix_error(f"attribute '{path[0]}' already defined at this level")
        if isinstance(target, AttrSet) and positions and positions[0] is not None:
            target.attr_positions[path[0]] = positions[0]
        target[path[0]] = value
        return target
    head = path[0]
    child = target.get(head)
    if child is None:
        child = AttrSet()
        if isinstance(target, AttrSet) and positions and positions[0] is not None:
            target.attr_positions[head] = positions[0]
        target[head] = child
    if not isinstance(child, dict):
        forced_child = force_value(child)
        if not isinstance(forced_child, dict):
            pnix_error(f"attribute path conflict: '{head}' is already a non-attrset value")
        child = forced_child
        target[head] = child
    set_path(child, path[1:], value, context, positions[1:] if positions else None)
    return target


def merge_defined_attrsets(left: dict[str, Any], right: dict[str, Any]) -> AttrSet:
    left_positions = getattr(left, "attr_positions", {})
    right_positions = getattr(right, "attr_positions", {})
    out = AttrSet(left, attr_positions=left_positions)
    for key, value in right.items():
        if key not in out:
            out[key] = value
            if key in right_positions:
                out.attr_positions[key] = right_positions[key]
            continue
        left_value = force_value(out[key])
        right_value = force_value(value)
        if isinstance(left_value, dict) and isinstance(right_value, dict):
            out[key] = merge_defined_attrsets(left_value, right_value)
            continue
        pnix_error(f"attribute '{key}' already defined at this level")
    return out


def binding_path_is_static(path: list[Any]) -> bool:
    return all(isinstance(part, str) for part in path)


def eval_binding_path(path: list[Any], env: dict[str, Any], ctx: dict[str, Any]) -> list[str]:
    out: list[str] = []
    for part in path:
        if isinstance(part, str):
            out.append(part)
        else:
            out.append(attr_key_value(eval_ast(part["expr"], env, ctx)))
    return out


def binding_positions_for(path: list[Any], positions: list[int | None] | None) -> list[int | None] | None:
    if not binding_path_is_static(path):
        return None
    return positions


def build_attrset(
    bindings: list[dict[str, Any]], env: dict[str, Any], ctx: dict[str, Any], recursive: bool
) -> dict[str, Any]:
    values: dict[str, Any] = AttrSet()
    if recursive:
        rec_env = dict(env)
        static_bindings = [binding for binding in bindings if binding_path_is_static(binding["path"])]
        dynamic_bindings = [binding for binding in bindings if not binding_path_is_static(binding["path"])]
        for binding in static_bindings:
            node = binding["value"]
            use_env = env if binding.get("inherit_plain") else rec_env
            set_path(
                values,
                eval_binding_path(binding["path"], rec_env, ctx),
                Thunk(lambda node=node, use_env=use_env: eval_ast(node, use_env, ctx)),
                positions=binding_positions_for(binding["path"], binding.get("path_positions")),
            )
        rec_env.update(values)
        for binding in dynamic_bindings:
            node = binding["value"]
            use_env = env if binding.get("inherit_plain") else rec_env
            set_path(
                values,
                eval_binding_path(binding["path"], rec_env, ctx),
                Thunk(lambda node=node, use_env=use_env: eval_ast(node, use_env, ctx)),
                positions=None,
            )
            rec_env.update(values)
        return values
    for binding in bindings:
        node = binding["value"]
        set_path(
            values,
            eval_binding_path(binding["path"], env, ctx),
            Thunk(lambda node=node: eval_ast(node, env, ctx)),
            positions=binding_positions_for(binding["path"], binding.get("path_positions")),
        )
    return values


def build_let_env(bindings: list[dict[str, Any]], env: dict[str, Any], ctx: dict[str, Any]) -> dict[str, Any]:
    rec_env = dict(env)
    values: dict[str, Any] = {}
    for binding in bindings:
        node = binding["value"]
        use_env = env if binding.get("inherit_plain") else rec_env
        set_path(
            values,
            eval_binding_path(binding["path"], rec_env, ctx),
            Thunk(lambda node=node, use_env=use_env: eval_ast(node, use_env, ctx)),
            "let",
            binding_positions_for(binding["path"], binding.get("path_positions")),
        )
    rec_env.update(values)
    return rec_env


def merge_attrsets(lhs: Any, rhs: Any) -> Any:
    left = force_value(lhs)
    right = force_value(rhs)
    if left is None:
        return right
    if right is None:
        return left
    merged = dict(attrset_value(left, "left side of //"))
    merged.update(attrset_value(right, "right side of //"))
    return merged


def eq_value(lhs: Any, rhs: Any, depth: int = 0) -> bool:
    if depth > VALUES_EQUAL_MAX_DEPTH:
        pnix_error("infinite recursion encountered during ==")
    left = force_value(lhs)
    right = force_value(rhs)
    if depth > 0 and left is right:
        return True
    if is_closure(left) or is_native(left) or is_closure(right) or is_native(right):
        return False
    if isinstance(left, PnixPath) or isinstance(right, PnixPath):
        return (
            isinstance(left, PnixPath)
            and isinstance(right, PnixPath)
            and normalize_pnix_path_text(left) == normalize_pnix_path_text(right)
        )
    if isinstance(left, PnixString):
        left = str(left)
    if isinstance(right, PnixString):
        right = str(right)
    if isinstance(left, ConstructValue) or isinstance(right, ConstructValue):
        if not isinstance(left, ConstructValue) or not isinstance(right, ConstructValue):
            return False
        return left.variant == right.variant and len(left.args) == len(right.args) and all(
            eq_value(l_arg, r_arg, depth + 1) for l_arg, r_arg in zip(left.args, right.args)
        )
    if isinstance(left, dict) or isinstance(right, dict):
        if not isinstance(left, dict) or not isinstance(right, dict):
            return False
        if set(left.keys()) != set(right.keys()):
            return False
        return all(eq_value(left[key], right[key], depth + 1) for key in left)
    if isinstance(left, list) or isinstance(right, list):
        if not isinstance(left, list) or not isinstance(right, list):
            return False
        return len(left) == len(right) and all(
            eq_value(l_item, r_item, depth + 1) for l_item, r_item in zip(left, right)
        )
    if type(left) is bool or type(right) is bool:
        return type(left) is bool and type(right) is bool and left == right
    if left is None or right is None:
        return left is None and right is None
    if (type(left) is int or type(left) is float) and (type(right) is int or type(right) is float):
        if type(left) is float or type(right) is float:
            return float(left) == float(right)
        return left == right
    if isinstance(left, str) or isinstance(right, str):
        return isinstance(left, str) and isinstance(right, str) and str(left) == str(right)
    return type(left) is type(right) and left == right


def attr_key_value(value: Any) -> str:
    value = force_value(value)
    if isinstance(value, PnixPath):
        return str(value)
    if is_string_value(value):
        return str(value)
    return value_to_string(value)


def eval_attr_segments(
    segments: list[dict[str, Any]], env: dict[str, Any], ctx: dict[str, Any]
) -> str:
    out: list[str] = []
    for segment in segments:
        if "lit" in segment:
            out.append(str(segment["lit"]))
        else:
            out.append(attr_key_value(eval_ast(segment["expr"], env, ctx)))
    return ".".join(out)


def eval_attr_path_segments(
    segments: list[dict[str, Any]], env: dict[str, Any], ctx: dict[str, Any]
) -> list[str]:
    out: list[str] = []
    for segment in segments:
        if "lit" in segment:
            out.append(str(segment["lit"]))
        else:
            out.append(attr_key_value(eval_ast(segment["expr"], env, ctx)))
    return out


def value_cell(value: Any) -> Any:
    if isinstance(value, Thunk):
        return value
    return Thunk(lambda value=value: value)


def merge_match_bindings(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any] | None:
    out = dict(left)
    for name, value in right.items():
        if name in out and not eq_value(out[name], value):
            return None
        out[name] = value
    return out


def match_pattern(
    pattern: dict[str, Any],
    value: Any,
    ctx: dict[str, Any],
    default_env: dict[str, Any] | None = None,
) -> dict[str, Any] | None:
    tag = pattern["tag"]
    if tag == "wildcard":
        return {}
    if tag == "as":
        matched = match_pattern(pattern["pattern"], value, ctx, default_env)
        if matched is None:
            return None
        return merge_match_bindings(matched, {pattern["name"]: value_cell(value)})
    if tag == "var":
        return {pattern["name"]: value_cell(value)}
    if tag == "literal":
        return {} if eq_value(value, pattern["value"]) else None
    if tag == "list":
        value = force_value(value)
        if not isinstance(value, list):
            return None
        rest_name = pattern.get("rest")
        if rest_name is None and len(value) != len(pattern["items"]):
            return None
        if rest_name is not None and len(value) < len(pattern["items"]):
            return None
        bindings: dict[str, Any] = {}
        for sub_pattern, item in zip(pattern["items"], value[: len(pattern["items"])]):
            matched = match_pattern(sub_pattern, item, ctx, default_env)
            if matched is None:
                return None
            bindings = merge_match_bindings(bindings, matched)
            if bindings is None:
                return None
        if rest_name is not None:
            bindings = merge_match_bindings(
                bindings,
                {rest_name: value_cell(value[len(pattern["items"]) :])},
            )
            if bindings is None:
                return None
        return bindings
    if tag == "attrset":
        value = force_value(value)
        if not isinstance(value, dict):
            return None
        bindings: dict[str, Any] = {}
        for field in pattern["fields"]:
            name = field["name"]
            if name in value:
                matched_value = value[name]
            elif "default" in field and default_env is not None:
                captured_env = dict(default_env)
                captured_env.update(bindings)
                matched_value = Thunk(
                    lambda node=field["default"], env=captured_env: eval_ast(node, env, ctx)
                )
            else:
                return None
            matched = match_pattern(field["pattern"], matched_value, ctx, default_env)
            if matched is None:
                return None
            bindings = merge_match_bindings(bindings, matched)
            if bindings is None:
                return None
        return bindings
    if tag == "constructor":
        value = force_value(value)
        if not isinstance(value, ConstructValue):
            return None
        if value.variant != pattern["variant"] or len(value.args) != len(pattern["args"]):
            return None
        bindings: dict[str, Any] = {}
        for sub_pattern, item in zip(pattern["args"], value.args):
            matched = match_pattern(sub_pattern, item, ctx, default_env)
            if matched is None:
                return None
            bindings = merge_match_bindings(bindings, matched)
            if bindings is None:
                return None
        return bindings
    pnix_error(f"unsupported match pattern tag {tag!r}")
    raise AssertionError("unreachable")


def marker_string(value: Any) -> bool:
    return isinstance(value, str) and value.startswith("#<pnix-hy-")


def nix_less_than(a: Any, b: Any, depth: int = 0) -> bool:
    """Nix `lessThan` over realized values: number/string/list (lexicographic).
    Booleans, null and mismatched types are incomparable (error), as in Nix."""
    if depth > VALUES_EQUAL_MAX_DEPTH:
        pnix_error("infinite recursion encountered during comparison")
    a = force_value(a)
    b = force_value(b)
    if depth > 0 and a is b:
        return False
    if isinstance(a, PnixPath) or isinstance(b, PnixPath):
        if isinstance(a, PnixPath) and isinstance(b, PnixPath):
            return normalize_pnix_path_text(a) < normalize_pnix_path_text(b)
        pnix_error(f"cannot compare {_type_of(a)} with {_type_of(b)}")
    if isinstance(a, PnixString):
        a = str(a)
    if isinstance(b, PnixString):
        b = str(b)
    if isinstance(a, bool) or isinstance(b, bool):
        pnix_error("cannot compare booleans with `<`")
    if (type(a) is int or type(a) is float) and (type(b) is int or type(b) is float):
        if type(a) is float or type(b) is float:
            return float(a) < float(b)
        return a < b
    if isinstance(a, str) and isinstance(b, str) and not marker_string(a) and not marker_string(b):
        # RAW-BYTE track: byte-lexicographic — surrogate-smuggled bytes
        # (U+DC80..) would misorder against clean multi-byte chars under
        # code-point comparison (oracle: substring 0 1 "가" < "한" is true).
        return px_str_bytes(a) < px_str_bytes(b)
    if isinstance(a, list) and isinstance(b, list):
        for x, y in zip(a, b):
            if eq_value(x, y, depth + 1):
                continue
            return nix_less_than(x, y, depth + 1)
        return len(a) < len(b)
    pnix_error(f"cannot compare {_type_of(a)} with {_type_of(b)}")
    raise AssertionError("unreachable")


def nix_compare(op: str, lhs: Any, rhs: Any) -> bool:
    if op == "<":
        return nix_less_than(lhs, rhs)
    if op == ">":
        return nix_less_than(rhs, lhs)
    if op == "<=":
        return not nix_less_than(rhs, lhs)
    return not nix_less_than(lhs, rhs)  # >=


# pnix integers are i64 with CHECKED overflow (~/pnix interpret.rs), unlike
# Python's arbitrary-precision int. Int arithmetic that escapes [i64::MIN, MAX]
# is a typed error, not a silent bignum. Float arithmetic is unchecked (overflow
# -> +inf, matching real Nix).
I64_MIN = -(2 ** 63)
I64_MAX = 2 ** 63 - 1


def check_i64(value: Any, op: str) -> Any:
    if type(value) is int and not (I64_MIN <= value <= I64_MAX):
        pnix_error(f"integer overflow in `{op}`", error_class="integer-overflow")
    return value


def _arith_pair(op: str, lhs: Any, rhs: Any) -> tuple[Any, Any]:
    left = force_value(lhs)
    right = force_value(rhs)
    if (type(left) not in (int, float)) or (type(right) not in (int, float)):
        pnix_error(
            f"operator {op}: unsupported operand types {_type_of(left)} and {_type_of(right)}",
            error_class="type-error",
        )
    return left, right


def _production_checked_i64(op: str, left: int, right: int) -> int:
    from . import primitive_kernel

    outcome = primitive_kernel.invoke_shadow(op, left, right)
    if outcome["kind"] == "ok":
        return outcome["value"]
    error_class = outcome["class"]
    if error_class == "division-by-zero":
        pnix_error("division by zero", error_class="division-by-zero")
    if error_class == "integer-overflow":
        pnix_error(f"integer overflow in `{op}`", error_class="integer-overflow")
    pnix_error("primitive contract violation")


def apply_binary(op: str, lhs: Any, rhs: Any) -> Any:
    if op == "+":
        left = force_value(lhs)
        right = force_value(rhs)
        if isinstance(left, PnixPath) and isinstance(right, PnixPath):
            return PnixPath(str(left) + str(right))
        if isinstance(left, PnixPath) and is_string_value(right):
            if context_of_string(right):
                pnix_error(
                    "operator +: path + context-bearing string would drop string context; "
                    "use builtins.unsafeDiscardStringContext to discard it explicitly"
                )
            return PnixPath(str(left) + right)
        if is_string_value(left) and isinstance(right, PnixPath):
            context = context_of_string(left)
            context.add(str(right))
            return make_context_string(str(left) + str(right), context)
        if is_string_value(left) and is_string_value(right):
            context = context_of_string(left)
            context.update(context_of_string(right))
            # RAW-BYTE track: a concat may recombine smuggled bytes into
            # valid UTF-8 — revalidate (oracle: two mid-byte halves of "가"
            # rejoin to "가").
            return make_context_string(px_revalidate(str(left) + str(right)), context)
        if isinstance(left, list) and isinstance(right, list):
            return left + right  # list concat: pnix `+` is polymorphic (~/pnix interpret.rs)
        if isinstance(left, dict) and isinstance(right, dict):
            return merge_attrsets(lhs, rhs)  # attrset merge
        if (type(left) not in (int, float)) or (type(right) not in (int, float)):
            pnix_error(
                f"operator +: unsupported operand types {_type_of(left)} and {_type_of(right)}",
                error_class="type-error",
            )
        if type(left) is int and type(right) is int:
            return _production_checked_i64("+", left, right)
        return check_i64(left + right, "+")
    if op == "-":
        left, right = _arith_pair("-", lhs, rhs)
        if type(left) is int and type(right) is int:
            return _production_checked_i64("-", left, right)
        return check_i64(left - right, "-")
    if op == "*":
        left, right = _arith_pair("*", lhs, rhs)
        if type(left) is int and type(right) is int:
            return _production_checked_i64("*", left, right)
        return check_i64(left * right, "*")
    if op == "/":
        left, right = _arith_pair("/", lhs, rhs)
        if type(left) is int and type(right) is int:
            return _production_checked_i64("/", left, right)
        if right == 0:
            pnix_error("division by zero", error_class="division-by-zero")
        if type(left) is float or type(right) is float:
            return left / right
        quotient = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            quotient = -quotient
        return check_i64(quotient, "/")
    if op == "%":
        left, right = _arith_pair("%", lhs, rhs)
        if right == 0:
            pnix_error("modulo by zero")
        if type(left) is float or type(right) is float:
            return math.fmod(left, right)
        if left == I64_MIN and right == -1:
            pnix_error("integer overflow in `%`", error_class="integer-overflow")
        q = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            q = -q
        return check_i64(left - q * right, "%")
    if op == "==":
        return eq_value(lhs, rhs)
    if op == "!=":
        return not eq_value(lhs, rhs)
    if op in ("<", "<=", ">", ">="):
        return nix_compare(op, lhs, rhs)
    if op == "//":
        return merge_attrsets(lhs, rhs)
    if op == "++":
        lhs_value = force_value(lhs)
        rhs_value = force_value(rhs)
        if isinstance(lhs_value, list) and isinstance(rhs_value, list):
            return lhs_value + rhs_value
        pnix_error("both sides of ++ must be lists")
    pnix_error(f"unsupported binary op `{op}`")
    raise AssertionError("unreachable")


def coerce_interp(value: Any, ctx: dict[str, Any] | None = None) -> str:
    if ctx is None:
        ctx = {}
    value = force_value(value)
    if isinstance(value, PnixPath):
        return make_context_string(str(value), {str(value)})
    if is_string_value(value):
        return make_context_string(str(value), context_of_string(value))
    if type(value) is int:
        pnix_error("cannot coerce a number to a string in interpolation: use builtins.toString")
    if type(value) is bool:
        pnix_error("cannot coerce a boolean to a string in interpolation: use builtins.toString")
    if value is None:
        pnix_error("cannot coerce null to a string in interpolation")
    if isinstance(value, list):
        pnix_error("cannot coerce a list to a string in interpolation")
    if isinstance(value, dict):
        # ~/pnix + Nix: a set coerces via __toString (called with the set as
        # `self`) then outPath; __toString takes priority. Recurse so nested
        # __toString/outPath chains resolve (e.g. outPath = { __toString = ...; }).
        value_id = id(value)
        stack = ctx.setdefault("__pnix_interp_coerce_stack__", set())
        if value_id in stack:
            pnix_error("interpolation coercion cycle involving __toString")
        stack.add(value_id)
        try:
            if "__toString" in value:
                fn = force_value(value["__toString"])
                return coerce_interp(apply_pnix(fn, Thunk(lambda v=value: v), ctx), ctx)
            if "outPath" in value:
                return coerce_interp(value["outPath"], ctx)
            pnix_error("cannot coerce a set to a string in interpolation: no __toString or outPath")
        finally:
            stack.discard(value_id)
    if isinstance(value, (Closure, NativeFunc)):
        pnix_error("cannot coerce a function to a string in interpolation")
    pnix_error("cannot coerce value to a string in interpolation")
    raise AssertionError("unreachable")


def lookup_env(env: dict[str, Any], name: str) -> Any:
    if name in env:
        return force_value(env[name])
    for frame in env.get(WITH_CHAIN_KEY, []):
        attrs = force_with_frame(frame)
        if name in attrs:
            return force_value(attrs[name])
    pnix_error(f"unknown variable `{name}`", error_class="unknown-variable")
    raise AssertionError("unreachable")


def _abort_show(value: Any) -> str:
    value = force_value(value)
    if type(value) is bool:
        return "true" if value else "false"
    if type(value) is int or type(value) is float:
        return str(value)
    if value is None:
        return "null"
    if is_string_value(value):
        return str(value)
    return _type_of(value)


def abort_value(msg: Any) -> Any:
    forced = force_value(msg)
    if not is_string_value(forced):
        pnix_error(f"builtins.abort: argument must be string, got {_abort_show(forced)}")
    pnix_error("evaluation aborted: " + str(forced))


def throw_value(msg: Any) -> Any:
    text = expected_string_value(msg, "builtins.throw")
    pnix_catchable_error(text)


def force_with_frame(frame: WithFrame) -> dict[str, Any]:
    if frame.cached is None:
        source = force_value(eval_ast(frame.source, frame.env, frame.ctx))
        if not isinstance(source, dict):
            pnix_error(f"with: argument must be attrset, got {_type_of(source)}")
        frame.cached = source
    return frame.cached


def with_env(env: dict[str, Any], source: dict[str, Any], ctx: dict[str, Any]) -> dict[str, Any]:
    new_env = dict(env)
    chain = list(env.get(WITH_CHAIN_KEY, []))
    new_env[WITH_CHAIN_KEY] = [WithFrame(source, dict(env), ctx)] + chain
    return new_env


def resolve_path_literal(value: str, ctx: dict[str, Any]) -> PnixPath:
    text = normalize_pnix_path_text(value)
    if (
        ctx.get("path_literals_absolute")
        and not (text.startswith("<") and text.endswith(">"))
        and not text.startswith("~")
        and not Path(text).is_absolute()
    ):
        return PnixPath(str((Path(str(ctx.get("base_dir", Path.cwd()))) / text).resolve()))
    return PnixPath(text)


def eval_path_interp(parts: list[dict[str, Any]], env: dict[str, Any], ctx: dict[str, Any]) -> PnixPath:
    out: list[str] = []
    for part in parts:
        if "lit" in part:
            out.append(str(part["lit"]))
        else:
            out.append(str(coerce_interp(eval_ast(part["expr"], env, ctx), ctx)))
    return resolve_path_literal("".join(out), ctx)


def mirror_event(ctx: dict[str, Any], event: dict[str, Any]) -> None:
    events = ctx.get("events")
    if events is not None:
        event = dict(event)
        event["idx"] = len(events)
        events.append(event)


def eval_ast(ast: dict[str, Any], env: dict[str, Any], ctx: dict[str, Any]) -> Any:
    tag = ast["tag"]
    mirror_event(ctx, {"event": "eval", "tag": tag})
    if tag == "int":
        return ast["value"]
    if tag == "float":
        return ast["value"]
    if tag == "path":
        return resolve_path_literal(ast["value"], ctx)
    if tag == "path_interp":
        return eval_path_interp(ast["parts"], env, ctx)
    if tag == "string":
        return ast["value"]
    if tag == "str_interp":
        out: list[str] = []
        context: set[str] = set()
        for part in ast["parts"]:
            if "lit" in part:
                out.append(part["lit"])
            else:
                expr = part["expr"]
                try:
                    coerced = coerce_interp(eval_ast(expr, env, ctx), ctx)
                    out.append(str(coerced))
                    context.update(context_of_string(coerced))
                except PnixError as exc:
                    if expr.get("tag") == "var" and str(exc) == f"unknown variable `{expr['name']}`":
                        out.append("${" + expr["name"] + "}")
                    else:
                        raise
        return make_context_string("".join(out), context)
    if tag == "bool":
        return ast["value"]
    if tag == "null":
        return None
    if tag == "var":
        if ast["name"] == "__curPos":
            return source_position_value(ctx, int(ast.get("pos", 0)))
        return lookup_env(env, ast["name"])
    if tag == "import":
        return import_value(ast["path"], ctx, "eval")
    if tag == "construct":
        return ConstructValue(
            ast["variant"],
            [Thunk(lambda arg=arg: eval_ast(arg, env, ctx)) for arg in ast["args"]],
        )
    if tag == "list":
        return [Thunk(lambda item=item: eval_ast(item, env, ctx)) for item in ast["items"]]
    if tag == "attrset":
        return build_attrset(ast["bindings"], env, ctx, bool(ast["recursive"]))
    if tag == "let":
        return eval_ast(ast["body"], build_let_env(ast["bindings"], env, ctx), ctx)
    if tag == "lambda":
        return Closure(
            ast.get("param"), ast["body"], dict(env), ast.get("pattern"), dict(ctx)
        )
    if tag == "apply":
        func = eval_ast(ast["func"], env, ctx)
        return apply_pnix(func, Thunk(lambda: eval_ast(ast["arg"], env, ctx)), ctx)
    if tag == "if":
        if bool_value(
            eval_ast(ast["cond"], env, ctx),
            "if condition",
            error_class="non-boolean-condition",
        ):
            return eval_ast(ast["then"], env, ctx)
        return eval_ast(ast["else"], env, ctx)
    if tag == "with":
        return eval_ast(ast["body"], with_env(env, ast["env"], ctx), ctx)
    if tag == "assert":
        if not bool_value(eval_ast(ast["cond"], env, ctx), "assert condition"):
            pnix_catchable_error("assertion failed")
        return eval_ast(ast["body"], env, ctx)
    if tag == "select":
        base = attrset_value(eval_ast(ast["base"], env, ctx), "select base")
        attr = ast["attr"]
        if attr not in base:
            pnix_error(f"missing attr `{attr}`", error_class="attribute-missing")
        return force_value(base[attr])
    if tag == "select_default":
        base = force_value(eval_ast(ast["base"], env, ctx))
        attr = ast["attr"]
        if isinstance(base, dict) and attr in base:
            return force_value(base[attr])
        return eval_ast(ast["default"], env, ctx)
    if tag == "dynamic_select":
        # Each segment is a SEPARATE select step (Nix: s.${k}.canonical walks
        # ${k} THEN canonical — joining them into one "k.canonical" key was a
        # real bug the korean-nl-mirror harvest exposed, 2026-07-11).
        val = eval_ast(ast["base"], env, ctx)
        for segment in ast["segments"]:
            base = attrset_value(val, "select base")
            attr = eval_attr_segments([segment], env, ctx)
            if attr not in base:
                pnix_error(f"missing attr `{attr}`", error_class="attribute-missing")
            val = force_value(base[attr])
        return val
    if tag == "dynamic_select_default":
        val = force_value(eval_ast(ast["base"], env, ctx))
        ok = True
        for segment in ast["segments"]:
            attr = eval_attr_segments([segment], env, ctx)
            if isinstance(val, dict) and attr in val:
                val = force_value(val[attr])
            else:
                ok = False
                break
        if ok:
            return val
        return eval_ast(ast["default"], env, ctx)
    if tag == "has_attr":
        # Nix `?` is false on ANY non-set base (`1 ? a` == false) — unlike
        # builtins.hasAttr, which errors on non-sets (oracle-pinned 2026-07-08;
        # divergence caught by the tri-host gate on kernel-prims-02).
        base = force_value(eval_ast(ast["base"], env, ctx))
        if not isinstance(base, dict):
            return False
        return has_attr_path_value(base, ast.get("path", str(ast["attr"]).split(".")))
    if tag == "dynamic_has_attr":
        base = force_value(eval_ast(ast["base"], env, ctx))
        if not isinstance(base, dict):
            return False
        return has_attr_path_value(base, eval_attr_path_segments(ast["segments"], env, ctx))
    if tag == "index":
        base = force_value(eval_ast(ast["base"], env, ctx))
        index = force_value(eval_ast(ast["index"], env, ctx))
        if isinstance(base, list):
            if type(index) is not int:
                pnix_error("index must be an integer")
            return force_value(base[index])
        if isinstance(base, dict) and isinstance(index, str):
            if index not in base:
                pnix_error(f"missing attr `{index}`", error_class="attribute-missing")
            return force_value(base[index])
        pnix_error("index target unsupported")
    if tag == "unary":
        if ast["op"] == "!":
            return not bool_value(eval_ast(ast["arg"], env, ctx), "argument of !")
        if ast["op"] == "-":
            return unary_neg_value(eval_ast(ast["arg"], env, ctx))
        pnix_error(f"unsupported unary op `{ast['op']}`")
    if tag == "match":
        scrutinee = eval_ast(ast["scrutinee"], env, ctx)
        for arm in ast["arms"]:
            bindings = match_pattern(arm["pattern"], scrutinee, ctx)
            if bindings is not None:
                matched_env = dict(env)
                matched_env.update(bindings)
                if "guard" in arm and not bool_value(eval_ast(arm["guard"], matched_env, ctx), "match guard"):
                    continue
                return eval_ast(arm["body"], matched_env, ctx)
        pnix_error("non-exhaustive match")
    if tag == "binary":
        op = ast["op"]
        if op == "&&":
            lhs = bool_value(eval_ast(ast["lhs"], env, ctx), "left operand of &&")
            return bool_value(eval_ast(ast["rhs"], env, ctx), "right operand of &&") if lhs else False
        if op == "||":
            lhs = bool_value(eval_ast(ast["lhs"], env, ctx), "left operand of ||")
            return True if lhs else bool_value(eval_ast(ast["rhs"], env, ctx), "right operand of ||")
        if op == "->":
            lhs = bool_value(eval_ast(ast["lhs"], env, ctx), "left operand of ->")
            return True if not lhs else bool_value(eval_ast(ast["rhs"], env, ctx), "right operand of ->")
        return apply_binary(op, eval_ast(ast["lhs"], env, ctx), eval_ast(ast["rhs"], env, ctx))
    pnix_error(f"unsupported AST tag {tag!r}")
    raise AssertionError("unreachable")


def check_formal_attrs(pattern: dict[str, Any], arg: Any) -> None:
    """A lambda's attrset formal pattern without `...` rejects unexpected attrs
    (~/pnix + Nix: `({ x }: x) { x = 1; y = 2; }` -> "unexpected attribute 'y'").
    Scoped to lambda formals so `match` arm semantics are untouched."""
    inner = pattern["pattern"] if pattern.get("tag") == "as" else pattern
    if inner.get("tag") != "attrset" or inner.get("ellipsis"):
        return
    value = force_value(arg)
    if not isinstance(value, dict):
        return
    allowed = {field["name"] for field in inner["fields"]}
    for key in value:
        if key not in allowed:
            pnix_error(f"unexpected attribute '{key}'")


def duplicate_formal_name(pattern: dict[str, Any]) -> str | None:
    bind_name = None
    inner = pattern
    if pattern.get("tag") == "as":
        bind_name = pattern.get("name")
        inner = pattern.get("pattern", {})
    if inner.get("tag") != "attrset":
        return None
    seen: set[str] = set()
    if bind_name is not None:
        seen.add(str(bind_name))
    for field in inner.get("fields", []):
        name = str(field["name"])
        if name in seen:
            return name
        seen.add(name)
    return None


def check_duplicate_formals(pattern: dict[str, Any]) -> None:
    duplicate = duplicate_formal_name(pattern)
    if duplicate is not None:
        pnix_error(f"duplicate formal function argument '{duplicate}'")


def apply_pnix(func: Any, arg_delay: Thunk, ctx: dict[str, Any]) -> Any:
    func = force_value(func)
    if isinstance(func, Closure):
        call_ctx = func.ctx if func.ctx is not None else ctx
        env = dict(func.env)
        if func.pattern is None:
            assert func.param is not None
            env[func.param] = arg_delay
        else:
            check_duplicate_formals(func.pattern)
            bindings = match_pattern(func.pattern, arg_delay, call_ctx, env)
            if bindings is None:
                inner = func.pattern["pattern"] if func.pattern.get("tag") == "as" else func.pattern
                if inner.get("tag") == "list":
                    pnix_error("function argument does not match list pattern")
                pnix_error("function argument does not match pattern")
            check_formal_attrs(func.pattern, arg_delay)
            env.update(bindings)
        return eval_ast(func.body, env, call_ctx)
    if isinstance(func, NativeFunc):
        return func(force_value(arg_delay) if func.force_arg else arg_delay)
    if callable(func):
        return func(force_value(arg_delay))
    pnix_error("apply target is not a function", error_class="not-callable")
    raise AssertionError("unreachable")


def value_to_string(value: Any, ctx: dict[str, Any] | None = None, seen: set[int] | None = None) -> str:
    """`builtins.toString`, matching ~/pnix interpret.rs (5498-5567):
    true->"1", false/null->"", int->str, float->fixed six decimals,
    list->space-joined (recursive), set/function -> error."""
    if seen is None:
        seen = ctx.setdefault("__pnix_to_string_seen__", set()) if ctx is not None else set()
    value = force_value(value)
    if value is None:
        return ""  # ~/pnix: toString null -> ""
    if type(value) is bool:
        return "1" if value else ""  # ~/pnix: true->"1", false->""
    if type(value) is int:
        return str(value)
    if type(value) is float:
        return format(value, ".6f")
    if isinstance(value, PnixPath):
        return make_context_string(str(value), {str(value)})
    if is_string_value(value):
        if value.startswith("#<pnix-hy-"):  # realized closure/native marker
            pnix_error("cannot coerce a function to a string")
        return make_context_string(str(value), context_of_string(value))
    if isinstance(value, list):
        out: list[str] = []
        context: set[str] = set()
        for item in value:
            coerced = value_to_string(item, ctx, seen)
            out.append(str(coerced))
            context.update(context_of_string(coerced))
        return make_context_string(" ".join(out), context)
    if isinstance(value, (Closure, NativeFunc)):
        pnix_error("cannot coerce a function to a string")
    if isinstance(value, dict):
        value_id = id(value)
        if value_id in seen:
            pnix_error("toString cycle detected")
        seen.add(value_id)
        try:
            if "__toString" in value:
                return value_to_string(
                    apply_pnix(force_value(value["__toString"]), Thunk(lambda value=value: value), ctx or runtime_context({})),
                    ctx,
                    seen,
                )
            if "outPath" in value:
                return value_to_string(value["outPath"], ctx, seen)
            pnix_error("cannot coerce a set to a string: missing __toString or outPath")
        finally:
            seen.discard(value_id)
    pnix_error("cannot coerce value to a string")
    raise AssertionError("unreachable")


def native_builtins(ctx: dict[str, Any]) -> dict[str, Any]:
    def force_list(value: Any, label: str = "builtin argument") -> list[Any]:
        return list_value(value, label)

    def force_map(value: Any, label: str = "builtin argument") -> dict[str, Any]:
        return attrset_value(value, label)

    def sort_list_arg(value: Any) -> list[Any]:
        forced = force_value(value)
        if isinstance(forced, list):
            return forced
        pnix_error(f"builtins.sort second argument must be list, got {schema_type_name(forced)}")
        raise AssertionError("unreachable")

    def cmp_with(pred: Any) -> Callable[[Any, Any], int]:
        def compare(lhs: Any, rhs: Any) -> int:
            left_less = predicate_bool_value(
                apply_pnix(apply_pnix(pred, Thunk(lambda: force_value(lhs)), ctx), Thunk(lambda: force_value(rhs)), ctx),
                "builtins.sort comparator",
            )
            right_less = predicate_bool_value(
                apply_pnix(apply_pnix(pred, Thunk(lambda: force_value(rhs)), ctx), Thunk(lambda: force_value(lhs)), ctx),
                "builtins.sort comparator",
            )
            if left_less:
                return -1
            if right_less:
                return 1
            return 0

        return compare

    def filter_attrs(pred: Any, value: Any) -> dict[str, Any]:
        out: dict[str, Any] = {}
        for key, item in force_map(value, "builtins.filterAttrs set").items():
            keep = bool_value(
                apply_pnix(
                    apply_pnix(pred, Thunk(lambda key=key: key), ctx),
                    Thunk(lambda item=item: force_value(item)),
                    ctx,
                ),
                "builtins.filterAttrs predicate",
            )
            if keep:
                out[key] = item
        return out

    def group_by(func: Any, value: Any) -> dict[str, Any]:
        value = force_value(value)
        if not isinstance(value, list):
            pnix_error(f"builtins.groupBy: second argument must be list, got {_type_of(value)}")
        out: dict[str, list[Any]] = {}
        for item in value:
            key_v = force_value(apply_pnix(func, Thunk(lambda item=item: force_value(item)), ctx))
            if not is_string_value(key_v):
                pnix_error(f"builtins.groupBy: key function must return string, got {_type_of(key_v)}")
            out.setdefault(str(key_v), []).append(force_value(item))
        return out

    def partition(pred: Any, value: Any) -> dict[str, Any]:
        right: list[Any] = []
        wrong: list[Any] = []
        for item in force_list(value, "builtins.partition list"):
            target = right if bool_value(
                apply_pnix(pred, Thunk(lambda item=item: force_value(item)), ctx),
                "builtins.partition predicate",
            ) else wrong
            target.append(force_value(item))
        return {"right": right, "wrong": wrong}

    def foldr_builtin(func: Any, init: Any, xs: list[Any]) -> Any:
        acc = init
        for item in reversed(xs):
            step = apply_pnix(func, Thunk(lambda item=item: force_value(item)), ctx)
            acc = apply_pnix(step, Thunk(lambda acc=acc: force_value(acc)), ctx)
        return acc

    builtins = {
        "currentSystem": current_system_value(),
        "nixVersion": "2.18.0-pnix",
        "langVersion": 6,
        "storeDir": "/nix/store",
        "import": lambda path: import_builtin_value(path, ctx, "eval"),
        "scopedImport": lambda scope: (lambda path: scoped_import_value(scope, path, ctx, "eval")),
        "pathExists": lambda path: resolve_fs_path(path, ctx, "builtins.pathExists").exists(),
        "readFile": lambda path: read_file_value(path, ctx),
        "readFileType": lambda path: read_file_type_value(resolve_fs_path(path, ctx, "builtins.readFileType")),
        "readDir": lambda path: read_dir_value(path, ctx),
        "toFile": lambda name: (lambda contents: to_file_value(name, contents)),
        "hashString": hash_string_function,
        "hashFile": lambda algo: (lambda path: hash_file_value(algo, path, ctx)),
        "baseNameOf": lambda path: base_name_value(path),
        "dirOf": lambda path: dir_of_value(path),
        "toPath": lambda value: to_path_string_value(value),
        "storePath": lambda value: PnixPath(path_text_value(value, ctx, "builtins.storePath")),
        "getEnv": lambda name: get_env_value(name),
        "placeholder": lambda name: make_context_string(
            "/pnix-placeholder/" + string_value(name, "builtins.placeholder name"),
            {"=placeholder!" + string_value(name, "builtins.placeholder name")},
        ),
        "break": lambda value: value,
        "warn": lambda msg: (lambda value: (string_value(msg, "builtins.warn message"), value)[1]),
        "traceVerbose": lambda msg: (lambda value: (string_value(msg, "builtins.traceVerbose message"), value)[1]),
        "attrNames": lambda m: sorted(attrset_arg_value(m, "attrNames").keys()),
        "hasAttr": lambda name: (lambda m: has_attr_value(name, m)),
        "getAttr": lambda name: (lambda m: get_attr_value(name, m)),
        "attrByPath": NativeFunc(
            lambda path: NativeFunc(
                lambda default: (lambda attrs: attr_by_path_value(path, default, attrs)),
                force_arg=False,
            ),
            force_arg=False,
        ),
        "removeAttrs": lambda attrs: (lambda names: remove_attrs_value(attrs, names)),
        "listToAttrs": lambda xs: list_to_attrs_value(xs),
        "filterAttrs": lambda pred: (lambda m: filter_attrs(pred, m)),
        "functionArgs": lambda value: function_args_value(value),
        "intersectAttrs": lambda lhs: (
            lambda rhs: {
                key: value
                for key, value in force_map(rhs, "builtins.intersectAttrs rhs").items()
                if key in force_map(lhs, "builtins.intersectAttrs lhs")
            }
        ),
        "zipAttrsWith": lambda func: (lambda xs: zip_attrs_with_value(func, xs, ctx)),
        "catAttrs": lambda name: (lambda xs: cat_attrs_value(name, xs)),
        "elemAt": lambda xs: (lambda i: list_elem_at_value(xs, i)),
        "length": lambda x: length_value(x),
        "head": lambda xs: list_head_value(xs),
        "tail": lambda xs: list_tail_value(xs),
        "toString": lambda x: value_to_string(x, ctx),
        "toJSON": lambda x: to_json_string_value(x),
        "map": lambda f: (lambda xs: map_list_value(f, xs, ctx)),
        "filter": lambda f: (lambda xs: filter_list_value(f, xs, ctx)),
        "foldl'": lambda f: (
            lambda init: (
                lambda xs: _foldl_builtin(f, init, list_arg_value(xs, "foldl'", "third arg"), ctx)
            )
        ),
        "fold": lambda f: (
            lambda init: (
                lambda xs: _foldl_builtin(f, init, list_arg_value(xs, "fold", "third arg"), ctx)
            )
        ),
        "foldl": lambda f: (
            lambda init: (
                lambda xs: _foldl_builtin(f, init, list_arg_value(xs, "foldl", "third arg"), ctx)
            )
        ),
        "foldr": lambda f: (
            lambda init: (
                lambda xs: foldr_builtin(f, init, list_arg_value(xs, "foldr", "third arg"))
            )
        ),
        "cons": lambda x: (lambda xs: [force_value(x)] + force_list(xs, "builtins.cons list")),
        "append": lambda xs: (lambda ys: force_list(xs, "builtins.append lhs") + force_list(ys, "builtins.append rhs")),
        "take": lambda count: (lambda xs: list_take_value(count, xs)),
        "drop": lambda count: (lambda xs: list_drop_value(count, xs)),
        "reverse": lambda xs: list(reversed(force_list(xs, "builtins.reverse list"))),
        "reverseList": lambda xs: list(reversed(force_list(xs, "builtins.reverseList list"))),
        "zip": lambda lhs: (lambda rhs: zip_lists_value(lhs, rhs)),
        "flatten": lambda xs: flatten_value(xs),
        "find": lambda needle: (lambda xs: find_value(needle, xs)),
        "get": lambda attrs: (lambda name: get_value(attrs, name)),
        "mapGet": lambda attrs: (lambda name: get_value(attrs, name)),
        "set": lambda attrs: (lambda name: (lambda value: set_value(attrs, name, value))),
        "mapSet": lambda attrs: (lambda name: (lambda value: set_value(attrs, name, value))),
        "keys": lambda attrs: sorted(force_map(attrs, "builtins.keys attrs").keys()),
        "mapKeys": lambda attrs: sorted(force_map(attrs, "builtins.mapKeys attrs").keys()),
        "values": lambda attrs: attr_values_value(attrs, "builtins.values attrs"),
        "mapValues": lambda attrs: attr_values_value(attrs, "builtins.mapValues attrs"),
        "merge": lambda lhs: (lambda rhs: merge_attrsets(lhs, rhs)),
        "mapMerge": lambda lhs: (lambda rhs: merge_attrsets(lhs, rhs)),
        "elem": lambda x: (lambda xs: any(eq_value(x, item, 1) for item in list_arg_value(xs, "elem", "second argument"))),
        "any": lambda pred: (lambda xs: any_list_value(pred, xs, ctx)),
        "all": lambda pred: (lambda xs: all_list_value(pred, xs, ctx)),
        "concatLists": lambda xss: concat_lists_value(xss),
        "concatMap": lambda f: (
            lambda xs: [
                force_value(item)
                for x in force_list(xs, "builtins.concatMap list")
                for item in force_list(
                    apply_pnix(f, Thunk(lambda x=x: force_value(x)), ctx),
                    "builtins.concatMap result",
                )
            ]
        ),
        "genList": lambda f: (lambda n: gen_list_value(f, n, ctx)),
        "groupBy": lambda f: (lambda xs: group_by(f, xs)),
        "partition": lambda pred: (lambda xs: partition(pred, xs)),
        "genericClosure": lambda arg: generic_closure_value(arg, ctx),
        "attrValues": lambda m: attr_values_value(m, "builtins.attrValues"),
        "getAttrs": lambda names: (lambda attrs: get_attrs_value(names, attrs)),
        "mapAttrs": lambda f: (lambda m: map_attrs_value(f, m, ctx)),
        "sort": lambda pred: (
            lambda xs: sorted(
                sort_list_arg(xs),
                key=cmp_to_key(cmp_with(pred)),
            )
        ),
        "substring": lambda start: (lambda length: (lambda s: substring_value(start, length, s))),
        "stringLength": lambda s: string_byte_length(s, "builtins.stringLength string"),
        "hasPrefix": lambda prefix: (
            lambda s: string_value(s, "builtins.hasPrefix string").startswith(string_value(prefix, "builtins.hasPrefix prefix"))
        ),
        "hasSuffix": lambda suffix: (
            lambda s: string_value(s, "builtins.hasSuffix string").endswith(string_value(suffix, "builtins.hasSuffix suffix"))
        ),
        "replaceStrings": lambda froms: (lambda tos: (lambda s: replace_strings_value(froms, tos, s))),
        "concatStringsSep": lambda sep: (lambda xs: concat_strings_sep_value(sep, xs)),
        "concatStrings": lambda xs: concat_strings_value(xs),
        "compareVersions": lambda lhs: (lambda rhs: compare_versions_value(lhs, rhs)),
        "splitVersion": lambda s: split_version_value(s),
        "parseDrvName": lambda s: parse_drv_name_value(s),
        "match": lambda pattern: (lambda value: regex_match_value(pattern, value)),
        "split": lambda pattern: (lambda value: regex_split_value(pattern, value)),
        "fromJSON": lambda value: from_json_value(value),
        "fromTOML": lambda value: from_toml_value(value),
        "schemaValidate": lambda schema: (lambda value: schema_validate_value(schema, value)),
        "schemaNormalize": lambda schema: (lambda value: schema_normalize_value(schema, value)),
        "schemaExplain": lambda schema: (lambda value: schema_explain_value(schema, value)),
        "xmlParse": lambda value: xml_parse_value(value),
        "xmlEmit": lambda value: markup_emit_value(value, html=False),
        "htmlParse": lambda value: html_parse_value(value),
        "htmlEmit": lambda value: markup_emit_value(value, html=True),
        "lessThan": lambda lhs: (lambda rhs: nix_less_than(lhs, rhs)),
        "add": lambda lhs: (lambda rhs: apply_binary("+", lhs, rhs)),
        "sub": lambda lhs: (lambda rhs: apply_binary("-", lhs, rhs)),
        "mul": lambda lhs: (lambda rhs: apply_binary("*", lhs, rhs)),
        "div": lambda lhs: (lambda rhs: apply_binary("/", lhs, rhs)),
        "mod": lambda lhs: (lambda rhs: builtins_mod_value(lhs, rhs)),
        "neg": lambda value: check_i64(-number_value(value, "builtins.neg argument"), "-"),
        "abs": lambda value: abs(number_value(value, "builtins.abs argument")),
        "bitAnd": lambda lhs: (lambda rhs: bit_op_value("bitAnd", lhs, rhs, lambda a, b: a & b)),
        "bitOr": lambda lhs: (lambda rhs: bit_op_value("bitOr", lhs, rhs, lambda a, b: a | b)),
        "bitXor": lambda lhs: (lambda rhs: bit_op_value("bitXor", lhs, rhs, lambda a, b: a ^ b)),
        "pow": lambda lhs: (lambda rhs: pow_value(lhs, rhs)),
        "sqrt": lambda value: math.sqrt(number_value(value, "builtins.sqrt argument")),
        "floor": lambda value: checked_float_to_i64(value, "builtins.floor", math.floor),
        "ceil": lambda value: checked_float_to_i64(value, "builtins.ceil", math.ceil),
        "exp": lambda value: math.exp(number_value(value, "builtins.exp argument")),
        "ln": lambda value: math.log(number_value(value, "builtins.ln argument")),
        "log": lambda value: math.log(number_value(value, "builtins.log argument")),
        "sin": lambda value: math.sin(number_value(value, "builtins.sin argument")),
        "cos": lambda value: math.cos(number_value(value, "builtins.cos argument")),
        "tan": lambda value: math.tan(number_value(value, "builtins.tan argument")),
        "atan2": lambda lhs: (lambda rhs: math.atan2(number_value(lhs, "builtins.atan2 y"), number_value(rhs, "builtins.atan2 x"))),
        "and": lambda lhs: (lambda rhs: bool_value(lhs, "builtins.and lhs") and bool_value(rhs, "builtins.and rhs")),
        "or": lambda lhs: (lambda rhs: bool_value(lhs, "builtins.or lhs") or bool_value(rhs, "builtins.or rhs")),
        "not": lambda value: not bool_value(value, "builtins.not argument"),
        "eq": lambda lhs: (lambda rhs: eq_value(lhs, rhs)),
        "lt": lambda lhs: (lambda rhs: nix_less_than(lhs, rhs)),
        "le": lambda lhs: (lambda rhs: not nix_less_than(rhs, lhs)),
        "gt": lambda lhs: (lambda rhs: nix_less_than(rhs, lhs)),
        "ge": lambda lhs: (lambda rhs: not nix_less_than(lhs, rhs)),
        "seq": lambda lhs: (lambda rhs: rhs),
        "deepSeq": lambda lhs: (lambda rhs: (deep_force_value(lhs), rhs)[1]),
        "tryEval": NativeFunc(lambda thunk: try_eval_value(thunk), force_arg=False),
        "derivationStrict": lambda attrs: derivation_value(attrs, "derivationStrict"),
        "derivation": lambda attrs: derivation_value(attrs, "derivation"),
        "addErrorContext": lambda msg: (lambda value: add_error_context_value(msg, value)),
        "unsafeGetAttrPos": lambda name: (lambda attrs: unsafe_get_attr_pos_value(name, attrs, ctx)),
        "unsafeDiscardStringContext": lambda value: unsafe_discard_string_context_value(value),
        "hasContext": lambda value: has_context_value(value),
        "getContext": lambda value: get_context_value(value),
        "appendContext": lambda value: (lambda extra: append_context_value(value, extra)),
        "addDrvOutputDependencies": lambda value: context_string_value(value, "addDrvOutputDependencies"),
        "unsafeDiscardOutputDependency": lambda value: context_string_value(value, "unsafeDiscardOutputDependency"),
        "unsafeAddOutputDependency": lambda value: context_string_value(value, "unsafeAddOutputDependency"),
        "unsafeAddOutputName": lambda name: (lambda value: unsafe_add_output_name_value(name, value)),
        "trace": lambda msg: (lambda value: value),
        "throw": throw_value,
        "abort": lambda msg: abort_value(msg),
        "typeOf": lambda x: _type_of(x),
        "isList": lambda x: isinstance(force_value(x), list),
        "isAttrs": lambda x: isinstance(force_value(x), dict),
        "isString": lambda x: is_string_value(force_value(x)),
        "isInt": lambda x: type(force_value(x)) is int,
        "isFloat": lambda x: type(force_value(x)) is float,
        "isFinite": lambda x: (
            True
            if type(force_value(x)) is int
            else math.isfinite(force_value(x))
            if type(force_value(x)) is float
            else False
        ),
        "isInf": lambda x: math.isinf(force_value(x)) if type(force_value(x)) is float else False,
        "isNaN": lambda x: math.isnan(force_value(x)) if type(force_value(x)) is float else False,
        "isBool": lambda x: type(force_value(x)) is bool,
        "isFunction": lambda x: isinstance(force_value(x), Closure) or callable(force_value(x)),
        "isNull": lambda x: force_value(x) is None,
        "isPath": lambda x: isinstance(force_value(x), PnixPath),
        # --- README / nixpkgs-ish extensions ---
        "toXML": lambda x: to_xml_value(x),
        "getAttrFromPath": lambda path: (lambda attrs: get_attr_from_path_value(path, attrs)),
        "hasAttrByPath": lambda path: (lambda attrs: has_attr_by_path_value(path, attrs)),
        "last": lambda xs: list_last_value(xs),
        "init": lambda xs: list_init_value(xs),
        "removePrefix": lambda prefix: (lambda s: remove_prefix_value(prefix, s)),
        "removeSuffix": lambda suffix: (lambda s: remove_suffix_value(suffix, s)),
        "splitString": lambda sep: (lambda s: split_string_value(sep, s)),
        "toLower": lambda s: string_value(s, "builtins.toLower").lower(),
        "toUpper": lambda s: string_value(s, "builtins.toUpper").upper(),
        "boolToString": lambda b: bool_to_string_value(b),
        "optional": lambda cond: (lambda x: [force_value(x)] if bool_value(cond, "builtins.optional") else []),
        "optionals": lambda cond: (lambda xs: list_value(xs, "builtins.optionals") if bool_value(cond, "builtins.optionals") else []),
        "optionalAttrs": lambda cond: (
            lambda attrs: dict(attrset_value(attrs, "builtins.optionalAttrs"))
            if bool_value(cond, "builtins.optionalAttrs")
            else {}
        ),
        "implies": lambda a: (lambda b: (not bool_value(a, "builtins.implies lhs")) or bool_value(b, "builtins.implies rhs")),
        "when": lambda cond: (lambda x: force_value(x) if bool_value(cond, "builtins.when") else None),
        "id": lambda x: force_value(x),
        "const": lambda x: (lambda _y: force_value(x)),
        "flip": lambda f: (
            lambda b: (
                lambda a: apply_pnix(
                    apply_pnix(f, Thunk(lambda a=a: force_value(a)), ctx),
                    Thunk(lambda b=b: force_value(b)),
                    ctx,
                )
            )
        ),
        "pipe": lambda value: (lambda fns: pipe_value(value, fns, ctx)),
        "min": lambda a: (lambda b: min(number_value(a, "builtins.min"), number_value(b, "builtins.min"))),
        "max": lambda a: (lambda b: max(number_value(a, "builtins.max"), number_value(b, "builtins.max"))),
        "range": lambda first: (lambda last: range_value(first, last)),
        "unique": lambda xs: unique_list_value(xs),
        "recursiveUpdate": lambda lhs: (lambda rhs: recursive_update_value(lhs, rhs)),
        "updateManyAttrs": lambda updates: (lambda original: update_many_attrs_value(updates, original)),
        "sum": lambda xs: sum_list_value(xs),
        "product": lambda xs: product_list_value(xs),
        "fix": lambda f: fix_value(f, ctx),
        "zipLists": lambda xl: (lambda yl: zip_lists_lib_value(xl, yl)),
        "zipListsWith": lambda f: (lambda xl: (lambda yl: zip_lists_with_value(f, xl, yl, ctx))),
        "getName": lambda x: get_name_value(x),
        "getVersion": lambda x: get_version_value(x),
        "getAttrFromPathOr": lambda attrs: (
            lambda path: (lambda default: get_attr_from_path_or_value(attrs, path, default))
        ),
        "filterAttrsRecursive": lambda pred: (lambda m: filter_attrs_recursive_value(pred, m, ctx)),
        "mapAttrsRecursive": lambda f: (lambda m: map_attrs_recursive_value(f, m, ctx)),
        "intersectLists": lambda a: (lambda b: intersect_lists_value(a, b)),
        "subtractLists": lambda e: (lambda l: subtract_lists_value(e, l)),
        "concatMapStringsSep": lambda sep: (
            lambda f: (lambda xs: concat_map_strings_sep_value(sep, f, xs, ctx))
        ),
        "mapAttrsToList": lambda f: (lambda m: map_attrs_to_list_value(f, m, ctx)),
        "zipAttrs": lambda xs: zip_attrs_value(xs),
        "assertMsg": lambda cond: (lambda msg: assert_msg_value(cond, msg)),
        # lib.assert / nonstandard alias of assertMsg
        "assert": lambda cond: (lambda msg: assert_msg_value(cond, msg)),
        "fetchurl": lambda arg: fetchurl_value(arg),
        "fetchTarball": lambda arg: fetch_tarball_value(arg),
        "fetchGit": lambda arg: fetch_git_value(arg),
    }
    builtins["true"] = True
    builtins["false"] = False
    builtins["null"] = None
    builtins["builtins"] = Thunk(lambda: builtins)
    return builtins


def _foldl_builtin(func: Any, init: Any, xs: list[Any], ctx: dict[str, Any]) -> Any:
    acc = init
    for item in xs:
        step = apply_pnix(func, Thunk(lambda acc=acc: force_value(acc)), ctx)
        acc = apply_pnix(step, Thunk(lambda item=item: force_value(item)), ctx)
    return acc


def _type_of(value: Any) -> str:
    value = force_value(value)
    if isinstance(value, Closure) or callable(value):
        return "lambda"
    if isinstance(value, PnixPath):
        return "path"
    if isinstance(value, list):
        return "list"
    if isinstance(value, dict):
        return "set"
    if isinstance(value, str):
        return "string"
    if type(value) is bool:
        return "bool"
    if type(value) is int:
        return "int"
    if type(value) is float:
        return "float"
    if value is None:
        return "null"
    return "unknown"


BUILTIN_ALIAS_NAMES = (
    "currentSystem",
    "nixVersion",
    "langVersion",
    "storeDir",
    "import",
    "scopedImport",
    "pathExists",
    "readFile",
    "readFileType",
    "readDir",
    "toFile",
    "hashString",
    "hashFile",
    "baseNameOf",
    "dirOf",
    "toPath",
    "storePath",
    "getEnv",
    "placeholder",
    "break",
    "warn",
    "traceVerbose",
    "attrNames",
    "hasAttr",
    "getAttr",
    "attrByPath",
    "removeAttrs",
    "listToAttrs",
    "filterAttrs",
    "functionArgs",
    "intersectAttrs",
    "zipAttrsWith",
    "catAttrs",
    "elemAt",
    "length",
    "head",
    "tail",
    "toString",
    "toJSON",
    "map",
    "filter",
    "foldl'",
    "fold",
    "foldl",
    "foldr",
    "cons",
    "append",
    "take",
    "drop",
    "reverse",
    "reverseList",
    "zip",
    "flatten",
    "find",
    "get",
    "mapGet",
    "set",
    "mapSet",
    "keys",
    "mapKeys",
    "values",
    "mapValues",
    "merge",
    "mapMerge",
    "elem",
    "any",
    "all",
    "concatLists",
    "concatMap",
    "genList",
    "groupBy",
    "partition",
    "genericClosure",
    "attrValues",
    "getAttrs",
    "mapAttrs",
    "sort",
    "substring",
    "stringLength",
    "hasPrefix",
    "hasSuffix",
    "replaceStrings",
    "concatStringsSep",
    "concatStrings",
    "compareVersions",
    "splitVersion",
    "parseDrvName",
    "match",
    "split",
    "fromJSON",
    "fromTOML",
    "schemaValidate",
    "schemaNormalize",
    "schemaExplain",
    "xmlParse",
    "xmlEmit",
    "htmlParse",
    "htmlEmit",
    "lessThan",
    "add",
    "sub",
    "mul",
    "div",
    "mod",
    "neg",
    "abs",
    "bitAnd",
    "bitOr",
    "bitXor",
    "pow",
    "sqrt",
    "floor",
    "ceil",
    "exp",
    "ln",
    "log",
    "sin",
    "cos",
    "tan",
    "atan2",
    "and",
    "or",
    "not",
    "eq",
    "lt",
    "le",
    "gt",
    "ge",
    "seq",
    "deepSeq",
    "tryEval",
    "derivationStrict",
    "derivation",
    "addErrorContext",
    "unsafeGetAttrPos",
    "unsafeDiscardStringContext",
    "hasContext",
    "getContext",
    "appendContext",
    "addDrvOutputDependencies",
    "unsafeDiscardOutputDependency",
    "unsafeAddOutputDependency",
    "unsafeAddOutputName",
    "trace",
    "throw",
    "abort",
    "typeOf",
    "isList",
    "isAttrs",
    "isString",
    "isInt",
    "isFloat",
    "isFinite",
    "isInf",
    "isNaN",
    "isBool",
    "isFunction",
    "isNull",
    "isPath",
    "toXML",
    "getAttrFromPath",
    "hasAttrByPath",
    "last",
    "init",
    "removePrefix",
    "removeSuffix",
    "splitString",
    "toLower",
    "toUpper",
    "boolToString",
    "optional",
    "optionals",
    "optionalAttrs",
    "implies",
    "when",
    "id",
    "const",
    "flip",
    "pipe",
    "min",
    "max",
    "range",
    "unique",
    "recursiveUpdate",
    "updateManyAttrs",
    "sum",
    "product",
    "fix",
    "zipLists",
    "zipListsWith",
    "getName",
    "getVersion",
    "getAttrFromPathOr",
    "filterAttrsRecursive",
    "mapAttrsRecursive",
    "intersectLists",
    "subtractLists",
    "concatMapStringsSep",
    "mapAttrsToList",
    "zipAttrs",
    "assertMsg",
    "fetchurl",
    "fetchTarball",
    "fetchGit",
)


def build_lib_attrset(builtins: dict[str, Any], ctx: dict[str, Any]) -> dict[str, Any]:
    """nixpkgs-style `lib`: re-export builtins plus pure helpers / nested attrsets."""
    lib = dict(builtins)
    # Nested namespaces used by README tests
    lib["attrsets"] = {
        "isAttrs": builtins["isAttrs"],
        "attrNames": builtins["attrNames"],
        "attrValues": builtins["attrValues"],
        "hasAttr": builtins["hasAttr"],
        "getAttr": builtins["getAttr"],
        "attrByPath": builtins["attrByPath"],
        "getAttrFromPath": builtins["getAttrFromPath"],
        "hasAttrByPath": builtins["hasAttrByPath"],
        "mapAttrs": builtins["mapAttrs"],
        "mapAttrsToList": builtins["mapAttrsToList"],
        "filterAttrs": builtins["filterAttrs"],
        "filterAttrsRecursive": builtins["filterAttrsRecursive"],
        "mapAttrsRecursive": builtins["mapAttrsRecursive"],
        "recursiveUpdate": builtins["recursiveUpdate"],
        "optionalAttrs": builtins["optionalAttrs"],
        "zipAttrs": builtins["zipAttrs"],
        "zipAttrsWith": builtins["zipAttrsWith"],
        "catAttrs": builtins["catAttrs"],
        "intersectAttrs": builtins["intersectAttrs"],
        "listToAttrs": builtins["listToAttrs"],
        "removeAttrs": builtins["removeAttrs"],
    }
    lib["lists"] = {
        "head": builtins["head"],
        "tail": builtins["tail"],
        "last": builtins["last"],
        "init": builtins["init"],
        "elem": builtins["elem"],
        "elemAt": builtins["elemAt"],
        "length": builtins["length"],
        "unique": builtins["unique"],
        "intersectLists": builtins["intersectLists"],
        "subtractLists": builtins["subtractLists"],
        "range": builtins["range"],
        "flatten": builtins["flatten"],
        "concatLists": builtins["concatLists"],
        "concatMap": builtins["concatMap"],
        "zipLists": builtins["zipLists"],
        "zipListsWith": builtins["zipListsWith"],
        "partition": builtins["partition"],
        "genList": builtins["genList"],
        "take": builtins["take"],
        "drop": builtins["drop"],
        "reverseList": builtins["reverseList"],
        "foldl": builtins["foldl"],
        "foldr": builtins["foldr"],
        "sum": builtins["sum"],
        "product": builtins["product"],
    }
    lib["strings"] = {
        "concatStringsSep": builtins["concatStringsSep"],
        "concatMapStringsSep": builtins["concatMapStringsSep"],
        "hasPrefix": builtins["hasPrefix"],
        "hasSuffix": builtins["hasSuffix"],
        "removePrefix": builtins["removePrefix"],
        "removeSuffix": builtins["removeSuffix"],
        "splitString": builtins["splitString"],
        "toLower": builtins["toLower"],
        "toUpper": builtins["toUpper"],
        "stringLength": builtins["stringLength"],
        "substring": builtins["substring"],
        "replaceStrings": builtins["replaceStrings"],
    }
    lib["trivial"] = {
        "id": builtins["id"],
        "const": builtins["const"],
        "flip": builtins["flip"],
        "pipe": builtins["pipe"],
        "min": builtins["min"],
        "max": builtins["max"],
        "mod": builtins["mod"],
        "compare": builtins.get("compareVersions"),
        "boolToString": builtins["boolToString"],
        "toHexString": lambda n: format(integer_value(n, "lib.toHexString"), "x"),
        "warn": builtins["warn"],
        "throw": builtins["throw"],
        "isFunction": builtins["isFunction"],
        "isInt": builtins["isInt"],
        "isBool": builtins["isBool"],
        "isString": builtins["isString"],
        "isList": builtins["isList"],
        "isAttrs": builtins["isAttrs"],
        "isPath": builtins["isPath"],
        "isNull": builtins["isNull"],
        "isFloat": builtins["isFloat"],
        "fix": builtins["fix"],
    }
    lib["asserts"] = {
        "assertMsg": builtins["assertMsg"],
    }
    # Convenience aliases matching common nixpkgs top-level re-exports
    lib["assertMsg"] = builtins["assertMsg"]
    lib["assert"] = builtins["assert"]
    return lib


def initial_env(ctx: dict[str, Any] | None = None) -> dict[str, Any]:
    ctx = ctx or {}
    builtins = native_builtins(ctx)
    lib = build_lib_attrset(builtins, ctx)
    env = {
        "builtins": builtins,
        "lib": lib,
        "true": True,
        "false": False,
        "null": None,
    }
    env.update({name: builtins[name] for name in BUILTIN_ALIAS_NAMES if name in builtins})
    env["builtins"] = builtins
    env["lib"] = lib
    return env


def runtime_context(opts: dict[str, Any] | None = None) -> dict[str, Any]:
    ctx = dict(opts or {})
    ctx.setdefault("base_dir", str(Path.cwd()))
    ctx.setdefault("import_cache", {})
    ctx.setdefault("import_stack", [])
    return ctx


def resolve_import_path(import_path: str, ctx: dict[str, Any]) -> Path:
    path = Path(import_path).expanduser()
    if not path.is_absolute():
        if not (import_path.startswith("./") or import_path.startswith("../")):
            pnix_error("only relative .px imports are supported")
        path = Path(str(ctx.get("base_dir", Path.cwd()))).expanduser() / path
    resolved = path.resolve()
    if resolved.suffix != ".px":
        pnix_error("import only supports .px files")
    return resolved


def file_context(ctx: dict[str, Any], path: Path) -> dict[str, Any]:
    child = dict(ctx)
    child["base_dir"] = str(path.parent)
    child["source_path"] = str(path)
    child["path_literals_absolute"] = True
    child["import_cache"] = ctx.setdefault("import_cache", {})
    child["import_stack"] = ctx.setdefault("import_stack", [])
    if "events" in ctx:
        child["events"] = ctx["events"]
    return child


def read_px_file(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        pnix_error(f"cannot read import `{path}`: {exc}")
        raise AssertionError("unreachable")


# --- deep evaluation lane -----------------------------------------------------
# The tree-walking evaluator (`eval_source_raw`) recurses once per AST node, so a
# deeply-nested but perfectly finite .px (real Nix evaluates it; pnix-clj gives it
# a 2GB stack thread; the compiled lane in compiled.py already does this) blows
# CPython's default 1000-frame recursion limit. Run it on a dedicated big-stack
# thread with a raised limit so pnix-hy matches the oracle instead of raising
# RecursionError. Re-entrant calls (imports evaluated from inside the lane) run
# inline: one thread hop per outermost entry.
_DEEP_STACK_BYTES = 1024 * 1024 * 1024
_DEEP_RECURSION_LIMIT = 200_000
_deep_lane = _threading.local()


def _run_on_deep_stack(fn):
    if getattr(_deep_lane, "active", False):
        return fn()
    box: dict[str, Any] = {}

    def go() -> None:
        _deep_lane.active = True
        old_limit = _sys.getrecursionlimit()
        _sys.setrecursionlimit(_DEEP_RECURSION_LIMIT)
        try:
            box["value"] = fn()
        except BaseException as exc:  # noqa: BLE001 - re-raised on the caller
            box["error"] = exc
        finally:
            _sys.setrecursionlimit(old_limit)
            _deep_lane.active = False

    old_stack = _threading.stack_size()
    _threading.stack_size(_DEEP_STACK_BYTES)
    try:
        worker = _threading.Thread(target=go, name="pnix-deep-eval")
        worker.start()
        worker.join()
    finally:
        _threading.stack_size(old_stack)
    if "error" in box:
        raise box["error"]
    return box["value"]


def eval_source_raw(source: str, ctx: dict[str, Any], *, realize: bool) -> Any:
    def go() -> Any:
        ctx["source_text"] = source
        ast = parse(source)
        env = initial_env(ctx)
        env.update(ctx.get("env", {}))
        value = eval_ast(ast, env, ctx)
        return realize_value(value) if realize else value

    return _run_on_deep_stack(go)


def import_value(
    import_path: str,
    ctx: dict[str, Any],
    engine: str,
    runtime_namespace: dict[str, Any] | None = None,
) -> Any:
    resolved = resolve_import_path(import_path, ctx)
    key = f"{engine}:{resolved}"
    cache = ctx.setdefault("import_cache", {})
    if key in cache:
        return cache[key]
    stack = ctx.setdefault("import_stack", [])
    if key in stack:
        pnix_error("import cycle: " + " -> ".join(stack + [key]))
    stack.append(key)
    if "events" in ctx:
        ctx["events"].append({"event": "import", "path": str(resolved)})
    try:
        child_ctx = file_context(ctx, resolved)
        source = read_px_file(resolved)
        if engine == "eval":
            value = eval_source_raw(source, child_ctx, realize=False)
        elif engine == "run":
            if runtime_namespace is None:
                pnix_error("compiler import requires a runtime namespace")
            value = run_px_source_raw(
                source,
                child_ctx,
                realize=False,
                runtime_namespace=runtime_namespace,
                include_prelude=False,
            )
        else:
            pnix_error(f"unknown import engine `{engine}`")
        cache[key] = value
        return value
    finally:
        stack.pop()


def import_path_value(value: Any, ctx: dict[str, Any], label: str) -> str:
    value = force_value(value)
    if isinstance(value, PnixPath):
        return str(value)
    if is_string_value(value):
        return str(value)
    pnix_error(f"{label}: expected path or string")
    raise AssertionError("unreachable")


def import_builtin_value(
    value: Any,
    ctx: dict[str, Any],
    engine: str,
    runtime_namespace: dict[str, Any] | None = None,
) -> Any:
    return import_value(import_path_value(value, ctx, "builtins.import"), ctx, engine, runtime_namespace)


def scoped_import_value(
    scope: Any,
    value: Any,
    ctx: dict[str, Any],
    engine: str,
    runtime_namespace: dict[str, Any] | None = None,
) -> Any:
    scoped_ctx = dict(ctx)
    scoped_env = dict(scoped_ctx.get("env", {}))
    scoped_env.update(attrset_value(scope, "builtins.scopedImport scope"))
    scoped_ctx["env"] = scoped_env
    return import_builtin_value(value, scoped_ctx, engine, runtime_namespace)


def eval_source(source: str, opts: dict[str, Any] | None = None) -> Any:
    # PARSE recurses per precedence level (parse_left/parse_add/parse_mul), so a
    # deeply-nested .px overflows CPython's 1000-frame default before evaluation
    # even starts. Run the whole entry — parse AND eval — on the deep lane.
    return _run_on_deep_stack(lambda: _eval_source_inner(source, opts))


def _eval_source_inner(source: str, opts: dict[str, Any] | None = None) -> Any:
    events: list[dict[str, Any]] = []
    ctx = runtime_context(opts)
    ctx["events"] = events
    ast = parse(source)
    value = eval_source_raw(source, ctx, realize=True)
    opts = opts or {}
    if opts.get("mirror"):
        return {
            "schema": MIRROR_SCHEMA,
            "runtime": RUNTIME_SCHEMA,
            "ready": True,
            "source": source,
            "ast": ast,
            "value": value,
            "events": events,
        }
    return value


def eval_from_ast(ast: dict[str, Any], opts: dict[str, Any] | None = None) -> Any:
    opts = opts or {}
    ctx = runtime_context(opts)
    ctx.setdefault("source_text", emit_source(ast))
    env = initial_env(ctx)
    env.update(opts.get("env", {}))
    return realize_value(eval_ast(ast, env, ctx))


def eval_normalized_source(source: str, opts: dict[str, Any] | None = None) -> Any:
    ast = parse(emit_source(parse(source)))
    return eval_from_ast(ast, opts)


def stable_data(value: Any) -> Any:
    value = realize_value(value)
    if isinstance(value, dict):
        return {str(key): stable_data(value[key]) for key in sorted(value)}
    if isinstance(value, list):
        return [stable_data(item) for item in value]
    if isinstance(value, tuple):
        return [stable_data(item) for item in value]
    if isinstance(value, Closure):
        return "#<pnix-hy-closure>"
    if callable(value):
        return "#<pnix-hy-native>"
    return value


def stable_json(value: Any) -> str:
    return json.dumps(stable_data(value), ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def term_hash(value: Any) -> str:
    return hashlib.sha256(stable_json(value).encode("utf-8")).hexdigest()


def ast_stable(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            str(key): ast_stable(val)
            for key, val in sorted(value.items())
            if key not in {"pos", "path_positions"}
        }
    if isinstance(value, list):
        return [ast_stable(item) for item in value]
    return stable_data(value)


def ast_hash(value: Any) -> str:
    data = json.dumps(ast_stable(value), ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(data.encode("utf-8")).hexdigest()


HY_RUNTIME_SCHEMA = "pnix-hy.hy-runtime.source-parser-ast-evaluator.v0"

HY_AST_EVALUATOR_SOURCE = r'''
(do
  (import builtins)
  (import json)
  (import functools)
  (import hashlib)
  (import math)
  (import os)
  (import platform)
  (import re)
  (import sys)
  (import tempfile)
  (import tomllib)
  (import xml.etree.ElementTree)

  (defn pnix-main []
    (setv raw-asts (json.loads __PNIX_ASTS_JSON__))
    (setv raw-sources (json.loads __PNIX_SOURCES_JSON__))
    (setv current-source "")

  (defn pnix-error [message]
    (raise (RuntimeError message)))

  (defclass PnixCatchableError [RuntimeError])

  (defn pnix-catchable-error [message]
    (raise (PnixCatchableError message)))

  (defn digit? [c]
    (in c "0123456789"))

  (defn ident-start? [c]
    (cond
      (.isalpha c) True
      (= c "_") True
      True False))

  (defn ident-char? [c]
    (cond
      (.isalnum c) True
      (in c "_-'") True
      True False))

  (defn uri-ascii-alpha? [c]
    (in c "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"))

  (defn uri-scheme-char? [c]
    (or (uri-ascii-alpha? c) (digit? c) (in c "+-.")))

  (defn uri-body-char? [c]
    (or (uri-ascii-alpha? c) (digit? c) (in c "%/?:@&=+$,-_.!~*'")))

  (defn scan-uri-scheme-end [source i]
    (if (and (< i (len source)) (uri-scheme-char? (get source i)))
      (scan-uri-scheme-end source (+ i 1))
      i))

  (defn scan-uri-body-end [source i]
    (if (and (< i (len source)) (uri-body-char? (get source i)))
      (scan-uri-body-end source (+ i 1))
      i))

  (defn uri-end [source start]
    (if (and (< start (len source)) (uri-ascii-alpha? (get source start)))
      (do
        (setv colon (scan-uri-scheme-end source (+ start 1)))
        (if (and (< colon (len source)) (= (get source colon) ":"))
          (do
            (setv body-start (+ colon 1))
            (setv end (scan-uri-body-end source body-start))
            (if (> end body-start) end None))
          None))
      None))

  (defn interp-open-at? [source i c]
    (if (= c "$")
      (if (< (+ i 1) (len source))
        (= (get source (+ i 1)) "{")
        False)
      False))

	  (defn two-op-at? [source i]
	    (if (< (+ i 1) (len source))
	      (in (cut source i (+ i 2)) ["&&" "||" "==" "!=" "<=" ">=" "//" "++" "->" "=>" "${"])
	      False))

	  (defn three-op-at? [source i]
	    (if (< (+ i 2) (len source))
	      (= (cut source i (+ i 3)) "...")
	      False))

	  (defn keyword? [word]
	    (in word ["let" "in" "rec" "if" "then" "else" "true" "false" "null" "import" "with" "assert" "match" "inherit" "or"]))

  (defn token [kind value pos]
    {"kind" kind "value" value "pos" pos})

  (defn skip-comment [source i]
    (if (>= i (len source))
      i
      (if (= (get source i) "\n")
        i
        (skip-comment source (+ i 1)))))

  (defn block-comment-at? [source i]
    (if (< (+ i 1) (len source))
      (= (cut source i (+ i 2)) "/*")
      False))

  (defn skip-block-comment [source i]
    (if (>= (+ i 1) (len source))
      (pnix-error "unterminated block comment")
      (if (= (cut source i (+ i 2)) "*/")
        (+ i 2)
        (skip-block-comment source (+ i 1)))))

  (defn indented-string-start? [source i]
    (if (< (+ i 1) (len source))
      (= (cut source i (+ i 2)) "''")
      False))

  (defn indented-quote-escape-at? [source i]
    (if (< (+ i 2) (len source))
      (and (indented-string-start? source i) (= (get source (+ i 2)) "'"))
      False))

  (defn indented-dollar-escape-at? [source i]
    (if (< (+ i 2) (len source))
      (= (cut source i (+ i 3)) "''$")
      False))

  (defn indented-backslash-escape-at? [source i]
    (if (< (+ i 2) (len source))
      (and (= (cut source i (+ i 2)) "''") (= (get source (+ i 2)) "\\"))
      False))

  (defn skip-indented-string-body [source i]
    (if (>= i (len source))
      (pnix-error "unterminated indented string literal")
      (cond
        (indented-quote-escape-at? source i) (skip-indented-string-body source (+ i 3))
        (indented-dollar-escape-at? source i) (skip-indented-string-body source (+ i 3))
        (indented-backslash-escape-at? source i)
          (skip-indented-string-body source (if (< (+ i 3) (len source)) (+ i 4) (+ i 3)))
        (indented-string-start? source i) (+ i 2)
        True (skip-indented-string-body source (+ i 1)))))

  (defn skip-indented-string-in-source [source i]
    (skip-indented-string-body source (+ i 2)))

  (defn skip-string-in-source [source i]
    (if (>= i (len source))
      (pnix-error "unterminated string literal")
      (do
        (setv c (get source i))
        (cond
          (= c "\\") (skip-string-in-source source (+ i 2))
          (= c "\"") (+ i 1)
          True (skip-string-in-source source (+ i 1))))))

  (defn scan-interp-body [source start i depth]
    (if (>= i (len source))
      (pnix-error "unterminated interpolation ${...} in string literal")
      (do
        (setv c (get source i))
        (cond
          (= c "{") (scan-interp-body source start (+ i 1) (+ depth 1))
          (= c "}")
            (if (= depth 1)
              [(cut source start i) (+ i 1)]
              (scan-interp-body source start (+ i 1) (- depth 1)))
          (= c "\"") (scan-interp-body source start (skip-string-in-source source (+ i 1)) depth)
          (indented-string-start? source i) (scan-interp-body source start (skip-indented-string-in-source source i) depth)
          (= c "#") (scan-interp-body source start (skip-comment source i) depth)
          (block-comment-at? source i) (scan-interp-body source start (skip-block-comment source (+ i 2)) depth)
          True (scan-interp-body source start (+ i 1) depth)))))

  (defn append-lit-part [parts buf]
    (if (= (len buf) 0)
      parts
      (+ parts [{"lit" buf}])))

  (defn escape-char [c]
    (cond
      (= c "n") "\n"
      (= c "r") "\r"
      (= c "t") "\t"
      (= c "\"") "\""
      (= c "\\") "\\"
      (= c "$") "$"
      True c))

  (defn indented-escape-char [c]
    (cond
      (= c "n") "\n"
      (= c "r") "\r"
      (= c "t") "\t"
      (= c "\\") "\\"
      True (+ "\\" c)))

  (defn string-lines-for-indent [raw]
    (do
      (setv lines (.split raw "\n"))
      (if (.endswith raw "\n")
        (cut lines 0 -1)
        lines)))

  (defn whitespace-only? [line]
    (if (= (len line) 0)
      True
      (if (.isspace (get line 0))
        (whitespace-only? (cut line 1 None))
        False)))

  (defn count-indent [line i]
    (if (< i (len line))
      (if (or (= (get line i) " ") (= (get line i) "\t"))
        (count-indent line (+ i 1))
        i)
      i))

  (defn min-indent-lines [lines current]
    (if (= (len lines) 0)
      (if (= current None) 0 current)
      (do
        (setv line (get lines 0))
        (if (whitespace-only? line)
          (min-indent-lines (cut lines 1 None) current)
          (do
            (setv indent (count-indent line 0))
            (min-indent-lines
              (cut lines 1 None)
              (if (= current None) indent (if (< indent current) indent current))))))))

  (defn common-indentation [raw]
    (min-indent-lines (string-lines-for-indent raw) None))

  (defn strip-indent-line [line min-indent i]
    (if (and (< i min-indent) (< i (len line)) (or (= (get line i) " ") (= (get line i) "\t")))
      (strip-indent-line line min-indent (+ i 1))
      (cut line i None)))

  (defn strip-indented-string-loop [lines min-indent first out]
    (if (= (len lines) 0)
      out
      (do
        (setv line (get lines 0))
        (if (and first (= line ""))
          (strip-indented-string-loop (cut lines 1 None) min-indent False out)
          (do
            (setv stripped (strip-indent-line line min-indent 0))
            (strip-indented-string-loop
              (cut lines 1 None)
              min-indent
              False
              (if (= out "") stripped (+ out (+ "\n" stripped)))))))))

  (defn strip-indented-string [raw]
    (do
      (setv stripped (strip-indented-string-loop (string-lines-for-indent raw) (common-indentation raw) True ""))
      (if (.endswith raw "\n")
        (+ stripped "\n")
        stripped)))

  (defn combine-string-parts [parts]
    (if (= (len parts) 0)
      ""
      (do
        (setv part (get parts 0))
        (+ (if (in "lit" part) (get part "lit") "${}")
           (combine-string-parts (cut parts 1 None))))))

  (defn strip-lit-chars [s i min-indent is-first at-line-start chars-stripped out]
    (if (>= i (len s))
      [out is-first at-line-start chars-stripped]
      (do
        (setv ch (get s i))
        (cond
          (= ch "\n")
            (if (and is-first (= out ""))
              (strip-lit-chars s (+ i 1) min-indent False True 0 out)
              (strip-lit-chars s (+ i 1) min-indent False True 0 (+ out "\n")))
          (and at-line-start (or (= ch " ") (= ch "\t")) (< chars-stripped min-indent))
            (strip-lit-chars s (+ i 1) min-indent is-first True (+ chars-stripped 1) out)
          True
            (strip-lit-chars s (+ i 1) min-indent False False chars-stripped (+ out ch))))))

  (defn strip-indented-parts-loop [parts min-indent is-first at-line-start chars-stripped result]
    (if (= (len parts) 0)
      result
      (do
        (setv part (get parts 0))
        (if (in "lit" part)
          (do
            (setv stripped (strip-lit-chars (get part "lit") 0 min-indent is-first at-line-start chars-stripped ""))
            (setv text (get stripped 0))
            (strip-indented-parts-loop
              (cut parts 1 None)
              min-indent
              (get stripped 1)
              (get stripped 2)
              (get stripped 3)
              (if (or (!= text "") (= (len result) 0)) (+ result [{"lit" text}]) result)))
          (strip-indented-parts-loop
            (cut parts 1 None)
            min-indent
            False
            False
            0
            (+ result [part]))))))

  (defn strip-indented-string-parts [parts]
    (strip-indented-parts-loop parts (common-indentation (combine-string-parts parts)) True True 0 []))

  (defn read-indented-backslash-escape [source i]
    (if (< (+ i 3) (len source))
      [(indented-escape-char (get source (+ i 3))) (+ i 4)]
      ["\\" (+ i 3)]))

  (defn read-indented-string-token [source start i buf parts]
    (if (>= i (len source))
      (pnix-error "unterminated indented string literal")
      (cond
        (indented-quote-escape-at? source i)
          (read-indented-string-token source start (+ i 3) (+ buf "''") parts)
        (indented-dollar-escape-at? source i)
          (read-indented-string-token source start (+ i 3) (+ buf "$") parts)
        (indented-backslash-escape-at? source i)
          (do
            (setv escaped (read-indented-backslash-escape source i))
            (read-indented-string-token source start (get escaped 1) (+ buf (get escaped 0)) parts))
        (indented-string-start? source i)
          (if (> (len parts) 0)
            [(token "string_interp" (strip-indented-string-parts (append-lit-part parts buf)) start) (+ i 2)]
            [(token "string" (strip-indented-string buf) start) (+ i 2)])
        (interp-open-at? source i (get source i))
          (do
            (setv scanned (scan-interp-body source (+ i 2) (+ i 2) 1))
            (setv raw (get scanned 0))
            (setv end (get scanned 1))
            (read-indented-string-token
              source
              start
              end
              ""
              (+ (append-lit-part parts buf) [{"expr" (parse-source raw)}])))
        True
          (read-indented-string-token source start (+ i 1) (+ buf (get source i)) parts))))

  (defn read-string-token [source start i buf parts]
    (if (>= i (len source))
      (pnix-error "unterminated string literal")
      (do
        (setv c (get source i))
        (cond
          (= c "\"")
            (if (> (len parts) 0)
              [(token "string_interp" (append-lit-part parts buf) start) (+ i 1)]
              [(token "string" buf start) (+ i 1)])
          (= c "\\")
            (if (>= (+ i 1) (len source))
              (pnix-error "unterminated string escape")
              (read-string-token source start (+ i 2) (+ buf (escape-char (get source (+ i 1)))) parts))
          (interp-open-at? source i c)
            (do
              (setv scanned (scan-interp-body source (+ i 2) (+ i 2) 1))
              (setv raw (get scanned 0))
              (setv end (get scanned 1))
              (read-string-token
                source
                start
                end
                ""
                (+ (append-lit-part parts buf) [{"expr" (parse-source raw)}])))
          True
            (read-string-token source start (+ i 1) (+ buf c) parts)))))

  (defn scan-digits [source i]
    (if (< i (len source))
      (if (digit? (get source i))
        (scan-digits source (+ i 1))
        i)
      i))

  (defn scan-number [source i]
    (do
      (setv leading-dot (= (get source i) "."))
      (setv j (scan-digits source (+ i 1)))
      (setv is-float leading-dot)
      (if (and (not leading-dot) (< j (len source)) (= (get source j) "."))
        (cond
          (and (< (+ j 1) (len source)) (digit? (get source (+ j 1))))
            (if (and (> (- j i) 1) (= (get source i) "0"))
              None
              (do
              (setv is-float True)
                (setv j (scan-digits source (+ j 2)))))
          (and (< (+ j 1) (len source)) (in (get source (+ j 1)) "eE"))
            (if (and (> (- j i) 1) (= (get source i) "0"))
              None
              (do
              (setv is-float True)
                (setv j (+ j 1))))
          True None)
        None)
      (if (and is-float (< j (len source)) (in (get source j) "eE"))
        (do
          (setv e (+ j 1))
          (if (and (< e (len source)) (in (get source e) "+-"))
            (setv e (+ e 1))
            None)
          (setv after-exp (scan-digits source e))
          (if (= after-exp e)
            (pnix-error "invalid numeric exponent")
            (setv j after-exp)))
        None)
      j))

  (defn float-lexeme? [lexeme]
    (or (in "." lexeme) (in "e" lexeme) (in "E" lexeme)))

  (defn nonzero-mantissa-digit? [lexeme i]
    (cond
      (>= i (len lexeme)) False
      (in (get lexeme i) "eE") False
      (in (get lexeme i) "123456789") True
      True (nonzero-mantissa-digit? lexeme (+ i 1))))

  (defn float-lexeme-value [lexeme]
    (do
      (setv value (float lexeme))
      (if (or (not (math.isfinite value))
              (and (= value 0.0) (nonzero-mantissa-digit? lexeme 0))
              (and (!= value 0.0) (< (abs value) 2.2250738585072014e-308)))
        (pnix-error (+ (+ "invalid float '" lexeme) "'"))
        value)))

  (defn scan-ident [source i]
    (if (< i (len source))
      (if (ident-char? (get source i))
        (scan-ident source (+ i 1))
        i)
      i))

  (defn rel-path-start? [source i]
    (if (= (get source i) ".")
      (if (< (+ i 1) (len source))
        (or (= (get source (+ i 1)) "/")
            (if (< (+ i 2) (len source))
              (and (= (get source (+ i 1)) ".") (= (get source (+ i 2)) "/"))
              False))
        False)
      False))

  (defn path-char? [c]
    (or (.isalnum c) (in c "/._-+~")))

  (defn abs-path-start? [source i]
    (and
      (= (get source i) "/")
      (< (+ i 1) (len source))
      (!= (get source (+ i 1)) "/")
      (!= (get source (+ i 1)) "*")
      (or (path-char? (get source (+ i 1)))
          (interp-open-at? source (+ i 1) (get source (+ i 1))))))

  (defn home-path-start? [source i]
    (and (= (get source i) "~") (< (+ i 1) (len source)) (= (get source (+ i 1)) "/")))

  (defn home-alone? [source i]
    (and
      (= (get source i) "~")
      (or (= (+ i 1) (len source))
          (.isspace (get source (+ i 1)))
          (in (get source (+ i 1)) "{}[]();\""))))

  (defn search-path-start? [source i]
    (and (= (get source i) "<") (< (+ i 1) (len source)) (ident-start? (get source (+ i 1)))))

  (defn append-path-lit [parts buf]
    (if (= buf "") parts (+ parts [{"lit" buf}])))

  (defn finish-path-token [start i buf parts]
    (do
      (setv all-parts (append-path-lit parts buf))
      (if (= (len all-parts) 0)
        (pnix-error "empty path")
        (if (> (len parts) 0)
          [(token "path_interp" all-parts start) i]
          [(token "path" buf start) i]))))

  (defn read-path-token [source start i buf parts]
    (if (>= i (len source))
      (finish-path-token start i buf parts)
      (do
        (setv c (get source i))
        (cond
          (interp-open-at? source i c)
            (do
              (setv scanned (scan-interp-body source (+ i 2) (+ i 2) 1))
              (setv raw (get scanned 0))
              (setv end (get scanned 1))
              (read-path-token
                source
                start
                end
                ""
                (+ (append-path-lit parts buf) [{"expr" (parse-source raw)}])))
          (path-char? c)
            (read-path-token source start (+ i 1) (+ buf c) parts)
          True
            (finish-path-token start i buf parts)))))

  (defn scan-search-path [source i]
    (if (>= i (len source))
      (pnix-error "unterminated search path")
      (if (= (get source i) ">")
        i
        (scan-search-path source (+ i 1)))))

  (defn read-search-path-token [source start]
    (do
      (setv end (scan-search-path source (+ start 1)))
      (setv body (cut source (+ start 1) end))
      (if (= body "")
        (pnix-error "empty search path")
        [(token "path" (+ (+ "<" body) ">") start) (+ end 1)])))

  (defn tokenize-at [source i tokens]
    (if (>= i (len source))
      tokens
      (do
        (setv c (get source i))
        (cond
          (.isspace c) (tokenize-at source (+ i 1) tokens)
          (= c "#") (tokenize-at source (skip-comment source i) tokens)
          (block-comment-at? source i) (tokenize-at source (skip-block-comment source (+ i 2)) tokens)
          (= c "\"")
            (do
              (setv string-pair (read-string-token source i (+ i 1) "" []))
              (tokenize-at source (get string-pair 1) (+ tokens [(get string-pair 0)])))
          (indented-string-start? source i)
            (do
              (setv string-pair (read-indented-string-token source i (+ i 2) "" []))
              (tokenize-at source (get string-pair 1) (+ tokens [(get string-pair 0)])))
          (or (digit? c)
              (and (= c ".") (< (+ i 1) (len source)) (digit? (get source (+ i 1)))))
            (do
              (setv digit-end (if (digit? c) (scan-digits source (+ i 1)) i))
              (if (and (digit? c)
                       (> (- digit-end i) 1)
                       (= c "0")
                       (< (+ digit-end 1) (len source))
                       (= (get source digit-end) ".")
                       (digit? (get source (+ digit-end 1))))
                (do
                  (setv end (scan-number source digit-end))
                  (setv prefix (cut source i digit-end))
                  (setv suffix (cut source digit-end end))
                  (tokenize-at source end
                    (+ tokens [(token "number_application"
                                      [(int prefix) (float-lexeme-value suffix)] i)])))
                (do
                  (setv end (scan-number source i))
                  (setv lexeme (cut source i end))
                  (tokenize-at source end (+ tokens [(token "number" (if (float-lexeme? lexeme) (float-lexeme-value lexeme) (int lexeme)) i)])))))
          (ident-start? c)
            (do
              (setv uri-stop (uri-end source i))
              (if (!= uri-stop None)
                (tokenize-at source uri-stop (+ tokens [(token "uri" (cut source i uri-stop) i)]))
                (do
                  (setv end (scan-ident source (+ i 1)))
                  (setv word (cut source i end))
                  (tokenize-at source end (+ tokens [(token (if (keyword? word) "kw" "ident") word i)])))))
          (rel-path-start? source i)
            (do
              (setv path-pair (read-path-token source i i "" []))
              (tokenize-at source (get path-pair 1) (+ tokens [(get path-pair 0)])))
          (home-path-start? source i)
            (do
              (setv path-pair (read-path-token source i i "" []))
              (tokenize-at source (get path-pair 1) (+ tokens [(get path-pair 0)])))
          (home-alone? source i)
            (tokenize-at source (+ i 1) (+ tokens [(token "path" "~" i)]))
          (abs-path-start? source i)
            (do
              (setv path-pair (read-path-token source i i "" []))
              (tokenize-at source (get path-pair 1) (+ tokens [(get path-pair 0)])))
          (search-path-start? source i)
            (do
              (setv path-pair (read-search-path-token source i))
              (tokenize-at source (get path-pair 1) (+ tokens [(get path-pair 0)])))
          (three-op-at? source i)
            (tokenize-at source (+ i 3) (+ tokens [(token "sym" "..." i)]))
          (two-op-at? source i)
            (tokenize-at source (+ i 2) (+ tokens [(token "sym" (cut source i (+ i 2)) i)]))
          (in c "{}[]();=.:,+-*/%!<>?|@")
            (tokenize-at source (+ i 1) (+ tokens [(token "sym" c i)]))
          True
            (pnix-error (+ "unexpected character `" (+ c "`")))))))

  (defn tokenize-source [source]
    (tokenize-at source 0 []))

  (defn tok [tokens pos]
    (if (< pos (len tokens))
      (get tokens pos)
      (token "eof" "" pos)))

  (defn tok-is [tokens pos kind value]
    (do
      (setv t (tok tokens pos))
      (if (= (get t "kind") kind)
        (= (get t "value") value)
        False)))

  (defn token-end [t]
    (+ (get t "pos") (len (str (get t "value")))))

  (defn adjacent? [tokens left-pos right-pos]
    (if (>= left-pos 0)
      (= (token-end (tok tokens left-pos)) (get (tok tokens right-pos) "pos"))
      False))

  (defn expect-token [tokens pos kind value]
    (if (tok-is tokens pos kind value)
      (+ pos 1)
      (pnix-error (+ "expected token `" (+ value "`")))))

	  (defn attr-token? [t]
	    (in (get t "kind") ["ident" "string" "kw"]))

	  (defn parse-path-segment [tokens pos]
	    (do
	      (setv t (tok tokens pos))
	      (cond
	        (tok-is tokens pos "sym" "${")
	          (do
	            (setv expr-pair (parse-expr tokens (+ pos 1)))
	            [{"expr" (get expr-pair 0)} None (expect-token tokens (get expr-pair 1) "sym" "}")])
	        (attr-token? t) [(str (get t "value")) (get t "pos") (+ pos 1)]
	        True (pnix-error "expected attribute name"))))

	  (defn parse-path-tail [tokens pos parts positions]
	    (if (tok-is tokens pos "sym" ".")
	      (do
	        (setv segment-pair (parse-path-segment tokens (+ pos 1)))
	        (parse-path-tail
	          tokens
	          (get segment-pair 2)
	          (+ parts [(get segment-pair 0)])
	          (+ positions [(get segment-pair 1)])))
	      [parts positions pos]))

	  (defn parse-path [tokens pos]
	    (do
	      (setv first-pair (parse-path-segment tokens pos))
	      (parse-path-tail
	        tokens
	        (get first-pair 2)
	        [(get first-pair 0)]
	        [(get first-pair 1)])))

	  (defn binding-path-static? [path]
	    (if (= (len path) 0)
	      True
	      (and (isinstance (get path 0) str) (binding-path-static? (cut path 1 None)))))

	  (defn parse-inherit-name [tokens pos]
	    (do
	      (setv t (tok tokens pos))
	      (cond
	        (attr-token? t) [(str (get t "value")) (+ pos 1) (get t "pos")]
	        (tok-is tokens pos "sym" "${")
	          (do
	            (setv name-token (tok tokens (+ pos 1)))
	            (if (= (get name-token "kind") "string")
	              [(str (get name-token "value")) (expect-token tokens (+ pos 2) "sym" "}") (get name-token "pos")]
	              (pnix-error "expected string literal inside `${...}` inherit name")))
	        True [None pos None])))

	  (defn inherit-value-node [scope name]
	    (if (= scope None)
	      {"tag" "var" "name" name}
	      {"tag" "select" "base" scope "attr" name}))

	  (defn inherit-binding-node [scope name name-pos]
	    (do
	      (setv binding {"path" [name] "path_positions" [name-pos] "value" (inherit-value-node scope name)})
	      (if (= scope None)
	        (setv (get binding "inherit_plain") True)
	        None)
	      binding))

	  (defn parse-inherit-bindings-names [tokens pos scope bindings]
	    (if (tok-is tokens pos "sym" ";")
	      (if (= (len bindings) 0)
	        (pnix-error "inherit requires at least one name")
	        [bindings (+ pos 1)])
	      (do
	        (if (= (get (tok tokens pos) "kind") "eof")
	          (pnix-error "expected `;` before end of inherit clause")
	          None)
	        (setv name-pair (parse-inherit-name tokens pos))
	        (setv name (get name-pair 0))
	        (if (= name None)
	          (pnix-error "expected inherited name")
	          (parse-inherit-bindings-names
	            tokens
	            (get name-pair 1)
	            scope
	            (+ bindings [(inherit-binding-node scope name (get name-pair 2))]))))))

	  (defn parse-inherit-bindings [tokens pos]
	    (do
	      (setv after-inherit (expect-token tokens pos "kw" "inherit"))
	      (if (tok-is tokens after-inherit "sym" "(")
	        (do
	          (setv scope-pair (parse-expr tokens (+ after-inherit 1)))
	          (parse-inherit-bindings-names
	            tokens
	            (expect-token tokens (get scope-pair 1) "sym" ")")
	            (get scope-pair 0)
	            []))
	        (parse-inherit-bindings-names tokens after-inherit None []))))

  (defn parse-bindings-until [tokens pos end-kind end-value mode bindings]
    (if (tok-is tokens pos end-kind end-value)
      [bindings (+ pos 1)]
      (do
        (if (= (get (tok tokens pos) "kind") "eof")
          (pnix-error (+ "expected `" (+ end-value "` before end of input")))
          None)
        (if (tok-is tokens pos "kw" "inherit")
          (do
            (setv inherited-pair (parse-inherit-bindings tokens pos))
            (parse-bindings-until
              tokens
              (get inherited-pair 1)
              end-kind
              end-value
              mode
              (+ bindings (get inherited-pair 0))))
          (do
        (setv path-pair (parse-path tokens pos))
        (setv path (get path-pair 0))
        (setv path-positions (get path-pair 1))
        (if (and (= mode "let") (not (binding-path-static? path)))
          (pnix-error "dynamic attributes not allowed in let")
          None)
        (setv after-path (expect-token tokens (get path-pair 2) "sym" "="))
        (setv value-pair (parse-expr tokens after-path))
        (setv after-value (expect-token tokens (get value-pair 1) "sym" ";"))
        (parse-bindings-until
          tokens
          after-value
          end-kind
          end-value
          mode
          (+ bindings [{"path" path "path_positions" path-positions "value" (get value-pair 0)}])))))))

  (defn parse-let [tokens pos]
    (do
      (setv bindings-pair (parse-bindings-until tokens (expect-token tokens pos "kw" "let") "kw" "in" "let" []))
      (setv body-pair (parse-expr tokens (get bindings-pair 1)))
      [{"tag" "let" "bindings" (get bindings-pair 0) "body" (get body-pair 0)} (get body-pair 1)]))

  (defn parse-if [tokens pos]
    (do
      (setv cond-pair (parse-expr tokens (expect-token tokens pos "kw" "if")))
      (setv then-pos (expect-token tokens (get cond-pair 1) "kw" "then"))
      (setv then-pair (parse-expr tokens then-pos))
      (setv else-pos (expect-token tokens (get then-pair 1) "kw" "else"))
      (setv else-pair (parse-expr tokens else-pos))
      [{"tag" "if" "cond" (get cond-pair 0) "then" (get then-pair 0) "else" (get else-pair 0)}
       (get else-pair 1)]))

  (defn parse-with [tokens pos]
    (do
      (setv env-pair (parse-expr tokens (expect-token tokens pos "kw" "with")))
      (setv body-pair (parse-expr tokens (expect-token tokens (get env-pair 1) "sym" ";")))
      [{"tag" "with" "env" (get env-pair 0) "body" (get body-pair 0)}
       (get body-pair 1)]))

  (defn parse-assert [tokens pos]
    (do
      (setv cond-pair (parse-expr tokens (expect-token tokens pos "kw" "assert")))
      (setv body-pair (parse-expr tokens (expect-token tokens (get cond-pair 1) "sym" ";")))
      [{"tag" "assert" "cond" (get cond-pair 0) "body" (get body-pair 0)}
       (get body-pair 1)]))

	  (defn parse-import [tokens pos]
	    (do
	      (setv path-token (tok tokens (expect-token tokens pos "kw" "import")))
	      (if (= (get path-token "kind") "path")
	        [{"tag" "import" "path" (str (get path-token "value"))} (+ pos 2)]
	        (pnix-error "import expects a relative path literal like `./file.px`"))))

	  (defn construct-head? [tokens pos]
	    (do
	      (setv t (tok tokens pos))
	      (if (= (get t "kind") "ident")
	        (if (> (len (str (get t "value"))) 0)
	          (and (.isupper (get (str (get t "value")) 0))
	               (tok-is tokens (+ pos 1) "sym" "(")
	               (adjacent? tokens pos (+ pos 1)))
	          False)
	        False)))

	  (defn parse-construct-args [tokens pos args]
	    (if (tok-is tokens pos "sym" ")")
	      [args (+ pos 1)]
	      (do
	        (if (= (get (tok tokens pos) "kind") "eof")
	          (pnix-error "expected `)` before end of constructor")
	          None)
	        (setv arg-pair (parse-expr tokens pos))
	        (setv next-pos (get arg-pair 1))
	        (if (tok-is tokens next-pos "sym" ",")
	          (setv next-pos (+ next-pos 1))
	          None)
	        (parse-construct-args tokens next-pos (+ args [(get arg-pair 0)])))))

	  (defn parse-construct [tokens pos]
	    (do
	      (setv head (tok tokens pos))
	      (setv args-pair (parse-construct-args tokens (expect-token tokens (+ pos 1) "sym" "(") []))
	      [{"tag" "construct" "variant" (str (get head "value")) "args" (get args-pair 0)}
	       (get args-pair 1)]))

	  (defn parse-list-items [tokens pos items]
	    (if (tok-is tokens pos "sym" "]")
	      [{"tag" "list" "items" items} (+ pos 1)]
      (do
        (if (= (get (tok tokens pos) "kind") "eof")
          (pnix-error "expected `]` before end of input")
          None)
        (setv item-pair
          (if (and (= (get (tok tokens pos) "kind") "sym")
                   (in (get (tok tokens pos) "value") ["!" "-"]))
            (parse-unary tokens pos)
            (parse-postfix tokens pos)))
        (setv next-pos (get item-pair 1))
        (if (tok-is tokens next-pos "sym" ",")
          (setv next-pos (+ next-pos 1))
          None)
        (parse-list-items tokens next-pos (+ items [(get item-pair 0)])))))

  (defn parse-list-form [tokens pos]
    (parse-list-items tokens (expect-token tokens pos "sym" "[") []))

	  (defn parse-attrset [tokens pos recursive]
	    (do
	      (setv body-pos (if recursive
	                       (expect-token tokens (expect-token tokens pos "kw" "rec") "sym" "{")
	                       (expect-token tokens pos "sym" "{")))
	      (setv bindings-pair (parse-bindings-until tokens body-pos "sym" "}" "attr" []))
	      [{"tag" "attrset" "recursive" recursive "bindings" (get bindings-pair 0)}
	       (get bindings-pair 1)]))

	  (defn parse-dynamic-attr-segment [tokens pos]
	    (do
	      (setv t (tok tokens pos))
	      (cond
	        (tok-is tokens pos "sym" "${")
	          (do
	            (setv expr-pair (parse-expr tokens (+ pos 1)))
	            [{"expr" (get expr-pair 0)}
	             (expect-token tokens (get expr-pair 1) "sym" "}")])
	        (attr-token? t)
	          [{"lit" (str (get t "value")) "quoted" (= (get t "kind") "string")} (+ pos 1)]
	        True
	          (pnix-error "expected attribute path segment"))))

	  (defn parse-attr-segments-tail [tokens pos segments]
	    (if (tok-is tokens pos "sym" ".")
	      (do
	        (setv segment-pair (parse-dynamic-attr-segment tokens (+ pos 1)))
	        (parse-attr-segments-tail tokens (get segment-pair 1) (+ segments [(get segment-pair 0)])))
	      [segments pos]))

	  (defn parse-attr-segments [tokens pos]
	    (do
	      (setv first-pair (parse-dynamic-attr-segment tokens pos))
	      (parse-attr-segments-tail tokens (get first-pair 1) [(get first-pair 0)])))

	  (defn static-segments-rec [segments i acc]
	    (if (>= i (len segments))
	      acc
	      (if (in "expr" (get segments i))
	        None
	        (static-segments-rec
	          segments
	          (+ i 1)
	          (+ acc (if (> i 0) "." "") (str (get (get segments i) "lit")))))))

	  (defn static-segments [segments]
	    (static-segments-rec segments 0 ""))

	  (defn static-segment-path-rec [segments i]
	    (if (>= i (len segments))
	      []
	      (+ [(str (get (get segments i) "lit"))]
	         (static-segment-path-rec segments (+ i 1)))))

	  (defn static-segment-path [segments]
	    (static-segment-path-rec segments 0))

	  (defn parse-pattern-list-after-rest [tokens pos items rest]
	    (do
	      (if (tok-is tokens pos "sym" ",")
	        (setv pos (+ pos 1))
	        None)
	      (if (tok-is tokens pos "sym" "]")
	        [{"tag" "list" "items" items "rest" rest} (+ pos 1)]
	        (pnix-error "expected `]` after list pattern rest"))))

	  (defn parse-pattern-list-items [tokens pos items]
	    (if (tok-is tokens pos "sym" "]")
	      [{"tag" "list" "items" items "rest" None} (+ pos 1)]
	      (do
	        (if (= (get (tok tokens pos) "kind") "eof")
	          (pnix-error "expected `]` before end of list pattern")
	          None)
	        (if (tok-is tokens pos "sym" "...")
	          (if (= (get (tok tokens (+ pos 1)) "kind") "ident")
	            (parse-pattern-list-after-rest tokens (+ pos 2) items (str (get (tok tokens (+ pos 1)) "value")))
	            (pnix-error "expected identifier after list pattern `...`"))
	          (do
	            (setv item-pair (parse-pattern tokens pos))
	            (setv next-pos (get item-pair 1))
	            (if (tok-is tokens next-pos "sym" ",")
	              (setv next-pos (+ next-pos 1))
	              None)
	            (parse-pattern-list-items tokens next-pos (+ items [(get item-pair 0)])))))))

	  (defn parse-pattern-list [tokens pos]
	    (parse-pattern-list-items tokens (expect-token tokens pos "sym" "[") []))

	  (defn parse-pattern-attr-fields [tokens pos fields ellipsis]
	    (if (tok-is tokens pos "sym" "}")
	      [{"tag" "attrset" "fields" fields "ellipsis" ellipsis} (+ pos 1)]
	      (do
	        (if (tok-is tokens pos "sym" "...")
	          (do
	            (setv next-pos (+ pos 1))
	            (if (or (tok-is tokens next-pos "sym" ",") (tok-is tokens next-pos "sym" ";"))
	              (setv next-pos (+ next-pos 1))
	              None)
	            (parse-pattern-attr-fields tokens next-pos fields True))
	          (do
	            (setv t (tok tokens pos))
	            (if (not (attr-token? t))
	              (pnix-error "expected attribute name in pattern")
	              None)
	            (setv name (str (get t "value")))
	            (setv after-name (+ pos 1))
	            (setv pattern-pair
	              (cond
	                (tok-is tokens after-name "sym" "=")
	                  (do
	                    (setv explicit-pair (parse-pattern tokens (+ after-name 1)))
	                    [(get explicit-pair 0) (get explicit-pair 1) None])
	                (tok-is tokens after-name "sym" "?")
	                  (do
	                    (setv default-pair (parse-expr tokens (+ after-name 1)))
	                    [{"tag" "var" "name" name} (get default-pair 1) (get default-pair 0)])
	                True
	                  [{"tag" "var" "name" name} after-name None]))
	            (setv next-pos (get pattern-pair 1))
	            (if (or (tok-is tokens next-pos "sym" ",") (tok-is tokens next-pos "sym" ";"))
	              (setv next-pos (+ next-pos 1))
	              None)
	            (setv field {"name" name "pattern" (get pattern-pair 0)})
	            (if (!= (get pattern-pair 2) None)
	              (setv (get field "default") (get pattern-pair 2))
	              None)
	            (parse-pattern-attr-fields
	              tokens
	              next-pos
	              (+ fields [field])
	              ellipsis))))))

	  (defn parse-pattern-attrset [tokens pos]
	    (parse-pattern-attr-fields tokens (expect-token tokens pos "sym" "{") [] False))

	  (defn parse-pattern-construct-args [tokens pos args]
	    (if (tok-is tokens pos "sym" ")")
	      [args (+ pos 1)]
	      (do
	        (if (= (get (tok tokens pos) "kind") "eof")
	          (pnix-error "expected `)` before end of constructor pattern")
	          None)
	        (setv arg-pair (parse-pattern tokens pos))
	        (setv next-pos (get arg-pair 1))
	        (if (tok-is tokens next-pos "sym" ",")
	          (setv next-pos (+ next-pos 1))
	          None)
	        (parse-pattern-construct-args tokens next-pos (+ args [(get arg-pair 0)])))))

	  (defn parse-pattern-construct [tokens pos]
	    (do
	      (setv head (tok tokens pos))
	      (setv args-pair (parse-pattern-construct-args tokens (expect-token tokens (+ pos 1) "sym" "(") []))
	      [{"tag" "constructor" "variant" (str (get head "value")) "args" (get args-pair 0)}
	       (get args-pair 1)]))

	  (defn parse-pattern-atom [tokens pos]
	    (do
	      (setv t (tok tokens pos))
	      (setv kind (get t "kind"))
	      (setv value (get t "value"))
	      (cond
	        (= kind "number") [{"tag" "literal" "value" value} (+ pos 1)]
	        (= kind "string") [{"tag" "literal" "value" value} (+ pos 1)]
	        (= kind "ident")
	          (cond
	            (tok-is tokens (+ pos 1) "sym" "@")
	              (do
	                (setv inner-pair (parse-pattern tokens (+ pos 2)))
	                [{"tag" "as" "name" (str value) "pattern" (get inner-pair 0)}
	                 (get inner-pair 1)])
	            (= value "_") [{"tag" "wildcard"} (+ pos 1)]
	            (construct-head? tokens pos) (parse-pattern-construct tokens pos)
	            True [{"tag" "var" "name" (str value)} (+ pos 1)])
	        (= kind "kw")
	          (cond
	            (= value "true") [{"tag" "literal" "value" True} (+ pos 1)]
	            (= value "false") [{"tag" "literal" "value" False} (+ pos 1)]
	            (= value "null") [{"tag" "literal" "value" None} (+ pos 1)]
	            True (pnix-error "unexpected keyword in pattern"))
	        (= kind "sym")
	          (cond
	            (= value "[") (parse-pattern-list tokens pos)
	            (= value "{") (parse-pattern-attrset tokens pos)
	            True (pnix-error "unexpected symbol in pattern"))
	        True (pnix-error "unexpected match pattern token"))))

	  (defn parse-pattern-as-tail [tokens pair]
	    (if (tok-is tokens (get pair 1) "sym" "@")
	      (do
	        (setv name-token (tok tokens (+ (get pair 1) 1)))
	        (if (!= (get name-token "kind") "ident")
	          (pnix-error "expected identifier after pattern @")
	          [{"tag" "as" "name" (str (get name-token "value")) "pattern" (get pair 0)}
	           (+ (get pair 1) 2)]))
	      pair))

	  (defn parse-pattern [tokens pos]
	    (parse-pattern-as-tail tokens (parse-pattern-atom tokens pos)))

	  (defn parse-match-arms [tokens pos arms]
	    (if (tok-is tokens pos "sym" "|")
	      (do
	        (setv pattern-pair (parse-pattern tokens (+ pos 1)))
	        (setv guard None)
	        (setv after-pattern (get pattern-pair 1))
	        (if (tok-is tokens after-pattern "kw" "if")
	          (do
	            (setv guard-pair (parse-expr tokens (+ after-pattern 1)))
	            (setv guard (get guard-pair 0))
	            (setv after-pattern (get guard-pair 1)))
	          None)
	        (setv body-pair (parse-expr tokens (expect-token tokens after-pattern "sym" "=>")))
	        (setv arm {"pattern" (get pattern-pair 0) "body" (get body-pair 0)})
	        (if (!= guard None)
	          (setv (get arm "guard") guard)
	          None)
	        (parse-match-arms
	          tokens
	          (get body-pair 1)
	          (+ arms [arm])))
	      [arms pos]))

	  (defn parse-match [tokens pos]
	    (do
	      (setv scrut-pair (parse-expr tokens (expect-token tokens pos "kw" "match")))
	      (setv arms-pair (parse-match-arms tokens (expect-token tokens (get scrut-pair 1) "kw" "with") []))
	      (if (= (len (get arms-pair 0)) 0)
	        (pnix-error "match requires at least one arm")
	        [{"tag" "match" "scrutinee" (get scrut-pair 0) "arms" (get arms-pair 0)}
	         (get arms-pair 1)])))

	  (defn parse-primary [tokens pos]
	    (do
	      (setv t (tok tokens pos))
      (setv kind (get t "kind"))
      (setv value (get t "value"))
      (cond
        (= kind "number") [(if (isinstance value float)
                             {"tag" "float" "value" value}
                             {"tag" "int" "value" value})
                           (+ pos 1)]
        (= kind "number_application")
          [{"tag" "apply"
            "func" {"tag" "int" "value" (get value 0)}
            "arg" {"tag" "float" "value" (get value 1)}}
           (+ pos 1)]
        (= kind "path") [{"tag" "path" "value" value} (+ pos 1)]
        (= kind "path_interp") [{"tag" "path_interp" "parts" value} (+ pos 1)]
	        (= kind "string") [{"tag" "string" "value" value} (+ pos 1)]
	        (= kind "uri") [{"tag" "string" "value" value} (+ pos 1)]
	        (= kind "string_interp") [{"tag" "str_interp" "parts" value} (+ pos 1)]
	        (= kind "ident")
	          (if (construct-head? tokens pos)
	            (parse-construct tokens pos)
	            [{"tag" "var" "name" value "pos" (get t "pos")} (+ pos 1)])
        (= kind "kw")
          (cond
            (= value "true") [{"tag" "bool" "value" True} (+ pos 1)]
            (= value "false") [{"tag" "bool" "value" False} (+ pos 1)]
            (= value "null") [{"tag" "null"} (+ pos 1)]
            (= value "let") (parse-let tokens pos)
            (= value "if") (parse-if tokens pos)
            (= value "rec") (parse-attrset tokens pos True)
	            (= value "import") (parse-import tokens pos)
	            (= value "with") (parse-with tokens pos)
	            (= value "assert") (parse-assert tokens pos)
	            (= value "match") (parse-match tokens pos)
	            True (pnix-error "unexpected keyword"))
        (= kind "sym")
          (cond
            (= value "(")
              (do
                (setv expr-pair (parse-expr tokens (+ pos 1)))
                [(get expr-pair 0) (expect-token tokens (get expr-pair 1) "sym" ")")])
            (= value "[") (parse-list-form tokens pos)
            (= value "{") (parse-attrset tokens pos False)
            True (pnix-error "unexpected symbol"))
        True (pnix-error "unexpected token"))))

  (defn make-select-node [node next-token]
    {"tag" "select" "base" node "attr" (str (get next-token "value"))})

  (defn parse-selected-default-tail [tokens selected default-pair]
    (parse-postfix-tail
      tokens
      {"tag" "select_default"
       "base" (get selected "base")
       "attr" (get selected "attr")
       "default" (get default-pair 0)}
      (get default-pair 1)))

  (defn parse-selected-tail [tokens selected after-pos]
    (if (tok-is tokens after-pos "kw" "or")
      (parse-selected-default-tail tokens selected (parse-expr tokens (+ after-pos 1)))
      (parse-postfix-tail tokens selected after-pos)))

	  (defn parse-dot-postfix [tokens node pos next-token]
	    (cond
	      (tok-is tokens (+ pos 1) "sym" "${")
	        (parse-dynamic-selected-tail tokens node (parse-attr-segments tokens (+ pos 1)))
	      (attr-token? next-token)
	        (parse-selected-tail tokens (make-select-node node next-token) (+ pos 2))
	      True
	        (pnix-error "expected selector after `.`")))

	  (defn parse-dynamic-selected-default-tail [tokens node segments default-pair]
	    (parse-postfix-tail
	      tokens
	      {"tag" "dynamic_select_default"
	       "base" node
	       "segments" segments
	       "default" (get default-pair 0)}
	      (get default-pair 1)))

	  (defn parse-dynamic-selected-tail [tokens node segments-pair]
	    (do
	      (setv segments (get segments-pair 0))
	      (setv after-pos (get segments-pair 1))
	      (if (tok-is tokens after-pos "kw" "or")
	        (parse-dynamic-selected-default-tail tokens node segments (parse-expr tokens (+ after-pos 1)))
	        (parse-postfix-tail
	          tokens
	          {"tag" "dynamic_select" "base" node "segments" segments}
	          after-pos))))

	  (defn parse-hasattr-postfix [tokens node pos next-token]
	    (if (or (attr-token? next-token) (tok-is tokens (+ pos 1) "sym" "${"))
	      (parse-hasattr-segments-tail tokens node (parse-attr-segments tokens (+ pos 1)))
	      (pnix-error "expected attribute name after `?`")))

	  (defn parse-hasattr-segments-tail [tokens node segments-pair]
	    (do
	      (setv segments (get segments-pair 0))
	      (setv static-name (static-segments segments))
	      (if (= static-name None)
	        (parse-hasattr-tail
	          tokens
	          {"tag" "dynamic_has_attr" "base" node "segments" segments}
	          (get segments-pair 1))
	        (parse-hasattr-tail
	          tokens
	          {"tag" "has_attr" "base" node "attr" static-name "path" (static-segment-path segments)}
	          (get segments-pair 1)))))

  (defn parse-index-postfix-tail [tokens node index-pair]
    (parse-postfix-tail
      tokens
      {"tag" "index" "base" node "index" (get index-pair 0)}
      (expect-token tokens (get index-pair 1) "sym" "]")))

  (defn parse-index-postfix [tokens node pos]
    (parse-index-postfix-tail tokens node (parse-expr tokens (+ pos 1))))

  (defn parse-postfix-tail [tokens node pos]
    (cond
      (tok-is tokens pos "sym" ".")
        (parse-dot-postfix tokens node pos (tok tokens (+ pos 1)))
      (and (tok-is tokens pos "sym" "[") (adjacent? tokens (- pos 1) pos))
        (parse-index-postfix tokens node pos)
      True [node pos]))

  (defn parse-postfix [tokens pos]
    (do
      (setv pair (parse-primary tokens pos))
      (parse-postfix-tail tokens (get pair 0) (get pair 1))))

  (defn primary-start? [t]
    (do
      (setv kind (get t "kind"))
      (setv value (get t "value"))
      (cond
	        (in kind ["number" "number_application" "path" "path_interp" "string" "string_interp" "uri" "ident"]) True
	        (= kind "kw") (in value ["true" "false" "null" "let" "if" "rec" "import"])
	        (= kind "sym") (in value ["(" "[" "{"])
	        True False)))

	  (defn parse-apply-tail [tokens node pos]
	    (if (in (get node "tag") ["var" "select" "select_default" "dynamic_select" "dynamic_select_default" "index" "apply" "lambda" "import" "with" "assert" "match" "construct"])
      (if (primary-start? (tok tokens pos))
        (do
          (setv arg-pair (parse-postfix tokens pos))
          (parse-apply-tail
            tokens
            {"tag" "apply" "func" node "arg" (get arg-pair 0)}
            (get arg-pair 1)))
        [node pos])
      [node pos]))

  (defn parse-apply [tokens pos]
    (do
      (setv pair (parse-postfix tokens pos))
      (parse-apply-tail tokens (get pair 0) (get pair 1))))

  (defn parse-hasattr-tail [tokens node pos]
    (if (tok-is tokens pos "sym" "?")
      (parse-hasattr-postfix tokens node pos (tok tokens (+ pos 1)))
      [node pos]))

  (defn parse-hasattr [tokens pos]
    (do
      (setv pair (parse-apply tokens pos))
      (parse-hasattr-tail tokens (get pair 0) (get pair 1))))

  (defn parse-unary [tokens pos]
    (if (= (get (tok tokens pos) "kind") "sym")
      (if (in (get (tok tokens pos) "value") ["!" "-"])
        (do
          (setv arg-pair (parse-unary tokens (+ pos 1)))
          [{"tag" "unary" "op" (get (tok tokens pos) "value") "arg" (get arg-pair 0)} (get arg-pair 1)])
        (parse-hasattr tokens pos))
      (parse-hasattr tokens pos)))

  (defn parse-left-tail [tokens node pos sub-parser ops]
    (if (= (get (tok tokens pos) "kind") "sym")
      (if (in (get (tok tokens pos) "value") ops)
        (do
          (setv op (get (tok tokens pos) "value"))
          (setv rhs-pair (sub-parser tokens (+ pos 1)))
          (parse-left-tail
            tokens
            {"tag" "binary" "op" op "lhs" node "rhs" (get rhs-pair 0)}
            (get rhs-pair 1)
            sub-parser
            ops))
        [node pos])
      [node pos]))

  (defn parse-left [tokens pos sub-parser ops]
    (do
      (setv first-pair (sub-parser tokens pos))
      (parse-left-tail tokens (get first-pair 0) (get first-pair 1) sub-parser ops)))

  (defn parse-mul [tokens pos]
    (parse-left tokens pos parse-unary ["*" "/" "%"]))

  (defn parse-add [tokens pos]
    (parse-left tokens pos parse-mul ["+" "-" "++"]))

  (defn make-merge-pair [left-pair right-pair]
    [{"tag" "binary" "op" "//" "lhs" (get left-pair 0) "rhs" (get right-pair 0)}
     (get right-pair 1)])

  (defn parse-merge-tail [tokens left-pair]
    (if (tok-is tokens (get left-pair 1) "sym" "//")
      (make-merge-pair left-pair (parse-merge tokens (+ (get left-pair 1) 1)))
      left-pair))

  (defn parse-merge [tokens pos]
    (parse-merge-tail tokens (parse-add tokens pos)))

  (defn parse-compare [tokens pos]
    (parse-left tokens pos parse-merge ["<" "<=" ">" ">="]))

  (defn parse-eq [tokens pos]
    (parse-left tokens pos parse-compare ["==" "!="]))

  (defn parse-and [tokens pos]
    (parse-left tokens pos parse-eq ["&&"]))

  (defn parse-or [tokens pos]
    (parse-left tokens pos parse-and ["||"]))

  (defn make-impl-pair [lhs-pair rhs-pair]
    [{"tag" "binary" "op" "->" "lhs" (get lhs-pair 0) "rhs" (get rhs-pair 0)}
     (get rhs-pair 1)])

  (defn parse-impl-tail [tokens lhs-pair]
    (if (tok-is tokens (get lhs-pair 1) "sym" "->")
      (make-impl-pair lhs-pair (parse-impl tokens (+ (get lhs-pair 1) 1)))
      lhs-pair))

  (defn parse-impl [tokens pos]
    (parse-impl-tail tokens (parse-or tokens pos)))

	  (defn balanced-open? [value]
	    (in value ["{" "[" "("]))

	  (defn balanced-close? [value]
	    (in value ["}" "]" ")"]))

	  (defn scan-balanced-end [tokens pos depth]
	    (do
	      (setv t (tok tokens pos))
	      (if (= (get t "kind") "eof")
	        None
	        (if (= (get t "kind") "sym")
	          (cond
	            (balanced-open? (get t "value"))
	              (scan-balanced-end tokens (+ pos 1) (+ depth 1))
	            (balanced-close? (get t "value"))
	              (if (= depth 1)
	                (+ pos 1)
	                (scan-balanced-end tokens (+ pos 1) (- depth 1)))
	            True (scan-balanced-end tokens (+ pos 1) depth))
	          (scan-balanced-end tokens (+ pos 1) depth)))))

	  (defn pattern-start-after-balanced [tokens after]
	    (cond
	      (= after None) None
	      (tok-is tokens after "sym" ":") after
	      (and (tok-is tokens after "sym" "@")
	           (= (get (tok tokens (+ after 1)) "kind") "ident")
	           (tok-is tokens (+ after 2) "sym" ":")) (+ after 2)
	      True None))

	  (defn pattern-start-after-parsed [tokens parsed]
	    (if (tok-is tokens (get parsed 1) "sym" ":")
	      (get parsed 1)
	      None))

	  (defn pattern-start-colon-pos-with-token [tokens pos kind value]
	    (cond
	      (and (= kind "sym") (in value ["{" "["]))
	        (pattern-start-after-balanced tokens (scan-balanced-end tokens pos 0))
	      (and (= kind "ident") (tok-is tokens (+ pos 1) "sym" "@"))
	        (pattern-start-after-parsed tokens (parse-pattern tokens pos))
	      True None))

	  (defn pattern-start-colon-pos [tokens pos]
	    (pattern-start-colon-pos-with-token
	      tokens
	      pos
	      (get (tok tokens pos) "kind")
	      (get (tok tokens pos) "value")))

	  (defn parse-pattern-lambda-head [tokens pos colon-pos]
	    (do
	      (setv pattern-pair (parse-pattern tokens pos))
	      [{"pattern" (get pattern-pair 0)} (+ colon-pos 1)]))

	  (defn parse-lambda-head-colon [tokens pos colon-pos]
	    (if (= colon-pos None)
	      None
	      (parse-pattern-lambda-head tokens pos colon-pos)))

	  (defn parse-lambda-head [tokens pos]
	    (if (= (get (tok tokens pos) "kind") "ident")
	      (if (tok-is tokens (+ pos 1) "sym" ":")
	        [{"param" (str (get (tok tokens pos) "value"))} (+ pos 2)]
	        (parse-lambda-head-colon tokens pos (pattern-start-colon-pos tokens pos)))
	      (parse-lambda-head-colon tokens pos (pattern-start-colon-pos tokens pos))))

	  (defn parse-lambda-body-pair [tokens head-pair]
	    (parse-expr tokens (get head-pair 1)))

	  (defn make-lambda-from-body [head-pair body-pair]
	    (if (in "param" (get head-pair 0))
	      [{"tag" "lambda" "param" (get (get head-pair 0) "param") "body" (get body-pair 0)}
	       (get body-pair 1)]
	      [{"tag" "lambda" "param" None "pattern" (get (get head-pair 0) "pattern") "body" (get body-pair 0)}
	       (get body-pair 1)]))

	  (defn parse-expr-with-head [tokens pos head-pair]
	    (if (= head-pair None)
	      (parse-impl tokens pos)
	      (make-lambda-from-body head-pair (parse-lambda-body-pair tokens head-pair))))

	  (defn parse-expr [tokens pos]
	    (parse-expr-with-head tokens pos (parse-lambda-head tokens pos)))

  (defn parse-source [source]
    (do
      (setv tokens (tokenize-source source))
      (setv parsed (parse-expr tokens 0))
      (if (!= (get parsed 1) (len tokens))
        (pnix-error "unexpected trailing tokens")
        (get parsed 0))))

  (defn parse-source-list [sources]
    (if (= (len sources) 0)
      []
      (+ [(parse-source (get sources 0))]
         (parse-source-list (cut sources 1 None)))))

  (defn thunk? [value]
    (and (isinstance value dict) (.get value "__pnix_thunk__" False)))

  (defn closure? [value]
    (and (isinstance value dict) (.get value "__pnix_closure__" False)))

  (defn make-native-lazy [func]
    {"__pnix_native_lazy__" True "func" func})

  (defn native-lazy? [value]
    (and (isinstance value dict) (.get value "__pnix_native_lazy__" False)))

	  (defn normalize-pnix-path-text [text]
	    (do
	      (setv text (str text))
	      (if (and (.startswith text "<") (.endswith text ">"))
	        text
	        (do
	          (setv absolute (.startswith text "/"))
	          (setv started-dot (or (= text ".") (.startswith text "./")))
	          (setv out [])
	          (for [part (.split text "/")]
	            (cond
	              (or (= part "") (= part ".")) None
	              (= part "..")
	                (if (and (> (len out) 0) (!= (get out -1) ".."))
	                  (.pop out)
	                  (if absolute None (.append out part)))
	              True (.append out part)))
	          (cond
	            absolute (if (= (len out) 0) "/" (+ "/" (.join "/" out)))
	            (= (len out) 0) "."
	            (and started-dot (!= (get out 0) "..")) (+ "./" (.join "/" out))
	            True (.join "/" out))))))

	  (defn make-path [value]
	    {"__pnix_path__" True "value" (normalize-pnix-path-text value)})

	  (defn path? [value]
	    (and (isinstance value dict) (.get value "__pnix_path__" False)))

  (defclass PnixString [builtins.str]
    (defn __new__ [cls text context]
      (do
        (setv obj (builtins.str.__new__ cls (str text)))
        (setv obj.context (set context))
        obj)))

  (defn string-context [value]
    (do
      (setv value (force-value value))
      (if (isinstance value PnixString)
        (set value.context)
        (set []))))

  (defn make-context-string [text context]
    (do
      (setv ctx (set context))
      (if (= (len ctx) 0)
        (str text)
        (PnixString (str text) ctx))))

  (defn merge-contexts [left right]
    (do
      (setv out (set left))
      (.update out right)
      out))

  (defn string-text-context [value label]
    (do
      (setv value (force-value value))
      (if (isinstance value str)
        [(str value) (string-context value)]
        (pnix-error (+ label (+ " (" (+ (type-of value) ") must be a string")))))))

	  (defn make-construct [variant args]
	    {"__pnix_construct__" True "variant" variant "args" args})

	  (defn construct? [value]
	    (and (isinstance value dict) (.get value "__pnix_construct__" False)))

  (defn make-thunk [func]
    {"__pnix_thunk__" True
     "kind" "func"
     "func" func
     "node" None
     "env" None
     "raw" None
     "forced" False
     "forcing" False
     "value" None})

  (defn make-node-thunk [node env]
    {"__pnix_thunk__" True
     "kind" "node"
     "func" None
     "node" node
     "env" env
     "raw" None
     "forced" False
     "forcing" False
     "value" None})

  (defn make-value-thunk [raw-value]
    {"__pnix_thunk__" True
     "kind" "value"
     "func" None
     "node" None
     "env" None
     "raw" raw-value
     "forced" False
     "forcing" False
     "value" None})

  (defn make-closure [param body env pattern]
    {"__pnix_closure__" True
     "param" param
     "body" body
     "env" env
     "pattern" pattern})

  (defn force-value [value]
    (if (thunk? value)
      (if (get value "forced")
        (get value "value")
        (do
          (if (get value "forcing")
            (pnix-error "infinite recursion encountered (recursive value forced itself)")
            None)
          (setv (get value "forcing") True)
          (setv result
                (try
                  (if (= (get value "kind") "node")
                    (eval-ast (get value "node") (get value "env"))
                    (if (= (get value "kind") "value")
                      (force-value (get value "raw"))
                      ((get value "func"))))
                  (except [Exception exc]
                    (setv (get value "forcing") False)
                    (raise exc))))
          (setv (get value "value") result)
          (setv (get value "forced") True)
          (setv (get value "forcing") False)
          result))
      value))

	  (defn realize-value [value]
	    (setv value (force-value value))
	    (cond
	      (path? value) (get value "value")
	      (construct? value) {"variant" (get value "variant") "args" (realize-list (get value "args"))}
	      (closure? value) "#<pnix-hy-closure>"
	      (callable value) "#<pnix-hy-native>"
      (isinstance value dict)
        (realize-dict value (sorted (.keys value)))
	      (isinstance value list) (realize-list value)
	      True value))

  (defn json-ready-cycle-check [value seen]
    (do
      (setv oid (builtins.id value))
      (if (in oid seen)
        (pnix-error "builtins.toJSON: infinite recursion encountered (cyclic value)")
        None)
      (.add seen oid)
      oid))

  (defn json-ready-value [value [seen None]]
    (if (= seen None) (setv seen (set [])) None)
    (setv value (force-value value))
    (cond
      (path? value) (get value "value")
      (construct? value)
        (do
          (setv oid (json-ready-cycle-check value seen))
          (setv result {"variant" (get value "variant") "args" (json-ready-list (get value "args") seen)})
          (.discard seen oid)
          result)
      (or (closure? value) (native-lazy? value) (callable value))
        (pnix-error "cannot serialize function as JSON")
      (and (isinstance value float) (not (math.isfinite value)))
        (pnix-error (+ "cannot serialize float " (+ (if (math.isnan value) "NaN" (if (> value 0) "+inf" "-inf")) " as JSON")))
      (isinstance value dict)
        (do
          (setv oid (json-ready-cycle-check value seen))
          (setv result (json-ready-dict value (sorted (.keys value)) seen))
          (.discard seen oid)
          result)
      (isinstance value list)
        (do
          (setv oid (json-ready-cycle-check value seen))
          (setv result (json-ready-list value seen))
          (.discard seen oid)
          result)
      True value))

  (defn json-ready-dict [value keys seen]
    (if (= (len keys) 0)
      {}
      (do
        (setv key (get keys 0))
        (setv out (json-ready-dict value (cut keys 1 None) seen))
        (setv (get out key) (json-ready-value (get value key) seen))
        out)))

  (defn json-ready-list [items seen]
    (if (= (len items) 0)
      []
      (+ [(json-ready-value (get items 0) seen)]
         (json-ready-list (cut items 1 None) seen))))

  (defn realize-dict [value keys]
    (if (= (len keys) 0)
      {}
      (do
        (setv key (get keys 0))
        (setv out (realize-dict value (cut keys 1 None)))
        (setv (get out key) (realize-value (get value key)))
        out)))

  (defn realize-list [items]
    (if (= (len items) 0)
      []
      (+ [(realize-value (get items 0))]
         (realize-list (cut items 1 None)))))

  (defn toml-to-pnix-value [value]
    (cond
      (isinstance value dict) (toml-to-pnix-dict value (sorted (.keys value)))
      (isinstance value list) (toml-to-pnix-list value)
      (or (isinstance value str) (isinstance value int)
          (isinstance value float) (isinstance value bool)) value
      True (str value)))

  (defn toml-to-pnix-dict [value keys]
    (if (= (len keys) 0)
      {}
      (do
        (setv key (get keys 0))
        (setv out (toml-to-pnix-dict value (cut keys 1 None)))
        (setv (get out key) (toml-to-pnix-value (get value key)))
        out)))

  (defn toml-to-pnix-list [items]
    (if (= (len items) 0)
      []
      (+ [(toml-to-pnix-value (get items 0))]
         (toml-to-pnix-list (cut items 1 None)))))

  (defn from-toml-builtin [value]
    (try
      (toml-to-pnix-value (tomllib.loads (expected-string-value value "builtins.fromTOML")))
      (except [tomllib.TOMLDecodeError exc]
        (pnix-error (+ "builtins.fromTOML: parse error: " (str exc))))))

  (defn nonempty-text? [value]
    (and (!= value None) (!= value "")))

  (defn element-attrs-from-keys [attrs keys]
    (if (= (len keys) 0)
      {}
      (do
        (setv key (get keys 0))
        (setv out (element-attrs-from-keys attrs (cut keys 1 None)))
        (setv (get out key) (str (get attrs key)))
        out)))

  (defn element-children-from-list [items i out]
    (if (>= i (len items))
      out
      (do
        (setv child (get items i))
        (setv with-child (+ out [(element-to-markup-node child)]))
        (element-children-from-list
          items
          (+ i 1)
          (if (nonempty-text? (getattr child "tail"))
            (+ with-child [{"kind" "text" "value" (getattr child "tail")}])
            with-child)))))

  (defn element-to-markup-node [element]
    (do
      (setv children
            (if (nonempty-text? (getattr element "text"))
              [{"kind" "text" "value" (getattr element "text")}]
              []))
      {"kind" "element"
       "name" (str (getattr element "tag"))
       "attrs" (element-attrs-from-keys (getattr element "attrib") (sorted (.keys (getattr element "attrib"))))
       "children" (element-children-from-list (list element) 0 children)}))

  (defn xml-parse-builtin [value]
    (element-to-markup-node
      (xml.etree.ElementTree.fromstring (string-arg-value value "xmlParse"))))

  (defn html-parse-children [wrapper items i out]
    (if (>= i (len items))
      out
      (do
        (setv child (get items i))
        (setv with-child (+ out [(element-to-markup-node child)]))
        (html-parse-children
          wrapper
          items
          (+ i 1)
          (if (nonempty-text? (getattr child "tail"))
            (+ with-child [{"kind" "text" "value" (getattr child "tail")}])
            with-child)))))

  (defn html-parse-builtin [value]
    (do
      (setv wrapper
            (xml.etree.ElementTree.fromstring
              (+ "<pnix-hy-document>" (string-arg-value value "htmlParse") "</pnix-hy-document>")))
      (setv children
            (if (nonempty-text? (getattr wrapper "text"))
              [{"kind" "text" "value" (getattr wrapper "text")}]
              []))
      {"kind" "document" "children" (html-parse-children wrapper (list wrapper) 0 children)}))

  (defn markup-escape [value attr-mode]
    (do
      (setv escaped (.replace (.replace (.replace value "&" "&amp;") "<" "&lt;") ">" "&gt;"))
      (if attr-mode (.replace escaped "\"" "&quot;") escaped)))

  (defn markup-scalar-string [value label]
    (do
      (setv value (force-value value))
      (cond
        (= value None) ""
        (isinstance value bool) (if value "true" "false")
        (or (isinstance value int) (isinstance value float)) (str value)
        (path? value) (get value "value")
        (isinstance value str) value
        True (pnix-error (+ label " must be string-compatible")))))

  (defn markup-children [node label]
    (if (in "children" node)
      (list-value (get node "children") label)
      []))

  (defn markup-attrs-from-map [attrs keys html-mode]
    (if (= (len keys) 0)
      []
      (do
        (setv key (get keys 0))
        (+ [[(if html-mode (.lower (str key)) (str key))
             (markup-scalar-string (get attrs key) "markup attr value")]]
           (markup-attrs-from-map attrs (cut keys 1 None) html-mode)))))

  (defn markup-attrs-from-list [items html-mode]
    (if (= (len items) 0)
      []
      (do
        (setv item (attrset-value (get items 0) "markup attr"))
        (if (not (in "name" item))
          (pnix-error "markup attr missing name")
          None)
        (if (not (in "value" item))
          (pnix-error "markup attr missing value")
          None)
        (+ [[(if html-mode
               (.lower (string-value (get item "name") "markup attr name"))
               (string-value (get item "name") "markup attr name"))
             (markup-scalar-string (get item "value") "markup attr value")]]
           (markup-attrs-from-list (cut items 1 None) html-mode)))))

  (defn markup-attrs [value html-mode]
    (do
      (setv value (force-value value))
      (cond
        (= value None) []
        (isinstance value dict) (sorted (markup-attrs-from-map value (sorted (.keys value)) html-mode))
        (isinstance value list) (sorted (markup-attrs-from-list value html-mode))
        True (pnix-error "markup attrs must be attrset or list"))))

  (defn markup-attrs-string [attrs]
    (if (= (len attrs) 0)
      ""
      (+ " " (get (get attrs 0) 0) "=\"" (markup-escape (get (get attrs 0) 1) True) "\""
         (markup-attrs-string (cut attrs 1 None)))))

  (setv HTML-VOID-ELEMENTS
    ["area" "base" "br" "col" "embed" "hr" "img" "input" "link" "meta" "param" "source" "track" "wbr"])

  (defn markup-emit-children [children html-mode]
    (if (= (len children) 0)
      ""
      (+ (markup-emit-node (get children 0) html-mode)
         (markup-emit-children (cut children 1 None) html-mode))))

  (defn markup-emit-element [node html-mode]
    (do
      (setv name (string-value (get node "name") "markup element name"))
      (if html-mode (setv name (.lower name)) None)
      (setv attr-text (markup-attrs-string (markup-attrs (.get node "attrs" {}) html-mode)))
      (setv children (markup-children node "markup element children"))
      (cond
        html-mode
          (if (in name HTML-VOID-ELEMENTS)
            (+ "<" name attr-text ">")
            (+ "<" name attr-text ">" (markup-emit-children children True) "</" name ">"))
        (= (len children) 0) (+ "<" name attr-text "/>")
        True (+ "<" name attr-text ">" (markup-emit-children children False) "</" name ">"))))

  (defn markup-emit-node [value html-mode]
    (do
      (setv node (attrset-value value "markup node"))
      (setv kind (string-value (get node "kind") "markup kind"))
      (cond
        (= kind "document") (markup-emit-children (markup-children node "markup document children") html-mode)
        (= kind "text") (markup-escape (markup-scalar-string (.get node "value" (.get node "text" "")) "markup text") False)
        (= kind "comment") (+ "<!--" (markup-scalar-string (.get node "value" "") "markup comment") "-->")
        (and (= kind "cdata") (not html-mode))
          (+ "<![CDATA[" (markup-scalar-string (.get node "value" (.get node "text" "")) "markup cdata") "]]>")
        (or (= kind "element") (in "name" node)) (markup-emit-element node html-mode)
        True (pnix-error (+ "markup.emit: unknown kind `" kind "`")))))

  (defn markup-emit-list [items html-mode]
    (if (= (len items) 0)
      ""
      (+ (markup-emit-node (get items 0) html-mode)
         (markup-emit-list (cut items 1 None) html-mode))))

  (defn markup-emit-builtin [value html-mode]
    (do
      (setv value (force-value value))
      (setv text
        (if (isinstance value list)
          (markup-emit-list value html-mode)
          (markup-emit-node value html-mode)))
      (make-context-string text (collect-json-context value))))

  (defn schema-root-source [source]
    (cond
      (in "kind" source) source
      (in "root" source) (attrset-value (get source "root") "schema.root")
      True (pnix-error "schema.validate: schema must define kind or root")))

  (defn schema-root-value [schema]
    (schema-root-source (attrset-value schema "schema")))

  (defn schema-kind [schema]
    (string-value (get schema "kind") "schema kind"))

  (defn schema-type-name-for-value [value]
    (cond
      (= value None) "null"
      (isinstance value bool) "bool"
      (and (isinstance value int) (not (isinstance value bool))) "int"
      (isinstance value float) "float"
      (isinstance value str) "string"
      (isinstance value list) "list"
      (isinstance value dict) "set"
      True (str (type value))))

  (defn schema-type-name [value]
    (schema-type-name-for-value (force-value value)))

  (defn schema-error [path code message]
    {"path" path "code" code "message" message})

  (defn schema-optional-items [schema]
    (if (in "optional" schema)
      (list-value (get schema "optional") "schema optional")
      []))

  (defn schema-optional-contains [items field]
    (if (= (len items) 0)
      False
      (if (= (string-value (get items 0) "schema optional item") field)
        True
        (schema-optional-contains (cut items 1 None) field))))

  (defn schema-validate-list-items [schema items i path]
    (if (>= i (len items))
      []
      (+ (schema-validate-errors schema (get items i) (+ path [(str i)]))
         (schema-validate-list-items schema items (+ i 1) path))))

  (defn schema-validate-record-field-with-map [field-schema field-map field optional value path rest]
    (cond
      (in field value)
        (+ (schema-validate-errors field-schema (get value field) (+ path [field])) rest)
      (or (schema-optional-contains optional field) (in "default" field-map)) rest
      True (+ [(schema-error (+ path [field]) "missing" (+ "missing required field " field))] rest)))

  (defn schema-validate-record-field [fields field optional value path rest]
    (schema-validate-record-field-with-map
      (get fields field)
      (attrset-value (get fields field) "schema field")
      field
      optional
      value
      path
      rest))

  (defn schema-validate-record-fields [fields keys optional value path]
    (if (= (len keys) 0)
      []
      (schema-validate-record-field
        fields
        (get keys 0)
        optional
        value
        path
        (schema-validate-record-fields fields (cut keys 1 None) optional value path))))

  (defn schema-validate-string [schema-map value path]
    (if (isinstance value str)
      (if (and (in "minLength" schema-map)
               (< (len value) (number-value (get schema-map "minLength") "schema minLength")))
        [(schema-error path "constraint" "expected min length")]
        [])
      [(schema-error path "type" (+ "expected string, got " (schema-type-name value)))]))

  (defn schema-validate-bool [value path]
    (if (isinstance value bool)
      []
      [(schema-error path "type" (+ "expected bool, got " (schema-type-name value)))]))

  (defn schema-validate-int [value path]
    (if (and (isinstance value int) (not (isinstance value bool)))
      []
      [(schema-error path "type" (+ "expected int, got " (schema-type-name value)))]))

  (defn schema-validate-number [value path]
    (if (and (or (isinstance value int) (isinstance value float)) (not (isinstance value bool)))
      []
      [(schema-error path "type" (+ "expected number, got " (schema-type-name value)))]))

  (defn schema-validate-list [schema-map value path]
    (if (isinstance value list)
      (if (in "elem" schema-map) (schema-validate-list-items (get schema-map "elem") value 0 path) [])
      [(schema-error path "type" (+ "expected list, got " (schema-type-name value)))]))

  (defn schema-validate-attrs [value path]
    (if (isinstance value dict)
      []
      [(schema-error path "type" (+ "expected set, got " (schema-type-name value)))]))

  (defn schema-validate-record-with-fields [schema-map fields value path]
    (schema-validate-record-fields fields (sorted (.keys fields)) (schema-optional-items schema-map) value path))

  (defn schema-validate-record [schema-map value path]
    (if (isinstance value dict)
      (schema-validate-record-with-fields
        schema-map
        (attrset-value (get schema-map "fields") "schema record fields")
        value
        path)
      [(schema-error path "type" (+ "expected record, got " (schema-type-name value)))]))

  (defn schema-validate-errors-for-kind [schema-map kind value path]
    (cond
      (= kind "any") []
      (= kind "string") (schema-validate-string schema-map value path)
      (= kind "bool") (schema-validate-bool value path)
      (= kind "int") (schema-validate-int value path)
      (in kind ["float" "number"]) (schema-validate-number value path)
      (= kind "list") (schema-validate-list schema-map value path)
      (in kind ["attrs" "map"]) (schema-validate-attrs value path)
      (= kind "record") (schema-validate-record schema-map value path)
      True [(schema-error path "schema" (+ "unsupported schema kind " kind))]))

  (defn schema-validate-errors-for-map [schema-map value path]
    (schema-validate-errors-for-kind schema-map (schema-kind schema-map) (force-value value) path))

  (defn schema-validate-errors [schema value path]
    (schema-validate-errors-for-map (schema-root-value schema) value path))

  (defn schema-normalize-record-field-with-map [field-schema field-map fields field rest-keys out]
    (do
      (if (in field out)
        (setv (get out field) (schema-normalize-value field-schema (get out field)))
        (if (in "default" field-map)
          (setv (get out field) (schema-normalize-value field-schema (get field-map "default")))
          None))
      (schema-normalize-record-fields fields rest-keys out)))

  (defn schema-normalize-record-field [fields field rest-keys out]
    (schema-normalize-record-field-with-map
      (get fields field)
      (attrset-value (get fields field) "schema field")
      fields
      field
      rest-keys
      out))

  (defn schema-normalize-record-fields [fields keys out]
    (if (= (len keys) 0)
      out
      (schema-normalize-record-field fields (get keys 0) (cut keys 1 None) out)))

  (defn schema-normalize-record-with-fields [fields source]
    (schema-normalize-record-fields fields (sorted (.keys fields)) (dict source)))

  (defn schema-normalize-record-with-source [schema-map source]
    (schema-normalize-record-with-fields
      (attrset-value (get schema-map "fields") "schema record fields")
      source))

  (defn schema-normalize-record [schema-map value]
    (schema-normalize-record-with-source schema-map (attrset-value value "schemaNormalize value")))

  (defn schema-normalize-value-for-map [schema-map value]
    (if (= (schema-kind schema-map) "record")
      (schema-normalize-record schema-map value)
      value))

  (defn schema-normalize-value [schema value]
    (schema-normalize-value-for-map (schema-root-value schema) (force-value value)))

  (defn schema-validate-result [errors]
    {"success" (= (len errors) 0) "ok" (= (len errors) 0) "errors" errors})

  (defn schema-validate-builtin [schema value]
    (schema-validate-result (schema-validate-errors schema value ["root"])))

  (defn schema-path-string [path]
    (if (= (len path) 0)
      ""
      (if (= (len path) 1)
        (get path 0)
        (+ (get path 0) "." (schema-path-string (cut path 1 None))))))

  (defn schema-explain-error-line [error]
    (+ (schema-path-string (get error "path")) ": " (get error "code") ": " (get error "message")))

  (defn schema-explain-errors [errors]
    (if (= (len errors) 0)
      ""
      (if (= (len errors) 1)
        (schema-explain-error-line (get errors 0))
        (+ (schema-explain-error-line (get errors 0)) "\n" (schema-explain-errors (cut errors 1 None))))))

  (defn schema-explain-builtin [schema value]
    (schema-explain-errors (schema-validate-errors schema value ["root"])))

  ;; canonical JSON serializer: mirrors Python
  ;; json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
  ;; using only stage7-supported primitives (index recursion, string concat).
  (defn cj-escape-char [c]
    (do
      (setv code (ord c))
      (cond
        (= c "\\") "\\\\"
        (= c "\"") "\\\""
        (= c "\n") "\\n"
        (= c "\r") "\\r"
        (= c "\t") "\\t"
        (= code 8) "\\b"
        (= code 12) "\\f"
        (< code 32) (+ "\\u" (format code "04x"))
        True c)))

  (defn cj-escape [s i acc]
    (if (>= i (len s))
      acc
      (cj-escape s (+ i 1) (+ acc (cj-escape-char (get s i))))))

  (defn cj-string [s]
    (+ "\"" (cj-escape s 0 "") "\""))

  (defn cj-items [items i acc]
    (if (>= i (len items))
      acc
      (cj-items items (+ i 1)
        (+ acc (if (= i 0) "" ",") (canonical-json (get items i))))))

  (defn cj-pairs [value keys i acc]
    (if (>= i (len keys))
      acc
      (do
        (setv key (get keys i))
        (cj-pairs value keys (+ i 1)
          (+ acc (if (= i 0) "" ",") (cj-string key) ":"
             (canonical-json (get value key)))))))

  (defn canonical-json [value]
    (cond
      (= value None) "null"
      (isinstance value bool) (if value "true" "false")
      (and (isinstance value int) (not (isinstance value bool))) (str value)
      (isinstance value str) (cj-string value)
      (isinstance value list) (+ "[" (cj-items value 0 "") "]")
      (isinstance value dict) (+ "{" (cj-pairs value (sorted (.keys value)) 0 "") "}")
      True (cj-string (str value))))

  (defn collect-json-context [value]
    (do
      (setv value (force-value value))
      (cond
        (isinstance value PnixString) (string-context value)
        (path? value) (set [(get value "value")])
        (construct? value)
          (collect-json-context-list (get value "args") 0 (set []))
        (isinstance value list)
          (collect-json-context-list value 0 (set []))
        (isinstance value dict)
          (collect-json-context-dict value (list (.keys value)) 0 (set []))
        True (set []))))

  (defn collect-json-context-list [items i out]
    (if (>= i (len items))
      out
      (do
        (.update out (collect-json-context (get items i)))
        (collect-json-context-list items (+ i 1) out))))

  (defn collect-json-context-dict [value keys i out]
    (if (>= i (len keys))
      out
      (do
        (.update out (collect-json-context (get value (get keys i))))
        (collect-json-context-dict value keys (+ i 1) out))))

  (defn to-json-builtin [value]
    (make-context-string
      (canonical-json (json-ready-value value))
      (collect-json-context value)))

  (defn deep-force-cycle-check [value seen label]
    (do
      (setv oid (builtins.id value))
      (if (in oid seen)
        (pnix-error (+ label ": infinite recursion encountered (cyclic value)"))
        None)
      (.add seen oid)
      oid))

  (defn deep-force-list [items i seen label]
    (if (>= i (len items))
      None
      (do
        (deep-force-value (get items i) seen label)
        (deep-force-list items (+ i 1) seen label))))

  (defn deep-force-dict [value keys i seen label]
    (if (>= i (len keys))
      None
      (do
        (deep-force-value (get value (get keys i)) seen label)
        (deep-force-dict value keys (+ i 1) seen label))))

  (defn deep-force-value [value [seen None] [label "builtins.deepSeq"]]
    (if (= seen None) (setv seen (set [])) None)
    (setv value (force-value value))
    (cond
      (construct? value)
        (do
          (setv oid (deep-force-cycle-check value seen label))
          (deep-force-list (get value "args") 0 seen label)
          (.discard seen oid)
          value)
      (isinstance value dict)
        (do
          (setv oid (deep-force-cycle-check value seen label))
          (deep-force-dict value (list (.keys value)) 0 seen label)
          (.discard seen oid)
          value)
      (isinstance value list)
        (do
          (setv oid (deep-force-cycle-check value seen label))
          (deep-force-list value 0 seen label)
          (.discard seen oid)
          value)
      True value))

  (defn number-value [value label]
    (setv value (force-value value))
    (if (and (or (isinstance value int) (isinstance value float)) (not (isinstance value bool)))
      value
      (pnix-error (+ label " must be a number"))))

  (defn numeric? [v]
    (and (or (isinstance v int) (isinstance v float)) (not (isinstance v bool))))

  (defn arith-pair [op a b]
    (do
      (setv x (force-value a))
      (setv y (force-value b))
      (if (not (and (numeric? x) (numeric? y)))
        (pnix-error (+ "operator " (+ op (+ ": unsupported operand types " (+ (type-of x) (+ " and " (type-of y)))))))
        None)
      [x y]))

  (defn integer-value [value label]
    (setv value (force-value value))
    (if (and (isinstance value int) (not (isinstance value bool)))
      value
      (pnix-error (+ label " must be an integer"))))

  (defn nonnegative-count-value [value label]
    (do
      (setv count (integer-value value label))
      (if (< count 0)
        (pnix-error (+ label ": negative count"))
        count)))

  (defn bool-value [value label]
    (setv value (force-value value))
    (if (isinstance value bool)
      value
      (pnix-error (+ label (+ ": expected bool, got " (type-of value))))))

  (defn predicate-bool-value [value label index]
    (do
      (setv forced (force-value value))
      (if (isinstance forced bool)
        forced
        (pnix-error
          (+ (+ (+ label " must return bool, got ") (type-of forced))
             (if (= index None) "" (+ " at index " (str index))))))))

  (defn list-value [value label]
    (setv value (force-value value))
    (if (isinstance value list)
      value
      (pnix-error (+ label " must be a list"))))

  (defn sort-list-value [value]
    (do
      (setv forced (force-value value))
      (if (isinstance forced list)
        forced
        (pnix-error (+ "builtins.sort second argument must be list, got " (type-of forced))))))

	  (defn attrset-value [value label]
	    (setv value (force-value value))
	    (if (and (isinstance value dict) (not (closure? value)) (not (thunk? value)) (not (path? value)) (not (construct? value)))
	      value
	      (pnix-error (+ label " must be an attrset"))))

	  (defn string-value [value label]
	    (setv value (force-value value))
	    (if (isinstance value str)
	      (str value)
	      (pnix-error (+ label (+ " (" (+ (type-of value) ") must be a string"))))))

	  (defn plain-string-value [value label]
	    (setv value (force-value value))
	    (if (isinstance value str)
	      (str value)
	      (pnix-error (+ label (+ " (" (+ (type-of value) ") must be string"))))))

	  (defn expected-string-value [value label]
	    (setv value (force-value value))
	    (if (isinstance value str)
	      (str value)
	      (pnix-error (+ label (+ ": expected string, got " (type-of value))))))

	  (defn plain-string-text-context [value label]
	    (setv value (force-value value))
	    (if (isinstance value str)
	      [(str value) (string-context value)]
	      (pnix-error (+ label (+ " (" (+ (type-of value) ") must be string"))))))

	  (defn string-byte-length [value label]
	    (len (.encode (string-value value label) "utf-8" "surrogateescape")))

	  (defn length-builtin [value]
	    (do
	      (setv value (force-value value))
	      (cond
	        (isinstance value list) (len value)
	        (isinstance value str) (len (.encode (str value) "utf-8" "surrogateescape"))
	        True (pnix-error (+ "builtins.length: expected list or string, got " (type-of value))))))

	  (defn substring-builtin-decode [data st end]
	    (.decode (cut data st end) "utf-8" "surrogateescape"))

	  (defn substring-builtin-data [st ln data]
	    (if (>= st (len data))
	      ""
	      (substring-builtin-decode data st (if (< ln 0) (len data) (min (+ st ln) (len data))))))

	  (defn substring-builtin-values [st ln text]
	    (cond
	      (or (isinstance st float) (isinstance ln float))
	        (pnix-error "builtins.substring start and length must be integers")
	      (< st 0)
	        (pnix-error "builtins.substring: negative start position not allowed")
	      True
	        (substring-builtin-data st ln (.encode text "utf-8" "surrogateescape"))))

	  (defn substring-builtin [start length text]
	    (substring-builtin-values
	      (number-value start "builtins.substring start")
	      (number-value length "builtins.substring length")
	      (string-value text "builtins.substring string")))

	  (defn replace-match-index [patterns tail i]
	    (if (>= i (len patterns))
	      None
	      (if (.startswith tail (get patterns i))
	        i
	        (replace-match-index patterns tail (+ i 1)))))

	  (defn force-string-list [items label i]
	    (if (>= i (len items))
	      []
	      (+ [(string-value (get items i) label)]
	         (force-string-list items label (+ i 1)))))

	  (defn replace-strings-loop-hit [patterns replacements text i out idx]
	    (if (= (len (get patterns idx)) 0)
	      (replace-strings-loop
	        patterns
	        replacements
	        text
	        (+ i 1)
	        (+ (+ out (string-value (get replacements idx) "builtins.replaceStrings to element"))
	           (if (< i (len text)) (get text i) "")))
	      (replace-strings-loop
	        patterns
	        replacements
	        text
	        (+ i (len (get patterns idx)))
	        (+ out (string-value (get replacements idx) "builtins.replaceStrings to element")))))

	  (defn replace-strings-loop-match [patterns replacements text i out idx]
	    (if (= idx None)
	      (replace-strings-loop
	        patterns replacements text (+ i 1)
	        (+ out (if (< i (len text)) (get text i) "")))
	      (replace-strings-loop-hit patterns replacements text i out idx)))

	  (defn replace-strings-loop [patterns replacements text i out]
	    (if (> i (len text))
	      out
	      (replace-strings-loop-match
	        patterns
	        replacements
	        text
	        i
	        out
	        (replace-match-index patterns (cut text i None) 0))))

	  (defn replace-from-elements [items i out]
	    (if (>= i (len items))
	      out
	      (do
	        (setv elem (force-value (get items i)))
	        (if (not (isinstance elem str))
	          (pnix-error (+ "builtins.replaceStrings: 'from' element must be string, got " (type-of elem)))
	          None)
	        (replace-from-elements items (+ i 1) (+ out [(str elem)])))))

	  (defn replace-strings-builtin-values [fs ts s]
	    (if (!= (len fs) (len ts))
	      (pnix-error "builtins.replaceStrings: `from` and `to` lists must have equal length")
	      (if (= (len fs) 0)
	        s
	        (replace-strings-loop
	          (replace-from-elements fs 0 [])
	          ts
	          s
	          0
	          ""))))

	  (defn replace-strings-builtin [froms tos text]
	    (do
	      (setv fs (force-value froms))
	      (if (not (isinstance fs list))
	        (pnix-error (+ "builtins.replaceStrings: 'from' must be list, got " (type-of fs)))
	        None)
	      (setv ts (force-value tos))
	      (if (not (isinstance ts list))
	        (pnix-error (+ "builtins.replaceStrings: 'to' must be list, got " (type-of ts)))
	        None)
	      (setv s (force-value text))
	      (if (not (isinstance s str))
	        (pnix-error (+ "builtins.replaceStrings: third argument must be string, got " (type-of s)))
	        None)
	      (replace-strings-builtin-values fs ts (str s))))

	  (defn list-to-attrs-items [items i out]
	    (if (>= i (len items))
	      out
	      (do
	        (setv entry (attrset-value (get items i) "builtins.listToAttrs element"))
	        (setv name (string-value (get entry "name") "builtins.listToAttrs name"))
	        (if (not (in "value" entry))
	          (pnix-error "builtins.listToAttrs element is missing `value`")
	          None)
	        (if (not (in name out))
	          (setv (get out name) (get entry "value"))
	          None)
	        (list-to-attrs-items items (+ i 1) out))))

	  (defn list-to-attrs-builtin [xs]
	    (list-to-attrs-items (list-value xs "builtins.listToAttrs list") 0 {}))

	  (defn names-to-remove [items i out]
	    (if (>= i (len items))
	      out
	      (do
	        (setv elem (force-value (get items i)))
	        (if (not (isinstance elem str))
	          (pnix-error (+ "builtins.removeAttrs: name-list element at index " (+ (str i) (+ " is not a string, got " (type-of elem)))))
	          None)
	        (.append out (str elem))
	        (names-to-remove items (+ i 1) out))))

	  (defn remove-attrs-copy [attrs keys i out]
	    (if (>= i (len keys))
	      out
	      (do
	        (setv key (get keys i))
	        (if (not (in key (get out "__remove__")))
	          (setv (get out key) (get attrs key))
	          None)
	        (remove-attrs-copy attrs keys (+ i 1) out))))

	  (defn has-attr-builtin [name m]
	    (do
	      (setv name (force-value name))
	      (if (not (isinstance name str))
	        (pnix-error (+ "builtins.hasAttr: first argument must be string, got " (type-of name)))
	        None)
	      (setv m (force-value m))
	      (if (not (and (isinstance m dict) (not (closure? m)) (not (thunk? m)) (not (path? m)) (not (construct? m))))
	        (pnix-error (+ "builtins.hasAttr: second argument must be attrset, got " (type-of m)))
	        None)
	      (in (str name) m)))

	  (defn remove-attrs-builtin [attrs names]
	    (do
	      (setv source (force-value attrs))
	      (if (not (and (isinstance source dict) (not (closure? source)) (not (thunk? source)) (not (path? source)) (not (construct? source))))
	        (pnix-error (+ "builtins.removeAttrs: first argument must be attrset, got " (type-of source)))
	        None)
	      (setv namelist (force-value names))
	      (if (not (isinstance namelist list))
	        (pnix-error (+ "builtins.removeAttrs: second argument must be list of strings, got " (type-of namelist)))
	        None)
	      (setv out {"__remove__" (names-to-remove namelist 0 [])})
	      (setv copied (remove-attrs-copy source (list (.keys source)) 0 out))
	      (.pop copied "__remove__")
	      copied))

	  (defn attr-by-path-loop [path i current default]
	    (if (>= i (len path))
	      current
	      (do
	        (setv key (string-value (get path i) "builtins.attrByPath path element"))
	        (if (and (isinstance current dict) (in key current))
	          (attr-by-path-loop path (+ i 1) (force-value (get current key)) default)
	          default))))

	  (defn attr-by-path-builtin [path default attrs]
	    (attr-by-path-loop
	      (list-value path "builtins.attrByPath path")
	      0
	      (attrset-value attrs "builtins.attrByPath attrs")
	      default))

	  (defn has-attr-path-parts [current parts i]
	    (if (>= i (len parts))
	      True
	      (if (and (isinstance current dict) (in (get parts i) current))
	        (if (= i (- (len parts) 1))
	          True
	          (has-attr-path-parts (force-value (get current (get parts i))) parts (+ i 1)))
	        False)))

	  (defn has-attr-path [attrs attr]
	    (has-attr-path-parts attrs attr 0))

	  (defn current-system []
	    (do
	      (setv machine (platform.machine))
	      (if (= machine "")
	        (setv machine "unknown")
	        None)
	      (cond
	        (= sys.platform "darwin") (+ machine "-darwin")
	        (.startswith sys.platform "linux") (+ machine "-linux")
	        True (+ (+ machine "-") sys.platform))))

	  (defn split-version-components [s]
	    (do
	      (setv result [])
	      (setv component-start None)
	      (setv last-was-digit None)
	      (for [[idx ch] (enumerate s)]
	        (do
	          (setv is-digit (and (.isascii ch) (.isdigit ch)))
	          (setv is-sep (in ch ["." "-"]))
	          (cond
	            is-sep
	              (do
	                (if (!= component-start None)
	                  (do
	                    (.append result (cut s component-start idx))
	                    (setv component-start None))
	                  None)
	                (setv last-was-digit None))
	            (and (!= last-was-digit None) (!= last-was-digit is-digit))
	              (do
	                (if (!= component-start None)
	                  (.append result (cut s component-start idx))
	                  None)
	                (setv component-start idx)
	                (setv last-was-digit is-digit))
	            True
	              (do
	                (if (= component-start None)
	                  (setv component-start idx)
	                  None)
	                (setv last-was-digit is-digit)))))
	      (if (!= component-start None)
	        (.append result (cut s component-start None))
	        None)
	      result))

	  (defn split-version [s]
	    (do
	      (setv tc (string-text-context s "builtins.splitVersion string"))
	      (setv text (get tc 0))
	      (setv ctx (get tc 1))
	      (list (map (fn [part] (make-context-string part ctx))
	                 (split-version-components text)))))

	  (defn parse-drv-name [s]
	    (do
	      (setv tc (string-text-context s "builtins.parseDrvName string"))
	      (setv text (get tc 0))
	      (setv ctx (get tc 1))
	      (setv split-idx None)
	      (for [idx (range 0 (max 0 (- (len text) 1)))]
	        (if (and (= split-idx None)
	                 (= (get text idx) "-")
	                 (.isascii (get text (+ idx 1)))
	                 (.isdigit (get text (+ idx 1))))
	          (setv split-idx idx)
	          None))
	      (if (= split-idx None)
	        {"name" (make-context-string text ctx) "version" (make-context-string "" ctx)}
	        {"name" (make-context-string (cut text 0 split-idx) ctx)
	         "version" (make-context-string (cut text (+ split-idx 1) None) ctx)})))

	  (defn compare-version-component [left right]
	    (do
	      (setv left-num (if (.isdigit left) (int left) None))
	      (setv right-num (if (.isdigit right) (int right) None))
	      (cond
	        (and (!= left-num None) (!= right-num None))
	          (cond (< left-num right-num) -1 (> left-num right-num) 1 True 0)
	        (!= left-num None) 1
	        (!= right-num None) -1
	        (= left right) 0
	        (= left "") (if (= right "pre") 1 -1)
	        (= right "") (if (= left "pre") -1 1)
	        (= left "pre") -1
	        (= right "pre") 1
	        (< left right) -1
	        True 1)))

	  (defn compare-version-parts [left right i]
	    (cond
	      (and (>= i (len left)) (>= i (len right))) 0
	      True
	        (do
	          (setv cmp (compare-version-component
	                      (if (< i (len left)) (get left i) "")
	                      (if (< i (len right)) (get right i) "")))
	          (if (!= cmp 0)
	            cmp
	            (compare-version-parts left right (+ i 1))))))

	  (defn compare-versions [left right]
	    (do
	      (setv l (force-value left))
	      (setv r (force-value right))
	      (if (not (and (isinstance l str) (isinstance r str)))
	        (pnix-error "builtins.compareVersions: expected two strings")
	        None)
	      (compare-version-parts
	        (split-version-components (str l))
	        (split-version-components (str r))
	        0)))

  (setv WITH-CHAIN-KEY "__pnix_hy_with_chain__")

  (defn make-with-frame [source env]
    {"source" source "env" (dict env) "done" False "cached" None})

  (defn abort-show [value]
    (setv value (force-value value))
    (cond
      (isinstance value bool) (if value "true" "false")
      (and (isinstance value int) (not (isinstance value bool))) (str value)
      (isinstance value float) (str value)
      (= value None) "null"
      (isinstance value str) (str value)
      True (type-of value)))

  (defn abort-value [msg]
    (do
      (setv forced (force-value msg))
      (if (not (isinstance forced str))
        (pnix-error (+ "builtins.abort: argument must be string, got " (abort-show forced)))
        (pnix-error (+ "evaluation aborted: " (str forced))))))

  (defn throw-value [msg]
    (pnix-catchable-error (expected-string-value msg "builtins.throw")))

  (defn force-with-frame [frame]
    (if (get frame "done")
      (get frame "cached")
      (do
        (setv attrs (force-value (eval-ast (get frame "source") (get frame "env"))))
        (if (not (and (isinstance attrs dict) (not (closure? attrs)) (not (thunk? attrs)) (not (path? attrs)) (not (construct? attrs))))
          (pnix-error (+ "with: argument must be attrset, got " (type-of attrs)))
          None)
        (setv (get frame "cached") attrs)
        (setv (get frame "done") True)
        attrs)))

  (defn lookup-with-chain [chain name]
    (if (= (len chain) 0)
      None
      (do
        (setv attrs (force-with-frame (get chain 0)))
        (if (in name attrs)
          {"found" True "value" (force-value (get attrs name))}
          (lookup-with-chain (cut chain 1 None) name)))))

  (defn lookup-env [env name]
    (if (in name env)
      (force-value (get env name))
      (do
        (setv found (if (in WITH-CHAIN-KEY env)
                      (lookup-with-chain (get env WITH-CHAIN-KEY) name)
                      None))
        (if (= found None)
          (pnix-error (+ (+ "unknown variable `" name) "`"))
          (get found "value")))))

  (defn with-env [env source]
    (do
      (setv e (dict env))
      (setv chain (if (in WITH-CHAIN-KEY env) (get env WITH-CHAIN-KEY) []))
      (setv (get e WITH-CHAIN-KEY) (+ [(make-with-frame source env)] chain))
      e))

  (defn attr-container? [value]
    (and (isinstance value dict) (not (closure? value)) (not (thunk? value)) (not (path? value)) (not (construct? value))))

  (setv AttrSet (type "AttrSet" (tuple [dict]) {}))

  (defn make-attrset []
    (do
      (setv d (AttrSet))
      (setattr d "attr_positions" {})
      d))

  (defn attrset-positions [value]
    (getattr value "attr_positions" {}))

  (defn record-attr-position [target name positions idx]
    (if (and (isinstance target AttrSet) (!= positions None) (< idx (len positions)) (!= (get positions idx) None))
      (setv (get (attrset-positions target) name) (get positions idx))
      None))

  (defn source-line-col-loop [source i pos line col]
    (if (>= i pos)
      [line col]
      (if (= (get source i) "\n")
        (source-line-col-loop source (+ i 1) pos (+ line 1) 1)
        (source-line-col-loop source (+ i 1) pos line (+ col 1)))))

  (defn source-position-value [pos]
    (do
      (setv p (int pos))
      (if (< p 0) (setv p 0) None)
      (if (> p (len current-source)) (setv p (len current-source)) None)
      (setv lc (source-line-col-loop current-source 0 p 1 1))
      {"file" "<pnix-stage7>" "line" (get lc 0) "column" (get lc 1)}))

  (defn merge-defined-attrsets [left right]
    (do
      (setv out (make-attrset))
      (.update out left)
      (for [key right]
        (if (not (in key out))
          (setv (get out key) (get right key))
          (do
            (setv left-value (force-value (get out key)))
            (setv right-value (force-value (get right key)))
            (if (and (attr-container? left-value) (attr-container? right-value))
              (setv (get out key) (merge-defined-attrsets left-value right-value))
              (pnix-error (+ (+ "attribute '" key) "' already defined at this level"))))))
      out))

  (defn set-path-with-context [target path value context positions idx]
    (if (= (len path) 1)
      (do
        (if (in (get path 0) target)
          (if (= context "let")
            (pnix-error (+ (+ "let: '" (get path 0)) "' bound more than once"))
            (do
              (setv existing (force-value (get target (get path 0))))
              (setv new-value (force-value value))
              (if (and (attr-container? existing) (attr-container? new-value))
                (setv (get target (get path 0)) (merge-defined-attrsets existing new-value))
                (pnix-error (+ (+ "attribute '" (get path 0)) "' already defined at this level")))))
          None)
        (if (not (in (get path 0) target))
          (do
            (record-attr-position target (get path 0) positions idx)
            (setv (get target (get path 0)) value))
          None)
        target)
      (do
        (setv head (get path 0))
        (setv child (.get target head None))
        (if (= child None)
          (do
            (setv child (make-attrset))
            (record-attr-position target head positions idx)
            (setv (get target head) child))
          None)
        (if (not (attr-container? child))
          (do
            (setv forced-child (force-value child))
            (if (not (attr-container? forced-child))
              (pnix-error (+ (+ "attribute path conflict: '" head) "' is already a non-attrset value"))
              (do
                (setv child forced-child)
                (setv (get target head) child))))
          None)
        (set-path-with-context child (cut path 1 None) value context positions (+ idx 1))
        target)))

  (defn set-path [target path value]
    (set-path-with-context target path value "attr" None 0))

  (defn eval-thunk [node env]
    (make-node-thunk node env))

  (defn install-rec-env [value rec-env]
    (cond
      (thunk? value)
        (setv (get value "env") rec-env)
      (and (isinstance value dict) (not (closure? value)))
        (for [key (.keys value)]
          (install-rec-env (get value key) rec-env))
      True None))

  (defn build-attrset [bindings env recursive]
    (setv values (make-attrset))
    (if recursive
      (do
        (setv rec-env (.copy env))
        (setv ordered [])
        (for [binding bindings]
          (if (binding-path-static? (get binding "path"))
            (.append ordered binding)
            None))
        (for [binding bindings]
          (if (not (binding-path-static? (get binding "path")))
            (.append ordered binding)
            None))
        (for [binding ordered]
          (do
            (set-path-with-context values (eval-binding-path (get binding "path") rec-env)
              (eval-thunk
                (get binding "value")
                (if (in "inherit_plain" binding) env rec-env))
              "attr"
              (binding-positions binding)
              0)
            (.update rec-env values)))
        (.update rec-env values)
        values)
      (do
        (for [binding bindings]
          (set-path-with-context
            values
            (eval-binding-path (get binding "path") env)
            (eval-thunk (get binding "value") env)
            "attr"
            (binding-positions binding)
            0))
        values)))

  (defn build-let-env [bindings env]
    (setv rec-env (.copy env))
    (setv values {})
    (for [binding bindings]
      (set-path-with-context values (eval-binding-path (get binding "path") rec-env)
        (eval-thunk
          (get binding "value")
          (if (in "inherit_plain" binding) env rec-env))
        "let"
        (binding-positions binding)
        0))
    (.update rec-env values)
    rec-env)

  (setv VALUES-EQUAL-MAX-DEPTH 64)

  (defn function-value? [value]
    (or (closure? value) (native-lazy? value) (callable value)))

  (defn eq-list-items [left right i depth]
    (cond
      (>= i (len left)) True
      (eq-value-at-depth (get left i) (get right i) (+ depth 1))
        (eq-list-items left right (+ i 1) depth)
      True False))

  (defn eq-dict-items [left right keys i depth]
    (cond
      (>= i (len keys)) True
      (eq-value-at-depth (get left (get keys i)) (get right (get keys i)) (+ depth 1))
        (eq-dict-items left right keys (+ i 1) depth)
      True False))

  (defn eq-construct-items [left right i depth]
    (cond
      (>= i (len (get left "args"))) True
      (eq-value-at-depth (get (get left "args") i) (get (get right "args") i) (+ depth 1))
        (eq-construct-items left right (+ i 1) depth)
      True False))

  (defn eq-value-at-depth [left right depth]
    (do
      (if (> depth VALUES-EQUAL-MAX-DEPTH)
        (pnix-error "infinite recursion encountered during ==")
        None)
      (setv l (force-value left))
      (setv r (force-value right))
      (cond
        (and (> depth 0) (is l r)) True
        (or (function-value? l) (function-value? r)) False
        (or (path? l) (path? r))
          (and (path? l) (path? r)
               (= (normalize-pnix-path-text (get l "value"))
                  (normalize-pnix-path-text (get r "value"))))
        (isinstance l PnixString) (eq-value-at-depth (str l) r depth)
        (isinstance r PnixString) (eq-value-at-depth l (str r) depth)
        (or (construct? l) (construct? r))
          (and (construct? l) (construct? r)
               (= (get l "variant") (get r "variant"))
               (= (len (get l "args")) (len (get r "args")))
               (eq-construct-items l r 0 depth))
        (or (isinstance l dict) (isinstance r dict))
          (and (isinstance l dict) (isinstance r dict)
               (= (set (.keys l)) (set (.keys r)))
               (eq-dict-items l r (list (.keys l)) 0 depth))
        (or (isinstance l list) (isinstance r list))
          (and (isinstance l list) (isinstance r list)
               (= (len l) (len r))
               (eq-list-items l r 0 depth))
        (or (isinstance l bool) (isinstance r bool))
          (and (isinstance l bool) (isinstance r bool) (= l r))
        (or (= l None) (= r None))
          (and (= l None) (= r None))
        (and (or (isinstance l int) (isinstance l float)) (not (isinstance l bool))
             (or (isinstance r int) (isinstance r float)) (not (isinstance r bool)))
          (if (or (isinstance l float) (isinstance r float))
            (= (float l) (float r))
            (= l r))
        (or (isinstance l str) (isinstance r str))
          (and (isinstance l str) (isinstance r str) (= (str l) (str r)))
        True (= l r))))

  (defn eq-value [left right]
    (eq-value-at-depth left right 0))

  (defn merge-attrsets [left right]
    (do
      (setv l (force-value left))
      (setv r (force-value right))
      (cond
        (= l None) r
        (= r None) l
        True
          (do
            (setv merged (.copy (attrset-value l "left side of //")))
            (.update merged (attrset-value r "right side of //"))
            merged))))

  (defn marker-string? [value]
    (if (isinstance value str)
      (if (>= (len value) 10)
        (= (cut value 0 10) "#<pnix-hy-")
        False)
      False))

  (setv to-string-seen (set []))

  (defn nix-list-less [left right depth]
    (cond
      (= (len left) 0) (> (len right) 0)
      (= (len right) 0) False
      (eq-value-at-depth (get left 0) (get right 0) (+ depth 1))
        (nix-list-less (cut left 1 None) (cut right 1 None) depth)
      True (nix-less-than-depth (get left 0) (get right 0) (+ depth 1))))

  (defn nix-less-than-depth [left right depth]
    (do
      (if (> depth VALUES-EQUAL-MAX-DEPTH)
        (pnix-error "infinite recursion encountered during comparison")
        None)
      (setv l (force-value left))
      (setv r (force-value right))
      (if (and (> depth 0) (is l r))
        (return False)
        None)
      (if (or (path? l) (path? r))
        (if (and (path? l) (path? r))
          (do
            (setv l (normalize-pnix-path-text (get l "value")))
            (setv r (normalize-pnix-path-text (get r "value"))))
          (pnix-error (+ (+ "cannot compare " (type-of l)) (+ " with " (type-of r)))))
        None)
      (if (isinstance l PnixString) (setv l (str l)) None)
      (if (isinstance r PnixString) (setv r (str r)) None)
      (cond
        (or (isinstance l bool) (isinstance r bool))
          (pnix-error "cannot compare booleans with `<`")
        (and (or (isinstance l int) (isinstance l float)) (not (isinstance l bool))
             (or (isinstance r int) (isinstance r float)) (not (isinstance r bool)))
          (if (or (isinstance l float) (isinstance r float))
            (< (float l) (float r))
            (< l r))
        (and (isinstance l str) (isinstance r str)
             (not (marker-string? l)) (not (marker-string? r)))
          (< l r)
        (and (isinstance l list) (isinstance r list))
          (nix-list-less l r depth)
        True
          (pnix-error (+ (+ "cannot compare " (type-of l)) (+ " with " (type-of r)))))))

  (defn nix-less-than [left right]
    (nix-less-than-depth left right 0))

  (defn nix-compare [op left right]
    (cond
      (= op "<") (nix-less-than left right)
      (= op ">") (nix-less-than right left)
      (= op "<=") (not (nix-less-than right left))
      True (not (nix-less-than left right))))

  (defn vts-list-parts [items seen]
    (if (= (len items) 0)
      ""
      (do
        (setv head-s (value-to-string (get items 0) seen))
        (setv rest-s (vts-list-parts (cut items 1 None) seen))
        (if (= (str rest-s) "")
          head-s
          (make-context-string
            (+ (+ (str head-s) " ") (str rest-s))
            (merge-contexts (string-context head-s) (string-context rest-s)))))))

  (defn vts-dict-parts [value keys]
    (if (= (len keys) 0)
      ""
      (do
        (setv key (get keys 0))
        (setv rest-s (vts-dict-parts value (cut keys 1 None)))
        (setv part (+ key " = " (value-to-string (get value key)) ";"))
        (if (= rest-s "")
          part
          (+ part " " rest-s)))))

  (defn value-to-string [value [seen None]]
    (if (= seen None) (setv seen to-string-seen) None)
    (setv value (force-value value))
    (cond
      (= value None) ""
      (isinstance value bool) (if value "1" "")
      (and (isinstance value int) (not (isinstance value bool))) (str value)
      (isinstance value float) (format value ".6f")
      (path? value) (make-context-string (get value "value") (set [(get value "value")]))
      (isinstance value str)
        (if (marker-string? value)
          (pnix-error "cannot coerce a function to a string")
          (make-context-string value (string-context value)))
      (isinstance value list)
        (vts-list-parts value seen)
      (or (closure? value) (native-lazy? value) (callable value))
        (pnix-error "cannot coerce a function to a string")
      (isinstance value dict)
        (do
          (setv oid (builtins.id value))
          (if (in oid seen)
            (pnix-error "toString cycle detected")
            None)
          (.add seen oid)
          ;; discard oid on BOTH success AND error (mirrors coerce-interp-dict
          ;; and the Python _vts try/finally). The prior code discarded only on
          ;; the success paths, so a toString that errored mid-traversal leaked
          ;; oid into the (shared) `seen` set permanently -> later values whose
          ;; id was reused (post-GC) tripped a FALSE "toString cycle detected".
          (try
            (do
              (setv result
                (cond
                  (in "__toString" value)
                    (value-to-string
                      (apply-pnix (force-value (get value "__toString")) (make-value-thunk value))
                      seen)
                  (in "outPath" value)
                    (value-to-string (get value "outPath") seen)
                  True
                    (pnix-error "cannot coerce a set to a string: missing __toString or outPath")))
              (.discard seen oid)
              result)
            (except [Exception exc]
              (do
                (.discard seen oid)
                (raise exc)))))
      True
        (pnix-error "cannot coerce value to a string")))

  (setv interp-coerce-stack (set []))

  (defn coerce-interp-dict [value oid]
    (do
      (.add interp-coerce-stack oid)
      (try
        (do
          (setv result
            (cond
              (in "__toString" value)
                (coerce-interp
                  (apply-pnix (force-value (get value "__toString")) (make-value-thunk value)))
              (in "outPath" value)
                (coerce-interp (get value "outPath"))
              True
                (pnix-error "cannot coerce a set to a string in interpolation: no __toString or outPath")))
          (.discard interp-coerce-stack oid)
          result)
        (except [Exception exc]
          (do
            (.discard interp-coerce-stack oid)
            (raise exc))))))

  (defn coerce-interp [value]
    (setv value (force-value value))
    (cond
      (path? value) (make-context-string (get value "value") (set [(get value "value")]))
      (isinstance value str) (make-context-string value (string-context value))
      (and (or (isinstance value int) (isinstance value float)) (not (isinstance value bool)))
        (pnix-error "cannot coerce a number to a string in interpolation: use builtins.toString")
      (isinstance value bool)
        (pnix-error "cannot coerce a boolean to a string in interpolation: use builtins.toString")
      (= value None)
        (pnix-error "cannot coerce null to a string in interpolation")
      (isinstance value list)
        (pnix-error "cannot coerce a list to a string in interpolation")
      (closure? value)
        (pnix-error "cannot coerce a function to a string in interpolation")
      (isinstance value dict)
        (do
          (setv oid (builtins.id value))
          (if (in oid interp-coerce-stack)
            (pnix-error "interpolation coercion cycle involving __toString")
            (coerce-interp-dict value oid)))
      True
        (pnix-error "cannot coerce value to a string in interpolation")))

	  (defn type-of [value]
	    (setv value (force-value value))
	    (cond
	      (closure? value) "lambda"
	      (native-lazy? value) "lambda"
	      (callable value) "lambda"
	      (path? value) "path"
	      (construct? value) "construct"
	      (isinstance value list) "list"
	      (and (isinstance value dict) (not (thunk? value)) (not (construct? value))) "set"
	      (isinstance value str) "string"
	      (isinstance value bool) "bool"
	      (and (isinstance value int) (not (isinstance value bool))) "int"
	      (isinstance value float) "float"
	      (= value None) "null"
	      True "unknown"))

	  (defn attr-key-value [value]
	    (setv value (force-value value))
	    (cond
	      (path? value) (get value "value")
	      (isinstance value str) value
	      True (value-to-string value)))

	  (defn binding-path-static? [path]
	    (if (= (len path) 0)
	      True
	      (and (isinstance (get path 0) str) (binding-path-static? (cut path 1 None)))))

	  (defn eval-binding-path [path env]
	    (if (= (len path) 0)
	      []
	      (do
	        (setv part (get path 0))
	        (+ [(if (isinstance part str)
	              part
	              (attr-key-value (eval-ast (get part "expr") env)))]
	           (eval-binding-path (cut path 1 None) env)))))

	  (defn binding-positions [binding]
	    (if (binding-path-static? (get binding "path"))
	      (.get binding "path_positions" None)
	      None))

	  (defn eval-attr-segments-rec [segments env i acc]
	    (if (>= i (len segments))
	      acc
	      (do
	        (setv segment (get segments i))
	        (setv part (if (in "lit" segment)
	                     (str (get segment "lit"))
	                     (attr-key-value (eval-ast (get segment "expr") env))))
	        (eval-attr-segments-rec
	          segments
	          env
	          (+ i 1)
	          (+ acc (if (> i 0) "." "") part)))))

	  (defn eval-attr-segments [segments env]
	    (eval-attr-segments-rec segments env 0 ""))

	  (defn eval-attr-path-segments [segments env i]
	    (if (>= i (len segments))
	      []
	      (do
	        (setv segment (get segments i))
	        (+ [(if (in "lit" segment)
	              (str (get segment "lit"))
	              (attr-key-value (eval-ast (get segment "expr") env)))]
	           (eval-attr-path-segments segments env (+ i 1))))))

	  (defn value-cell [value]
	    (if (thunk? value) value (make-value-thunk value)))

	  (defn merge-match-key-conflict? [out right name]
	    (if (in name out)
	      (not (eq-value (get out name) (get right name)))
	      False))

	  (defn merge-match-bindings-keys-with-name [out right keys name]
	    (if (merge-match-key-conflict? out right name)
	      None
	      (do
	        (setv (get out name) (get right name))
	        (merge-match-bindings-keys out right (cut keys 1 None)))))

	  (defn merge-match-bindings-keys [out right keys]
	    (if (= (len keys) 0)
	      out
	      (merge-match-bindings-keys-with-name out right keys (get keys 0))))

	  (defn merge-match-bindings [left right]
	    (merge-match-bindings-keys (dict left) right (list (.keys right))))

	  (defn match-pattern-list-after-merge [patterns values ctx merged]
	    (if (= merged None)
	      None
	      (match-pattern-list (cut patterns 1 None) (cut values 1 None) ctx merged)))

	  (defn match-pattern-list-with-match [patterns values ctx bindings matched]
	    (if (= matched None)
	      None
	      (match-pattern-list-after-merge
	        patterns
	        values
	        ctx
	        (merge-match-bindings bindings matched))))

	  (defn match-pattern-list [patterns values ctx bindings]
	    (if (= (len patterns) 0)
	      bindings
	      (match-pattern-list-with-match
	        patterns
	        values
	        ctx
	        bindings
	        (match-pattern (get patterns 0) (get values 0) ctx))))

	  (defn match-pattern-fields-after-merge [fields value ctx merged]
	    (if (= merged None)
	      None
	      (match-pattern-fields (cut fields 1 None) value ctx merged)))

	  (defn match-pattern-fields-with-match [fields value ctx bindings matched]
	    (if (= matched None)
	      None
	      (match-pattern-fields-after-merge
	        fields
	        value
	        ctx
	        (merge-match-bindings bindings matched))))

	  (defn match-pattern-fields-with-name [fields value ctx bindings field name]
	    (if (not (in name value))
	      (if (in "default" field)
	        (match-pattern-fields-with-match
	          fields
	          value
	          ctx
	          bindings
	          (match-pattern
	            (get field "pattern")
	            (eval-thunk
	              (get field "default")
	              (env-update-all (dict ctx) bindings))
	            ctx))
	        None)
	      (match-pattern-fields-with-match
	        fields
	        value
	        ctx
	        bindings
	        (match-pattern (get field "pattern") (get value name) ctx))))

	  (defn match-pattern-fields [fields value ctx bindings]
	    (if (= (len fields) 0)
	      bindings
	      (match-pattern-fields-with-name
	        fields
	        value
	        ctx
	        bindings
	        (get fields 0)
	        (get (get fields 0) "name"))))

	  (defn match-pattern-list-rest [pattern values bindings]
	    (if (= bindings None)
	      None
	      (if (= (.get pattern "rest" None) None)
	        bindings
	        (merge-match-bindings bindings {(get pattern "rest") (value-cell (cut values (len (get pattern "items")) None))}))))

	  (defn match-pattern-list-value [pattern value ctx v]
	    (if (isinstance v list)
	      (if (= (.get pattern "rest" None) None)
	        (if (= (len v) (len (get pattern "items")))
	          (match-pattern-list (get pattern "items") v ctx {})
	          None)
	        (if (>= (len v) (len (get pattern "items")))
	          (match-pattern-list-rest pattern v (match-pattern-list (get pattern "items") v ctx {}))
	          None))
	      None))

	  (defn match-pattern-attrset-value [pattern value ctx v]
	    (if (isinstance v dict)
	      (if (construct? v)
	        None
	        (if (path? v)
	          None
	          (match-pattern-fields (get pattern "fields") v ctx {})))
	      None))

	  (defn match-pattern-constructor-value [pattern value ctx v]
	    (if (construct? v)
	      (if (= (get v "variant") (get pattern "variant"))
	        (if (= (len (get v "args")) (len (get pattern "args")))
	          (match-pattern-list (get pattern "args") (get v "args") ctx {})
	          None)
	        None)
	      None))

	  (defn match-pattern-as-after [pattern value ctx matched]
	    (if (= matched None)
	      None
	      (merge-match-bindings matched {(get pattern "name") (value-cell value)})))

	  (defn match-pattern-with-tag [pattern value ctx tag]
	    (cond
	      (= tag "wildcard") {}
	      (= tag "as")
	        (match-pattern-as-after pattern value ctx (match-pattern (get pattern "pattern") value ctx))
	      (= tag "var") {(get pattern "name") (value-cell value)}
	      (= tag "literal") (if (eq-value value (get pattern "value")) {} None)
	      (= tag "list")
	        (match-pattern-list-value pattern value ctx (force-value value))
	      (= tag "attrset")
	        (match-pattern-attrset-value pattern value ctx (force-value value))
	      (= tag "constructor")
	        (match-pattern-constructor-value pattern value ctx (force-value value))
	      True (pnix-error "unsupported match pattern")))

	  (defn match-pattern [pattern value ctx]
	    (match-pattern-with-tag pattern value ctx (get pattern "tag")))

  (defn duplicate-formal-field-name [fields seen]
    (if (= (len fields) 0)
      None
      (do
        (setv name (get (get fields 0) "name"))
        (if (in name seen)
          name
          (do
            (.add seen name)
            (duplicate-formal-field-name (cut fields 1 None) seen))))))

  (defn duplicate-formal-name [pattern]
    (do
      (setv bind-name None)
      (setv inner pattern)
      (if (= (.get pattern "tag" None) "as")
        (do
          (setv bind-name (get pattern "name"))
          (setv inner (get pattern "pattern")))
        None)
      (if (!= (.get inner "tag" None) "attrset")
        None
        (do
          (setv seen (set []))
          (if (!= bind-name None) (.add seen bind-name) None)
          (duplicate-formal-field-name (get inner "fields") seen)))))

	  (defn eval-matched-arm-with-env [arm matched-env]
	    (eval-ast (get arm "body") matched-env))

	  (defn env-update-all [target source]
	    (do
	      (.update target source)
	      target))

	  (defn eval-matched-arm [arm env bindings]
	    (do
	      (setv matched-env (env-update-all (dict env) bindings))
	      (if (and (in "guard" arm) (not (bool-value (eval-ast (get arm "guard") matched-env) "match guard")))
	        [False None]
	        [True (eval-matched-arm-with-env arm matched-env)])))

	  (defn eval-match-arm-result [scrutinee arms env arm bindings]
	    (if (= bindings None)
	      (eval-match-arms scrutinee (cut arms 1 None) env)
	      (do
	        (setv result (eval-matched-arm arm env bindings))
	        (if (not (get result 0))
	          (eval-match-arms scrutinee (cut arms 1 None) env)
	          (get result 1)))))

	  (defn eval-match-arms [scrutinee arms env]
	    (if (= (len arms) 0)
	      (pnix-error "non-exhaustive match")
	      (eval-match-arm-result
	        scrutinee
	        arms
	        env
	        (get arms 0)
	        (match-pattern (get (get arms 0) "pattern") scrutinee {}))))

	  (defn apply-pnix [func arg-delay]
    (setv func (force-value func))
    (cond
      (closure? func)
        (do
          (setv env (.copy (get func "env")))
          (if (= (.get func "pattern" None) None)
            (setv (get env (get func "param")) arg-delay)
            (do
              (setv pat (get func "pattern"))
              (setv inner (if (= (.get pat "tag" None) "as") (get pat "pattern") pat))
              (setv dup-name (duplicate-formal-name pat))
              (if (!= dup-name None)
                (pnix-error (+ "duplicate formal function argument '" (+ dup-name "'")))
                None)
              (setv bindings (match-pattern pat arg-delay env))
              (if (= bindings None)
                (if (= (.get inner "tag" None) "list")
                  (pnix-error "function argument does not match list pattern")
                  (pnix-error "function argument does not match pattern"))
                (do
                  (if (and (= (.get inner "tag" None) "attrset") (not (.get inner "ellipsis" False)))
                    (do
                      (setv fv (force-value arg-delay))
                      (if (isinstance fv dict)
                        (do
                          (setv allowed (lfor ff (get inner "fields") (get ff "name")))
                          (for [kk fv]
                            (if (in kk allowed)
                              None
                              (pnix-error (+ "unexpected attribute '" (+ kk "'"))))))
                        None))
                    None)
                  (.update env bindings)))))
	          (eval-ast (get func "body") env))
      (native-lazy? func)
        ((get func "func") arg-delay)
      (callable func)
        (func (force-value arg-delay))
      True
        (pnix-error "apply target is not a function")))

  (defn foldl-builtin [func init xs]
    (if (= (len xs) 0)
      init
      (do
        (setv first-item (get xs 0))
        (setv step (apply-pnix func (make-value-thunk init)))
        (foldl-builtin
          func
          (apply-pnix step (make-value-thunk first-item))
          (cut xs 1 None)))))

  (defn foldr-builtin-index [func xs i acc]
    (if (< i 0)
      acc
      (do
        (setv step (apply-pnix func (make-value-thunk (get xs i))))
        (foldr-builtin-index
          func
          xs
          (- i 1)
          (apply-pnix step (make-value-thunk acc))))))

  (defn foldr-builtin [func init xs]
    (foldr-builtin-index func xs (- (len xs) 1) init))

  (defn map-builtin [func xs]
    (if (= (len xs) 0)
      []
      (+ [(make-thunk (fn [] (apply-pnix func (make-value-thunk (get xs 0)))))]
         (map-builtin func (cut xs 1 None)))))

  (defn filter-builtin-index [func xs index]
    (if (= (len xs) 0)
      []
      (do
        (setv first-item (get xs 0))
        (setv rest-items (filter-builtin-index func (cut xs 1 None) (+ index 1)))
        (if (predicate-bool-value
              (apply-pnix func (make-value-thunk first-item))
              "builtins.filter predicate"
              index)
          (+ [(force-value first-item)] rest-items)
          rest-items))))

  (defn filter-builtin [func xs]
    (filter-builtin-index func xs 0))

  (defn any-builtin-index [pred xs index]
    (if (= (len xs) 0)
      False
      (if (predicate-bool-value
            (apply-pnix pred (make-value-thunk (get xs 0)))
            "builtins.any predicate"
            index)
        True
        (any-builtin-index pred (cut xs 1 None) (+ index 1)))))

  (defn any-builtin [pred xs]
    (any-builtin-index pred xs 0))

  (defn all-builtin-index [pred xs index]
    (if (= (len xs) 0)
      True
      (if (predicate-bool-value
            (apply-pnix pred (make-value-thunk (get xs 0)))
            "builtins.all predicate"
            index)
        (all-builtin-index pred (cut xs 1 None) (+ index 1))
        False)))

  (defn all-builtin [pred xs]
    (all-builtin-index pred xs 0))

  (defn force-list-items [xs]
    (if (= (len xs) 0)
      []
      (+ [(force-value (get xs 0))]
         (force-list-items (cut xs 1 None)))))

  (defn concat-lists-loop [xss i out]
    (if (>= i (len xss))
      out
      (do
        (setv xs (force-value (get xss i)))
        (if (not (isinstance xs list))
          (pnix-error (+ "builtins.concatLists: element at index " (+ (str i) (+ " is not a list, got " (type-of xs)))))
          None)
        (concat-lists-loop xss (+ i 1) (+ out xs)))))

  (defn concat-lists-builtin [xss]
    (do
      (setv forced (force-value xss))
      (if (not (isinstance forced list))
        (pnix-error (+ "builtins.concatLists: argument must be list, got " (type-of forced)))
        None)
      (concat-lists-loop forced 0 [])))

  (defn concat-map-builtin [func xs]
    (if (= (len xs) 0)
      []
      (+ (force-list-items
           (list-value
             (apply-pnix func (make-value-thunk (get xs 0)))
             "builtins.concatMap result"))
         (concat-map-builtin func (cut xs 1 None)))))

  (defn builtin-int-count [value label]
    (nonnegative-count-value value label))

  (defn take-builtin [count xs]
    (cut (list-value xs "builtins.take list") 0 (builtin-int-count count "builtins.take count")))

  (defn drop-builtin [count xs]
    (cut (list-value xs "builtins.drop list") (builtin-int-count count "builtins.drop count") None))

  (defn reverse-list-items [xs i out]
    (if (< i 0)
      out
      (reverse-list-items xs (- i 1) (+ out [(get xs i)]))))

  (defn reverse-list-builtin [xs]
    (reverse-list-items (list-value xs "builtins.reverseList list") (- (len (list-value xs "builtins.reverseList list")) 1) []))

  (defn zip-list-items [left right i]
    (if (or (>= i (len left)) (>= i (len right)))
      []
      (+ [[(force-value (get left i)) (force-value (get right i))]]
         (zip-list-items left right (+ i 1)))))

  (defn zip-builtin [left right]
    (zip-list-items (list-value left "builtins.zip lhs") (list-value right "builtins.zip rhs") 0))

  (defn flatten-one [item out]
    (if (isinstance item list)
      (flatten-items item 0 out)
      (+ out [item])))

  (defn flatten-items [xs i out]
    (if (>= i (len xs))
      out
      (flatten-items xs (+ i 1) (flatten-one (force-value (get xs i)) out))))

  (defn flatten-builtin [xs]
    (flatten-items (list-value xs "builtins.flatten list") 0 []))

  (defn find-items [needle xs i]
    (if (>= i (len xs))
      None
      (if (eq-value-at-depth needle (force-value (get xs i)) 1)
        (force-value (get xs i))
        (find-items needle xs (+ i 1)))))

  (defn find-builtin [needle xs]
    (find-items needle (list-value xs "builtins.find list") 0))

  (defn get-builtin [attrs name]
    (do
      (setv source (attrset-value attrs "builtins.get attrs"))
      (setv key (string-value name "builtins.get name"))
      (if (in key source) (force-value (get source key)) None)))

  (defn set-builtin [attrs name value]
    (do
      (setv out (dict (attrset-value attrs "builtins.set attrs")))
      (setv (get out (string-value name "builtins.set name")) value)
      out))

  (defn get-attrs-items [names attrs i out]
    (if (>= i (len names))
      out
      (do
        (setv key (string-value (get names i) "builtins.getAttrs name"))
        (if (not (in key attrs))
          (pnix-error (+ "builtins.getAttrs: attribute '" (+ key "' missing in set")))
          None)
        (setv (get out key) (get attrs key))
        (get-attrs-items names attrs (+ i 1) out))))

  (defn get-attrs-builtin [names attrs]
    (get-attrs-items
      (list-value names "builtins.getAttrs names")
      (attrset-value attrs "builtins.getAttrs attrs")
      0
      {}))

  (defn concat-strings-items [items i out context]
    (if (>= i (len items))
      (make-context-string out context)
      (do
        (setv pair
          (string-text-context
            (get items i)
            (+ "builtins.concatStrings element at index " (+ (str i) " is not a string"))))
        (concat-strings-items
          items
          (+ i 1)
          (+ out (get pair 0))
          (merge-contexts context (get pair 1))))))

  (defn concat-strings-builtin [xs]
    (concat-strings-items (list-arg-value xs "concatStrings" "argument") 0 "" (set [])))

  (defn concat-strings-sep-items [sep-text sep-context items i out context]
    (if (>= i (len items))
      (make-context-string out context)
      (do
        (setv pair
          (string-text-context
            (get items i)
            (+ "builtins.concatStringsSep element at index " (+ (str i) " is not a string"))))
        (concat-strings-sep-items
          sep-text
          sep-context
          items
          (+ i 1)
          (+ (+ out (if (> i 0) sep-text "")) (get pair 0))
          (merge-contexts
            (merge-contexts context (if (> i 0) sep-context (set [])))
            (get pair 1))))))

  (defn concat-strings-sep-builtin [sep xs]
    (do
      (setv sf (force-value sep))
      (if (not (isinstance sf str))
        (pnix-error (+ "builtins.concatStringsSep: separator must be string, got " (type-of sf)))
        None)
      (setv sep-pair (string-text-context sf "builtins.concatStringsSep separator"))
      (concat-strings-sep-items
        (get sep-pair 0)
        (get sep-pair 1)
        (list-value xs "builtins.concatStringsSep list")
        0
        ""
        (set []))))

  (defn context-string-builtin [value label]
    (do
      (setv vf (force-value value))
      (if (not (isinstance vf str))
        (pnix-error (+ "builtins." (+ label (+ ": expected string, got " (type-of vf)))))
        None)
      (setv pair (string-text-context vf (+ "builtins." (+ label " string"))))
      (setv text (get pair 0))
      (setv context (get pair 1))
      (cond
        (= label "addDrvOutputDependencies")
          (.add context (+ "!out!" text))
        (= label "unsafeDiscardOutputDependency")
          (do
            (setv filtered (set []))
            (for [item context]
              (if (or (.startswith item "!out!") (.startswith item "!") (.startswith item "="))
                None
                (.add filtered item)))
            (setv context filtered))
        (= label "unsafeAddOutputDependency")
          (do
            (for [item (list context)]
              (if (or (.startswith item "!") (.startswith item "="))
                None
                (.add context (+ "!out!" item)))))
        True None)
      (make-context-string text context)))

  (defn unsafe-add-output-name-builtin [name value]
    (do
      (setv nf (force-value name))
      (if (not (isinstance nf str))
        (pnix-error (+ "builtins.unsafeAddOutputName: first arg must be string, got " (type-of nf)))
        None)
      (setv output-name (str nf))
      (setv vf (force-value value))
      (if (not (isinstance vf str))
        (pnix-error (+ "builtins.unsafeAddOutputName: second arg must be string, got " (type-of vf)))
        None)
      (setv pair (string-text-context vf "builtins.unsafeAddOutputName string"))
      (setv context (get pair 1))
      (for [item (list context)]
        (if (or (.startswith item "!") (.startswith item "="))
          None
          (.add context (+ (+ (+ "!" output-name) "!") item))))
      (make-context-string (get pair 0) context)))

  (defn get-context-builtin [value]
    (do
      (setv pair (string-text-context value "builtins.getContext string"))
      (setv out {})
      (for [item (sorted (get pair 1))]
        (setv (get out item) {"path" True}))
      out))

  (defn append-outputs-check [key outs i]
    (if (>= i (len outs))
      None
      (do
        (setv of (force-value (get outs i)))
        (if (not (isinstance of str))
          (pnix-error (+ "builtins.appendContext: '" (+ key (+ "'.outputs element at index " (+ (str i) (+ " is not a string, got " (type-of of)))))))
          None)
        (append-outputs-check key outs (+ i 1)))))

  (defn append-context-builtin [value extra]
    (do
      (setv pair (string-text-context value "builtins.appendContext string"))
      (setv text (get pair 0))
      (setv context (get pair 1))
      (setv source (attrset-value extra "builtins.appendContext context"))
      (for [key source]
        (do
          (setv spec (force-value (get source key)))
          (if (not (and (isinstance spec dict) (not (closure? spec)) (not (thunk? spec)) (not (path? spec)) (not (construct? spec))))
            (pnix-error (+ "builtins.appendContext: context value for '" (+ key (+ "' must be an attrset, got " (type-of spec)))))
            None)
          (if (in "path" spec)
            (do
              (setv pf (force-value (get spec "path")))
              (if (not (isinstance pf bool))
                (pnix-error (+ "builtins.appendContext: '" (+ key (+ "'.path must be bool, got " (type-of pf)))))
                None))
            None)
          (if (in "allOutputs" spec)
            (do
              (setv af (force-value (get spec "allOutputs")))
              (if (not (isinstance af bool))
                (pnix-error (+ "builtins.appendContext: '" (+ key (+ "'.allOutputs must be bool, got " (type-of af)))))
                None))
            None)
          (if (in "outputs" spec)
            (do
              (setv outs (force-value (get spec "outputs")))
              (if (not (isinstance outs list))
                (pnix-error (+ "builtins.appendContext: '" (+ key (+ "'.outputs must be list of strings, got " (type-of outs)))))
                None)
              (append-outputs-check key outs 0))
            None)
          (.add context key)))
      (make-context-string text context)))

  (defn finite-builtin [value]
    (do
      (setv v (force-value value))
      (cond
        (and (isinstance v int) (not (isinstance v bool))) True
        (isinstance v float) (math.isfinite v)
        True False)))

  (defn inf-builtin [value]
    (do
      (setv v (force-value value))
      (if (isinstance v float) (math.isinf v) False)))

  (defn nan-builtin [value]
    (do
      (setv v (force-value value))
      (if (isinstance v float) (math.isnan v) False)))

  (defn derivation-name [source label]
    (if (in "name" source)
      (string-value (get source "name") (+ "builtins." (+ label " name")))
      "unnamed"))

  (defn derivation-builtin [attrs label]
    (do
      (setv source (attrset-value attrs (+ "builtins." (+ label " attrs"))))
      (setv out (dict source))
      (setv name (derivation-name source label))
      (setv placeholder (make-context-string
                          (+ "/pnix-placeholder/derivation/" name)
                          (set [(+ "!out!" name)])))
      (if (not (in "outPath" out)) (setv (get out "outPath") placeholder) None)
      (if (not (in "drvPath" out)) (setv (get out "drvPath") placeholder) None)
      (if (not (in "type" out)) (setv (get out "type") "derivation") None)
      out))

  (defn fs-path-text [value label]
    (do
      (setv value (force-value value))
      (cond
        (path? value) (get value "value")
        (isinstance value str) value
        True (pnix-error (+ label ": expected string or path (expected path or string)")))))

  (defn fs-path [value label]
    (do
      (setv text (fs-path-text value label))
      (if (= text "")
        (pnix-error (+ label ": empty string is not a valid path"))
        None)
      (os.path.abspath (os.path.expanduser (normalize-pnix-path-text text)))))

  (defn to-path-string [value]
    (do
      (setv tc (string-text-context value "builtins.toPath string"))
      (setv text (get tc 0))
      (setv ctx (get tc 1))
      (if (not (.startswith text "/"))
        (pnix-error (+ (+ "string '" text) "' doesn't represent an absolute path"))
        None)
      (make-context-string (normalize-pnix-path-text text) ctx)))

  (defn file-type-builtin [path]
    (cond
      (os.path.islink path) "symlink"
      (os.path.isdir path) "directory"
      (os.path.isfile path) "regular"
      True "unknown"))

  (defn read-file-type-builtin [path]
    (do
      (if (not (os.path.lexists path))
        (pnix-error (+ (+ "builtins.readFileType: failed to get metadata for `" path) "`: No such file or directory"))
        None)
      (file-type-builtin path)))

  (defn read-file-builtin [value]
    (do
      (setv path (fs-path value "builtins.readFile"))
      (try
        (do
          (setv f (open path "r" :encoding "utf-8"))
          (setv text (.read f))
          (.close f)
          text)
        (except [OSError exc]
          (pnix-error (+ (+ (+ "builtins.readFile: failed to read `" path) "`: ") (str exc)))))))

  (defn read-dir-items [path names i out]
    (if (>= i (len names))
      out
      (do
        (setv name (get names i))
        (setv (get out name) (file-type-builtin (os.path.join path name)))
        (read-dir-items path names (+ i 1) out))))

  (defn read-dir-builtin [value]
    (do
      (setv path (fs-path value "builtins.readDir"))
      (try
        (read-dir-items path (sorted (os.listdir path)) 0 {})
        (except [OSError exc]
          (pnix-error (+ (+ (+ "builtins.readDir: failed to read `" path) "`: ") (str exc)))))))

  (defn safe-store-char [c]
    (if (or (.isalnum c) (in c ".-_+")) c "_"))

  (defn safe-store-name-rec [name i out]
    (if (>= i (len name))
      out
      (safe-store-name-rec name (+ i 1) (+ out (safe-store-char (get name i))))))

  (defn safe-store-name [name]
    (do
      (setv out (safe-store-name-rec name 0 ""))
      (if (= out "") "unnamed" out)))

  (defn to-file-builtin [name contents]
    (do
      (setv nf (force-value name))
      (if (not (isinstance nf str))
        (pnix-error (+ "builtins.toFile: first argument must be string, got " (type-of nf)))
        None)
      (setv file-name (str nf))
      (setv content-value (force-value contents))
      (if (not (isinstance content-value str))
        (pnix-error (+ "builtins.toFile: second argument must be string, got " (type-of content-value)))
        None)
      (setv contents-pair (string-text-context content-value "builtins.toFile contents"))
      (setv text (get contents-pair 0))
      (if (> (len (get contents-pair 1)) 0)
        (pnix-error "builtins.toFile: contents must not have string context; use builtins.unsafeDiscardStringContext to discard it")
        None)
      (setv digest (cut (.hexdigest (hashlib.sha256 (.encode text "utf-8"))) 0 32))
      (setv store-dir (os.path.join (tempfile.gettempdir) "pnix-nix-store"))
      (os.makedirs store-dir :exist_ok True)
      (setv out (os.path.abspath (os.path.join store-dir (+ (+ digest "-") (safe-store-name file-name)))))
      (setv f (open out "w" :encoding "utf-8"))
      (.write f text)
      (.close f)
      (make-path out)))

  (defn hash-bytes-builtin [algo data label allow-legacy]
    (cond
      (and allow-legacy (= algo "md5")) (.hexdigest (hashlib.md5 data :usedforsecurity False))
      (and allow-legacy (= algo "sha1")) (.hexdigest (hashlib.sha1 data :usedforsecurity False))
      (= algo "sha256") (.hexdigest (hashlib.sha256 data))
      (= algo "sha512") (.hexdigest (hashlib.sha512 data))
      (in algo ["md5" "sha1"]) (pnix-error (+ (+ (+ (+ label ": algorithm '" algo) "' is not supported (`") algo) "`); cryptographically broken; use 'sha256' or 'sha512'"))
      True (pnix-error (+ (+ (+ (+ label ": unsupported algorithm '" algo) "' (`") algo) (if allow-legacy "`); supported: 'md5', 'sha1', 'sha256', 'sha512'" "`); supported: 'sha256', 'sha512'")))))

  (defn hash-string-algorithm [algo]
    (do
      (setv algo-pair (plain-string-text-context algo "builtins.hashString algo"))
      (setv algorithm (get algo-pair 0))
      (if (> (len (get algo-pair 1)) 0)
        (pnix-error (+ (+ "builtins.hashString algo: the string '" algorithm) "' is not allowed to refer to a store path"))
        None)
      (if (in algorithm ["md5" "sha1" "sha256" "sha512"])
        algorithm
        (hash-bytes-builtin algorithm (.encode "" "utf-8") "builtins.hashString" True))))

  (defn hash-string-builtin [algorithm data]
    (do
      (setv data-pair (plain-string-text-context data "builtins.hashString data"))
      (hash-bytes-builtin
        algorithm
        (.encode (get data-pair 0) "utf-8" "surrogateescape")
        "builtins.hashString"
        True)))

  (defn hash-string-function [algo]
    (do
      (setv algorithm (hash-string-algorithm algo))
      (make-native-lazy (fn [data] (hash-string-builtin algorithm data)))))

  (defn hash-file-builtin [algo value]
    (do
      (setv path-arg (force-value value))
      (setv path-ctx (if (path? path-arg) (set [(get path-arg "value")]) (string-context path-arg)))
      (setv f (open (fs-path path-arg "builtins.hashFile") "rb"))
      (setv data (.read f))
      (.close f)
      (make-context-string
        (hash-bytes-builtin
          (plain-string-value algo "builtins.hashFile algo")
          data
          "builtins.hashFile"
          False)
        path-ctx)))

  (defn trim-trailing-slash [text]
    (if (and (!= text "") (!= text "/") (.endswith text "/"))
      (cut text 0 -1)
      text))

  (defn base-name-builtin [value]
    (do
      (setv forced (force-value value))
      (setv ctx (if (path? forced) (set []) (string-context forced)))
      (setv text (trim-trailing-slash (fs-path-text forced "builtins.baseNameOf")))
      (make-context-string
        (if (or (= text "") (= text "/"))
          ""
          (get (.rsplit text "/" 1) -1))
        ctx)))

  (defn dir-of-builtin [value]
    (do
      (setv forced (force-value value))
      (setv text (trim-trailing-slash (fs-path-text forced "builtins.dirOf")))
      (setv dir
        (cond
          (= text "/") "/"
          (not (in "/" text)) "."
          True
            (do
              (setv head (get (.rsplit text "/" 1) 0))
              (if (= head "") "/" head))))
      (if (path? forced)
        (make-path dir)
        (make-context-string dir (string-context forced)))))

  (defn get-env-builtin [name]
    (do
      (setv key (string-arg-value name "getEnv"))
      (if (.startswith key "PNIX_") (.get os.environ key "") "")))

  (defn gen-list-builtin [func index count]
    (if (>= index count)
      []
      (+ [(make-thunk (fn [] (apply-pnix func (make-value-thunk index))))]
         (gen-list-builtin func (+ index 1) count))))

  (defn checked-gen-list [func count]
    (do
      (setv n (nonnegative-count-value count "builtins.genList length"))
      (if (> n (* 16 1024 1024))
        (pnix-error (+ (+ "builtins.genList count " (str n)) " exceeds maximum 16777216"))
        (gen-list-builtin func 0 n))))

  (defn attr-values-from-keys [m keys]
    (if (= (len keys) 0)
      []
      (+ [(get m (get keys 0))]
         (attr-values-from-keys m (cut keys 1 None)))))

  (defn map-attrs-from-keys [func m keys]
    (if (= (len keys) 0)
      {}
      (do
        (setv key (get keys 0))
        (setv out (map-attrs-from-keys func m (cut keys 1 None)))
        (setv (get out key)
              (make-thunk
                (fn []
                  (apply-pnix
                    (apply-pnix func (make-value-thunk key))
                    (make-value-thunk (get m key))))))
        out)))

  (defn filter-attrs-from-keys [func m keys]
    (if (= (len keys) 0)
      {}
      (do
        (setv key (get keys 0))
        (setv out (filter-attrs-from-keys func m (cut keys 1 None)))
        (if (bool-value
              (apply-pnix
                (apply-pnix func (make-value-thunk key))
                (make-value-thunk (get m key)))
              "builtins.filterAttrs predicate")
          (setv (get out key) (get m key))
          None)
        out)))

  (defn intersect-attrs-from-keys [left right keys]
    (if (= (len keys) 0)
      {}
      (do
        (setv key (get keys 0))
        (setv out (intersect-attrs-from-keys left right (cut keys 1 None)))
        (if (in key left)
          (setv (get out key) (get right key))
          None)
        out)))

  (defn cat-attrs-from-list [name xs]
    (if (= (len xs) 0)
      []
      (do
        (setv item (attrset-value (get xs 0) "builtins.catAttrs element"))
        (setv rest (cat-attrs-from-list name (cut xs 1 None)))
        (if (in name item)
          (+ [(get item name)] rest)
          rest))))

  (defn fold-list-arg [xs]
    (do
      (setv v (force-value xs))
      (if (not (isinstance v list))
        (pnix-error (+ "builtins.fold: third arg must be list, got " (type-of v)))
        None)
      v))

  (defn list-arg-value [xs builtin position]
    (do
      (setv v (force-value xs))
      (if (not (isinstance v list))
        (pnix-error (+ "builtins." (+ builtin (+ ": " (+ position (+ " must be list, got " (type-of v)))))))
        None)
      v))

  (defn attrset-arg-value [value builtin phrase]
    (do
      (setv v (force-value value))
      (if (not (and (isinstance v dict) (not (closure? v)) (not (thunk? v)) (not (path? v)) (not (construct? v))))
        (pnix-error (+ "builtins." (+ builtin (+ ": " (+ phrase (+ ", got " (type-of v)))))))
        None)
      v))

  (defn string-arg-value [value builtin]
    (do
      (setv v (force-value value))
      (if (not (isinstance v str))
        (pnix-error (+ "builtins." (+ builtin (+ ": expected string, got " (type-of v)))))
        None)
      (str v)))

  (defn get-attr-builtin [name m]
    (do
      (setv k (str (force-value name)))
      (setv d (attrset-arg-value m "getAttr" "expected attrset"))
      (if (not (in k d))
        (pnix-error (+ "builtins.getAttr: attribute '" (+ k "' missing")))
        None)
      (force-value (get d k))))

  (defn group-by-list-arg [xs]
    (do
      (setv v (force-value xs))
      (if (not (isinstance v list))
        (pnix-error (+ "builtins.groupBy: second argument must be list, got " (type-of v)))
        None)
      v))

  (defn group-by-items [func xs out]
    (if (= (len xs) 0)
      out
      (do
        (setv item (get xs 0))
        (setv keyv (force-value (apply-pnix func (make-value-thunk item))))
        (if (not (isinstance keyv str))
          (pnix-error (+ "builtins.groupBy: key function must return string, got " (type-of keyv)))
          None)
        (setv key (str keyv))
        (setv current (.get out key []))
        (setv (get out key) (+ current [(force-value item)]))
        (group-by-items func (cut xs 1 None) out))))

  (defn partition-items [pred xs right wrong]
    (if (= (len xs) 0)
      {"right" right "wrong" wrong}
      (do
        (setv item (get xs 0))
        (if (bool-value (apply-pnix pred (make-value-thunk item)) "builtins.partition predicate")
          (partition-items pred (cut xs 1 None) (+ right [(force-value item)]) wrong)
          (partition-items pred (cut xs 1 None) right (+ wrong [(force-value item)]))))))

  (defn generic-closure-key-signature [value]
    (.hexdigest
      (hashlib.sha256
        (.encode (canonical-json (realize-value value)) "utf-8"))))

  (defn generic-closure-mark-seen [seen signature]
    (do
      (setv (get seen signature) True)
      seen))

  (defn generic-closure-check-limits [work steps]
    (do
      (if (> steps 10000)
        (pnix-error "builtins.genericClosure: maximum depth 10000 exceeded")
        None)
      (if (> (len work) 100000)
        (pnix-error "builtins.genericClosure: work list size 100000 exceeded")
        None)
      True))

  (defn generic-closure-step-next [operator rest seen out steps item next-items]
    (generic-closure-loop
      operator
      (+ rest (reverse-list-builtin next-items))
      seen
      (+ out [item])
      (+ steps 1)))

  (defn generic-closure-step-attrs [operator rest seen out steps item attrs]
    (if (not (in "key" attrs))
      (pnix-error "builtins.genericClosure: item missing 'key' attribute")
      (if (in (generic-closure-key-signature (get attrs "key")) seen)
        (generic-closure-loop operator rest seen out (+ steps 1))
        (generic-closure-step-next
          operator
          rest
          (generic-closure-mark-seen seen (generic-closure-key-signature (get attrs "key")))
          out
          steps
          item
          (list-value
            (apply-pnix operator (make-value-thunk item))
            "builtins.genericClosure operator result")))))

  (defn generic-closure-step [operator work seen out steps item]
    (do
      (generic-closure-check-limits work steps)
      (generic-closure-step-attrs
        operator
        (cut work 0 (- (len work) 1))
        seen
        out
        steps
        item
        (attrset-value item "builtins.genericClosure item"))))

  (defn generic-closure-loop [operator work seen out steps]
    (if (= (len work) 0)
      out
      (generic-closure-step
        operator
        work
        seen
        out
        steps
        (force-value (get work (- (len work) 1))))))

  (defn generic-closure-builtin [arg]
    (do
      (setv attrs (attrset-value arg "builtins.genericClosure argument"))
      (if (not (in "startSet" attrs))
        (pnix-error "builtins.genericClosure: argument missing required attribute 'startSet'")
        None)
      (if (not (in "operator" attrs))
        (pnix-error "builtins.genericClosure: argument missing required attribute 'operator'")
        None)
      (generic-closure-loop
        (get attrs "operator")
        (list-value (get attrs "startSet") "builtins.genericClosure startSet")
        {}
        []
        0)))

  (defn function-args-pattern [pattern]
    (if (and (isinstance pattern dict) (= (.get pattern "tag" None) "as"))
      (function-args-pattern (get pattern "pattern"))
      pattern))

  (defn function-args-fields [fields i out]
    (if (>= i (len fields))
      out
      (do
        (setv (get out (get (get fields i) "name")) (in "default" (get fields i)))
        (function-args-fields fields (+ i 1) out))))

  (defn function-args-forced [value]
    (cond
      (closure? value)
        (function-args-pattern-result (function-args-pattern (.get value "pattern" None)))
      (or (native-lazy? value) (callable value)) {}
      True (pnix-error (+ "builtins.functionArgs: expected function, got " (type-of value)))))

  (defn function-args-pattern-result [pattern]
    (if (and (isinstance pattern dict) (= (.get pattern "tag" None) "attrset"))
      (function-args-fields (get pattern "fields") 0 {})
      {}))

  (defn function-args-builtin [value]
    (function-args-forced (force-value value)))

  (defn regex-groups [m context]
    (list (map
      (fn [g] (if (= g None) None (make-context-string g context)))
      (list (.groups m)))))

  (setv POSIX-REGEX-CLASSES
    {"[:alnum:]" "A-Za-z0-9"
     "[:alpha:]" "A-Za-z"
     "[:blank:]" "\\x09\\x20"
     "[:cntrl:]" "\\x00-\\x1f\\x7f"
     "[:digit:]" "0-9"
     "[:graph:]" "\\x21-\\x7e"
     "[:lower:]" "a-z"
     "[:print:]" "\\x20-\\x7e"
     "[:punct:]" "\\x21-\\x2f\\x3a-\\x40\\x5b-\\x60\\x7b-\\x7e"
     "[:space:]" "\\x09-\\x0d\\x20"
     "[:upper:]" "A-Z"
     "[:xdigit:]" "A-Fa-f0-9"})

  (defn regex-translate-loop [pattern i in-bracket bracket-has-member out]
    (if (>= i (len pattern))
      out
      (do
        (setv c (get pattern i))
        (cond
          (and (= c "\\") (< (+ i 1) (len pattern)))
            (regex-translate-loop
              pattern (+ i 2) in-bracket
              (if in-bracket True bracket-has-member)
              (+ out (cut pattern i (+ i 2))))
          (not in-bracket)
            (regex-translate-loop
              pattern (+ i 1) (= c "[") False (+ out c))
          (and (= c "[") (< (+ i 1) (len pattern)) (= (get pattern (+ i 1)) ":"))
            (do
              (setv end (.find pattern ":]" (+ i 2)))
              (if (< end 0)
                (raise (re.error "unterminated POSIX character class"))
                None)
              (setv marker (cut pattern i (+ end 2)))
              (setv replacement (.get POSIX-REGEX-CLASSES marker None))
              (if (= replacement None)
                (raise (re.error (+ "unknown POSIX character class '" (+ (cut pattern (+ i 2) end) "'"))))
                None)
              (regex-translate-loop pattern (+ end 2) True True (+ out replacement)))
          (and (= c "]") bracket-has-member)
            (regex-translate-loop pattern (+ i 1) False False (+ out c))
          True
            (regex-translate-loop
              pattern (+ i 1) True
              (if (and (= c "^") (not bracket-has-member)) False True)
              (+ out c))))))

  (defn regex-translate [pattern]
    (regex-translate-loop (str pattern) 0 False False ""))

  (defn invalid-regex-message [exc]
    (do
      (setv detail (str exc))
      (if (and (in "unterminated" detail) (not (in "unclosed" detail)))
        (setv detail (+ detail " (unclosed)"))
        None)
      (+ "invalid regex: " detail)))

  (defn regex-compile [pattern label]
    (try
      (re.compile (regex-translate pattern))
      (except [re.error exc]
        (pnix-error (+ label (+ ": " (invalid-regex-message exc)))))))

  (defn regex-match-builtin [pattern value]
    (do
      (setv pair (string-text-context value "builtins.match string"))
      (setv compiled (regex-compile (string-value pattern "builtins.match regex") "builtins.match"))
      (setv m (.fullmatch compiled (get pair 0)))
      (if (= m None) None (regex-groups m (get pair 1)))))

  (defn regex-split-loop [matches text context i last out]
    (if (>= i (len matches))
      (+ out [(make-context-string (cut text last None) context)])
      (do
        (setv m (get matches i))
        (regex-split-loop
          matches
          text
          context
          (+ i 1)
          (.end m)
          (+ out [(make-context-string (cut text last (.start m)) context) (regex-groups m context)])))))

  (defn regex-split-builtin [pattern value]
    (do
      (setv p (string-value pattern "builtins.split regex"))
      (if (= p "")
        (pnix-error "builtins.split: regex pattern cannot be empty")
        None)
      (setv pair (string-text-context value "builtins.split string"))
      (setv compiled (regex-compile p "builtins.split"))
      (regex-split-loop (list (.finditer compiled (get pair 0))) (get pair 0) (get pair 1) 0 0 [])))

  (defn zip-keys-from-map [keys i out]
    (if (>= i (len keys))
      out
      (zip-keys-from-map
        keys
        (+ i 1)
        (if (in (get keys i) out) out (+ out [(get keys i)])))))

  (defn zip-keys-from-maps [maps i out]
    (if (>= i (len maps))
      (sorted out)
      (zip-keys-from-maps
        maps
        (+ i 1)
        (zip-keys-from-map (list (.keys (get maps i))) 0 out))))

  (defn zip-values-for-key [maps key i]
    (if (>= i (len maps))
      []
      (if (in key (get maps i))
        (+ [(get (get maps i) key)]
           (zip-values-for-key maps key (+ i 1)))
        (zip-values-for-key maps key (+ i 1)))))

  (defn zip-attrs-with-keys [func maps keys i out]
    (if (>= i (len keys))
      out
      (do
        (setv key (get keys i))
        (setv (get out key)
          (make-thunk
            (fn []
              (apply-pnix
                (apply-pnix func (make-value-thunk key))
                (make-value-thunk (zip-values-for-key maps key 0))))))
        (zip-attrs-with-keys func maps keys (+ i 1) out))))

  (defn zip-attrs-with-builtin [func xs]
    (do
      (setv maps (list (map (fn [item] (attrset-arg-value item "zipAttrsWith" "list element must be attrset"))
                            (list-arg-value xs "zipAttrsWith" "second argument"))))
      (zip-attrs-with-keys func maps (zip-keys-from-maps maps 0 []) 0 {})))

  (defn try-eval-builtin [thunk]
    (try
      {"success" True "value" (force-value thunk)}
      (except [PnixCatchableError]
        {"success" False "value" False})))

  (defn from-json-int [token]
    (if (= token "-0")
      -0.0
      (do
        (setv parsed (int token))
        (if (not (and (>= parsed (- 0 (** 2 63))) (<= parsed (- (** 2 63) 1))))
          (pnix-error (+ "builtins.fromJSON: integer literal too large for i64: " token))
          parsed))))

  (defn from-json-constant [token]
    (pnix-error (+ "builtins.fromJSON: invalid JSON numeric constant " token)))

  (defn from-json-builtin [value]
    (try
      (json.loads
        (string-value value "builtins.fromJSON string")
        :parse_int from-json-int
        :parse_constant from-json-constant)
      (except [json.JSONDecodeError exc]
        (pnix-error (+ "builtins.fromJSON: parse error: " (str exc))))))

  (defn bit-int-arg [value name position]
    (do
      (setv v (force-value value))
      (if (or (not (isinstance v int)) (isinstance v bool))
        (pnix-error (+ "builtins." (+ name (+ ": " (+ position (+ " arg must be int, got " (type-of v)))))))
        None)
      v))

  (defn bit-and-builtin [left right]
    (& (bit-int-arg left "bitAnd" "first") (bit-int-arg right "bitAnd" "second")))

  (defn bit-or-builtin [left right]
    (| (bit-int-arg left "bitOr" "first") (bit-int-arg right "bitOr" "second")))

  (defn bit-xor-builtin [left right]
    (^ (bit-int-arg left "bitXor" "first") (bit-int-arg right "bitXor" "second")))

  (defn add-error-context-builtin [message value]
    (do
      (setv m (force-value message))
      (if (not (isinstance m str))
        (pnix-error (+ "builtins.addErrorContext: context must be string, got " (type-of m)))
        None)
      value))

  (defn unsafe-get-attr-pos-builtin [name attrs]
    (do
      (setv attr-name (string-value name "builtins.unsafeGetAttrPos name"))
      (setv source (attrset-value attrs "builtins.unsafeGetAttrPos attrs"))
      (if (in attr-name source)
        (do
          (setv pos (.get (attrset-positions source) attr-name None))
          (if (= pos None)
            None
            (source-position-value pos)))
        None)))

  (defn pow-builtin [left right]
    (do
      (setv l (force-value left))
      (setv r (force-value right))
      (if (and (isinstance l int) (not (isinstance l bool))
               (isinstance r int) (not (isinstance r bool)) (>= r 0))
        (do
          (setv exact (pow l r))
          (if (and (>= exact (- 0 (** 2 63))) (<= exact (- (** 2 63) 1)))
            exact
            (math.pow (number-value l "builtins.pow base")
                      (number-value r "builtins.pow exponent"))))
        (math.pow (number-value l "builtins.pow base")
                  (number-value r "builtins.pow exponent")))))

  (defn check-i64 [v op]
    ;; pnix ints are i64 with checked overflow (~/pnix); float is unchecked.
    (do
      (if (and (isinstance v int) (not (isinstance v bool))
               (not (and (>= v (- 0 (** 2 63))) (<= v (- (** 2 63) 1)))))
        (pnix-error (+ "integer overflow in `" (+ op "`")))
        None)
      v))

  (defn checked-float-to-i64 [value label func]
    (do
      (setv v (number-value value (+ label " argument")))
      (if (and (isinstance v int) (not (isinstance v bool)))
        (do
          (setv as-float (float v))
          (if (or (>= as-float (float (** 2 63))) (< as-float (float (- 0 (** 2 63)))))
            (pnix-error (+ label ": integer outside i64 range after f64 conversion"))
            None)
          (if (!= (int as-float) v)
            (pnix-error (+ label ": integer loses precision when converted to f64"))
            None)
          (setv v as-float))
        None)
      (if (math.isnan v)
        (pnix-error (+ label ": NaN outside i64 range"))
        None)
      (if (math.isinf v)
        (pnix-error (+ label (if (> v 0) ": +inf outside i64 range" ": -inf outside i64 range")))
        None)
      (setv rounded (func v))
      (if (or (>= rounded (** 2 63)) (< rounded (- 0 (** 2 63))))
        (pnix-error (+ label ": value outside i64 range"))
        None)
      (check-i64 (int rounded) label)))

  (defn unary-neg-value [value]
    (do
      (setv v (number-value value "argument of unary -"))
      (if (and (isinstance v float) (= v 0.0))
        0.0
        (check-i64 (- 0 v) "-"))))

  (defn add-builtin [left right]
    (do
      (setv l (force-value left))
      (setv r (force-value right))
      (cond
        (and (isinstance l str) (isinstance r str)) (+ l r)
        (and (isinstance l list) (isinstance r list)) (+ l r)
        (and (isinstance l dict) (isinstance r dict)) (merge-attrsets l r)
        True (check-i64 (+ (number-value l "left side of builtins.add")
                           (number-value r "right side of builtins.add")) "+"))))

  (defn mod-builtin [left right left-label right-label zero-msg]
    (do
      (setv l (number-value left left-label))
      (setv r (number-value right right-label))
      (if (= r 0)
        (pnix-error zero-msg)
        None)
      (if (or (isinstance l float) (isinstance r float))
        (math.fmod l r)
        (do
          (if (and (= l (- 0 (** 2 63))) (= r -1))
            (pnix-error "integer overflow in `%`")
            None)
          (setv q (// (abs l) (abs r)))
          (if (!= (< l 0) (< r 0))
            (setv q (- 0 q))
            None)
          (check-i64 (- l (* q r)) "%")))))

  (defn div-builtin [left right left-label right-label]
    (do
      (setv l (number-value left left-label))
      (setv r (number-value right right-label))
      (if (= r 0)
        (pnix-error "division by zero")
        None)
      (if (or (isinstance l float) (isinstance r float))
        (/ l r)
        (do
          (setv q (// (abs l) (abs r)))
          (if (!= (< l 0) (< r 0))
            (setv q (- 0 q))
            None)
          (check-i64 q "/")))))

  (defn elem-at-builtin [xs index]
    (do
      (setv items (list-value xs "builtins.elemAt list"))
      (setv i (integer-value index "builtins.elemAt index"))
      (cond
        (< i 0) (pnix-error "builtins.elemAt: negative index")
        (>= i (len items)) (pnix-error "builtins.elemAt: index out of bounds")
        True (force-value (get items i)))))

  (defn head-builtin [xs]
    (do
      (setv items (list-value xs "builtins.head list"))
      (if (= (len items) 0)
        (pnix-error "builtins.head: list is empty")
        (force-value (get items 0)))))

  (defn tail-builtin [xs]
    (do
      (setv items (list-value xs "builtins.tail list"))
      (if (= (len items) 0)
        (pnix-error "builtins.tail: list is empty")
        (cut items 1 None))))

  (defn compare-with [pred]
    (fn [left right]
	      (setv left-less
	            (predicate-bool-value
	              (apply-pnix
	                (apply-pnix pred (make-thunk (fn [] left)))
	                (make-thunk (fn [] right)))
	              "builtins.sort comparator"
	              None))
	      (setv right-less
	            (predicate-bool-value
	              (apply-pnix
	                (apply-pnix pred (make-thunk (fn [] right)))
	                (make-thunk (fn [] left)))
	              "builtins.sort comparator"
	              None))
      (if left-less
        -1
        (if right-less 1 0))))

  (defn native-builtins []
    (do
      (setv b {"currentSystem" (current-system)
     "nixVersion" "2.18.0-pnix"
     "langVersion" 6
     "storeDir" "/nix/store"
     "import" (fn [path] (pnix-error "builtins.import requires run_px/run_px_source host file context"))
     "scopedImport" (fn [scope] (fn [path] (pnix-error "builtins.scopedImport requires run_px/run_px_source host file context")))
     "pathExists" (fn [path] (os.path.exists (fs-path path "builtins.pathExists")))
     "readFile" (fn [path] (read-file-builtin path))
     "readFileType" (fn [path] (read-file-type-builtin (fs-path path "builtins.readFileType")))
     "readDir" (fn [path] (read-dir-builtin path))
     "toFile" (fn [name] (fn [contents] (to-file-builtin name contents)))
     "hashString" (fn [algo] (hash-string-function algo))
     "hashFile" (fn [algo] (fn [path] (hash-file-builtin algo path)))
     "baseNameOf" (fn [path] (base-name-builtin path))
     "dirOf" (fn [path] (dir-of-builtin path))
     "toPath" (fn [value] (to-path-string value))
     "storePath" (fn [value] (make-path (fs-path value "builtins.storePath")))
     "getEnv" (fn [name] (get-env-builtin name))
     "placeholder" (fn [name]
       (do
         (setv n (string-value name "builtins.placeholder name"))
         (make-context-string (+ "/pnix-placeholder/" n) (set [(+ "=placeholder!" n)]))))
     "break" (fn [value] value)
     "warn" (fn [msg] (fn [value] (do (string-value msg "builtins.warn message") value)))
     "traceVerbose" (fn [msg] (fn [value] (do (string-value msg "builtins.traceVerbose message") value)))
     "attrNames" (fn [m] (sorted (.keys (attrset-arg-value m "attrNames" "expected attrset"))))
     "hasAttr" (fn [name] (fn [m] (has-attr-builtin name m)))
     "getAttr" (fn [name] (fn [m] (get-attr-builtin name m)))
     "attrByPath" (make-native-lazy
       (fn [path]
         (make-native-lazy
           (fn [default]
             (fn [m] (attr-by-path-builtin path default m))))))
     "removeAttrs" (fn [m] (fn [names] (remove-attrs-builtin m names)))
     "listToAttrs" (fn [xs] (list-to-attrs-builtin xs))
     "functionArgs" (fn [value] (function-args-builtin value))
     "filterAttrs" (fn [f] (fn [m]
       (do
         (setv mm (attrset-value m "builtins.filterAttrs set"))
         (filter-attrs-from-keys f mm (sorted (.keys mm))))))
     "intersectAttrs" (fn [left] (fn [right]
       (do
         (setv l (attrset-value left "builtins.intersectAttrs lhs"))
         (setv r (attrset-value right "builtins.intersectAttrs rhs"))
         (intersect-attrs-from-keys l r (sorted (.keys r))))))
     "zipAttrsWith" (fn [f] (fn [xs] (zip-attrs-with-builtin f xs)))
     "catAttrs" (fn [name] (fn [xs]
       (cat-attrs-from-list (string-value name "builtins.catAttrs name") (list-value xs "builtins.catAttrs list"))))
     "elemAt" (fn [xs] (fn [i] (elem-at-builtin xs i)))
     "length" (fn [x] (length-builtin x))
     "head" (fn [xs] (head-builtin xs))
     "tail" (fn [xs] (tail-builtin xs))
     "toString" (fn [x] (value-to-string x))
     "toJSON" (fn [x] (to-json-builtin x))
     "map" (fn [f] (fn [xs] (map-builtin f (list-value xs "builtins.map list"))))
     "filter" (fn [f] (fn [xs] (filter-builtin f (list-arg-value xs "filter" "second argument"))))
     "foldl'" (fn [f] (fn [init] (fn [xs]
       (foldl-builtin f init (list-arg-value xs "foldl'" "third arg")))))
     "fold" (fn [f] (fn [init] (fn [xs]
       (foldl-builtin f init (list-arg-value xs "fold" "third arg")))))
     "foldl" (fn [f] (fn [init] (fn [xs]
       (foldl-builtin f init (list-arg-value xs "foldl" "third arg")))))
     "foldr" (fn [f] (fn [init] (fn [xs]
       (foldr-builtin f init (list-arg-value xs "foldr" "third arg")))))
     "cons" (fn [x] (fn [xs] (+ [(force-value x)] (list-value xs "builtins.cons list"))))
     "append" (fn [xs] (fn [ys] (+ (list-value xs "builtins.append lhs") (list-value ys "builtins.append rhs"))))
     "take" (fn [count] (fn [xs] (take-builtin count xs)))
     "drop" (fn [count] (fn [xs] (drop-builtin count xs)))
     "reverse" (fn [xs] (reverse-list-builtin xs))
     "reverseList" (fn [xs] (reverse-list-builtin xs))
     "zip" (fn [left] (fn [right] (zip-builtin left right)))
     "flatten" (fn [xs] (flatten-builtin xs))
     "find" (fn [needle] (fn [xs] (find-builtin needle xs)))
     "get" (fn [attrs] (fn [name] (get-builtin attrs name)))
     "mapGet" (fn [attrs] (fn [name] (get-builtin attrs name)))
     "set" (fn [attrs] (fn [name] (fn [value] (set-builtin attrs name value))))
     "mapSet" (fn [attrs] (fn [name] (fn [value] (set-builtin attrs name value))))
     "keys" (fn [attrs] (sorted (.keys (attrset-value attrs "builtins.keys attrs"))))
     "mapKeys" (fn [attrs] (sorted (.keys (attrset-value attrs "builtins.mapKeys attrs"))))
     "values" (fn [attrs]
       (do
         (setv mm (attrset-value attrs "builtins.values attrs"))
         (attr-values-from-keys mm (sorted (.keys mm)))))
     "mapValues" (fn [attrs]
       (do
         (setv mm (attrset-value attrs "builtins.mapValues attrs"))
         (attr-values-from-keys mm (sorted (.keys mm)))))
     "merge" (fn [left] (fn [right] (merge-attrsets left right)))
     "mapMerge" (fn [left] (fn [right] (merge-attrsets left right)))
     "elem" (fn [x] (fn [xs]
       (any (map (fn [item] (eq-value-at-depth x item 1)) (list-arg-value xs "elem" "second argument")))))
     "any" (fn [pred] (fn [xs] (any-builtin pred (list-arg-value xs "any" "second argument"))))
     "all" (fn [pred] (fn [xs] (all-builtin pred (list-arg-value xs "all" "second argument"))))
     "concatLists" (fn [xss] (concat-lists-builtin xss))
     "concatMap" (fn [f] (fn [xs] (concat-map-builtin f (list-value xs "builtins.concatMap list"))))
     "genList" (fn [f] (fn [n] (checked-gen-list f n)))
     "groupBy" (fn [f] (fn [xs] (group-by-items f (group-by-list-arg xs) {})))
     "partition" (fn [pred] (fn [xs] (partition-items pred (list-value xs "builtins.partition list") [] [])))
     "genericClosure" (fn [arg] (generic-closure-builtin arg))
     "attrValues" (fn [m]
       (do
         (setv mm (attrset-arg-value m "attrValues" "expected attrset"))
         (attr-values-from-keys mm (sorted (.keys mm)))))
     "getAttrs" (fn [names] (fn [attrs] (get-attrs-builtin names attrs)))
     "mapAttrs" (fn [f] (fn [m]
       (do
         (setv mm (attrset-value m "builtins.mapAttrs set"))
         (map-attrs-from-keys f mm (sorted (.keys mm))))))
     "sort" (fn [pred] (fn [xs]
       (sorted
         (sort-list-value xs)
         :key (functools.cmp_to_key (compare-with pred)))))
     "substring" (fn [start] (fn [length] (fn [s] (substring-builtin start length s))))
     "stringLength" (fn [s] (string-byte-length s "builtins.stringLength string"))
     "hasPrefix" (fn [prefix] (fn [s] (.startswith (string-value s "builtins.hasPrefix string")
                                                   (string-value prefix "builtins.hasPrefix prefix"))))
     "hasSuffix" (fn [suffix] (fn [s] (.endswith (string-value s "builtins.hasSuffix string")
                                                 (string-value suffix "builtins.hasSuffix suffix"))))
     "replaceStrings" (fn [froms] (fn [tos] (fn [s] (replace-strings-builtin froms tos s))))
     "concatStringsSep" (fn [sep] (fn [xs] (concat-strings-sep-builtin sep xs)))
     "concatStrings" (fn [xs] (concat-strings-builtin xs))
     "compareVersions" (fn [left] (fn [right] (compare-versions left right)))
     "splitVersion" (fn [s] (split-version s))
     "parseDrvName" (fn [s] (parse-drv-name s))
     "match" (fn [pattern] (fn [value] (regex-match-builtin pattern value)))
     "split" (fn [pattern] (fn [value] (regex-split-builtin pattern value)))
     "fromJSON" (fn [value] (from-json-builtin value))
     "fromTOML" (fn [value] (from-toml-builtin value))
     "schemaValidate" (fn [schema] (fn [value] (schema-validate-builtin schema value)))
     "schemaNormalize" (fn [schema] (fn [value] (schema-normalize-value schema value)))
     "schemaExplain" (fn [schema] (fn [value] (schema-explain-builtin schema value)))
     "xmlParse" (fn [value] (xml-parse-builtin value))
     "xmlEmit" (fn [value] (markup-emit-builtin value False))
     "htmlParse" (fn [value] (html-parse-builtin value))
     "htmlEmit" (fn [value] (markup-emit-builtin value True))
     "lessThan" (fn [left] (fn [right] (nix-less-than left right)))
     "add" (fn [left] (fn [right] (add-builtin left right)))
     "sub" (fn [left] (fn [right] (check-i64 (- (number-value left "left side of builtins.sub")
                                      (number-value right "right side of builtins.sub")) "-")))
     "mul" (fn [left] (fn [right] (check-i64 (* (number-value left "left side of builtins.mul")
                                      (number-value right "right side of builtins.mul")) "*")))
     "div" (fn [left] (fn [right] (div-builtin left right "left side of builtins.div" "right side of builtins.div")))
     "mod" (fn [left] (fn [right] (mod-builtin left right "left side of builtins.mod" "right side of builtins.mod" "builtins.mod: division by zero")))
     "neg" (fn [value] (check-i64 (- 0 (number-value value "builtins.neg argument")) "-"))
     "abs" (fn [value] (abs (number-value value "builtins.abs argument")))
     "bitAnd" (fn [left] (fn [right] (bit-and-builtin left right)))
     "bitOr" (fn [left] (fn [right] (bit-or-builtin left right)))
     "bitXor" (fn [left] (fn [right] (bit-xor-builtin left right)))
     "pow" (fn [left] (fn [right] (pow-builtin left right)))
     "sqrt" (fn [value] (math.sqrt (number-value value "builtins.sqrt argument")))
     "floor" (fn [value] (checked-float-to-i64 value "builtins.floor" math.floor))
     "ceil" (fn [value] (checked-float-to-i64 value "builtins.ceil" math.ceil))
     "exp" (fn [value] (math.exp (number-value value "builtins.exp argument")))
     "ln" (fn [value] (math.log (number-value value "builtins.ln argument")))
     "log" (fn [value] (math.log (number-value value "builtins.log argument")))
     "sin" (fn [value] (math.sin (number-value value "builtins.sin argument")))
     "cos" (fn [value] (math.cos (number-value value "builtins.cos argument")))
     "tan" (fn [value] (math.tan (number-value value "builtins.tan argument")))
     "atan2" (fn [y] (fn [x] (math.atan2 (number-value y "builtins.atan2 y")
                                           (number-value x "builtins.atan2 x"))))
     "and" (fn [left] (fn [right] (and (bool-value left "builtins.and lhs")
                                        (bool-value right "builtins.and rhs"))))
     "or" (fn [left] (fn [right] (or (bool-value left "builtins.or lhs")
                                      (bool-value right "builtins.or rhs"))))
     "not" (fn [value] (not (bool-value value "builtins.not argument")))
     "eq" (fn [left] (fn [right] (eq-value left right)))
     "lt" (fn [left] (fn [right] (nix-less-than left right)))
     "le" (fn [left] (fn [right] (not (nix-less-than right left))))
     "gt" (fn [left] (fn [right] (nix-less-than right left)))
     "ge" (fn [left] (fn [right] (not (nix-less-than left right))))
     "seq" (fn [left] (fn [right] (do (force-value left) right)))
     "deepSeq" (fn [left] (fn [right] (do (deep-force-value left) right)))
     "tryEval" (make-native-lazy try-eval-builtin)
     "derivationStrict" (fn [attrs] (derivation-builtin attrs "derivationStrict"))
     "derivation" (fn [attrs] (derivation-builtin attrs "derivation"))
     "addErrorContext" (fn [msg] (fn [value] (add-error-context-builtin msg value)))
     "unsafeGetAttrPos" (fn [name] (fn [attrs] (unsafe-get-attr-pos-builtin name attrs)))
     "unsafeDiscardStringContext" (fn [value] (string-value value "builtins.unsafeDiscardStringContext string"))
     "hasContext" (fn [value] (> (len (string-context value)) 0))
     "getContext" (fn [value] (get-context-builtin value))
     "appendContext" (fn [value] (fn [extra] (append-context-builtin value extra)))
     "addDrvOutputDependencies" (fn [value] (context-string-builtin value "addDrvOutputDependencies"))
     "unsafeDiscardOutputDependency" (fn [value] (context-string-builtin value "unsafeDiscardOutputDependency"))
     "unsafeAddOutputDependency" (fn [value] (context-string-builtin value "unsafeAddOutputDependency"))
     "unsafeAddOutputName" (fn [name] (fn [value] (unsafe-add-output-name-builtin name value)))
     "trace" (fn [msg] (fn [value] value))
     "throw" throw-value
     "abort" (fn [msg] (abort-value msg))
     "typeOf" (fn [x] (type-of x))
     "isList" (fn [x] (isinstance (force-value x) list))
     "isAttrs" (fn [x] (and (isinstance (force-value x) dict)
                            (not (closure? (force-value x)))
                            (not (thunk? (force-value x)))
                            (not (path? (force-value x)))))
     "isString" (fn [x] (isinstance (force-value x) str))
     "isInt" (fn [x] (and (isinstance (force-value x) int)
                          (not (isinstance (force-value x) bool))))
     "isFloat" (fn [x] (isinstance (force-value x) float))
     "isFinite" (fn [x] (finite-builtin x))
     "isInf" (fn [x] (inf-builtin x))
     "isNaN" (fn [x] (nan-builtin x))
     "isBool" (fn [x] (isinstance (force-value x) bool))
     "isFunction" (fn [x] (or (closure? (force-value x)) (native-lazy? (force-value x)) (callable (force-value x))))
     "isNull" (fn [x] (= (force-value x) None))
     "isPath" (fn [x] (path? (force-value x)))})
      (setv (get b "true") True)
      (setv (get b "false") False)
      (setv (get b "null") None)
      (setv (get b "builtins") (make-thunk (fn [] b)))
      b))

  (setv BUILTIN-ALIAS-NAMES
    ["currentSystem" "nixVersion" "langVersion" "storeDir"
     "import" "scopedImport"
     "pathExists" "readFile" "readFileType" "readDir" "toFile" "hashString" "hashFile"
     "baseNameOf" "dirOf" "toPath" "storePath" "getEnv" "placeholder" "break" "warn" "traceVerbose"
     "attrNames" "hasAttr" "getAttr" "attrByPath" "removeAttrs" "listToAttrs"
     "filterAttrs" "functionArgs" "intersectAttrs" "zipAttrsWith" "catAttrs"
     "elemAt" "length" "head" "tail" "toString" "toJSON"
     "map" "filter" "foldl'" "fold" "foldl" "foldr" "cons" "append" "take" "drop" "reverse" "reverseList"
     "zip" "flatten" "find" "get" "mapGet" "set" "mapSet" "keys" "mapKeys" "values" "mapValues" "merge" "mapMerge"
     "elem" "any" "all" "concatLists" "concatMap"
     "genList" "groupBy" "partition" "genericClosure" "attrValues" "getAttrs" "mapAttrs" "sort"
     "substring" "stringLength" "hasPrefix" "hasSuffix" "replaceStrings"
     "concatStringsSep" "concatStrings" "compareVersions" "splitVersion" "parseDrvName"
     "match" "split" "fromJSON" "fromTOML" "schemaValidate" "schemaNormalize" "schemaExplain"
     "xmlParse" "xmlEmit" "htmlParse" "htmlEmit"
     "lessThan" "add" "sub" "mul" "div" "mod" "neg" "abs" "bitAnd" "bitOr" "bitXor" "pow" "sqrt" "floor" "ceil"
     "exp" "ln" "log" "sin" "cos" "tan" "atan2" "and" "or" "not" "eq" "lt" "le" "gt" "ge"
     "seq" "deepSeq" "tryEval" "derivationStrict" "derivation" "addErrorContext"
     "unsafeGetAttrPos" "unsafeDiscardStringContext" "hasContext" "getContext" "appendContext"
     "addDrvOutputDependencies" "unsafeDiscardOutputDependency" "unsafeAddOutputDependency" "unsafeAddOutputName"
     "trace" "throw" "abort"
     "typeOf" "isList" "isAttrs" "isString" "isInt" "isFloat" "isFinite" "isInf" "isNaN" "isBool"
     "isFunction" "isNull" "isPath"])

  (defn initial-env-aliases [env builtins names]
    (if (= (len names) 0)
      env
      (do
        (if (in (get names 0) builtins)
          (setv (get env (get names 0)) (get builtins (get names 0)))
          None)
        (initial-env-aliases env builtins (cut names 1 None)))))

  (defn initial-env-with-builtins [builtins]
    (initial-env-aliases
      {"builtins" builtins
       "true" True
       "false" False
       "null" None}
      builtins
      BUILTIN-ALIAS-NAMES))

  (defn initial-env []
    (initial-env-with-builtins (native-builtins)))

  (defn eval-list-items [items env]
    (if (= (len items) 0)
      []
      (+ [(eval-thunk (get items 0) env)]
         (eval-list-items (cut items 1 None) env))))

  (defn eval-list [items env]
    (eval-list-items items env))

  (defn interp-placeholder? [interp-expr interp-env]
    (if (= (get interp-expr "tag") "var")
      (not (in (get interp-expr "name") interp-env))
      False))

  (defn eval-str-interp-part [interp-part interp-env]
    (if (in "lit" interp-part)
      (get interp-part "lit")
      (if (interp-placeholder? (get interp-part "expr") interp-env)
        (+ (+ "${" (get (get interp-part "expr") "name")) "}")
        (coerce-interp (eval-ast (get interp-part "expr") interp-env)))))

  (defn eval-str-interp-parts [interp-parts interp-env]
    (if (= (len interp-parts) 0)
      ""
      (do
        (setv head (eval-str-interp-part (get interp-parts 0) interp-env))
        (setv tail (eval-str-interp-parts (cut interp-parts 1 None) interp-env))
        (make-context-string
          (+ (str head) (str tail))
          (merge-contexts (string-context head) (string-context tail))))))

  (defn eval-str-interp [interp-parts interp-env]
    (eval-str-interp-parts interp-parts interp-env))

  (defn eval-path-interp-part [interp-part interp-env]
      (if (in "lit" interp-part)
      (get interp-part "lit")
      (str (coerce-interp (eval-ast (get interp-part "expr") interp-env)))))

  (defn eval-path-interp-parts [interp-parts interp-env]
    (if (= (len interp-parts) 0)
      ""
      (+ (eval-path-interp-part (get interp-parts 0) interp-env)
         (eval-path-interp-parts (cut interp-parts 1 None) interp-env))))

  (defn eval-binary [op lhs rhs env]
    (cond
      (= op "&&")
        (if (bool-value (eval-ast lhs env) "left operand of &&")
          (bool-value (eval-ast rhs env) "right operand of &&")
          False)
      (= op "||")
        (if (bool-value (eval-ast lhs env) "left operand of ||")
          True
          (bool-value (eval-ast rhs env) "right operand of ||"))
      (= op "->")
        (if (not (bool-value (eval-ast lhs env) "left operand of ->"))
          True
          (bool-value (eval-ast rhs env) "right operand of ->"))
      True
        (do
          (setv left (eval-ast lhs env))
          (setv right (eval-ast rhs env))
          (cond
            (= op "+")
              (do
                (setv l (force-value left))
                (setv r (force-value right))
                (cond
                  (and (path? l) (path? r))
                    (make-path (+ (get l "value") (get r "value")))
                  (and (path? l) (isinstance r str))
                    (do
                      (if (> (len (string-context r)) 0)
                        (pnix-error "operator +: path + context-bearing string would drop string context; use builtins.unsafeDiscardStringContext to discard it explicitly")
                        None)
                      (make-path (+ (get l "value") (str r))))
                  (and (isinstance l str) (path? r))
                    (do
                      (setv ctx (string-context l))
                      (.add ctx (get r "value"))
                      (make-context-string (+ (str l) (get r "value")) ctx))
                  (and (isinstance l str) (isinstance r str))
                    (make-context-string
                      (+ (str l) (str r))
                      (merge-contexts (string-context l) (string-context r)))
                  (and (isinstance l list) (isinstance r list)) (+ l r)
                  (and (isinstance l dict) (isinstance r dict)) (merge-attrsets l r)
                  True (do
                         (if (not (and (numeric? l) (numeric? r)))
                           (pnix-error (+ "operator +: unsupported operand types " (+ (type-of l) (+ " and " (type-of r)))))
                           None)
                         (check-i64 (+ l r) "+"))))
            (= op "-") (do (setv p (arith-pair "-" left right)) (check-i64 (- (get p 0) (get p 1)) "-"))
            (= op "*") (do (setv p (arith-pair "*" left right)) (check-i64 (* (get p 0) (get p 1)) "*"))
            (= op "/")
              (do (arith-pair "/" left right) (div-builtin left right "left side of /" "right side of /"))
	            (= op "%") (do (arith-pair "%" left right) (mod-builtin left right "left side of %" "right side of %" "modulo by zero"))
            (= op "==") (eq-value left right)
            (= op "!=") (not (eq-value left right))
            (in op ["<" "<=" ">" ">="]) (nix-compare op left right)
            (= op "//") (merge-attrsets left right)
            (= op "++")
              (do
                (setv l (force-value left))
                (setv r (force-value right))
                (if (and (isinstance l list) (isinstance r list))
                  (+ l r)
                  (pnix-error "both sides of ++ must be lists")))
            True (pnix-error (+ "unsupported binary op `" (+ op "`")))))))

  (defn eval-ast [ast env]
    (setv tag (get ast "tag"))
    (cond
      (= tag "int") (get ast "value")
      (= tag "float") (get ast "value")
      (= tag "path") (make-path (get ast "value"))
      (= tag "path_interp") (make-path (eval-path-interp-parts (get ast "parts") env))
      (= tag "string") (get ast "value")
      (= tag "str_interp") (eval-str-interp (get ast "parts") env)
      (= tag "bool") (get ast "value")
      (= tag "null") None
	      (= tag "var")
	        (if (= (get ast "name") "__curPos")
	          (source-position-value (.get ast "pos" 0))
	          (lookup-env env (get ast "name")))
	      (= tag "import") (pnix-error "import requires run_px/run_px_source host file context")
	      (= tag "construct")
	        (make-construct (get ast "variant") (eval-list (get ast "args") env))
	      (= tag "list") (eval-list (get ast "items") env)
      (= tag "attrset") (build-attrset (get ast "bindings") env (get ast "recursive"))
      (= tag "let") (eval-ast (get ast "body") (build-let-env (get ast "bindings") env))
      (= tag "lambda") (make-closure (get ast "param") (get ast "body") env (.get ast "pattern" None))
      (= tag "apply") (apply-pnix (eval-ast (get ast "func") env)
                                  (eval-thunk (get ast "arg") env))
      (= tag "if") (if (bool-value (eval-ast (get ast "cond") env) "if condition")
                     (eval-ast (get ast "then") env)
                     (eval-ast (get ast "else") env))
      (= tag "with") (eval-ast (get ast "body") (with-env env (get ast "env")))
      (= tag "assert")
        (if (bool-value (eval-ast (get ast "cond") env) "assert condition")
          (eval-ast (get ast "body") env)
          (pnix-catchable-error "assertion failed"))
      (= tag "select")
        (do
          (setv base (attrset-value (eval-ast (get ast "base") env) "select base"))
          (force-value (get base (get ast "attr"))))
      (= tag "select_default")
        (do
          (setv base (force-value (eval-ast (get ast "base") env)))
          (setv attr (get ast "attr"))
          (if (and (isinstance base dict) (not (closure? base)) (not (thunk? base)) (not (path? base)) (in attr base))
	            (force-value (get base attr))
	            (eval-ast (get ast "default") env)))
	      (= tag "dynamic_select")
	        (do
	          (setv base (attrset-value (eval-ast (get ast "base") env) "select base"))
	          (setv attr (eval-attr-segments (get ast "segments") env))
	          (force-value (get base attr)))
	      (= tag "dynamic_select_default")
	        (do
	          (setv base (force-value (eval-ast (get ast "base") env)))
	          (setv attr (eval-attr-segments (get ast "segments") env))
	          (if (and (isinstance base dict) (not (closure? base)) (not (thunk? base)) (not (path? base)) (not (construct? base)) (in attr base))
	            (force-value (get base attr))
	            (eval-ast (get ast "default") env)))
	      (= tag "has_attr")
	        ;; Nix `?` is false on ANY non-set base (unlike builtins.hasAttr,
	        ;; which errors) — oracle-pinned 2026-07-08; same guard set as
	        ;; dynamic_select_default (closure/thunk/path are marker dicts).
	        (do
	          (setv base (force-value (eval-ast (get ast "base") env)))
	          (if (and (isinstance base dict) (not (closure? base)) (not (thunk? base)) (not (path? base)) (not (construct? base)))
	            (has-attr-path base (get ast "path"))
	            False))
	      (= tag "dynamic_has_attr")
	        (do
	          (setv base (force-value (eval-ast (get ast "base") env)))
	          (if (and (isinstance base dict) (not (closure? base)) (not (thunk? base)) (not (path? base)) (not (construct? base)))
	            (has-attr-path base (eval-attr-path-segments (get ast "segments") env 0))
	            False))
	      (= tag "index")
        (do
          (setv base (force-value (eval-ast (get ast "base") env)))
          (setv idx (force-value (eval-ast (get ast "index") env)))
          (cond
            (isinstance base list)
              (if (and (isinstance idx int) (not (isinstance idx bool)))
                (force-value (get base idx))
                (pnix-error "index must be an integer"))
            (and (isinstance base dict) (not (path? base)) (isinstance idx str))
              (force-value (get base idx))
	            True
	              (pnix-error "index target unsupported")))
	      (= tag "match")
	        (eval-match-arms (eval-ast (get ast "scrutinee") env) (get ast "arms") env)
	      (= tag "unary")
	        (if (= (get ast "op") "!")
	          (not (bool-value (eval-ast (get ast "arg") env) "argument of !"))
	          (unary-neg-value (eval-ast (get ast "arg") env)))
      (= tag "binary")
        (eval-binary (get ast "op") (get ast "lhs") (get ast "rhs") env)
      True
        (pnix-error (+ "unsupported AST tag " tag))))

  (setv outputs [])
  (if (> (len raw-sources) 0)
    (for [source raw-sources]
      (do
        (setv current-source source)
        (.append outputs (realize-value (eval-ast (parse-source source) (initial-env))))))
    (for [ast raw-asts]
      (.append outputs (realize-value (eval-ast ast (initial-env))))))
  (json.dumps outputs))
  (pnix-main))
'''


def hy_runtime_source_for_asts(asts: list[dict[str, Any]]) -> str:
    asts_json = json.dumps(stable_data(asts), ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    sources_json = json.dumps([], ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return (
        HY_AST_EVALUATOR_SOURCE.replace("__PNIX_ASTS_JSON__", json.dumps(asts_json))
        .replace("__PNIX_SOURCES_JSON__", json.dumps(sources_json))
    )


def hy_runtime_source_for_ast(ast: dict[str, Any]) -> str:
    return hy_runtime_source_for_asts([ast])


def hy_runtime_source_for_sources(sources: list[str]) -> str:
    sources_json = json.dumps(sources, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    asts_json = json.dumps([], ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return (
        HY_AST_EVALUATOR_SOURCE.replace("__PNIX_ASTS_JSON__", json.dumps(asts_json))
        .replace("__PNIX_SOURCES_JSON__", json.dumps(sources_json))
    )


# --- pnix -> Python compiler lane -------------------------------------------
#
# Unlike HY_AST_EVALUATOR_SOURCE (a tree-walking interpreter kept as the
# semantics mirror), this lane *compiles* each pnix AST to a Python expression
# and lets host CPython execute the compiled bytecode -- no per-node dispatch at
# run time. The emitter is written in the same stage7 Hy subset and runs INSIDE
# stage7, so it is a genuine self-host compiler (mirroring what hy-meta does for
# Hy). COMPILER_PRELUDE is the tiny runtime the emitted code targets (_T thunks,
# _C closures, _force, _apply, _bin, builtins, _realize); it is injected as a
# string the same way ASTs are. Laziness is preserved by emitting `_T(lambda:..)`
# thunks; rec/let forward refs resolve through Python late-binding closures.
COMPILER_PRELUDE = r'''
import hashlib
import json
import math
import os
import platform
import re
import sys
import tempfile
import tomllib
import xml.etree.ElementTree as ET
from functools import cmp_to_key
class _Catch(Exception):
    pass
class _T:
    __slots__=("f","d","v","ing")
    def __init__(s,f): s.f=f; s.d=False; s.v=None; s.ing=False
def _force(x):
    while type(x) is _T:
        if x.d: x=x.v; continue
        if x.ing: raise Exception("infinite recursion encountered (recursive value forced itself)")
        x.ing=True
        try: x.v=x.f()
        except BaseException:
            x.ing=False
            raise
        x.d=True; x.ing=False; x=x.v
    return x
def _tv(v):
    t=_T(None); t.d=True; t.v=v; return t
class _C:
    __slots__=("fn","args")
    def __init__(s,fn,args=None): s.fn=fn; s.args=args
def _normpath_text(text):
    text=str(text)
    if text.startswith("<") and text.endswith(">"): return text
    absolute=text.startswith("/")
    started_dot=(text=="." or text.startswith("./"))
    out=[]
    for part in text.split("/"):
        if part=="" or part==".": continue
        if part=="..":
            if out and out[-1]!="..": out.pop()
            elif not absolute: out.append(part)
            continue
        out.append(part)
    if absolute: return "/" + "/".join(out) if out else "/"
    if not out: return "."
    body="/".join(out)
    return "./"+body if started_dot and out[0]!=".." else body
class _P(str):
    def __new__(cls,text):
        return str.__new__(cls,_normpath_text(text))
class _S(str):
    def __new__(cls,text,ctx=None):
        obj=str.__new__(cls,str(text)); obj.ctx=frozenset(str(x) for x in (ctx or ())); return obj
_TO_STRING_SEEN=set()
def _isstr(x): return isinstance(x,str) and type(x) is not _P
def _ctx(x):
    x=_force(x)
    return set(x.ctx) if type(x) is _S else set()
def _mkstr(text,ctx=None):
    c=set(str(x) for x in (ctx or ()))
    return _S(str(text),c) if c else str(text)
def _strctx(x,label):
    x=_force(x)
    if _isstr(x): return str(x),_ctx(x)
    raise Exception(label+" ("+_typeof(x)+") must be a string")
class _A(dict):
    def __init__(s,*args,attr_positions=None,**kwargs):
        super().__init__(*args,**kwargs)
        s.attr_positions=dict(attr_positions or {})
class _K:
    __slots__=("variant","args")
    def __init__(s,variant,args): s.variant=variant; s.args=args
class _WF:
    __slots__=("source","prev","done","attrs")
    def __init__(s,source,prev): s.source=source; s.prev=prev; s.done=False; s.attrs=None
def _apply(f,a):
    f=_force(f)
    if type(f) is _C: return f.fn(a)
    raise Exception("apply target is not a function")
def _unknownvar(n): raise Exception("unknown variable `%s`"%n)
def _assert_fail(): raise _Catch("assertion failed")
def _with(source,prev): return _WF(source,prev)
def _with_attrs(frame):
    if not frame.done:
        attrs=_force(frame.source)
        if not isinstance(attrs,dict): raise Exception("with: argument must be attrset, got "+_typeof(attrs))
        frame.attrs=attrs; frame.done=True
    return frame.attrs
def _abortshow(v):
    v=_force(v)
    if type(v) is bool: return "true" if v else "false"
    if type(v) is int or type(v) is float: return str(v)
    if v is None: return "null"
    if _isstr(v): return str(v)
    return _typeof(v)
def _abortval(msg):
    forced=_force(msg)
    if not _isstr(forced): raise Exception("builtins.abort: argument must be string, got "+_abortshow(forced))
    raise Exception("evaluation aborted: "+str(forced))
def _with_lookup(frame,name):
    cur=frame
    while cur is not None:
        attrs=_with_attrs(cur)
        if name in attrs: return _force(attrs[name])
        cur=cur.prev
    return _unknownvar(name)
def _setpath(d,path,thunk,context="attr",positions=None):
    cur=d
    for i,k in enumerate(path[:-1]):
        if k not in cur:
            cur[k]=_A()
            if isinstance(cur,_A) and positions and positions[i] is not None:
                cur.attr_positions[k]=positions[i]
        elif not isinstance(cur[k],dict):
            forced=_force(cur[k])
            if not isinstance(forced,dict):
                raise Exception("attribute path conflict: '%s' is already a non-attrset value"%k)
            cur[k]=forced
        cur=cur[k]
    if path[-1] in cur:
        if context=="let":
            raise Exception("let: '%s' bound more than once"%path[-1])
        existing=_force(cur[path[-1]])
        new_value=_force(thunk)
        if isinstance(existing,dict) and isinstance(new_value,dict):
            cur[path[-1]]=_merge_defined(existing,new_value)
            return
        raise Exception("attribute '%s' already defined at this level"%path[-1])
    if isinstance(cur,_A) and positions and positions[-1] is not None:
        cur.attr_positions[path[-1]]=positions[-1]
    cur[path[-1]]=thunk
def _merge_defined(left,right):
    out=_A(left,attr_positions=getattr(left,"attr_positions",{}))
    right_pos=getattr(right,"attr_positions",{})
    for k,v in right.items():
        if k not in out:
            out[k]=v
            if k in right_pos: out.attr_positions[k]=right_pos[k]
            continue
        lv=_force(out[k]); rv=_force(v)
        if isinstance(lv,dict) and isinstance(rv,dict):
            out[k]=_merge_defined(lv,rv)
            continue
        raise Exception("attribute '%s' already defined at this level"%k)
    return out
def _realize(x):
    x=_force(x)
    if type(x) is _P: return str(x)
    if type(x) is _S: return str(x)
    if type(x) is _K: return {"variant":x.variant,"args":[_realize(a) for a in x.args]}
    if isinstance(x,dict): return {k:_realize(v) for k,v in x.items()}
    if isinstance(x,list): return [_realize(e) for e in x]
    if type(x) is _C: return "#<pnix-hy-closure>"
    return x
_VALUES_EQUAL_MAX_DEPTH=64
def _eq(a,b,depth=0):
    if depth>_VALUES_EQUAL_MAX_DEPTH: raise Exception("infinite recursion encountered during ==")
    A=_force(a); B=_force(b)
    if depth>0 and A is B: return True
    if type(A) is _C or callable(A) or type(B) is _C or callable(B): return False
    if type(A) is _P or type(B) is _P:
        return type(A) is _P and type(B) is _P and _normpath_text(A)==_normpath_text(B)
    if type(A) is _S: A=str(A)
    if type(B) is _S: B=str(B)
    if type(A) is _K or type(B) is _K:
        return type(A) is _K and type(B) is _K and A.variant==B.variant and len(A.args)==len(B.args) and all(_eq(x,y,depth+1) for x,y in zip(A.args,B.args))
    if isinstance(A,dict) or isinstance(B,dict):
        return isinstance(A,dict) and isinstance(B,dict) and set(A.keys())==set(B.keys()) and all(_eq(A[k],B[k],depth+1) for k in A.keys())
    if isinstance(A,list) or isinstance(B,list):
        return isinstance(A,list) and isinstance(B,list) and len(A)==len(B) and all(_eq(x,y,depth+1) for x,y in zip(A,B))
    if type(A) is bool or type(B) is bool: return type(A) is bool and type(B) is bool and A==B
    if A is None or B is None: return A is None and B is None
    if (type(A) is int or type(A) is float) and (type(B) is int or type(B) is float):
        return float(A)==float(B) if type(A) is float or type(B) is float else A==B
    if isinstance(A,str) or isinstance(B,str): return isinstance(A,str) and isinstance(B,str) and str(A)==str(B)
    return type(A) is type(B) and A==B
def _jsonready(x,seen=None):
    seen=set() if seen is None else seen
    x=_force(x)
    if type(x) is _C or callable(x): raise Exception("cannot serialize function as JSON")
    if type(x) is float and not math.isfinite(x): raise Exception("cannot serialize float "+("NaN" if math.isnan(x) else ("+inf" if x>0 else "-inf"))+" as JSON")
    if type(x) is _P: return str(x)
    if type(x) is _S: return str(x)
    if type(x) is _K:
        oid=id(x)
        if oid in seen: raise Exception("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(oid)
        try: return {"variant":x.variant,"args":[_jsonready(a,seen) for a in x.args]}
        finally: seen.discard(oid)
    if isinstance(x,dict):
        oid=id(x)
        if oid in seen: raise Exception("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(oid)
        try: return {k:_jsonready(v,seen) for k,v in sorted(x.items())}
        finally: seen.discard(oid)
    if isinstance(x,list):
        oid=id(x)
        if oid in seen: raise Exception("builtins.toJSON: infinite recursion encountered (cyclic value)")
        seen.add(oid)
        try: return [_jsonready(e,seen) for e in x]
        finally: seen.discard(oid)
    return x
def _bool(x,label):
    x=_force(x)
    if type(x) is bool: return x
    raise Exception(label+": expected bool, got "+_typeof(x))
def _predbool(x,label,index=None):
    x=_force(x)
    if type(x) is bool: return x
    suffix="" if index is None else " at index "+str(index)
    raise Exception(label+" must return bool, got "+_typeof(x)+suffix)
def _num(x,label):
    x=_force(x)
    if type(x) is int or type(x) is float: return x
    raise Exception(label+" must be a number")
def _int(x,label):
    x=_force(x)
    if type(x) is int: return x
    raise Exception(label+" must be an integer")
def _list(x,label):
    x=_force(x)
    if isinstance(x,list): return x
    raise Exception(label+" must be a list")
def _attrs(x,label):
    x=_force(x)
    if isinstance(x,dict): return x
    raise Exception(label+" must be an attrset")
def _sortlist(x):
    x=_force(x)
    if isinstance(x,list): return x
    raise Exception("builtins.sort second argument must be list, got "+_typeof(x))
def _nonneg(x,label):
    n=_int(x,label)
    if n<0: raise Exception(label+": negative count")
    return n
def _str(x,label):
    x=_force(x)
    if _isstr(x): return str(x)
    raise Exception(label+" ("+_typeof(x)+") must be a string")
def _strplain(x,label):
    x=_force(x)
    if _isstr(x): return str(x)
    raise Exception(label+" ("+_typeof(x)+") must be string")
def _strplainctx(x,label):
    x=_force(x)
    if _isstr(x): return str(x),_ctx(x)
    raise Exception(label+" ("+_typeof(x)+") must be string")
def _expectedstr(x,label):
    x=_force(x)
    if _isstr(x): return str(x)
    raise Exception(label+": expected string, got "+_typeof(x))
def _sbytes(x): return str(x).encode("utf-8",errors="surrogateescape")
def _strlen(x): return len(_sbytes(_str(x,"builtins.stringLength string")))
def _length(x):
    x=_force(x)
    if isinstance(x,list): return len(x)
    if _isstr(x): return len(_sbytes(x))
    raise Exception("builtins.length: expected list or string, got "+_typeof(x))
def _substr(start,length,text):
    st=_num(start,"builtins.substring start"); ln=_num(length,"builtins.substring length")
    if type(st) is float or type(ln) is float: raise Exception("builtins.substring start and length must be integers")
    if st<0: raise Exception("builtins.substring: negative start position %s not allowed"%st)
    data=_sbytes(_str(text,"builtins.substring string"))
    st=int(st)
    if st>=len(data): return ""
    end=len(data) if ln<0 else min(st+int(ln),len(data))
    return data[st:end].decode("utf-8","surrogateescape")
def _replace(froms,tos,text):
    fs=_force(froms); ts=_force(tos)
    if not isinstance(fs,list): raise Exception("builtins.replaceStrings: 'from' must be list, got "+_typeof(fs))
    if not isinstance(ts,list): raise Exception("builtins.replaceStrings: 'to' must be list, got "+_typeof(ts))
    if len(fs)!=len(ts): raise Exception("builtins.replaceStrings: `from` and `to` lists must have equal length")
    sf=_force(text)
    if not _isstr(sf): raise Exception("builtins.replaceStrings: third argument must be string, got "+_typeof(sf))
    s=str(sf)
    if not fs: return s
    pats=[]
    for x in fs:
        xf=_force(x)
        if not _isstr(xf): raise Exception("builtins.replaceStrings: 'from' element must be string, got "+_typeof(xf))
        pats.append(str(xf))
    reps=[None]*len(ts)
    out=[]; i=0
    while i<=len(s):
        hit=None
        tail=s[i:]
        for idx,p in enumerate(pats):
            if tail.startswith(p):
                hit=(idx,len(p)); break
        if hit is None:
            if i<len(s): out.append(s[i])
            i+=1; continue
        idx,plen=hit
        if reps[idx] is None: reps[idx]=_str(ts[idx],"builtins.replaceStrings to element")
        out.append(reps[idx])
        if plen==0:
            if i<len(s): out.append(s[i])
            i+=1
        else:
            i+=plen
    return "".join(out)
def _listtoattrs(xs):
    out={}
    for item in _force(xs):
        d=_force(item)
        if not isinstance(d,dict): raise Exception("builtins.listToAttrs element must be attrset")
        n=_str(d.get("name"),"builtins.listToAttrs name")
        if "value" not in d: raise Exception("builtins.listToAttrs element is missing `value`")
        if n not in out: out[n]=d["value"]
    return out
def _hasattr(nm,m):
    nm=_force(nm)
    if not _isstr(nm): raise Exception("builtins.hasAttr: first argument must be string, got "+_typeof(nm))
    m=_force(m)
    if not isinstance(m,dict): raise Exception("builtins.hasAttr: second argument must be attrset, got "+_typeof(m))
    return str(nm) in m
def _rmattrs(attrs,names):
    d=_force(attrs)
    if not isinstance(d,dict): raise Exception("builtins.removeAttrs: first argument must be attrset, got "+_typeof(d))
    names=_force(names)
    if not isinstance(names,list): raise Exception("builtins.removeAttrs: second argument must be list of strings, got "+_typeof(names))
    ns=set()
    for index,n in enumerate(names):
        n=_force(n)
        if not _isstr(n): raise Exception("builtins.removeAttrs: name-list element at index "+str(index)+" is not a string, got "+_typeof(n))
        ns.add(str(n))
    return {k:v for k,v in d.items() if k not in ns}
def _attrbypath(path,default,attrs):
    cur=_force(attrs)
    if not isinstance(cur,dict): raise Exception("builtins.attrByPath attrs must be an attrset")
    for p in _list(path,"builtins.attrByPath path"):
        k=_str(p,"builtins.attrByPath path element")
        if not isinstance(cur,dict) or k not in cur: return default
        cur=_force(cur[k])
    return cur
def _filterattrs(pred,m):
    d=_force(m); out={}
    for k,v in d.items():
        if _bool(_apply(_apply(pred,_tv(k)),_tv(_force(v))),"builtins.filterAttrs predicate"):
            out[k]=v
    return out
def _intersectattrs(lhs,rhs):
    l=_force(lhs); r=_force(rhs)
    return {k:v for k,v in r.items() if k in l}
def _catattrs(name,xs):
    n=_str(name,"builtins.catAttrs name"); out=[]
    for item in _list(xs,"builtins.catAttrs list"):
        d=_force(item)
        if not isinstance(d,dict): raise Exception("builtins.catAttrs element must be an attrset")
        if n in d: out.append(d[n])
    return out
def _listarg(value,builtin,position):
    value=_force(value)
    if not isinstance(value,list): raise Exception("builtins."+builtin+": "+position+" must be list, got "+_typeof(value))
    return value
def _attrsarg(value,builtin,phrase="expected attrset"):
    value=_force(value)
    if not isinstance(value,dict): raise Exception("builtins."+builtin+": "+phrase+", got "+_typeof(value))
    return value
def _groupby(fn,xs):
    xs=_force(xs)
    if not isinstance(xs,list): raise Exception("builtins.groupBy: second argument must be list, got "+_typeof(xs))
    out={}
    for item in xs:
        kv=_force(_apply(fn,_tv(_force(item))))
        if not _isstr(kv): raise Exception("builtins.groupBy: key function must return string, got "+_typeof(kv))
        out.setdefault(str(kv),[]).append(_force(item))
    return out
def _partition(pred,xs):
    right=[]; wrong=[]
    for item in _force(xs):
        (right if _bool(_apply(pred,_tv(_force(item))),"builtins.partition predicate") else wrong).append(_force(item))
    return {"right":right,"wrong":wrong}
def _fnargs(f):
    f=_force(f)
    if type(f) is not _C: raise Exception("builtins.functionArgs: expected function, got "+_typeof(f))
    p=f.args
    while isinstance(p,dict) and p.get("tag")=="as":
        p=p.get("pattern")
    if not isinstance(p,dict) or p.get("tag")!="attrset": return {}
    return {field["name"]: ("default" in field) for field in p.get("fields",[])}
def _zipattrswith(fn,xs):
    maps=[]
    for x in _listarg(xs,"zipAttrsWith","second argument"):
        maps.append(_attrsarg(x,"zipAttrsWith","list element must be attrset"))
    keys=sorted({k for m in maps for k in m})
    out={}
    for k in keys:
        vals=[m[k] for m in maps if k in m]
        out[k]=_T(lambda k=k,vals=vals: _apply(_apply(fn,_tv(k)),_tv(vals)))
    return out

def _maplist(f,xs):
    return [_T(lambda it=it: _apply(f,_T(lambda it=it: _force(it)))) for it in _list(xs,"builtins.map list")]

def _concatlists(xss):
    xss=_force(xss)
    if not isinstance(xss,list): raise Exception("builtins.concatLists: argument must be list, got "+_typeof(xss))
    out=[]
    for index,xs in enumerate(xss):
        xs=_force(xs)
        if not isinstance(xs,list): raise Exception("builtins.concatLists: element at index "+str(index)+" is not a list, got "+_typeof(xs))
        out.extend(xs)
    return out
def _take(n,xs):
    N=_nonneg(n,"builtins.take count")
    return _list(xs,"builtins.take list")[:N]
def _drop(n,xs):
    N=_nonneg(n,"builtins.drop count")
    return _list(xs,"builtins.drop list")[N:]
def _elemat(xs,i):
    items=_list(xs,"builtins.elemAt list")
    idx=_int(i,"builtins.elemAt index")
    if idx<0: raise Exception("builtins.elemAt: negative index")
    if idx>=len(items): raise Exception("builtins.elemAt: index out of bounds")
    return _force(items[idx])
def _head(xs):
    items=_list(xs,"builtins.head list")
    if not items: raise Exception("builtins.head: list is empty")
    return _force(items[0])
def _tail(xs):
    items=_list(xs,"builtins.tail list")
    if not items: raise Exception("builtins.tail: list is empty")
    return items[1:]
def _ziplist(a,b):
    return [[_force(x),_force(y)] for x,y in zip(_force(a),_force(b))]
def _flatten(xs):
    out=[]
    for item in _force(xs):
        v=_force(item)
        if isinstance(v,list): out.extend(_flatten(v))
        else: out.append(v)
    return out
def _find(needle,xs):
    for item in _force(xs):
        v=_force(item)
        if _eq(needle,v,1): return v
    return None
def _get(m,name):
    d=_force(m); k=_str(name,"builtins.get name")
    return _force(d[k]) if k in d else None
def _set(m,name,value):
    out=dict(_force(m)); out[_str(name,"builtins.set name")]=_force(value); return out
def _keys(m,label="builtins.keys attrs"):
    d=_force(m)
    if not isinstance(d,dict): raise Exception(label+" must be an attrset")
    return sorted(d.keys())
def _values(m,label="builtins.values"):
    d=_force(m)
    if not isinstance(d,dict): raise Exception(label+": expected attrset, got "+_typeof(d))
    return [d[k] for k in sorted(d)]

def _mapattrs(f,m):
    d=_force(m)
    if not isinstance(d,dict): raise Exception("builtins.mapAttrs set must be an attrset")
    return {k:_T(lambda k=k,v=v: _apply(_apply(f,_tv(k)),_T(lambda v=v: _force(v)))) for k,v in d.items()}
def _getattr_builtin(name,m):
    k=str(_force(name)); d=_attrsarg(m,"getAttr")
    if k not in d: raise Exception("builtins.getAttr: attribute '%s' missing"%k)
    return _force(d[k])
def _merge(a,b):
    out=dict(_force(a)); out.update(_force(b)); return out
def _getattrs(names,attrs):
    ns=_force(names); d=_force(attrs); out={}
    if not isinstance(ns,list): raise Exception("builtins.getAttrs: first argument must be list")
    if not isinstance(d,dict): raise Exception("builtins.getAttrs: second argument must be attrset")
    for n in ns:
        k=_str(n,"builtins.getAttrs name")
        if k not in d: raise Exception("builtins.getAttrs: attribute '%s' missing in set"%k)
        out[k]=d[k]
    return out
def _collectctx(x):
    x=_force(x)
    if type(x) is _S: return set(x.ctx)
    if type(x) is _P: return {str(x)}
    if type(x) is _K:
        out=set()
        for a in x.args: out.update(_collectctx(a))
        return out
    if isinstance(x,dict):
        out=set()
        for v in x.values(): out.update(_collectctx(v))
        return out
    if isinstance(x,list):
        out=set()
        for v in x: out.update(_collectctx(v))
        return out
    return set()
def _concatstrings(xs):
    out=[]; ctx=set()
    for i,it in enumerate(_listarg(xs,"concatStrings","argument")):
        text,c=_strctx(it,"builtins.concatStrings element at index %d is not a string"%i)
        out.append(text); ctx.update(c)
    return _mkstr("".join(out),ctx)
def _concatstringssep(sep,xs):
    sf=_force(sep)
    if not _isstr(sf): raise Exception("builtins.concatStringsSep: separator must be string, got "+_typeof(sf))
    sep_text,sep_ctx=_strctx(sf,"builtins.concatStringsSep separator")
    out=[]; ctx=set()
    for i,it in enumerate(_force(xs)):
        if i>0:
            out.append(sep_text); ctx.update(sep_ctx)
        text,c=_strctx(it,"builtins.concatStringsSep element at index %d is not a string"%i)
        out.append(text); ctx.update(c)
    return _mkstr("".join(out),ctx)
def _ctxstr(x,label):
    xf=_force(x)
    if not _isstr(xf): raise Exception("builtins.%s: expected string, got %s"%(label,_typeof(xf)))
    text,ctx=_strctx(xf,"builtins.%s string"%label)
    if label=="addDrvOutputDependencies":
        ctx.add("!out!"+text)
    elif label=="unsafeDiscardOutputDependency":
        ctx={v for v in ctx if not (v.startswith("!out!") or v.startswith("!") or v.startswith("="))}
    elif label=="unsafeAddOutputDependency":
        ctx.update("!out!"+v for v in list(ctx) if not v.startswith("!") and not v.startswith("="))
    return _mkstr(text,ctx)
def _addoutname(name,x):
    nf=_force(name)
    if not _isstr(nf): raise Exception("builtins.unsafeAddOutputName: first arg must be string, got "+_typeof(nf))
    n=str(nf)
    xf=_force(x)
    if not _isstr(xf): raise Exception("builtins.unsafeAddOutputName: second arg must be string, got "+_typeof(xf))
    text,ctx=_strctx(xf,"builtins.unsafeAddOutputName string")
    ctx.update("!"+n+"!"+v for v in list(ctx) if not v.startswith("!") and not v.startswith("="))
    return _mkstr(text,ctx)
def _getctx(x):
    text,ctx=_strctx(x,"builtins.getContext string")
    return {k:{"path":True} for k in sorted(ctx)}
def _hasctx(x):
    text,ctx=_strctx(x,"builtins.hasContext string")
    return bool(ctx)
def _appendcontext(x,ctx):
    text,context=_strctx(x,"builtins.appendContext string")
    d=_force(ctx)
    if not isinstance(d,dict): raise Exception("builtins.appendContext: second arg must be attrset")
    for k,v in d.items():
        spec=_force(v)
        if not isinstance(spec,dict): raise Exception("builtins.appendContext: context value for '%s' must be an attrset, got %s"%(k,_typeof(spec)))
        if "path" in spec:
            pf=_force(spec["path"])
            if type(pf) is not bool: raise Exception("builtins.appendContext: '%s'.path must be bool, got %s"%(k,_typeof(pf)))
        if "allOutputs" in spec:
            af=_force(spec["allOutputs"])
            if type(af) is not bool: raise Exception("builtins.appendContext: '%s'.allOutputs must be bool, got %s"%(k,_typeof(af)))
        if "outputs" in spec:
            outs=_force(spec["outputs"])
            if not isinstance(outs,list): raise Exception("builtins.appendContext: '%s'.outputs must be list of strings, got %s"%(k,_typeof(outs)))
            for index,out in enumerate(outs):
                of=_force(out)
                if not _isstr(of): raise Exception("builtins.appendContext: '%s'.outputs element at index %d is not a string, got %s"%(k,index,_typeof(of)))
        context.add(str(k))
    return _mkstr(text,context)
def _isfinite(x):
    x=_force(x)
    if type(x) is int: return True
    if type(x) is float: return math.isfinite(x)
    return False
def _isinf(x):
    x=_force(x)
    return math.isinf(x) if type(x) is float else False
def _isnan(x):
    x=_force(x)
    return math.isnan(x) if type(x) is float else False
def _derivation(attrs,label):
    src=_force(attrs)
    if not isinstance(src,dict): raise Exception("builtins.%s: expected attrset"%label)
    out=dict(src)
    name=_str(src["name"],"builtins.%s name"%label) if "name" in src else "unnamed"
    placeholder="/pnix-placeholder/derivation/"+name
    path_value=_mkstr(placeholder,{"!out!"+name})
    out.setdefault("outPath",path_value)
    out.setdefault("drvPath",path_value)
    out.setdefault("type","derivation")
    return out
def _fspath(x,label):
    x=_force(x)
    if type(x) is _P: text=str(x)
    elif _isstr(x): text=str(x)
    else: raise Exception(label+": expected string or path (expected path or string)")
    if text=="": raise Exception(label+": empty string is not a valid path")
    p=os.path.expanduser(_normpath_text(text))
    if not os.path.isabs(p):
        p=os.path.join(str(globals().get("_PX_BASE_DIR",os.getcwd())),p)
    return os.path.abspath(os.path.normpath(p))
def _pathlit(text):
    return _normpath_text(text)
def _ftype(path):
    if os.path.islink(path): return "symlink"
    if os.path.isdir(path): return "directory"
    if os.path.isfile(path): return "regular"
    return "unknown"
def _readfiletype(path):
    if not os.path.lexists(path):
        raise Exception("builtins.readFileType: failed to get metadata for `%s`: No such file or directory"%path)
    return _ftype(path)
def _readfile(x):
    path=_fspath(x,"builtins.readFile")
    try:
        with open(path,"r",encoding="utf-8") as f:
            return f.read()
    except OSError as exc:
        raise Exception("builtins.readFile: failed to read `%s`: %s"%(path,exc))
def _readdir(x):
    path=_fspath(x,"builtins.readDir")
    try:
        names=sorted(os.listdir(path))
    except OSError as exc:
        raise Exception("builtins.readDir: failed to read `%s`: %s"%(path,exc))
    return {name:_ftype(os.path.join(path,name)) for name in names}
def _safe_store_name(name):
    out="".join(ch if ch.isalnum() or ch in ".-_+" else "_" for ch in name)
    return out or "unnamed"
def _strarg(value,builtin):
    value=_force(value)
    if not _isstr(value): raise Exception("builtins."+builtin+": expected string, got "+_typeof(value))
    return str(value)
def _tofile(name,contents):
    nf=_force(name)
    if not _isstr(nf): raise Exception("builtins.toFile: first argument must be string, got "+_typeof(nf))
    file_name=str(nf)
    content_value=_force(contents)
    if not _isstr(content_value):
        raise Exception("builtins.toFile: second argument must be string, got "+_typeof(content_value))
    text,ctx=_strctx(content_value,"builtins.toFile contents")
    if ctx: raise Exception("builtins.toFile: contents must not have string context; use builtins.unsafeDiscardStringContext to discard it")
    digest=hashlib.sha256(text.encode("utf-8")).hexdigest()[:32]
    store_dir=os.path.join(tempfile.gettempdir(),"pnix-nix-store")
    os.makedirs(store_dir,exist_ok=True)
    out=os.path.abspath(os.path.join(store_dir,digest+"-"+_safe_store_name(file_name)))
    with open(out,"w",encoding="utf-8") as f:
        f.write(text)
    return _P(out)
def _hashbytes(algo,data,label,allow_legacy):
    if allow_legacy and algo=="md5": return hashlib.md5(data,usedforsecurity=False).hexdigest()
    if allow_legacy and algo=="sha1": return hashlib.sha1(data,usedforsecurity=False).hexdigest()
    if algo=="sha256": return hashlib.sha256(data).hexdigest()
    if algo=="sha512": return hashlib.sha512(data).hexdigest()
    if algo in {"md5","sha1"}:
        raise Exception(label+": algorithm '%s' is not supported (`%s`); cryptographically broken; use 'sha256' or 'sha512'"%(algo,algo))
    supported="'md5', 'sha1', 'sha256', 'sha512'" if allow_legacy else "'sha256', 'sha512'"
    raise Exception(label+": unsupported algorithm '%s' (`%s`); supported: %s"%(algo,algo,supported))
def _hashstrdata(algorithm,data):
    text,ctx=_strplainctx(data,"builtins.hashString data")
    return _hashbytes(algorithm,_sbytes(text),"builtins.hashString",True)
def _hashstr(algo):
    algorithm,ctx=_strplainctx(algo,"builtins.hashString algo")
    if ctx: raise Exception("builtins.hashString algo: the string '%s' is not allowed to refer to a store path"%algorithm)
    if algorithm not in {"md5","sha1","sha256","sha512"}: _hashbytes(algorithm,b"","builtins.hashString",True)
    return _C(lambda data:_hashstrdata(algorithm,data))
def _hashfile(algo,path):
    path_arg=_force(path)
    path_ctx={str(path_arg)} if type(path_arg) is _P else _ctx(path_arg)
    fp=_fspath(path_arg,"builtins.hashFile")
    try:
        with open(fp,"rb") as f:
            data=f.read()
    except OSError as exc:
        raise Exception("builtins.hashFile: failed to read `%s`: %s"%(fp,exc))
    return _mkstr(_hashbytes(_strplain(algo,"builtins.hashFile algo"),data,"builtins.hashFile",False),path_ctx)
def _basename(x):
    x=_force(x)
    if type(x) is _P:
        text=str(x); ctx=set()
    else:
        text,ctx=_strctx(x,"builtins.baseNameOf path")
    if text=="" or text=="/": return _mkstr("",ctx)
    if text.endswith("/") and text!="/": text=text[:-1]
    return _mkstr(text.rsplit("/",1)[-1],ctx)
def _dirof(x):
    x=_force(x)
    if type(x) is _P:
        text=str(x)
        if text=="/": return _P("/")
        if text.endswith("/") and text!="/": text=text[:-1]
        if "/" not in text: return _P(".")
        head=text.rsplit("/",1)[0]
        return _P("/" if head=="" else head)
    else:
        text,ctx=_strctx(x,"builtins.dirOf path")
    if text=="/": return _mkstr("/",ctx)
    if text.endswith("/") and text!="/": text=text[:-1]
    if "/" not in text: return _mkstr(".",ctx)
    head=text.rsplit("/",1)[0]
    return _mkstr("/" if head=="" else head,ctx)
def _topath(x,label):
    return _P(_fspath(x,label))
def _topathstr(x):
    text,ctx=_strctx(x,"builtins.toPath string")
    if not text.startswith("/"): raise Exception("string '%s' doesn't represent an absolute path"%text)
    return _mkstr(_normpath_text(text),ctx)
def _getenv(name):
    key=_strarg(name,"getEnv")
    return os.environ.get(key,"") if key.startswith("PNIX_") else ""
_RX_POSIX={"[:alnum:]":"A-Za-z0-9","[:alpha:]":"A-Za-z","[:blank:]":r"\x09\x20","[:cntrl:]":r"\x00-\x1f\x7f","[:digit:]":"0-9","[:graph:]":r"\x21-\x7e","[:lower:]":"a-z","[:print:]":r"\x20-\x7e","[:punct:]":r"\x21-\x2f\x3a-\x40\x5b-\x60\x7b-\x7e","[:space:]":r"\x09-\x0d\x20","[:upper:]":"A-Z","[:xdigit:]":"A-Fa-f0-9"}
def _rxpat(p):
    p=str(p); out=[]; i=0; in_bracket=False; bracket_has_member=False
    while i<len(p):
        c=p[i]
        if c=="\\" and i+1<len(p):
            out.append(p[i:i+2])
            if in_bracket: bracket_has_member=True
            i+=2; continue
        if not in_bracket:
            out.append(c)
            if c=="[": in_bracket=True; bracket_has_member=False
            i+=1; continue
        if c=="[" and p.startswith("[:",i):
            end=p.find(":]",i+2)
            if end<0: raise re.error("unterminated POSIX character class")
            marker=p[i:end+2]
            replacement=_RX_POSIX.get(marker)
            if replacement is None: raise re.error("unknown POSIX character class '%s'"%p[i+2:end])
            out.append(replacement); bracket_has_member=True; i=end+2; continue
        out.append(c)
        if c=="]" and bracket_has_member: in_bracket=False
        elif not (c=="^" and not bracket_has_member): bracket_has_member=True
        i+=1
    return "".join(out)
def _rxerrmsg(exc):
    detail=str(exc)
    if "unterminated" in detail and "unclosed" not in detail:
        detail=detail+" (unclosed)"
    return "invalid regex: "+detail
def _rxcompile(p,label):
    try:
        return re.compile(_rxpat(p))
    except re.error as exc:
        raise Exception(label+": "+_rxerrmsg(exc))
def _rxmatch(pattern,text):
    s,ctx=_strctx(text,"builtins.match string")
    m=_rxcompile(_str(pattern,"builtins.match regex"),"builtins.match").fullmatch(s)
    if m is None: return None
    return [_mkstr(g,ctx) if g is not None else None for g in m.groups()]
def _rxsplit(pattern,text):
    p=_str(pattern,"builtins.split regex")
    if p=="": raise Exception("builtins.split: regex pattern cannot be empty")
    s,ctx=_strctx(text,"builtins.split string"); out=[]; last=0
    for m in _rxcompile(p,"builtins.split").finditer(s):
        out.append(_mkstr(s[last:m.start()],ctx))
        out.append([_mkstr(g,ctx) if g is not None else None for g in m.groups()])
        last=m.end()
    out.append(_mkstr(s[last:],ctx))
    return out
def _tryeval(x):
    try:
        return {"success":True,"value":_force(x)}
    except _Catch:
        return {"success":False,"value":False}
def _jsonint(token):
    if token=="-0": return -0.0
    parsed=int(token)
    if not (-(2**63) <= parsed <= 2**63-1):
        raise Exception("builtins.fromJSON: integer literal too large for i64: "+token)
    return parsed
def _jsonconstant(token):
    raise Exception("builtins.fromJSON: invalid JSON numeric constant "+token)
def _fromjson(x):
    try:
        return json.loads(_str(x,"builtins.fromJSON string"),parse_int=_jsonint,parse_constant=_jsonconstant)
    except json.JSONDecodeError as exc:
        raise Exception("builtins.fromJSON: parse error: "+str(exc))
def _tomlval(x):
    if isinstance(x,dict): return {str(k):_tomlval(v) for k,v in x.items()}
    if isinstance(x,list): return [_tomlval(v) for v in x]
    if type(x) in (str,int,float,bool): return x
    return str(x)
def _fromtoml(x):
    try:
        return _tomlval(tomllib.loads(_expectedstr(x,"builtins.fromTOML")))
    except tomllib.TOMLDecodeError as exc:
        raise Exception("builtins.fromTOML: parse error: "+str(exc))
def _mesc(s,attr=False):
    s=s.replace("&","&amp;").replace("<","&lt;").replace(">","&gt;")
    return s.replace('"',"&quot;") if attr else s
def _mstr(x,label):
    x=_force(x)
    if x is None: return ""
    if type(x) is bool: return "true" if x else "false"
    if type(x) in (int,float): return str(x)
    if type(x) is _P: return str(x)
    if _isstr(x): return str(x)
    raise Exception(label+" must be string-compatible")
def _etnode(el):
    children=[]
    if el.text: children.append({"kind":"text","value":el.text})
    for child in list(el):
        children.append(_etnode(child))
        if child.tail: children.append({"kind":"text","value":child.tail})
    return {"kind":"element","name":str(el.tag),"attrs":{str(k):str(v) for k,v in sorted(el.attrib.items())},"children":children}
def _xmlparse(x): return _etnode(ET.fromstring(_strarg(x,"xmlParse")))
def _htmlparse(x):
    wrapper=ET.fromstring("<pnix-hy-document>"+_strarg(x,"htmlParse")+"</pnix-hy-document>")
    children=[]
    if wrapper.text: children.append({"kind":"text","value":wrapper.text})
    for child in list(wrapper):
        children.append(_etnode(child))
        if child.tail: children.append({"kind":"text","value":child.tail})
    return {"kind":"document","children":children}
def _mattrs(x,html=False):
    x=_force(x)
    if x is None: return []
    out=[]
    if isinstance(x,dict):
        for k,v in x.items():
            name=str(k).lower() if html else str(k)
            out.append((name,_mstr(v,"markup attr value")))
    elif isinstance(x,list):
        for item in x:
            d=_force(item)
            if not isinstance(d,dict): raise Exception("markup attr must be attrset")
            name=_str(d.get("name"),"markup attr name")
            if "value" not in d: raise Exception("markup attr missing value")
            out.append((name.lower() if html else name,_mstr(d["value"],"markup attr value")))
    else:
        raise Exception("markup attrs must be attrset or list")
    return sorted(out)
_HTML_VOID={"area","base","br","col","embed","hr","img","input","link","meta","param","source","track","wbr"}
def _memitnode(x,html=False):
    d=_force(x)
    if not isinstance(d,dict): raise Exception("markup node must be attrset")
    kind=_str(d.get("kind"),"markup kind")
    if kind=="document": return "".join(_memitnode(c,html) for c in _force(d.get("children",[])))
    if kind=="text": return _mesc(_mstr(d.get("value",d.get("text","")),"markup text"))
    if kind=="comment": return "<!--"+_mstr(d.get("value",""),"markup comment")+"-->"
    if kind=="cdata" and not html: return "<![CDATA["+_mstr(d.get("value",d.get("text","")),"markup cdata")+"]]>"
    if kind!="element" and "name" not in d: raise Exception("markup.emit: unknown kind `%s`"%kind)
    name=_str(d.get("name"),"markup element name")
    if html: name=name.lower()
    attrs="".join(" %s=\"%s\""%(k,_mesc(v,True)) for k,v in _mattrs(d.get("attrs",{}),html))
    children=_force(d.get("children",[]))
    if html:
        if name in _HTML_VOID: return "<"+name+attrs+">"
        return "<"+name+attrs+">"+"".join(_memitnode(c,True) for c in children)+"</"+name+">"
    if not children: return "<"+name+attrs+"/>"
    return "<"+name+attrs+">"+"".join(_memitnode(c,False) for c in children)+"</"+name+">"
def _memit(x,html=False):
    x=_force(x)
    if isinstance(x,list): text="".join(_memitnode(v,html) for v in x)
    else: text=_memitnode(x,html)
    return _mkstr(text,_collectctx(x))
def _schemaroot(schema):
    d=_force(schema)
    if not isinstance(d,dict): raise Exception("schema.validate: schema must be attrset")
    if "kind" in d: return d
    if "root" in d: return _force(d["root"])
    raise Exception("schema.validate: schema must define kind or root")
def _schematype(x):
    x=_force(x)
    if x is None: return "null"
    if type(x) is bool: return "bool"
    if type(x) is int: return "int"
    if type(x) is float: return "float"
    if type(x) is str: return "string"
    if isinstance(x,list): return "list"
    if isinstance(x,dict): return "set"
    return type(x).__name__
def _schemaerr(path,code,msg): return {"path":path,"code":code,"message":msg}
def _schemaerrors(schema,value,path=None):
    path=path or ["root"]; sm=_schemaroot(schema); value=_force(value)
    kind=_str(sm.get("kind"),"schema kind")
    if kind=="any": return []
    if kind=="string":
        if not isinstance(value,str): return [_schemaerr(path,"type","expected string, got "+_schematype(value))]
        if "minLength" in sm and len(value)<_force(sm["minLength"]): return [_schemaerr(path,"constraint","expected min length")]
        return []
    if kind=="bool": return [] if type(value) is bool else [_schemaerr(path,"type","expected bool, got "+_schematype(value))]
    if kind=="int": return [] if type(value) is int else [_schemaerr(path,"type","expected int, got "+_schematype(value))]
    if kind in {"float","number"}: return [] if type(value) in (int,float) and type(value) is not bool else [_schemaerr(path,"type","expected number, got "+_schematype(value))]
    if kind=="list":
        if not isinstance(value,list): return [_schemaerr(path,"type","expected list, got "+_schematype(value))]
        if "elem" not in sm: return []
        out=[]
        for i,item in enumerate(value): out.extend(_schemaerrors(sm["elem"],item,path+[str(i)]))
        return out
    if kind in {"attrs","map"}: return [] if isinstance(value,dict) else [_schemaerr(path,"type","expected set, got "+_schematype(value))]
    if kind=="record":
        if not isinstance(value,dict): return [_schemaerr(path,"type","expected record, got "+_schematype(value))]
        fields=_force(sm.get("fields"))
        if not isinstance(fields,dict): raise Exception("schema.validate: record fields must be attrset")
        optional={_str(x,"schema optional item") for x in _force(sm.get("optional",[]))}
        out=[]
        for field in sorted(fields):
            fs=_force(fields[field])
            if field in value: out.extend(_schemaerrors(fields[field],value[field],path+[field]))
            elif field not in optional and not (isinstance(fs,dict) and "default" in fs):
                out.append(_schemaerr(path+[field],"missing","missing required field "+field))
        return out
    return [_schemaerr(path,"schema","unsupported schema kind "+kind)]
def _schemanorm(schema,value):
    sm=_schemaroot(schema); value=_force(value)
    if _str(sm.get("kind"),"schema kind")!="record": return value
    if not isinstance(value,dict): raise Exception("schemaNormalize value must be attrset")
    fields=_force(sm.get("fields"))
    out=dict(value)
    for field in sorted(fields):
        fs=_force(fields[field])
        if field in out: out[field]=_schemanorm(fields[field],out[field])
        elif isinstance(fs,dict) and "default" in fs: out[field]=_schemanorm(fields[field],fs["default"])
    return out
def _schemavalidate(schema,value):
    errors=_schemaerrors(schema,value)
    ok=(len(errors)==0)
    return {"success":ok,"ok":ok,"errors":errors}
def _schemaexplain(schema,value):
    return "\n".join(".".join(e["path"])+": "+e["code"]+": "+e["message"] for e in _schemaerrors(schema,value))
def _gcsig(x): return json.dumps(_realize(x), ensure_ascii=False, sort_keys=True, separators=(",",":"))
def _genericclosure(arg):
    d=_force(arg)
    if not isinstance(d,dict): raise Exception("builtins.genericClosure: expected attrset")
    if "startSet" not in d: raise Exception("builtins.genericClosure: argument missing required attribute 'startSet'")
    if "operator" not in d: raise Exception("builtins.genericClosure: argument missing required attribute 'operator'")
    start=_force(d["startSet"])
    if not isinstance(start,list): raise Exception("builtins.genericClosure: startSet must be list")
    op=d["operator"]; work=list(start); seen=set(); out=[]; steps=0
    while work:
        steps+=1
        if steps>10000: raise Exception("builtins.genericClosure: maximum depth 10000 exceeded")
        if len(work)>100000: raise Exception("builtins.genericClosure: work list size 100000 exceeded")
        item=_force(work.pop())
        if not isinstance(item,dict): raise Exception("builtins.genericClosure: item must be attrset")
        if "key" not in item: raise Exception("builtins.genericClosure: item missing 'key' attribute")
        sig=_gcsig(item["key"])
        if sig in seen: continue
        seen.add(sig); out.append(item)
        nxt=_force(_apply(op,_tv(item)))
        if not isinstance(nxt,list): raise Exception("builtins.genericClosure: operator must return list")
        work.extend(reversed(nxt))
    return out
def _pow(a,b):
    A=_force(a); B=_force(b)
    if type(A) is int and type(B) is int and B>=0:
        exact=A**B
        if -(2**63) <= exact <= 2**63-1: return exact
    return math.pow(_num(A,"builtins.pow base"),_num(B,"builtins.pow exponent"))
def _bit(name,a,b,fn):
    a=_force(a)
    if type(a) is not int: raise Exception("builtins."+name+": first arg must be int, got "+_typeof(a))
    b=_force(b)
    if type(b) is not int: raise Exception("builtins."+name+": second arg must be int, got "+_typeof(b))
    return fn(a,b)
def _addctx(msg,value):
    m=_force(msg)
    if not _isstr(m): raise Exception("builtins.addErrorContext: context must be string, got "+_typeof(m))
    return _force(value)
def _attrpos(name,attrs):
    n=_str(name,"builtins.unsafeGetAttrPos name")
    d=_force(attrs)
    if not isinstance(d,dict): raise Exception("builtins.unsafeGetAttrPos attrs must be an attrset")
    if n not in d: return None
    pos=getattr(d,"attr_positions",{}).get(n)
    return None if pos is None else _srcpos(pos)
def _splitparts(s):
    out=[]; start=None; last_digit=None
    for idx,ch in enumerate(s):
        is_digit=ch.isascii() and ch.isdigit()
        if ch=="." or ch=="-":
            if start is not None:
                out.append(s[start:idx]); start=None
            last_digit=None
        elif last_digit is not None and last_digit!=is_digit:
            if start is not None: out.append(s[start:idx])
            start=idx; last_digit=is_digit
        else:
            if start is None: start=idx
            last_digit=is_digit
    if start is not None: out.append(s[start:])
    return out
def _splitver(x):
    s,ctx=_strctx(x,"builtins.splitVersion string")
    return [_mkstr(part,ctx) for part in _splitparts(s)]
def _parsedrv(x):
    s,ctx=_strctx(x,"builtins.parseDrvName string")
    idx=None
    for i in range(0,max(0,len(s)-1)):
        if s[i]=="-" and s[i+1].isascii() and s[i+1].isdigit():
            idx=i; break
    if idx is None: return {"name":_mkstr(s,ctx),"version":_mkstr("",ctx)}
    return {"name":_mkstr(s[:idx],ctx),"version":_mkstr(s[idx+1:],ctx)}
def _cmpver(a,b):
    A=_force(a); B=_force(b)
    if not _isstr(A) or not _isstr(B): raise Exception("builtins.compareVersions: expected two strings")
    A=_splitparts(str(A)); B=_splitparts(str(B))
    def cmpcomp(x,y):
        xn=int(x) if x.isdigit() else None
        yn=int(y) if y.isdigit() else None
        if xn is not None and yn is not None: return -1 if xn<yn else (1 if xn>yn else 0)
        if xn is not None: return 1
        if yn is not None: return -1
        if x==y: return 0
        if x=="": return 1 if y=="pre" else -1
        if y=="": return -1 if x=="pre" else 1
        if x=="pre": return -1
        if y=="pre": return 1
        return -1 if x<y else 1
    for i in range(max(len(A),len(B))):
        c=cmpcomp(A[i] if i<len(A) else "",B[i] if i<len(B) else "")
        if c!=0: return c
    return 0
def _system():
    m=platform.machine() or "unknown"
    if sys.platform=="darwin": return m+"-darwin"
    if sys.platform.startswith("linux"): return m+"-linux"
    return m+"-"+sys.platform
def _srcfile():
    s=str(globals().get("_PX_SOURCE_PATH","<pnix-px>"))
    return "fixtures/"+s.split("/fixtures/",1)[1] if "/fixtures/" in s else s
def _srcpos(pos):
    src=str(globals().get("_PX_SOURCE_TEXT",""))
    pos=max(0,min(int(pos),len(src)))
    line=src.count("\n",0,pos)+1
    start=src.rfind("\n",0,pos)
    col=pos+1 if start<0 else pos-start
    return {"file":_srcfile(),"line":line,"column":col}
def _less(a,b,depth=0):
    if depth>_VALUES_EQUAL_MAX_DEPTH: raise Exception("infinite recursion encountered during comparison")
    A=_force(a); B=_force(b)
    if depth>0 and A is B: return False
    if type(A) is _P or type(B) is _P:
        if type(A) is _P and type(B) is _P: return _normpath_text(A)<_normpath_text(B)
        raise Exception("cannot compare "+_typeof(A)+" with "+_typeof(B))
    if type(A) is _S: A=str(A)
    if type(B) is _S: B=str(B)
    if isinstance(A,bool) or isinstance(B,bool): raise Exception("cannot compare booleans with `<`")
    if (type(A) is int or type(A) is float) and (type(B) is int or type(B) is float):
        return float(A)<float(B) if type(A) is float or type(B) is float else A<B
    if isinstance(A,str) and isinstance(B,str):
        if A.startswith("#<pnix-hy-") or B.startswith("#<pnix-hy-"): raise Exception("cannot compare functions with `<`")
        return A<B
    if isinstance(A,list) and isinstance(B,list):
        for x,y in zip(A,B):
            if _eq(x,y,depth+1): continue
            return _less(x,y,depth+1)
        return len(A)<len(B)
    raise Exception("cannot compare "+_typeof(A)+" with "+_typeof(B))
def _ci(v,op):
    if type(v) is int and not (-(2**63) <= v <= 2**63-1): raise Exception("integer overflow in `%s`"%op)
    return v
def _uneg(v):
    v=_num(v,"argument of unary -")
    return 0.0 if type(v) is float and v==0.0 else _ci(-v,"-")
def _floati64(x,label,func):
    v=_num(x,label+" argument")
    if type(v) is int:
        as_float=float(v)
        if as_float>=float(2**63) or as_float<float(-(2**63)):
            raise Exception(label+": integer outside i64 range after f64 conversion")
        if int(as_float)!=v: raise Exception(label+": integer loses precision when converted to f64")
        v=as_float
    if math.isnan(v): raise Exception(label+": NaN outside i64 range")
    if math.isinf(v): raise Exception(label+(": +inf outside i64 range" if v>0 else ": -inf outside i64 range"))
    rounded=func(v)
    if rounded>=2**63 or rounded<-(2**63): raise Exception(label+": value outside i64 range")
    return _ci(int(rounded),label)
def _apair(op,A,B):
    if (type(A) not in (int,float)) or (type(B) not in (int,float)):
        raise Exception("operator "+op+": unsupported operand types "+_typeof(A)+" and "+_typeof(B))
    return A,B
def _modbuiltin(a,b):
    L=_num(a,"builtins.mod"); R=_num(b,"builtins.mod")
    if R==0: raise Exception("builtins.mod: division by zero")
    if type(L) is float or type(R) is float: return math.fmod(L,R)
    if L==-(2**63) and R==-1: raise Exception("integer overflow in `%`")
    q=abs(L)//abs(R)
    if (L<0)!=(R<0): q=-q
    return _ci(L-q*R,"%")
def _bin(op,a,b):
    A=_force(a); B=_force(b)
    if op=="+":
        if type(A) is _P and type(B) is _P:
            return _P(str(A)+str(B))
        if type(A) is _P and _isstr(B):
            if _ctx(B):
                raise Exception("operator +: path + context-bearing string would drop string context; use builtins.unsafeDiscardStringContext to discard it explicitly")
            return _P(str(A)+str(B))
        if _isstr(A) and type(B) is _P:
            c=_ctx(A); c.add(str(B)); return _mkstr(str(A)+str(B),c)
        if _isstr(A) and _isstr(B):
            c=_ctx(A); c.update(_ctx(B)); return _mkstr(str(A)+str(B),c)
        if isinstance(A,list) and isinstance(B,list): return A+B
        if isinstance(A,dict) and isinstance(B,dict):
            m=dict(A); m.update(B); return m
        _apair("+",A,B); return _ci(A+B,"+")
    if op=="-": L,R=_apair("-",A,B); return _ci(L-R,"-")
    if op=="*": L,R=_apair("*",A,B); return _ci(L*R,"*")
    if op=="/":
        L,R=_apair("/",A,B)
        if R==0: raise Exception("division by zero")
        if type(L) is float or type(R) is float: return L/R
        q=abs(L)//abs(R)
        if (L<0)!=(R<0): q=-q
        return _ci(q,"/")
    if op=="%":
        L,R=_apair("%",A,B)
        if R==0: raise Exception("modulo by zero")
        if type(L) is float or type(R) is float: return math.fmod(L,R)
        if L==-(2**63) and R==-1: raise Exception("integer overflow in `%`")
        q=abs(L)//abs(R)
        if (L<0)!=(R<0): q=-q
        return _ci(L-q*R,"%")
    if op=="==": return _eq(A,B)
    if op=="!=": return not _eq(A,B)
    if op=="<": return _less(A,B)
    if op=="<=": return not _less(B,A)
    if op==">": return _less(B,A)
    if op==">=": return not _less(A,B)
    if op=="->": return (not _bool(A,"left operand of ->")) or _bool(B,"right operand of ->")
    if op=="//":
        if A is None: return B
        if B is None: return A
        m=dict(A); m.update(B); return m
    if op=="++":
        if isinstance(A,list) and isinstance(B,list): return A+B
        raise Exception("both sides of ++ must be lists")
    raise Exception("binop "+op)
def _sel(base,attr):
    b=_force(base)
    if not isinstance(b,dict): raise Exception("select base must be an attrset")
    if attr not in b: raise Exception("missing attr `%s`"%attr)
    return _force(b[attr])
def _seldef(base,attr,default):
    b=_force(base)
    if isinstance(b,dict) and attr in b: return _force(b[attr])
    return _force(default)
def _attrkey(v):
    v=_force(v)
    if type(v) is _P: return str(v)
    if _isstr(v): return str(v)
    return _vts(v)
def _attrpath(parts):
    return ".".join(_attrkey(p) for p in parts)
def _attrparts(parts):
    return [_attrkey(p) for p in parts]
def _dynsel(base,parts):
    # each part is a SEPARATE select step (Nix: s.${k}.c walks ${k} THEN c;
    # joining was a real bug — korean-nl-mirror harvest, 2026-07-11)
    v=_force(base)
    for p in parts:
        a=_attrpath([p])
        if not isinstance(v,dict): raise Exception("select base must be an attrset")
        if a not in v: raise Exception("missing attr `%s`"%a)
        v=_force(v[a])
    return v
def _dyndef(base,parts,default):
    v=_force(base)
    for p in parts:
        a=_attrpath([p])
        if isinstance(v,dict) and a in v: v=_force(v[a])
        else: return _force(default)
    return v
def _dynhas(base,parts):
    b=_force(base)
    # Nix `?` is false on non-sets (unlike builtins.hasAttr, which errors).
    if not isinstance(b,dict): return False
    return _has(b,_attrparts(parts))
def _has(d,path):
    cur=d
    last=len(path)-1
    for idx,p in enumerate(path):
        if not isinstance(cur,dict) or p not in cur: return False
        if idx==last: return True
        cur=_force(cur[p])
    return True
def _index(base,index):
    b=_force(base); i=_force(index)
    if isinstance(b,list):
        if type(i) is not int: raise Exception("index must be an integer")
        return _force(b[i])
    if isinstance(b,dict) and isinstance(i,str):
        if i not in b: raise Exception("missing attr `%s`"%i)
        return _force(b[i])
    raise Exception("index target unsupported")
_COERCE_STACK=set()
def _coerce(v):
    v=_force(v)
    if type(v) is _P: return _mkstr(str(v),{str(v)})
    if _isstr(v): return _mkstr(str(v),_ctx(v))
    if type(v) is int: raise Exception("cannot coerce a number to a string in interpolation: use builtins.toString")
    if type(v) is bool: raise Exception("cannot coerce a boolean to a string in interpolation: use builtins.toString")
    if v is None: raise Exception("cannot coerce null to a string in interpolation")
    if isinstance(v,list): raise Exception("cannot coerce a list to a string in interpolation")
    if isinstance(v,dict):
        oid=id(v)
        if oid in _COERCE_STACK: raise Exception("interpolation coercion cycle involving __toString")
        _COERCE_STACK.add(oid)
        try:
            if "__toString" in v: return _coerce(_apply(v["__toString"],_tv(v)))
            if "outPath" in v: return _coerce(v["outPath"])
            raise Exception("cannot coerce a set to a string in interpolation: no __toString or outPath")
        finally:
            _COERCE_STACK.discard(oid)
    if type(v) is _C: raise Exception("cannot coerce a function to a string in interpolation")
    raise Exception("cannot coerce value to a string in interpolation")
def _vts(x,seen=None):
    seen=_TO_STRING_SEEN if seen is None else seen
    x=_force(x)
    if x is None: return ""
    if x is True: return "1"
    if x is False: return ""
    if type(x) is int: return str(x)
    if type(x) is float: return format(x,".6f")
    if type(x) is _P: return _mkstr(str(x),{str(x)})
    if _isstr(x):
        if x.startswith("#<pnix-hy-"): raise Exception("cannot coerce a function to a string")
        return _mkstr(str(x),_ctx(x))
    if isinstance(x,list):
        out=[]; ctx=set()
        for i in x:
            s=_vts(i,seen)
            out.append(str(s)); ctx.update(_ctx(s))
        return _mkstr(" ".join(out),ctx)
    if type(x) is _C: raise Exception("cannot coerce a function to a string")
    if isinstance(x,dict):
        oid=id(x)
        if oid in seen: raise Exception("toString cycle detected")
        seen.add(oid)
        try:
            if "__toString" in x: return _vts(_apply(_force(x["__toString"]),_tv(x)),seen)
            if "outPath" in x: return _vts(x["outPath"],seen)
            raise Exception("cannot coerce a set to a string: missing __toString or outPath")
        finally:
            seen.discard(oid)
    raise Exception("cannot coerce value to a string")
def _deepforce(x,seen=None,label="builtins.deepSeq"):
    seen=set() if seen is None else seen
    x=_force(x)
    if type(x) is _K:
        oid=id(x)
        if oid in seen: raise Exception(label+": infinite recursion encountered (cyclic value)")
        seen.add(oid)
        try:
            for a in x.args: _deepforce(a,seen,label)
        finally:
            seen.discard(oid)
    elif isinstance(x,dict):
        oid=id(x)
        if oid in seen: raise Exception(label+": infinite recursion encountered (cyclic value)")
        seen.add(oid)
        try:
            for v in x.values(): _deepforce(v,seen,label)
        finally:
            seen.discard(oid)
    elif isinstance(x,list):
        oid=id(x)
        if oid in seen: raise Exception(label+": infinite recursion encountered (cyclic value)")
        seen.add(oid)
        try:
            for v in x: _deepforce(v,seen,label)
        finally:
            seen.discard(oid)
    return x
def _cjson(x): return _mkstr(json.dumps(_jsonready(x), ensure_ascii=False, sort_keys=True, separators=(",",":")),_collectctx(x))
def _typeof(x):
    x=_force(x)
    if isinstance(x,bool): return "bool"
    if isinstance(x,int): return "int"
    if isinstance(x,float): return "float"
    if type(x) is _P: return "path"
    if isinstance(x,str): return "string"
    if isinstance(x,list): return "list"
    if isinstance(x,dict): return "set"
    if x is None: return "null"
    if type(x) is _C: return "lambda"
    if type(x) is _K: return "construct"
    return "unknown"
def _cell(x): return x if type(x) is _T else _tv(x)
def _merge_bind(a,b):
    out=dict(a)
    for k,v in b.items():
        if k in out and not _eq(out[k],v): return None
        out[k]=v
    return out
def _pat(p,v):
    tag=p["tag"]
    if tag=="wildcard": return {}
    if tag=="as":
        m=_pat(p["pattern"],v)
        if m is None: return None
        return _merge_bind(m,{p["name"]:_cell(v)})
    if tag=="var": return {p["name"]:_cell(v)}
    if tag=="literal": return {} if _eq(v,p["value"]) else None
    if tag=="list":
        xs=_force(v)
        rest=p.get("rest")
        if not isinstance(xs,list): return None
        if rest is None and len(xs)!=len(p["items"]): return None
        if rest is not None and len(xs)<len(p["items"]): return None
        out={}
        for sp,it in zip(p["items"],xs[:len(p["items"])]):
            m=_pat(sp,it)
            if m is None: return None
            out=_merge_bind(out,m)
            if out is None: return None
        if rest is not None:
            out=_merge_bind(out,{rest:_cell(xs[len(p["items"]):])})
            if out is None: return None
        return out
    if tag=="attrset":
        d=_force(v)
        if not isinstance(d,dict): return None
        out={}
        for f in p["fields"]:
            n=f["name"]
            if n in d:
                pv=d[n]
            elif "default" in f:
                pv=f["default"](out)
            else:
                return None
            m=_pat(f["pattern"],pv)
            if m is None: return None
            out=_merge_bind(out,m)
            if out is None: return None
        return out
    if tag=="constructor":
        k=_force(v)
        if type(k) is not _K or k.variant!=p["variant"] or len(k.args)!=len(p["args"]): return None
        out={}
        for sp,it in zip(p["args"],k.args):
            m=_pat(sp,it)
            if m is None: return None
            out=_merge_bind(out,m)
            if out is None: return None
        return out
    raise Exception("unsupported match pattern")
def _bindpat(p,v):
    dup=_formaldup(p)
    if dup is not None: raise Exception("duplicate formal function argument '%s'"%dup)
    q=p["pattern"] if p.get("tag")=="as" else p
    m=_pat(p,v)
    if m is None:
        if q.get("tag")=="list": raise Exception("function argument does not match list pattern")
        raise Exception("function argument does not match pattern")
    if q.get("tag")=="attrset" and not q.get("ellipsis"):
        fv=_force(v)
        if isinstance(fv,dict):
            allowed=set(f["name"] for f in q["fields"])
            for k in fv:
                if k not in allowed: raise Exception("unexpected attribute '%s'"%k)
    return m
def _formaldup(p):
    bind=None
    q=p
    if p.get("tag")=="as":
        bind=p.get("name")
        q=p.get("pattern",{})
    if q.get("tag")!="attrset": return None
    seen=set()
    if bind is not None: seen.add(str(bind))
    for f in q.get("fields",[]):
        name=str(f["name"])
        if name in seen: return name
        seen.add(name)
    return None
def _match(v,arms):
    for arm in arms:
        if len(arm)==2:
            p,fn=arm; guard=None
        else:
            p,guard,fn=arm
        m=_pat(p,v)
        if m is not None and (guard is None or _bool(guard(m),"match guard")): return fn(m)
    raise Exception("non-exhaustive match")
def _fold(f,init,xs):
    acc=init
    for it in _list(xs,"builtins.foldl' list"):
        acc=_apply(_apply(f,_tv(_force(acc))),_tv(_force(it)))
    return acc
def _foldr(f,init,xs):
    acc=init
    for it in reversed(_list(xs,"builtins.foldr list")):
        acc=_apply(_apply(f,_tv(_force(it))),_tv(_force(acc)))
    return acc
def _seq(a,b):
    _force(a)
    return _force(b)
def _genlist(f,n):
    count=_nonneg(n,"builtins.genList length")
    if count>16*1024*1024:
        raise Exception("builtins.genList count %s exceeds maximum 16777216"%count)
    return [_T(lambda i=i: _apply(f,_tv(i))) for i in range(count)]
def _filterlist(f,xs):
    out=[]
    for index,it in enumerate(_listarg(xs,"filter","second argument")):
        if _predbool(_apply(f,_tv(_force(it))),"builtins.filter predicate",index):
            out.append(_force(it))
    return out
def _anylist(p,xs):
    for index,it in enumerate(_listarg(xs,"any","second argument")):
        if _predbool(_apply(p,_tv(_force(it))),"builtins.any predicate",index):
            return True
    return False
def _alllist(p,xs):
    for index,it in enumerate(_listarg(xs,"all","second argument")):
        if not _predbool(_apply(p,_tv(_force(it))),"builtins.all predicate",index):
            return False
    return True
def _sort(p,xs):
    def cmp(a,b):
        l=_predbool(_apply(_apply(p,_tv(_force(a))),_tv(_force(b))),"builtins.sort comparator")
        r=_predbool(_apply(_apply(p,_tv(_force(b))),_tv(_force(a))),"builtins.sort comparator")
        return -1 if l else (1 if r else 0)
    return sorted(xs,key=cmp_to_key(cmp))
def _bi():
    b={}
    b["currentSystem"]=_system()
    b["nixVersion"]="2.18.0-pnix"
    b["langVersion"]=6
    b["storeDir"]="/nix/store"
    b["import"]=_C(lambda path:_import(path))
    b["scopedImport"]=_C(lambda scope:_C(lambda path:_scopedimport(scope,path)))
    b["pathExists"]=_C(lambda p: os.path.exists(_fspath(p,"builtins.pathExists")))
    b["readFile"]=_C(lambda p:_readfile(p))
    b["readFileType"]=_C(lambda p:_readfiletype(_fspath(p,"builtins.readFileType")))
    b["readDir"]=_C(lambda p:_readdir(p))
    b["toFile"]=_C(lambda name:_C(lambda contents:_tofile(name,contents)))
    b["hashString"]=_C(lambda algo:_hashstr(algo))
    b["hashFile"]=_C(lambda algo:_C(lambda path:_hashfile(algo,path)))
    b["baseNameOf"]=_C(lambda p:_basename(p))
    b["dirOf"]=_C(lambda p:_dirof(p))
    b["toPath"]=_C(lambda value:_topathstr(value))
    b["storePath"]=_C(lambda value:_topath(value,"builtins.storePath"))
    b["getEnv"]=_C(lambda name:_getenv(name))
    b["placeholder"]=_C(lambda name:_mkstr("/pnix-placeholder/"+_str(name,"builtins.placeholder name"),{"=placeholder!"+_str(name,"builtins.placeholder name")}))
    b["break"]=_C(lambda value:_force(value))
    b["warn"]=_C(lambda msg:_C(lambda value: (_str(msg,"builtins.warn message"),_force(value))[1]))
    b["traceVerbose"]=_C(lambda msg:_C(lambda value: (_str(msg,"builtins.traceVerbose message"),_force(value))[1]))
    b["attrNames"]=_C(lambda m: sorted(_attrsarg(m,"attrNames").keys()))
    b["hasAttr"]=_C(lambda nm:_C(lambda m:_hasattr(nm,m)))
    b["getAttr"]=_C(lambda nm:_C(lambda m: _getattr_builtin(nm,m)))
    b["attrByPath"]=_C(lambda p:_C(lambda default:_C(lambda m:_attrbypath(p,default,m))))
    b["removeAttrs"]=_C(lambda m:_C(lambda names:_rmattrs(m,names)))
    b["listToAttrs"]=_C(lambda xs:_listtoattrs(xs))
    b["filterAttrs"]=_C(lambda p:_C(lambda m:_filterattrs(p,m)))
    b["functionArgs"]=_C(lambda f:_fnargs(f))
    b["intersectAttrs"]=_C(lambda a:_C(lambda b2:_intersectattrs(a,b2)))
    b["zipAttrsWith"]=_C(lambda f:_C(lambda xs:_zipattrswith(f,xs)))
    b["catAttrs"]=_C(lambda name:_C(lambda xs:_catattrs(name,xs)))
    b["elemAt"]=_C(lambda xs:_C(lambda i:_elemat(xs,i)))
    b["length"]=_C(lambda x: _length(x))
    b["head"]=_C(lambda xs:_head(xs))
    b["tail"]=_C(lambda xs:_tail(xs))
    b["toString"]=_C(lambda x:_vts(x))
    b["toJSON"]=_C(lambda x:_cjson(x))
    b["map"]=_C(lambda f:_C(lambda xs:_maplist(f,xs)))
    b["filter"]=_C(lambda f:_C(lambda xs:_filterlist(f,xs)))
    b["foldl'"]=_C(lambda f:_C(lambda init:_C(lambda xs:_fold(f,init,_listarg(xs,"foldl'","third arg")))))
    b["fold"]=_C(lambda f:_C(lambda init:_C(lambda xs:_fold(f,init,_listarg(xs,"fold","third arg")))))
    b["foldl"]=_C(lambda f:_C(lambda init:_C(lambda xs:_fold(f,init,_listarg(xs,"foldl","third arg")))))
    b["foldr"]=_C(lambda f:_C(lambda init:_C(lambda xs:_foldr(f,init,_listarg(xs,"foldr","third arg")))))
    b["cons"]=_C(lambda x:_C(lambda xs:[_force(x)]+_force(xs)))
    b["append"]=_C(lambda xs:_C(lambda ys:_force(xs)+_force(ys)))
    b["take"]=_C(lambda n:_C(lambda xs:_take(n,xs)))
    b["drop"]=_C(lambda n:_C(lambda xs:_drop(n,xs)))
    b["reverse"]=_C(lambda xs:list(reversed(_force(xs))))
    b["reverseList"]=_C(lambda xs:list(reversed(_force(xs))))
    b["zip"]=_C(lambda a:_C(lambda b2:_ziplist(a,b2)))
    b["flatten"]=_C(lambda xs:_flatten(xs))
    b["find"]=_C(lambda needle:_C(lambda xs:_find(needle,xs)))
    b["get"]=_C(lambda m:_C(lambda name:_get(m,name)))
    b["mapGet"]=_C(lambda m:_C(lambda name:_get(m,name)))
    b["set"]=_C(lambda m:_C(lambda name:_C(lambda value:_set(m,name,value))))
    b["mapSet"]=_C(lambda m:_C(lambda name:_C(lambda value:_set(m,name,value))))
    b["keys"]=_C(lambda m:_keys(m,"builtins.keys attrs"))
    b["mapKeys"]=_C(lambda m:_keys(m,"builtins.mapKeys attrs"))
    b["values"]=_C(lambda m:_values(m))
    b["mapValues"]=_C(lambda m:_values(m))
    b["merge"]=_C(lambda a:_C(lambda b2:_merge(a,b2)))
    b["mapMerge"]=_C(lambda a:_C(lambda b2:_merge(a,b2)))
    b["elem"]=_C(lambda x:_C(lambda xs: any(_eq(x,it,1) for it in _listarg(xs,"elem","second argument"))))
    b["any"]=_C(lambda p:_C(lambda xs:_anylist(p,xs)))
    b["all"]=_C(lambda p:_C(lambda xs:_alllist(p,xs)))
    b["concatLists"]=_C(lambda xss:_concatlists(xss))
    b["concatMap"]=_C(lambda f:_C(lambda xs:[_force(it) for x in _force(xs) for it in _force(_apply(f,_tv(_force(x))))]))
    b["genList"]=_C(lambda f:_C(lambda n:_genlist(f,n)))
    b["groupBy"]=_C(lambda f:_C(lambda xs:_groupby(f,xs)))
    b["partition"]=_C(lambda p:_C(lambda xs:_partition(p,xs)))
    b["genericClosure"]=_C(lambda arg:_genericclosure(arg))
    b["attrValues"]=_C(lambda m:_values(m,"builtins.attrValues"))
    b["getAttrs"]=_C(lambda names:_C(lambda attrs:_getattrs(names,attrs)))
    b["mapAttrs"]=_C(lambda f:_C(lambda m:_mapattrs(f,m)))
    b["sort"]=_C(lambda p:_C(lambda xs:_sort(p,_sortlist(xs))))
    b["substring"]=_C(lambda start:_C(lambda length:_C(lambda s:_substr(start,length,s))))
    b["stringLength"]=_C(lambda s: _strlen(s))
    b["hasPrefix"]=_C(lambda p:_C(lambda s:_str(s,"builtins.hasPrefix string").startswith(_str(p,"builtins.hasPrefix prefix"))))
    b["hasSuffix"]=_C(lambda p:_C(lambda s:_str(s,"builtins.hasSuffix string").endswith(_str(p,"builtins.hasSuffix suffix"))))
    b["replaceStrings"]=_C(lambda fs:_C(lambda ts:_C(lambda s:_replace(fs,ts,s))))
    b["concatStringsSep"]=_C(lambda sep:_C(lambda xs:_concatstringssep(sep,xs)))
    b["concatStrings"]=_C(lambda xs:_concatstrings(xs))
    b["compareVersions"]=_C(lambda a:_C(lambda b2:_cmpver(a,b2)))
    b["splitVersion"]=_C(lambda s:_splitver(s))
    b["parseDrvName"]=_C(lambda s:_parsedrv(s))
    b["match"]=_C(lambda p:_C(lambda s:_rxmatch(p,s)))
    b["split"]=_C(lambda p:_C(lambda s:_rxsplit(p,s)))
    b["fromJSON"]=_C(lambda s:_fromjson(s))
    b["fromTOML"]=_C(lambda s:_fromtoml(s))
    b["schemaValidate"]=_C(lambda schema:_C(lambda value:_schemavalidate(schema,value)))
    b["schemaNormalize"]=_C(lambda schema:_C(lambda value:_schemanorm(schema,value)))
    b["schemaExplain"]=_C(lambda schema:_C(lambda value:_schemaexplain(schema,value)))
    b["xmlParse"]=_C(lambda s:_xmlparse(s))
    b["xmlEmit"]=_C(lambda value:_memit(value,False))
    b["htmlParse"]=_C(lambda s:_htmlparse(s))
    b["htmlEmit"]=_C(lambda value:_memit(value,True))
    b["lessThan"]=_C(lambda a:_C(lambda b2:_less(a,b2)))
    b["add"]=_C(lambda a:_C(lambda b2:_bin("+",a,b2)))
    b["sub"]=_C(lambda a:_C(lambda b2:_bin("-",a,b2)))
    b["mul"]=_C(lambda a:_C(lambda b2:_bin("*",a,b2)))
    b["div"]=_C(lambda a:_C(lambda b2:_bin("/",a,b2)))
    b["mod"]=_C(lambda a:_C(lambda b2:_modbuiltin(a,b2)))
    b["neg"]=_C(lambda a:_ci(-_num(a,"builtins.neg argument"),"-"))
    b["abs"]=_C(lambda a: abs(_num(a,"builtins.abs argument")))
    b["bitAnd"]=_C(lambda a:_C(lambda b2:_bit("bitAnd",a,b2,lambda x,y:x&y)))
    b["bitOr"]=_C(lambda a:_C(lambda b2:_bit("bitOr",a,b2,lambda x,y:x|y)))
    b["bitXor"]=_C(lambda a:_C(lambda b2:_bit("bitXor",a,b2,lambda x,y:x^y)))
    b["pow"]=_C(lambda a:_C(lambda b2:_pow(a,b2)))
    b["sqrt"]=_C(lambda a: math.sqrt(_num(a,"builtins.sqrt argument")))
    b["floor"]=_C(lambda a: _floati64(a,"builtins.floor",math.floor))
    b["ceil"]=_C(lambda a: _floati64(a,"builtins.ceil",math.ceil))
    b["exp"]=_C(lambda a: math.exp(_num(a,"builtins.exp argument")))
    b["ln"]=_C(lambda a: math.log(_num(a,"builtins.ln argument")))
    b["log"]=_C(lambda a: math.log(_num(a,"builtins.log argument")))
    b["sin"]=_C(lambda a: math.sin(_num(a,"builtins.sin argument")))
    b["cos"]=_C(lambda a: math.cos(_num(a,"builtins.cos argument")))
    b["tan"]=_C(lambda a: math.tan(_num(a,"builtins.tan argument")))
    b["atan2"]=_C(lambda y:_C(lambda x: math.atan2(_num(y,"builtins.atan2 y"),_num(x,"builtins.atan2 x"))))
    b["and"]=_C(lambda a:_C(lambda b2:_bool(a,"builtins.and lhs") and _bool(b2,"builtins.and rhs")))
    b["or"]=_C(lambda a:_C(lambda b2:_bool(a,"builtins.or lhs") or _bool(b2,"builtins.or rhs")))
    b["not"]=_C(lambda a: not _bool(a,"builtins.not argument"))
    b["eq"]=_C(lambda a:_C(lambda b2:_eq(a,b2)))
    b["lt"]=_C(lambda a:_C(lambda b2:_less(a,b2)))
    b["le"]=_C(lambda a:_C(lambda b2:not _less(b2,a)))
    b["gt"]=_C(lambda a:_C(lambda b2:_less(b2,a)))
    b["ge"]=_C(lambda a:_C(lambda b2:not _less(a,b2)))
    b["seq"]=_C(lambda a:_C(lambda b2:_seq(a,b2)))
    b["deepSeq"]=_C(lambda a:_C(lambda b2: (_deepforce(a),_force(b2))[1]))
    b["tryEval"]=_C(lambda x:_tryeval(x))
    b["derivationStrict"]=_C(lambda attrs:_derivation(attrs,"derivationStrict"))
    b["derivation"]=_C(lambda attrs:_derivation(attrs,"derivation"))
    b["addErrorContext"]=_C(lambda msg:_C(lambda value:_addctx(msg,value)))
    b["unsafeGetAttrPos"]=_C(lambda name:_C(lambda attrs:_attrpos(name,attrs)))
    b["unsafeDiscardStringContext"]=_C(lambda value:_str(value,"builtins.unsafeDiscardStringContext string"))
    b["hasContext"]=_C(lambda value:_hasctx(value))
    b["getContext"]=_C(lambda value:_getctx(value))
    b["appendContext"]=_C(lambda value:_C(lambda ctx:_appendcontext(value,ctx)))
    b["addDrvOutputDependencies"]=_C(lambda value:_ctxstr(value,"addDrvOutputDependencies"))
    b["unsafeDiscardOutputDependency"]=_C(lambda value:_ctxstr(value,"unsafeDiscardOutputDependency"))
    b["unsafeAddOutputDependency"]=_C(lambda value:_ctxstr(value,"unsafeAddOutputDependency"))
    b["unsafeAddOutputName"]=_C(lambda name:_C(lambda value:_addoutname(name,value)))
    b["trace"]=_C(lambda msg:_C(lambda value:_force(value)))
    b["throw"]=_C(lambda msg: (_ for _ in ()).throw(_Catch(_str(msg,"builtins.throw"))))
    b["abort"]=_C(lambda msg:_abortval(msg))
    b["typeOf"]=_C(lambda x:_typeof(x))
    b["isList"]=_C(lambda x: isinstance(_force(x),list))
    b["isAttrs"]=_C(lambda x: isinstance(_force(x),dict))
    b["isString"]=_C(lambda x: _isstr(_force(x)))
    b["isInt"]=_C(lambda x: type(_force(x)) is int)
    b["isFloat"]=_C(lambda x: type(_force(x)) is float)
    b["isFinite"]=_C(lambda x:_isfinite(x))
    b["isInf"]=_C(lambda x:_isinf(x))
    b["isNaN"]=_C(lambda x:_isnan(x))
    b["isBool"]=_C(lambda x: type(_force(x)) is bool)
    b["isFunction"]=_C(lambda x: type(_force(x)) is _C)
    b["isNull"]=_C(lambda x: _force(x) is None)
    b["isPath"]=_C(lambda x: type(_force(x)) is _P)
    b["true"]=True
    b["false"]=False
    b["null"]=None
    b["builtins"]=_T(lambda:b)
    return b
def _import(path):
    if "_PX_IMPORT" not in globals(): raise Exception("import requires run_px/run_px_source host file context")
    path=_force(path)
    if type(path) is _P or _isstr(path): return _PX_IMPORT(str(path))
    raise Exception("builtins.import: expected path or string")
def _scopedimport(scope,path):
    if "_PX_SCOPED_IMPORT" not in globals(): raise Exception("scopedImport requires run_px/run_px_source host file context")
    path=_force(path)
    if type(path) is _P or _isstr(path): return _PX_SCOPED_IMPORT(_realize(scope),str(path))
    raise Exception("builtins.scopedImport: expected path or string")
v_builtins=_tv(_bi())
'''


# The emitter, written in the stage7 Hy subset (recursion only -- stage7's `for`
# and `while` cannot capture enclosing-block vars). It turns a pnix AST (JSON) into
# Python source, then compile()+exec() inside stage7. `__PRELUDE__` and `__ASTS__`
# are filled in by hy_compiler_source_for_asts.
HY_AST_COMPILER_SOURCE = r'''
(do
  (import json)
  (import math)
  (setv PRELUDE __PRELUDE__)
__PARSER__
  (setv COUNTER {"n" 0})
  (defn fresh []
    (setv (get COUNTER "n") (+ (get COUNTER "n") 1))
    (get COUNTER "n"))
  (defn safe-char [c]
    (if (or (and (>= c "a") (<= c "z"))
            (and (>= c "A") (<= c "Z"))
            (and (>= c "0") (<= c "9"))
            (= c "_"))
        c "_"))
  (defn mung-rec [n i acc]
    (if (>= i (len n)) acc
        (mung-rec n (+ i 1) (+ acc (safe-char (get n i))))))
  (defn mung0 [n]
    (setv r (mung-rec n 0 ""))
    (if (= r "") "x" r))
  (defn env-with [env name val]
    (setv e (dict env))
    (setv (get e name) val)
    e)
  (defn join-sep [parts sep i acc]
    (if (>= i (len parts)) acc
        (join-sep parts sep (+ i 1) (+ acc (if (> i 0) sep "") (get parts i)))))
  (defn emit-items [items env i acc]
    (if (>= i (len items)) acc
        (emit-items items env (+ i 1)
          (+ acc (if (> i 0) "," "") "_T(lambda:" (emit (get items i) env) ")"))))
  (defn emit-interp-expr [ex env]
    (if (and (= (get ex "tag") "var") (not (in (get ex "name") env)))
        (repr (+ "${" (get ex "name") "}"))
        (+ "_coerce(" (emit ex env) ")")))
  (defn emit-interp-seg [p env]
    (if (in "lit" p)
        (repr (get p "lit"))
        (emit-interp-expr (get p "expr") env)))
  (defn emit-interp-parts [parts env i acc]
    (if (>= i (len parts))
        acc
        (emit-interp-parts parts env (+ i 1)
          (+ acc (if (> i 0) "," "") (emit-interp-seg (get parts i) env)))))
  (defn binding-path-static? [path]
    (if (= (len path) 0)
      True
      (and (isinstance (get path 0) str) (binding-path-static? (cut path 1 None)))))
  (defn emit-binding-path-items [path env i acc]
    (if (>= i (len path))
        (+ "[" acc "]")
        (do
          (setv part (get path i))
          (setv code (if (isinstance part str)
                       (repr part)
                       (+ "_attrkey(" (emit (get part "expr") env) ")")))
          (emit-binding-path-items path env (+ i 1)
            (+ acc (if (> i 0) "," "") code)))))
  (defn emit-binding-path [path env]
    (if (binding-path-static? path)
      (repr path)
      (emit-binding-path-items path env 0 "")))
  (defn binding-positions-code [binding]
    (if (binding-path-static? (get binding "path"))
      (repr (.get binding "path_positions" None))
      "None"))
  (defn let-build-env [binds env dn i]
    (if (>= i (len binds)) env
        (do
          (setv path (get (get binds i) "path"))
          (if (binding-path-static? path)
            (do
              (setv nm (get path 0))
              (let-build-env binds (env-with env nm (+ dn "[" (repr nm) "]")) dn (+ i 1)))
            (let-build-env binds env dn (+ i 1))))))
  (defn let-assigns [binds outer-env env2 dn i acc]
    (if (>= i (len binds)) acc
        (do
          (setv b (get binds i))
          (setv use-env (if (in "inherit_plain" b) outer-env env2))
          (let-assigns binds outer-env env2 dn (+ i 1)
            (+ acc [(+ "_setpath(" dn "," (emit-binding-path (get b "path") env2) ",_T(lambda:" (emit (get b "value") use-env) "),'let'," (binding-positions-code b) ")")])))))
  (defn attrset-rec-env [binds env dn i]
    (if (>= i (len binds)) env
        (do
          (setv path (get (get binds i) "path"))
          (if (binding-path-static? path)
            (do
              (setv top (get path 0))
              (attrset-rec-env binds (env-with env top (+ dn "[" (repr top) "]")) dn (+ i 1)))
            (attrset-rec-env binds env dn (+ i 1))))))
  (defn ordered-rec-bindings [binds]
    (do
      (setv out [])
      (for [binding binds]
        (if (binding-path-static? (get binding "path"))
          (.append out binding)
          None))
      (for [binding binds]
        (if (not (binding-path-static? (get binding "path")))
          (.append out binding)
          None))
      out))
  (defn attrset-steps [binds env outer-env dn i acc]
    (if (>= i (len binds)) acc
        (do
          (setv b (get binds i))
          (setv use-env (if (in "inherit_plain" b) outer-env env))
          (attrset-steps binds env outer-env dn (+ i 1)
            (+ acc [(+ "_setpath("
                       dn
                       ","
                       (emit-binding-path (get b "path") use-env)
                       ",_T(lambda:"
                       (emit (get b "value") use-env)
                       "),'attr',"
                       (binding-positions-code b)
                       ")")])))))
	  (defn emit-binary [nd env]
	    (setv op (get nd "op"))
	    (cond
	      (= op "&&") (+ "(_bool(" (emit (get nd "lhs") env) ",'left operand of &&') and _bool(" (emit (get nd "rhs") env) ",'right operand of &&'))")
	      (= op "||") (+ "(_bool(" (emit (get nd "lhs") env) ",'left operand of ||') or _bool(" (emit (get nd "rhs") env) ",'right operand of ||'))")
	      (= op "->") (+ "(True if not _bool(" (emit (get nd "lhs") env) ",'left operand of ->') else _bool(" (emit (get nd "rhs") env) ",'right operand of ->'))")
	      True (+ "_bin(" (repr op) "," (emit (get nd "lhs") env) "," (emit (get nd "rhs") env) ")")))
	  (defn emit-construct-args [args env i acc]
	    (if (>= i (len args)) acc
	        (emit-construct-args args env (+ i 1)
	          (+ acc (if (> i 0) "," "") "_T(lambda:" (emit (get args i) env) ")"))))
	  (defn add-bound [names name]
	    (if (in name names) names (+ names [name])))
	  (defn pattern-bound-fields [fields i names]
	    (if (>= i (len fields)) names
	        (pattern-bound-fields fields (+ i 1)
	          (pattern-bound (get (get fields i) "pattern") names))))
	  (defn pattern-bound-items [items i names]
	    (if (>= i (len items)) names
	        (pattern-bound-items items (+ i 1)
	          (pattern-bound (get items i) names))))
	  (defn pattern-bound [pat names]
	    (setv tag (get pat "tag"))
	    (cond
	      (= tag "as") (add-bound (pattern-bound (get pat "pattern") names) (get pat "name"))
	      (= tag "var") (add-bound names (get pat "name"))
	      (= tag "list")
	        (if (= (.get pat "rest" None) None)
	          (pattern-bound-items (get pat "items") 0 names)
	          (add-bound (pattern-bound-items (get pat "items") 0 names) (get pat "rest")))
	      (= tag "attrset") (pattern-bound-fields (get pat "fields") 0 names)
	      (= tag "constructor") (pattern-bound-items (get pat "args") 0 names)
	      True names))
	  (defn pattern-default-env [pat env]
	    (env-with-pattern-names env (pattern-bound pat []) 0))
	  (defn pattern-code-field-default [field env]
	    (if (in "default" field)
	      (+ ",'default':lambda _m:_T(lambda:" (emit (get field "default") env) ")")
	      ""))
	  (defn pattern-code-fields [fields i acc env default-env]
	    (if (>= i (len fields)) acc
	        (pattern-code-fields fields (+ i 1)
	          (+ acc (if (> i 0) "," "")
	             "{'name':" (repr (get (get fields i) "name"))
	             ",'pattern':" (pattern-code (get (get fields i) "pattern") env)
	             (pattern-code-field-default (get fields i) default-env)
	             "}")
	          env
	          default-env)))
	  (defn pattern-code-items [items i acc env]
	    (if (>= i (len items)) acc
	        (pattern-code-items items (+ i 1)
	          (+ acc (if (> i 0) "," "") (pattern-code (get items i) env))
	          env)))
	  (defn pattern-code [pat env]
	    (setv tag (get pat "tag"))
	    (cond
	      (= tag "wildcard") "{'tag':'wildcard'}"
	      (= tag "as") (+ "{'tag':'as','name':" (repr (get pat "name")) ",'pattern':" (pattern-code (get pat "pattern") env) "}")
	      (= tag "var") (+ "{'tag':'var','name':" (repr (get pat "name")) "}")
	      (= tag "literal") (+ "{'tag':'literal','value':" (repr (get pat "value")) "}")
	      (= tag "list") (+ "{'tag':'list','items':[" (pattern-code-items (get pat "items") 0 "" env) "]"
	                        (if (= (.get pat "rest" None) None) "" (+ ",'rest':" (repr (get pat "rest"))))
	                        "}")
	      (= tag "attrset") (+ "{'tag':'attrset','fields':[" (pattern-code-fields (get pat "fields") 0 "" env (pattern-default-env pat env)) "],'ellipsis':" (if (get pat "ellipsis") "True" "False") "}")
	      (= tag "constructor") (+ "{'tag':'constructor','variant':" (repr (get pat "variant")) ",'args':[" (pattern-code-items (get pat "args") 0 "" env) "]}")
	      True (pnix-error "unsupported compiler pattern tag")))
	  (defn emit-attr-parts [segments env i acc]
	    (if (>= i (len segments))
	        (+ "[" acc "]")
	        (do
	          (setv segment (get segments i))
	          (setv part (if (in "lit" segment)
	                       (repr (str (get segment "lit")))
	                       (+ "_T(lambda:" (emit (get segment "expr") env) ")")))
	          (emit-attr-parts segments env (+ i 1)
	            (+ acc (if (> i 0) "," "") part)))))
	  (defn emit-float-value [value]
	    (cond
	      (math.isnan value) "float('nan')"
	      (math.isinf value) (if (> value 0) "float('inf')" "float('-inf')")
	      True (repr value)))
	  (defn env-with-pattern-names [env names i]
	    (if (>= i (len names)) env
	        (env-with-pattern-names
	          (env-with env (get names i) (+ "_m[" (repr (get names i)) "]"))
	          names
	          (+ i 1))))
	  (defn emit-match-arms [arms env i acc]
	    (if (>= i (len arms)) (+ "[" acc "]")
	        (do
	          (setv arm (get arms i))
	          (setv names (pattern-bound (get arm "pattern") []))
	          (setv env2 (env-with-pattern-names env names 0))
	          (setv guard (if (in "guard" arm)
	                        (+ "lambda _m:" (emit (get arm "guard") env2))
	                        "None"))
	          (emit-match-arms arms env (+ i 1)
	            (+ acc (if (> i 0) "," "")
	               "(" (pattern-code (get arm "pattern") env) "," guard ",lambda _m:"
	               (emit (get arm "body") env2) ")")))))
  (setv FOLDING {"enabled" True})
  (defn source-sensitive-list? [items]
    (if (= (len items) 0)
      False
      (or (source-sensitive? (get items 0))
          (source-sensitive-list? (cut items 1 None)))))
  (defn source-sensitive? [value]
    (cond
      (isinstance value dict)
        (cond
          (and (= (.get value "tag" None) "var")
               (in (.get value "name" "") ["__curPos" "unsafeGetAttrPos"])) True
          (and (= (.get value "tag" None) "select")
               (= (.get value "attr" None) "unsafeGetAttrPos")) True
          True (source-sensitive-list? (list (.values value))))
      (isinstance value list) (source-sensitive-list? value)
      True False))
  (defn no-fold-tag? [tag]
    (in tag ["int" "float" "path" "string" "bool" "null" "var" "lambda"
             "import" "with" "construct" "match" "path_interp"
             "dynamic_select" "dynamic_select_default" "dynamic_has_attr"]))
  (defn scalar-code [value]
    (cond
      (= value True) "True"
      (= value False) "False"
      (= value None) "None"
      (= (type value) int) (repr value)
      (= (type value) float) (emit-float-value value)
      (= (type value) str)
        (if (.startswith value "#<pnix-hy-") None (repr value))
      True None))
  (defn try-fold [nd env]
    (if (or (not (get FOLDING "enabled"))
            (source-sensitive? nd)
            (no-fold-tag? (get nd "tag")))
      None
      (do
        (setv oldn (get COUNTER "n"))
        (setv (get FOLDING "enabled") False)
        (try
          (do
            (setv expr (emit-raw nd env))
            (setv (get FOLDING "enabled") True)
            (setv ns (dict))
            (exec PRELUDE ns)
            (setv code (scalar-code (eval expr ns)))
            (setv (get COUNTER "n") oldn)
            code)
          (except [Exception exc]
            (do
              (setv (get FOLDING "enabled") True)
              (setv (get COUNTER "n") oldn)
              None))))))
	  (defn emit-raw [nd env]
    (setv tag (get nd "tag"))
    (cond
	      (= tag "int") (str (get nd "value"))
	      (= tag "float") (emit-float-value (get nd "value"))
	      (= tag "path") (+ "_P(_pathlit(" (repr (get nd "value")) "))")
	      (= tag "path_interp") (+ "_P(_pathlit(_concatstrings([" (emit-interp-parts (get nd "parts") env 0 "") "])))")
	      (= tag "string") (repr (get nd "value"))
	      (= tag "bool") (if (get nd "value") "True" "False")
	      (= tag "null") "None"
	      (= tag "var")
	        (if (= (get nd "name") "__curPos")
	            (+ "_srcpos(" (str (.get nd "pos" 0)) ")")
	        (if (in (get nd "name") env)
	            (+ "_force(" (get env (get nd "name")) ")")
		            (if (in "__pnix_hy_with_chain__" env)
		                (+ "_with_lookup(" (get env "__pnix_hy_with_chain__") "," (repr (get nd "name")) ")")
		                (+ "_unknownvar(" (repr (get nd "name")) ")"))))
		      (= tag "construct")
		        (+ "_K(" (repr (get nd "variant")) ",[" (emit-construct-args (get nd "args") env 0 "") "])")
		      (= tag "list") (+ "[" (emit-items (get nd "items") env 0 "") "]")
	      (= tag "if")
	        (+ "(" (emit (get nd "then") env) " if _bool(" (emit (get nd "cond") env) ",'if condition') else " (emit (get nd "else") env) ")")
	      (= tag "with")
	        (do
	          (setv wn (+ "_w" (str (fresh))))
	          (setv env2 (env-with env "__pnix_hy_with_chain__" wn))
	          (+ "((" wn ":=_with(_T(lambda:" (emit (get nd "env") env) "),"
	             (if (in "__pnix_hy_with_chain__" env) (get env "__pnix_hy_with_chain__") "None")
	             "))," (emit (get nd "body") env2) ")[-1]"))
	      (= tag "assert")
	        (+ "(" (emit (get nd "body") env)
	           " if _bool(" (emit (get nd "cond") env) ",'assert condition') else _assert_fail())")
	      (= tag "unary")
	        (if (= (get nd "op") "-")
	            (+ "_uneg(" (emit (get nd "arg") env) ")")
	            (+ "(not _bool(" (emit (get nd "arg") env) ",'argument of !'))"))
      (= tag "binary") (emit-binary nd env)
      (= tag "lambda")
        (if (= (.get nd "pattern" None) None)
          (do
            (setv pn (+ "v_" (mung0 (get nd "param")) "_" (str (fresh))))
            (+ "_C(lambda " pn ":" (emit (get nd "body") (env-with env (get nd "param") pn)) ")"))
          (do
            (setv pn (+ "v_arg_" (str (fresh))))
            (setv names (pattern-bound (get nd "pattern") []))
            (setv env2 (env-with-pattern-names env names 0))
            (+ "_C(lambda " pn ":((_m:=_bindpat("
               (pattern-code (get nd "pattern") env)
               "," pn "))," (emit (get nd "body") env2) ")[-1],"
               (pattern-code (get nd "pattern") env) ")")))
	      (= tag "apply")
	        (+ "_apply(" (emit (get nd "func") env) ",_T(lambda:" (emit (get nd "arg") env) "))")
	      (= tag "select")
	        (+ "_sel(_T(lambda:" (emit (get nd "base") env) ")," (repr (get nd "attr")) ")")
		      (= tag "select_default")
		        (+ "_seldef(_T(lambda:" (emit (get nd "base") env) "),"
		           (repr (get nd "attr")) ",_T(lambda:" (emit (get nd "default") env) "))")
		      (= tag "dynamic_select")
		        (+ "_dynsel(_T(lambda:" (emit (get nd "base") env) "),"
		           (emit-attr-parts (get nd "segments") env 0 "") ")")
		      (= tag "dynamic_select_default")
		        (+ "_dyndef(_T(lambda:" (emit (get nd "base") env) "),"
		           (emit-attr-parts (get nd "segments") env 0 "")
		           ",_T(lambda:" (emit (get nd "default") env) "))")
		      (= tag "has_attr")
		        (+ "_has(_force(" (emit (get nd "base") env) ")," (repr (get nd "path")) ")")
		      (= tag "dynamic_has_attr")
		        (+ "_dynhas(_T(lambda:" (emit (get nd "base") env) "),"
		           (emit-attr-parts (get nd "segments") env 0 "") ")")
		      (= tag "index")
		        (+ "_index(" (emit (get nd "base") env) "," (emit (get nd "index") env) ")")
		      (= tag "match")
		        (+ "_match(_T(lambda:" (emit (get nd "scrutinee") env) "),"
		           (emit-match-arms (get nd "arms") env 0 "") ")")
      (= tag "str_interp")
        (+ "_concatstrings([" (emit-interp-parts (get nd "parts") env 0 "") "])")
      (= tag "let")
        (do
          (setv binds (get nd "bindings"))
          (setv dn (+ "_d" (str (fresh))))
          (setv env2 (let-build-env binds env dn 0))
          (setv steps (+ [(+ "(" dn ":={})")] (let-assigns binds env env2 dn 0 [])))
          (+ "((" (join-sep steps "),(" 0 "") ")," (emit (get nd "body") env2) ")[-1]"))
      (= tag "attrset")
        (do
          (setv binds (get nd "bindings"))
          (setv dn (+ "_d" (str (fresh))))
          (setv env2 (if (get nd "recursive") (attrset-rec-env binds env dn 0) env))
          (setv ordered (if (get nd "recursive") (ordered-rec-bindings binds) binds))
          (setv steps (attrset-steps ordered env2 env dn 0 [(+ "(" dn ":=_A())")]))
          (+ "((" (join-sep steps "),(" 0 "") ")," dn ")[-1]"))
      (= tag "import")
        (+ "_import(" (repr (get nd "path")) ")")
      True (+ "_unhandled_" tag)))
  (defn emit [nd env]
    (do
      (setv folded (try-fold nd env))
      (if (= folded None) (emit-raw nd env) folded)))
  (setv COMPILER-BUILTIN-ALIASES
    ["currentSystem" "nixVersion" "langVersion" "storeDir"
     "import" "scopedImport"
     "pathExists" "readFile" "readFileType" "readDir" "toFile" "hashString" "hashFile"
     "baseNameOf" "dirOf" "toPath" "storePath" "getEnv" "placeholder" "break" "warn" "traceVerbose"
     "attrNames" "hasAttr" "getAttr" "attrByPath" "removeAttrs" "listToAttrs"
     "filterAttrs" "functionArgs" "intersectAttrs" "zipAttrsWith" "catAttrs"
     "elemAt" "length" "head" "tail" "toString" "toJSON"
     "map" "filter" "foldl'" "fold" "foldl" "foldr" "cons" "append" "take" "drop" "reverse" "reverseList"
     "zip" "flatten" "find" "get" "mapGet" "set" "mapSet" "keys" "mapKeys" "values" "mapValues" "merge" "mapMerge"
     "elem" "any" "all" "concatLists" "concatMap"
     "genList" "groupBy" "partition" "genericClosure" "attrValues" "getAttrs" "mapAttrs" "sort"
     "substring" "stringLength" "hasPrefix" "hasSuffix" "replaceStrings"
     "concatStringsSep" "concatStrings" "compareVersions" "splitVersion" "parseDrvName"
     "match" "split" "fromJSON" "fromTOML" "schemaValidate" "schemaNormalize" "schemaExplain"
     "xmlParse" "xmlEmit" "htmlParse" "htmlEmit"
     "lessThan" "add" "sub" "mul" "div" "mod" "neg" "abs" "bitAnd" "bitOr" "bitXor" "pow" "sqrt" "floor" "ceil"
     "exp" "ln" "log" "sin" "cos" "tan" "atan2" "and" "or" "not" "eq" "lt" "le" "gt" "ge"
     "seq" "deepSeq" "tryEval" "derivationStrict" "derivation" "addErrorContext"
     "unsafeGetAttrPos" "unsafeDiscardStringContext" "hasContext" "getContext" "appendContext"
     "addDrvOutputDependencies" "unsafeDiscardOutputDependency" "unsafeAddOutputDependency" "unsafeAddOutputName"
     "trace" "throw" "abort"
     "typeOf" "isList" "isAttrs" "isString" "isInt" "isFloat" "isFinite" "isInf" "isNaN" "isBool"
     "isFunction" "isNull" "isPath"])

  (defn compiler-env-aliases [env names]
    (if (= (len names) 0)
      env
      (do
        (setv (get env (get names 0)) (+ "_force(v_builtins)[" (repr (get names 0)) "]"))
        (compiler-env-aliases env (cut names 1 None)))))

  (setv ENV0 (compiler-env-aliases {"builtins" "v_builtins"} COMPILER-BUILTIN-ALIASES))
  (defn compile-one [node source]
    (setv (get COUNTER "n") 0)
    (+ PRELUDE "\n_PX_SOURCE_PATH='<pnix-stage7>'\n_PX_SOURCE_TEXT="
       (repr source)
       "\n_RESULT=_realize(" (emit node ENV0) ")\n"))
  (defn run-asts [asts i acc]
    (if (>= i (len asts)) acc
        (do
          (setv source (if (> (len RAW-SOURCES) i) (get RAW-SOURCES i) ""))
          (setv code (compile-one (get asts i) source))
          (setv ns (dict))
          (exec (compile code "<pnix-compiled>" "exec") ns)
          (run-asts asts (+ i 1) (+ acc [(get ns "_RESULT")])))))
  (defn emit-asts [asts i acc]
    (if (>= i (len asts)) acc
        (do
          (setv (get COUNTER "n") 0)
          (emit-asts asts (+ i 1) (+ acc [(emit (get asts i) ENV0)])))))
  (setv RAW-SOURCES (json.loads __SOURCES__))
  (setv asts (if (> (len RAW-SOURCES) 0) (parse-source-list RAW-SOURCES) (json.loads __ASTS__)))
  (json.dumps (__DRIVERFN__ asts 0 [])))
'''


# Reuse the evaluator's Hy parser (pnix-error .. parse-source-list) verbatim so
# the compiler can take pnix SOURCE directly: tokenize + parse + compile + exec
# ALL inside stage7, leaving Python as a pure verification oracle. Sliced out of
# HY_AST_EVALUATOR_SOURCE so there is one parser definition, not two copies.
_PARSER_BEGIN = "  (defn pnix-error [message]"
_PARSER_END = "(parse-source-list (cut sources 1 None)))))"
PNIX_PARSER_DEFS = HY_AST_EVALUATOR_SOURCE[
    HY_AST_EVALUATOR_SOURCE.index(_PARSER_BEGIN) : HY_AST_EVALUATOR_SOURCE.index(_PARSER_END)
    + len(_PARSER_END)
]

_EMPTY_JSON = json.dumps(json.dumps([]))


def hy_compiler_source_for_asts(asts: list[dict[str, Any]]) -> str:
    """ast-lane: Python parses; stage7 compiles + execs. No parser injected."""
    asts_json = json.dumps(stable_data(asts), ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return (
        HY_AST_COMPILER_SOURCE.replace("__PRELUDE__", json.dumps(COMPILER_PRELUDE))
        .replace("__PARSER__", "")
        .replace("__DRIVERFN__", "run-asts")
        .replace("__ASTS__", json.dumps(asts_json))
        .replace("__SOURCES__", _EMPTY_JSON)
    )


def hy_compiler_source_for_ast(ast: dict[str, Any]) -> str:
    return hy_compiler_source_for_asts([ast])


def hy_compiler_source_for_sources(sources: list[str]) -> str:
    """source-lane: stage7 parses (reused Hy parser) + compiles + execs. Python
    never parses -- it only supplies the source strings and checks the result."""
    sources_json = json.dumps(sources, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return (
        HY_AST_COMPILER_SOURCE.replace("__PRELUDE__", json.dumps(COMPILER_PRELUDE))
        .replace("__PARSER__", PNIX_PARSER_DEFS)
        .replace("__DRIVERFN__", "run-asts")
        .replace("__ASTS__", _EMPTY_JSON)
        .replace("__SOURCES__", json.dumps(sources_json))
    )


def hy_compiler_emit_for_asts(asts: list[dict[str, Any]]) -> str:
    """Emit-only lane: returns the generated Python source for each AST (no
    exec), for self-host fixed-point checks. The emit is deterministic (the
    per-AST name counter is reset before each), so the same pnix AST yields the
    same Python source across runs and across the stage tower -- the analogue of
    hy-meta's compiler_ast_stage7_mirror."""
    asts_json = json.dumps(stable_data(asts), ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return (
        HY_AST_COMPILER_SOURCE.replace("__PRELUDE__", json.dumps(COMPILER_PRELUDE))
        .replace("__PARSER__", "")
        .replace("__DRIVERFN__", "emit-asts")
        .replace("__ASTS__", json.dumps(asts_json))
        .replace("__SOURCES__", _EMPTY_JSON)
    )


# ---------------------------------------------------------------------------
# Entry point A: host-direct pnix compiler ("the real one", production speed).
#
# pnix source -> Python source -> compile -> exec, all on host CPython (NO
# stage7 bootstrap). The emit algorithm here is identical to the one in
# HY_AST_COMPILER_SOURCE (the stage7 self-host mirror): this host form is the
# fast path, the Hy form is the verified mirror, and hy_compiler_batch binds
# them with parity. A `.px` file is just pnix source.
# ---------------------------------------------------------------------------

_PX_CTR = [0]
_PX_EMIT_CTX: list[dict[str, Any]] = [{}]
_PX_FOLD = [True]  # partial-evaluation (constant folding) switch. ON for run
# (entry A, fast). Turn OFF for introspection (entry B) so the FULL emitted code
# is available to inspect -- so meta-circular introspection covers every node.


def _px_fresh() -> int:
    _PX_CTR[0] += 1
    return _PX_CTR[0]


def _px_mung(name: str) -> str:
    s = "".join(c if (c.isalnum() or c == "_") else "_" for c in name)
    return s or "x"


_PX_FOLD_BOUND = frozenset({"builtins"})


def _px_float_literal(value: float) -> str:
    if math.isnan(value):
        return "float('nan')"
    if math.isinf(value):
        return "float('inf')" if value > 0 else "float('-inf')"
    return repr(value)


def _px_initial_env(extra_env: dict[str, str] | None = None) -> dict[str, str]:
    env = {"builtins": "v_builtins"}
    for name in BUILTIN_ALIAS_NAMES:
        env[name] = "_force(v_builtins)[%r]" % name
    env.update(extra_env or {})
    return env


def _px_freevars(nd: dict[str, Any], bound: frozenset) -> set:
    """Free variable names in `nd` not bound by `bound`."""
    tag = nd["tag"]
    if tag == "var":
        return set() if nd["name"] in bound else {nd["name"]}
    if tag in ("int", "float", "path", "string", "bool", "null", "import"):
        return set()
    if tag == "path_interp":
        out = set()
        for part in nd["parts"]:
            if "expr" in part:
                out |= _px_freevars(part["expr"], bound)
        return out
    if tag == "construct":
        out = set()
        for arg in nd["args"]:
            out |= _px_freevars(arg, bound)
        return out
    if tag == "list":
        out: set = set()
        for item in nd["items"]:
            out |= _px_freevars(item, bound)
        return out
    if tag == "if":
        return _px_freevars(nd["cond"], bound) | _px_freevars(nd["then"], bound) | _px_freevars(nd["else"], bound)
    if tag == "unary":
        return _px_freevars(nd["arg"], bound)
    if tag == "binary":
        return _px_freevars(nd["lhs"], bound) | _px_freevars(nd["rhs"], bound)
    if tag == "apply":
        return _px_freevars(nd["func"], bound) | _px_freevars(nd["arg"], bound)
    if tag in ("select", "has_attr"):
        return _px_freevars(nd["base"], bound)
    if tag == "select_default":
        return _px_freevars(nd["base"], bound) | _px_freevars(nd["default"], bound)
    if tag in ("dynamic_select", "dynamic_has_attr"):
        out = _px_freevars(nd["base"], bound)
        for segment in nd["segments"]:
            if "expr" in segment:
                out |= _px_freevars(segment["expr"], bound)
        return out
    if tag == "dynamic_select_default":
        out = _px_freevars(nd["base"], bound) | _px_freevars(nd["default"], bound)
        for segment in nd["segments"]:
            if "expr" in segment:
                out |= _px_freevars(segment["expr"], bound)
        return out
    if tag == "index":
        return _px_freevars(nd["base"], bound) | _px_freevars(nd["index"], bound)
    if tag == "with":
        return _px_freevars(nd["env"], bound) | _px_freevars(nd["body"], bound)
    if tag == "assert":
        return _px_freevars(nd["cond"], bound) | _px_freevars(nd["body"], bound)
    if tag == "lambda":
        if nd.get("pattern") is not None:
            return _px_pattern_freevars(nd["pattern"], bound) | _px_freevars(
                nd["body"], bound | _px_pattern_bound(nd["pattern"])
            )
        return _px_freevars(nd["body"], bound | {nd["param"]})
    if tag == "str_interp":
        out = set()
        for part in nd["parts"]:
            if "expr" in part:
                out |= _px_freevars(part["expr"], bound)
        return out
    if tag == "match":
        out = _px_freevars(nd["scrutinee"], bound)
        for arm in nd["arms"]:
            arm_bound = bound | _px_pattern_bound(arm["pattern"])
            if "guard" in arm:
                out |= _px_freevars(arm["guard"], arm_bound)
            out |= _px_freevars(arm["body"], arm_bound)
        return out
    if tag in ("let", "attrset"):
        names = {b["path"][0] for b in nd["bindings"] if _px_path_is_static(b["path"])}
        inner = bound | names if (tag == "let" or nd.get("recursive")) else bound
        out = set()
        for binding in nd["bindings"]:
            value_bound = bound if binding.get("inherit_plain") else inner
            for part in binding["path"]:
                if not isinstance(part, str):
                    out |= _px_freevars(part["expr"], value_bound)
            out |= _px_freevars(binding["value"], value_bound)
        if tag == "let":
            out |= _px_freevars(nd["body"], inner)
        return out
    return set()


def _px_try_fold(nd: dict[str, Any]) -> "str | None":
    """Partial evaluation: if `nd` is closed (free vars subset of {builtins}) and
    evaluates to a scalar, fold it to a Python literal at compile time. pnix is
    pure, so this is always sound; the interpreter is the oracle. Complex results
    (lists/attrsets/closures) and open terms fall through to normal emit."""
    if not _PX_FOLD[0]:
        return None
    if _px_source_sensitive(nd):
        return None
    tag = nd["tag"]
    if tag in (
        "int",
        "float",
        "path",
        "string",
        "bool",
        "null",
        "var",
        "lambda",
        "import",
        "with",
        "construct",
        "match",
        "path_interp",
        "dynamic_select",
        "dynamic_select_default",
        "dynamic_has_attr",
    ):
        return None
    if not (_px_freevars(nd, frozenset()) <= _PX_FOLD_BOUND):
        return None
    try:
        value = eval_source_raw(emit_source(nd), runtime_context({}), realize=False)
    except Exception:
        return None
    if value is True:
        return "True"
    if value is False:
        return "False"
    if value is None:
        return "None"
    if type(value) is int:
        return repr(value)
    if type(value) is float:
        return _px_float_literal(value)
    if isinstance(value, PnixString) and value.context:
        return None
    if is_string_value(value):
        if value.startswith("#<pnix-hy-"):
            return None  # realized native/closure marker, not a real scalar
        return repr(str(value))
    return None


def _px_source_sensitive(value: Any) -> bool:
    if isinstance(value, dict):
        if value.get("tag") == "var" and value.get("name") in {"__curPos", "unsafeGetAttrPos"}:
            return True
        if value.get("tag") == "select" and value.get("attr") == "unsafeGetAttrPos":
            return True
        return any(_px_source_sensitive(item) for item in value.values())
    if isinstance(value, list):
        return any(_px_source_sensitive(item) for item in value)
    return False


def _px_t(nd: dict[str, Any], env: dict[str, str]) -> str:
    """Lazy position: wrap the value in a thunk. Matches the interpreter's
    `Thunk(lambda: eval_ast(node))` exactly (a `var` becomes
    `_T(lambda:_force(cell))`, same as the interpreter)."""
    return "_T(lambda:%s)" % _px_emit(nd, env)


def _px_path_is_static(path: list[Any]) -> bool:
    return all(isinstance(part, str) for part in path)


def _px_binding_path(path: list[Any], env: dict[str, str]) -> str:
    if _px_path_is_static(path):
        return repr(path)
    parts: list[str] = []
    for part in path:
        if isinstance(part, str):
            parts.append(repr(part))
        else:
            parts.append("_attrkey(%s)" % _px_emit(part["expr"], env))
    return "[" + ",".join(parts) + "]"


def _px_binding_positions(binding: dict[str, Any]) -> Any:
    return binding.get("path_positions") if _px_path_is_static(binding["path"]) else None


def _px_pattern_bound(pattern: dict[str, Any]) -> frozenset[str]:
    tag = pattern["tag"]
    if tag == "as":
        return frozenset({pattern["name"]}) | _px_pattern_bound(pattern["pattern"])
    if tag == "var":
        return frozenset({pattern["name"]})
    if tag == "list":
        out: set[str] = set()
        for item in pattern["items"]:
            out |= set(_px_pattern_bound(item))
        if pattern.get("rest") is not None:
            out.add(pattern["rest"])
        return frozenset(out)
    if tag == "attrset":
        out: set[str] = set()
        for field in pattern["fields"]:
            out |= set(_px_pattern_bound(field["pattern"]))
        return frozenset(out)
    if tag == "constructor":
        out: set[str] = set()
        for arg in pattern["args"]:
            out |= set(_px_pattern_bound(arg))
        return frozenset(out)
    return frozenset()


def _px_pattern_freevars(pattern: dict[str, Any], bound: frozenset[str]) -> set[str]:
    tag = pattern["tag"]
    if tag == "as":
        return _px_pattern_freevars(pattern["pattern"], bound)
    if tag == "list":
        out: set[str] = set()
        for item in pattern["items"]:
            out |= _px_pattern_freevars(item, bound)
        return out
    if tag == "attrset":
        out: set[str] = set()
        for field in pattern["fields"]:
            if "default" in field:
                out |= _px_freevars(field["default"], bound)
            out |= _px_pattern_freevars(field["pattern"], bound)
        return out
    if tag == "constructor":
        out: set[str] = set()
        for arg in pattern["args"]:
            out |= _px_pattern_freevars(arg, bound)
        return out
    return set()


def _px_pattern(pattern: dict[str, Any], env: dict[str, str] | None = None) -> str:
    tag = pattern["tag"]
    if tag == "wildcard":
        return "{'tag':'wildcard'}"
    if tag == "as":
        return "{'tag':'as','name':%r,'pattern':%s}" % (
            pattern["name"],
            _px_pattern(pattern["pattern"], env),
        )
    if tag == "var":
        return "{'tag':'var','name':%r}" % pattern["name"]
    if tag == "literal":
        return "{'tag':'literal','value':%r}" % pattern["value"]
    if tag == "list":
        rest = pattern.get("rest")
        rest_part = "" if rest is None else ",'rest':%r" % rest
        return "{'tag':'list','items':[%s]%s}" % (
            ",".join(_px_pattern(item, env) for item in pattern["items"]),
            rest_part,
        )
    if tag == "attrset":
        fields = []
        default_env = dict(env or {})
        for name in _px_pattern_bound(pattern):
            default_env[name] = "_m[%r]" % name
        for field in pattern["fields"]:
            items = ["'name':%r" % field["name"], "'pattern':%s" % _px_pattern(field["pattern"], env)]
            if "default" in field:
                items.append("'default':lambda _m:_T(lambda:%s)" % _px_emit(field["default"], default_env))
            fields.append("{" + ",".join(items) + "}")
        return "{'tag':'attrset','fields':[%s],'ellipsis':%s}" % (
            ",".join(fields),
            "True" if pattern.get("ellipsis") else "False",
        )
    if tag == "constructor":
        return "{'tag':'constructor','variant':%r,'args':[%s]}" % (
            pattern["variant"],
            ",".join(_px_pattern(arg, env) for arg in pattern["args"]),
        )
    pnix_error(f"unsupported compiler pattern tag {tag!r}")
    raise AssertionError("unreachable")


def _px_attr_parts(segments: list[dict[str, Any]], env: dict[str, str]) -> str:
    parts = []
    for segment in segments:
        if "lit" in segment:
            parts.append(repr(str(segment["lit"])))
        else:
            parts.append(_px_t(segment["expr"], env))
    return "[" + ",".join(parts) + "]"


def _px_match_arms(arms: list[dict[str, Any]], env: dict[str, str]) -> str:
    emitted = []
    for arm in arms:
        env2 = dict(env)
        for name in _px_pattern_bound(arm["pattern"]):
            env2[name] = "_m[%r]" % name
        guard = "None" if "guard" not in arm else "lambda _m:%s" % _px_emit(arm["guard"], env2)
        emitted.append(
            "(%s,%s,lambda _m:%s)" % (
                _px_pattern(arm["pattern"], env),
                guard,
                _px_emit(arm["body"], env2),
            )
        )
    return "[" + ",".join(emitted) + "]"


def _px_emit(nd: dict[str, Any], env: dict[str, str]) -> str:
    """Strict position: a Python value expression with NO thunk wrapping. Thunks
    appear ONLY at genuinely lazy positions -- function args, and list/attrset/
    let cells -- via `_px_t`. Strict operands (arithmetic, `if` condition, unary,
    `select`/`?` base, function position) are emitted forced inline, so the hot
    path approaches native Python. Closed scalar subterms are constant-folded by
    `_px_try_fold` (partial evaluation). Semantics are identical to the
    interpreter; corpus parity guards this."""
    folded = _px_try_fold(nd)
    if folded is not None:
        return folded
    tag = nd["tag"]
    if tag == "int":
        return repr(nd["value"])
    if tag == "float":
        return _px_float_literal(nd["value"])
    if tag == "path":
        return "_P(%r)" % str(resolve_path_literal(nd["value"], _PX_EMIT_CTX[0]))
    if tag == "path_interp":
        segs = []
        for part in nd["parts"]:
            if "lit" in part:
                segs.append("%r" % part["lit"])
            else:
                segs.append("_coerce(%s)" % _px_emit(part["expr"], env))
        return "_P(_pathlit(_concatstrings([%s])))" % ",".join(segs)
    if tag == "string":
        return repr(nd["value"])
    if tag == "bool":
        return "True" if nd["value"] else "False"
    if tag == "null":
        return "None"
    if tag == "var":
        name = nd["name"]
        if name == "__curPos":
            return "_srcpos(%d)" % int(nd.get("pos", 0))
        if name in env:
            return "_force(%s)" % env[name]
        if WITH_CHAIN_KEY in env:
            return "_with_lookup(%s,%r)" % (env[WITH_CHAIN_KEY], name)
        return "_unknownvar(%r)" % name
    if tag == "import":
        return "_import(%r)" % nd["path"]
    if tag == "construct":
        return "_K(%r,[%s])" % (nd["variant"], ",".join(_px_t(arg, env) for arg in nd["args"]))
    if tag == "list":
        return "[" + ",".join(_px_t(i, env) for i in nd["items"]) + "]"
    if tag == "if":
        return "(%s if %s else %s)" % (
            _px_emit(nd["then"], env),
            "_bool(%s,'if condition')" % _px_emit(nd["cond"], env),
            _px_emit(nd["else"], env),
        )
    if tag == "with":
        wn = "_w%d" % _px_fresh()
        env2 = dict(env)
        env2[WITH_CHAIN_KEY] = wn
        return "((%s:=_with(_T(lambda:%s),%s)),%s)[-1]" % (
            wn,
            _px_emit(nd["env"], env),
            env.get(WITH_CHAIN_KEY, "None"),
            _px_emit(nd["body"], env2),
        )
    if tag == "assert":
        return "(%s if _bool(%s,'assert condition') else _assert_fail())" % (
            _px_emit(nd["body"], env),
            _px_emit(nd["cond"], env),
        )
    if tag == "unary":
        if nd["op"] == "-":
            return "_uneg(%s)" % _px_emit(nd["arg"], env)
        return "(not _bool(%s,'argument of !'))" % _px_emit(nd["arg"], env)
    if tag == "binary":
        op = nd["op"]
        if op == "&&":
            return "(_bool(%s,'left operand of &&') and _bool(%s,'right operand of &&'))" % (
                _px_emit(nd["lhs"], env),
                _px_emit(nd["rhs"], env),
            )
        if op == "||":
            return "(_bool(%s,'left operand of ||') or _bool(%s,'right operand of ||'))" % (
                _px_emit(nd["lhs"], env),
                _px_emit(nd["rhs"], env),
            )
        if op == "->":
            return "(True if not _bool(%s,'left operand of ->') else _bool(%s,'right operand of ->'))" % (
                _px_emit(nd["lhs"], env),
                _px_emit(nd["rhs"], env),
            )
        return "_bin(%r,%s,%s)" % (op, _px_emit(nd["lhs"], env), _px_emit(nd["rhs"], env))
    if tag == "lambda":
        if nd.get("pattern") is not None:
            pn = "v_arg_%d" % _px_fresh()
            env2 = dict(env)
            for name in _px_pattern_bound(nd["pattern"]):
                env2[name] = "_m[%r]" % name
            return "_C(lambda %s:((_m:=_bindpat(%s,%s)),%s)[-1],%s)" % (
                pn,
                _px_pattern(nd["pattern"], env),
                pn,
                _px_emit(nd["body"], env2),
                _px_pattern(nd["pattern"], env),
            )
        pn = "v_%s_%d" % (_px_mung(nd["param"]), _px_fresh())
        env2 = dict(env)
        env2[nd["param"]] = pn
        return "_C(lambda %s:%s)" % (pn, _px_emit(nd["body"], env2))
    if tag == "apply":
        return "_apply(%s,%s)" % (_px_emit(nd["func"], env), _px_t(nd["arg"], env))
    if tag == "select":
        return "_sel(_T(lambda:%s),%r)" % (_px_emit(nd["base"], env), nd["attr"])
    if tag == "select_default":
        return "_seldef(_T(lambda:%s),%r,_T(lambda:%s))" % (
            _px_emit(nd["base"], env),
            nd["attr"],
            _px_emit(nd["default"], env),
        )
    if tag == "dynamic_select":
        return "_dynsel(_T(lambda:%s),%s)" % (
            _px_emit(nd["base"], env),
            _px_attr_parts(nd["segments"], env),
        )
    if tag == "dynamic_select_default":
        return "_dyndef(_T(lambda:%s),%s,_T(lambda:%s))" % (
            _px_emit(nd["base"], env),
            _px_attr_parts(nd["segments"], env),
            _px_emit(nd["default"], env),
        )
    if tag == "has_attr":
        return "_has(_force(%s),%r)" % (_px_emit(nd["base"], env), nd.get("path", str(nd["attr"]).split(".")))
    if tag == "dynamic_has_attr":
        return "_dynhas(_T(lambda:%s),%s)" % (
            _px_emit(nd["base"], env),
            _px_attr_parts(nd["segments"], env),
        )
    if tag == "index":
        return "_index(%s,%s)" % (_px_emit(nd["base"], env), _px_emit(nd["index"], env))
    if tag == "match":
        return "_match(_T(lambda:%s),%s)" % (
            _px_emit(nd["scrutinee"], env),
            _px_match_arms(nd["arms"], env),
        )
    if tag == "str_interp":
        segs = []
        for part in nd["parts"]:
            if "lit" in part:
                segs.append("%r" % part["lit"])
            else:
                expr = part["expr"]
                if expr["tag"] == "var" and expr["name"] not in env:
                    segs.append("%r" % ("${" + expr["name"] + "}"))
                else:
                    segs.append("_coerce(%s)" % _px_emit(expr, env))
        return "_concatstrings([%s])" % ",".join(segs)
    if tag == "let":
        binds = nd["bindings"]
        env2 = dict(env)
        dn = "_d%d" % _px_fresh()
        for binding in binds:
            if _px_path_is_static(binding["path"]):
                top = binding["path"][0]
                env2[top] = "%s[%r]" % (dn, top)
        steps = ["(%s:={})" % dn]
        for binding in binds:
            use_env = env if binding.get("inherit_plain") else env2
            steps.append(
                "_setpath(%s,%s,%s,'let',%r)"
                % (dn, _px_binding_path(binding["path"], env2), _px_t(binding["value"], use_env), _px_binding_positions(binding))
            )
        return "((" + "),(".join(steps) + ")," + _px_emit(nd["body"], env2) + ")[-1]"
    if tag == "attrset":
        binds = nd["bindings"]
        recursive = bool(nd["recursive"])
        dn = "_d%d" % _px_fresh()
        env2 = dict(env)
        if recursive:
            for binding in binds:
                if _px_path_is_static(binding["path"]):
                    top = binding["path"][0]
                    env2[top] = "%s[%r]" % (dn, top)
        steps = ["(%s:=_A())" % dn]
        ordered = (
            [binding for binding in binds if _px_path_is_static(binding["path"])]
            + [binding for binding in binds if not _px_path_is_static(binding["path"])]
            if recursive
            else binds
        )
        for binding in ordered:
            use_env = env if binding.get("inherit_plain") else (env2 if recursive else env)
            steps.append(
                "_setpath(%s,%s,%s,'attr',%r)"
                % (
                    dn,
                    _px_binding_path(binding["path"], use_env),
                    _px_t(binding["value"], use_env),
                    _px_binding_positions(binding),
                )
            )
        return "((" + "),(".join(steps) + ")," + dn + ")[-1]"
    pnix_error("cannot compile node tag %r" % tag)
    raise AssertionError("unreachable")


_COMPILER_RUNTIME_NAMES = (
    "hashlib",
    "json",
    "math",
    "os",
    "re",
    "tempfile",
    "tomllib",
    "ET",
    "cmp_to_key",
    "_T",
    "_force",
    "_tv",
    "_C",
    "_P",
    "_S",
    "_isstr",
    "_ctx",
    "_mkstr",
    "_strctx",
    "_strplain",
    "_strplainctx",
    "_sbytes",
    "_expectedstr",
    "_A",
    "_K",
    "_WF",
    "_apply",
    "_unknownvar",
    "_assert_fail",
    "_with",
    "_with_attrs",
    "_with_lookup",
    "_setpath",
    "_realize",
    "_VALUES_EQUAL_MAX_DEPTH",
    "_eq",
    "_bool",
    "_num",
    "_int",
    "_attrs",
    "_less",
    "_uneg",
    "_bin",
    "_sel",
    "_seldef",
    "_attrkey",
    "_attrpath",
    "_attrparts",
    "_dynsel",
    "_dyndef",
    "_dynhas",
    "_has",
    "_index",
    "_COERCE_STACK",
    "_coerce",
    "_vts",
    "_deepforce",
    "_cjson",
    "_typeof",
    "_cell",
    "_merge_bind",
    "_pat",
    "_bindpat",
    "_formaldup",
    "_match",
    "_fold",
    "_foldr",
    "_seq",
    "_length",
    "_sort",
    "_sortlist",
    "_fnargs",
    "_zipattrswith",
    "_maplist",
    "_concatlists",
    "_take",
    "_drop",
    "_ziplist",
    "_flatten",
    "_find",
    "_get",
    "_set",
    "_keys",
    "_values",
    "_mapattrs",
    "_getattr_builtin",
    "_merge",
    "_getattrs",
    "_collectctx",
    "_concatstrings",
    "_concatstringssep",
    "_ctxstr",
    "_addoutname",
    "_getctx",
    "_hasctx",
    "_appendcontext",
    "_isfinite",
    "_isinf",
    "_isnan",
    "_derivation",
    "_fspath",
    "_pathlit",
    "_ftype",
    "_readfile",
    "_readdir",
    "_safe_store_name",
    "_tofile",
    "_hashbytes",
    "_hashstr",
    "_hashfile",
    "_basename",
    "_dirof",
    "_topath",
    "_getenv",
    "_RX_POSIX",
    "_rxpat",
    "_rxerrmsg",
    "_rxcompile",
    "_rxmatch",
    "_rxsplit",
    "_tryeval",
    "_fromjson",
    "_tomlval",
    "_fromtoml",
    "_mesc",
    "_mstr",
    "_etnode",
    "_xmlparse",
    "_htmlparse",
    "_mattrs",
    "_HTML_VOID",
    "_memitnode",
    "_memit",
    "_schemaroot",
    "_schematype",
    "_schemaerr",
    "_schemaerrors",
    "_schemanorm",
    "_schemavalidate",
    "_schemaexplain",
    "_gcsig",
    "_genericclosure",
    "_pow",
    "_bit",
    "_addctx",
    "_attrpos",
    "_splitparts",
    "_splitver",
    "_parsedrv",
    "_cmpver",
    "_srcfile",
    "_srcpos",
    "_bi",
    "v_builtins",
)


_HOST_EXEC_MOD: Any = None
_HOST_EXEC_TRIED = False


def _host_exec_source(code_source: str, namespace: dict[str, Any], filename: str) -> None:
    """SEP2: run the compiler lane's emitted Python through hy-meta's host-exec FLOOR
    (pnix-hy uses hy-meta as its host); fall back to a direct, byte-identical compile/exec
    if hy-meta is unavailable (so pnix_runtime still runs standalone). The pnix->Python
    EMITTER (_px_*) stays here; only host EXECUTION is delegated."""
    global _HOST_EXEC_MOD, _HOST_EXEC_TRIED
    if not _HOST_EXEC_TRIED:
        _HOST_EXEC_TRIED = True
        try:
            import importlib.util as _ilu
            path = Path(__file__).resolve().parents[2] / "hy-meta" / "host_exec.py"
            spec = _ilu.spec_from_file_location("pnix_hy_host_exec", str(path))
            if spec is not None and spec.loader is not None:
                mod = _ilu.module_from_spec(spec)
                spec.loader.exec_module(mod)
                _HOST_EXEC_MOD = mod
        except Exception:  # noqa: BLE001 - host floor unavailable -> fall back below
            _HOST_EXEC_MOD = None
    if _HOST_EXEC_MOD is not None:
        _HOST_EXEC_MOD.run_python_source(code_source, namespace, filename)
    else:
        exec(compile(code_source, filename, "exec"), namespace)  # fallback (standalone)


def compiler_import_namespace(runtime_namespace: dict[str, Any], ctx: dict[str, Any]) -> dict[str, Any]:
    namespace = {name: runtime_namespace[name] for name in _COMPILER_RUNTIME_NAMES if name in runtime_namespace}
    namespace["_PX_IMPORT"] = lambda path: import_value(path, ctx, "run", runtime_namespace)
    namespace["_PX_SCOPED_IMPORT"] = lambda scope, path: scoped_import_value(scope, path, ctx, "run", runtime_namespace)
    exec(
        "def _import(path):\n"
        "    return _PX_IMPORT(str(path))\n"
        "def _scopedimport(scope, path):\n"
        "    return _PX_SCOPED_IMPORT(_realize(scope), str(path))\n",
        namespace,
    )
    return namespace


def compile_px_source(
    source: str,
    *,
    realize: bool = True,
    include_prelude: bool = True,
    ctx: dict[str, Any] | None = None,
) -> str:
    """pnix source -> Python module source for the host compiler lane."""
    node = parse(source)
    _PX_CTR[0] = 0
    emit_ctx = runtime_context(ctx)
    emit_ctx["source_text"] = source
    _PX_EMIT_CTX[0] = emit_ctx
    expr = _px_emit(node, _px_initial_env(emit_ctx.get("compiler_env_names", {})))
    result_expr = "_realize(" + expr + ")" if realize else expr
    prefix = COMPILER_PRELUDE + "\n" if include_prelude else ""
    return prefix + "_RESULT=" + result_expr + "\n"


def run_px_source_raw(
    source: str,
    ctx: dict[str, Any],
    *,
    realize: bool,
    runtime_namespace: dict[str, Any] | None = None,
    include_prelude: bool = True,
) -> Any:
    """Compile and execute source. Imports use raw results to preserve functions."""
    if runtime_namespace is None:
        namespace: dict[str, Any] = {}
        runtime_namespace = namespace
    elif include_prelude:
        namespace = runtime_namespace
    else:
        namespace = compiler_import_namespace(runtime_namespace, ctx)
    namespace["_PX_IMPORT"] = lambda path: import_value(path, ctx, "run", runtime_namespace)
    namespace["_PX_SCOPED_IMPORT"] = lambda scope, path: scoped_import_value(scope, path, ctx, "run", runtime_namespace)
    filename = str(ctx.get("source_path", "<pnix-px>"))
    namespace["_PX_SOURCE_PATH"] = filename
    namespace["_PX_SOURCE_TEXT"] = source
    namespace["_PX_BASE_DIR"] = str(ctx.get("base_dir", Path.cwd()))
    compile_ctx = ctx
    if ctx.get("env"):
        compile_ctx = dict(ctx)
        compiler_env_names: dict[str, str] = {}
        for index, (key, value) in enumerate(ctx["env"].items()):
            var_name = f"_PX_ENV_{index}"
            namespace[var_name] = value
            compiler_env_names[str(key)] = var_name
        compile_ctx["compiler_env_names"] = compiler_env_names
    code = compile_px_source(source, realize=realize, include_prelude=include_prelude, ctx=compile_ctx)
    _host_exec_source(code, namespace, filename)  # SEP2: host execution via hy-meta floor (fallback inline)
    return namespace["_RESULT"]


def run_px_source(source: str, opts: dict[str, Any] | None = None) -> Any:
    """Entry point A: compile pnix source and execute on host CPython."""
    return run_px_source_raw(source, runtime_context(opts), realize=True)


def run_px(path: str) -> Any:
    """Entry point A for a `.px` file: read it and run it at host speed."""
    resolved = Path(path).expanduser().resolve()
    ctx = runtime_context(
        {"base_dir": str(resolved.parent), "source_path": str(resolved), "path_literals_absolute": True}
    )
    key = f"run:{resolved}"
    stack = ctx.setdefault("import_stack", [])
    if key in stack:
        pnix_error("import cycle: " + " -> ".join(stack + [key]))
    stack.append(key)
    try:
        return run_px_source_raw(read_px_file(resolved), ctx, realize=True)
    finally:
        stack.pop()


def import_self_test_cases() -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []

    def record(name: str, fn: Callable[[], Any], expect: Any) -> None:
        try:
            actual = fn()
            cases.append({"name": name, "expect": expect, "actual": actual, "ok": actual == expect})
        except Exception as exc:  # noqa: BLE001 - self-test payload should report failures.
            cases.append({"name": name, "expect": expect, "error": str(exc), "ok": False})

    with tempfile.TemporaryDirectory(prefix="pnix-hy-import-test-") as tmp:
        root = Path(tmp)
        (root / "lib.px").write_text("{ x = 41; f = n: n + 1; }\n", encoding="utf-8")
        (root / "main.px").write_text("let lib = import ./lib.px; in lib.f lib.x\n", encoding="utf-8")
        (root / "builtin-main.px").write_text("let lib = builtins.import ./lib.px; in lib.f lib.x\n", encoding="utf-8")
        (root / "scoped.px").write_text("x + y\n", encoding="utf-8")
        (root / "sub").mkdir()
        (root / "sub" / "nested.px").write_text("let lib = import ../lib.px; in lib.x + 1\n", encoding="utf-8")
        (root / "sub" / "closure.px").write_text(
            "B: { v = import ./sibling.px; }\n", encoding="utf-8"
        )
        (root / "sub" / "sibling.px").write_text("42\n", encoding="utf-8")
        (root / "sibling.px").write_text("13\n", encoding="utf-8")
        (root / "closure-main.px").write_text(
            "((import ./sub/closure.px) builtins).v\n", encoding="utf-8"
        )
        (root / "a.px").write_text("import ./b.px\n", encoding="utf-8")
        (root / "b.px").write_text("import ./a.px\n", encoding="utf-8")

        def eval_cached_import() -> dict[str, Any]:
            report = eval_source(
                "let a = import ./lib.px; b = import ./lib.px; in a.x + b.x",
                {"base_dir": str(root), "mirror": True},
            )
            return {
                "value": report["value"],
                "imports": len([event for event in report["events"] if event.get("event") == "import"]),
            }

        def cycle_error() -> str:
            try:
                run_px(str(root / "a.px"))
            except PnixError as exc:
                return "import cycle" if "import cycle" in str(exc) else str(exc)
            return "no error"

        record("import-eval-relative-cache", eval_cached_import, {"value": 82, "imports": 1})
        record(
            "builtin-import-eval-relative",
            lambda: eval_source('let lib = builtins.import ./lib.px; in lib.f lib.x', {"base_dir": str(root)}),
            42,
        )
        record("import-run-relative", lambda: run_px(str(root / "main.px")), 42)
        record("builtin-import-run-relative", lambda: run_px(str(root / "builtin-main.px")), 42)
        record(
            "builtin-scopedImport-run-relative",
            lambda: run_px_source(
                'builtins.scopedImport { x = 40; y = 2; } ./scoped.px',
                {"base_dir": str(root)},
            ),
            42,
        )
        record("import-run-nested-relative", lambda: run_px(str(root / "sub" / "nested.px")), 42)
        record(
            "import-eval-closure-keeps-module-base",
            lambda: eval_source(
                (root / "closure-main.px").read_text(encoding="utf-8"),
                {"base_dir": str(root)},
            ),
            42,
        )
        record(
            "import-run-closure-keeps-module-base",
            lambda: run_px(str(root / "closure-main.px")),
            42,
        )
        record("import-run-cycle", cycle_error, "import cycle")

    return cases


LOCAL_FIXTURES_DIR = Path(__file__).resolve().parents[1] / "fixtures" / "pnix_expr"


def fixture_report(fixtures_dir: str | None = None) -> dict[str, Any]:
    # Optional oracle: pnix-hy does NOT depend on any external repo. Provide the
    # fixtures via the `fixtures_dir` argument or the PNIX_HY_FIXTURES_DIR env var;
    # with neither set, the repo-local corpus is used (no hardcoded ~/pnix path).
    fixtures_dir = fixtures_dir or os.environ.get("PNIX_HY_FIXTURES_DIR")
    if not fixtures_dir and LOCAL_FIXTURES_DIR.exists():
        fixtures_dir = str(LOCAL_FIXTURES_DIR)
    if not fixtures_dir:
        return {
            "schema": "pnix-hy.fixture-parity.v0",
            "ready": False,
            "available": False,
            "fixtures_dir": None,
            "cases": [{"name": "fixtures-dir-set", "ok": False, "error": "no fixtures dir; pass fixtures_dir= or set PNIX_HY_FIXTURES_DIR"}],
        }
    root = Path(fixtures_dir).expanduser()
    cases: list[dict[str, Any]] = []
    if not root.exists():
        return {
            "schema": "pnix-hy.fixture-parity.v0",
            "ready": False,
            "available": False,
            "fixtures_dir": str(root),
            "cases": [{"name": "fixtures-dir-exists", "ok": False, "error": "missing fixtures directory"}],
        }
    for expected_path in sorted(root.glob("scenario*.expected.json")):
        source_path = expected_path.with_suffix("").with_suffix(".px")
        case: dict[str, Any] = {"name": source_path.name, "source": str(source_path), "expected": str(expected_path)}
        try:
            expected = json.loads(expected_path.read_text(encoding="utf-8"))
            if not source_path.exists():
                pnix_error(f"missing fixture source `{source_path}`")
            actual = stable_data(run_px(str(source_path)))
            case.update({"expect": expected, "actual": actual, "ok": actual == expected})
        except Exception as exc:  # noqa: BLE001 - report all fixture failures.
            case.update({"ok": False, "error": str(exc)})
        cases.append(case)
    return {
        "schema": "pnix-hy.fixture-parity.v0",
        "ready": bool(cases) and all(case["ok"] for case in cases),  # empty dir must not pass vacuously
        "available": True,
        "fixtures_dir": str(root),
        "count": len(cases),
        "cases": cases,
    }


# ---------------------------------------------------------------------------
# ORIGINAL ~/pnix oracle (highest-fidelity ground truth).
#
# Wraps `~/pnix/target/release/pnixc-meta <file>.px` (the meta-circular `.px`
# evaluator -- ".px evaluates .px") as a live cross-implementation parity oracle,
# exactly parallel to fixture_report() but against the running original binary
# instead of pre-baked expected.json. The installed binary is the v0 meta-circular
# interpreter and supports only a CORE subset (no rec/with/match/dynamic-select/
# most builtins/lambda patterns), so cases the original cannot parse/eval are
# classified `unsupported` rather than failed; only a genuine value divergence is
# a `disagree` and fails the report. Source is written to a temp `.px` so the
# original and pnix-hy `run_px` resolve relative paths identically.
# ---------------------------------------------------------------------------

# Optional oracle binary location: env-only, no hardcoded ~/pnix path (pnix-hy must
# not depend on any external repo). Set PNIX_ORIGINAL_PNIXC_META to enable.
ORIGINAL_V0_KNOWN_DIVERGENCES: dict[str, str] = {
    "builtin-mod-float": "v0 pnixc-meta truncates float modulo; full interpret.rs uses fmod",
    "attr-two-explicit-merge": "v0 pnixc-meta overwrites duplicate explicit attrsets; full interpret.rs merges distinct attrset leaves",
    "guard-bitops-happy": "v0 pnixc-meta lacks bitAnd/bitOr/bitXor; full interpret.rs exposes the bitop builtins",
}


def original_pnixc_meta_path() -> str | None:
    """Locate the original pnix meta-circular evaluator, or None if absent."""
    env = os.environ.get("PNIX_ORIGINAL_PNIXC_META")
    if env and Path(env).exists():
        return env
    return None


_HOST_CLEAN_REPLAY_MOD: Any = None
_HOST_CLEAN_REPLAY_TRIED = False


def _host_clean_probe(command: list[str], *, timeout: int) -> dict[str, Any] | None:
    """SR3: ask hy-meta to own clean subprocess replay when it is available."""
    global _HOST_CLEAN_REPLAY_MOD, _HOST_CLEAN_REPLAY_TRIED
    if not _HOST_CLEAN_REPLAY_TRIED:
        _HOST_CLEAN_REPLAY_TRIED = True
        path = Path(__file__).resolve().parents[2] / "hy-meta" / "clean_replay.py"
        if path.exists():
            added = False
            hy_meta_dir = str(path.parent)
            if hy_meta_dir not in sys.path:
                sys.path.insert(0, hy_meta_dir)
                added = True
            try:
                import importlib.util as _ilu
                spec = _ilu.spec_from_file_location("pnix_hy_clean_replay", str(path))
                if spec is not None and spec.loader is not None:
                    mod = _ilu.module_from_spec(spec)
                    spec.loader.exec_module(mod)
                    _HOST_CLEAN_REPLAY_MOD = mod
            except Exception:  # noqa: BLE001 - keep standalone oracle fallback below
                _HOST_CLEAN_REPLAY_MOD = None
            finally:
                if added:
                    try:
                        sys.path.remove(hy_meta_dir)
                    except ValueError:
                        pass
    if _HOST_CLEAN_REPLAY_MOD is None:
        return None
    try:
        return _HOST_CLEAN_REPLAY_MOD.run_clean_probe(
            command,
            timeout=timeout,
            parse_json=False,
        )
    except (Exception, SystemExit):  # noqa: BLE001 - unsupported hy-meta Python -> fallback
        return None


def _run_original_px(binary: str, source: str) -> tuple[Any, str]:
    """Evaluate `source` through the original binary; return (parsed-json, tmppath).

    The caller owns deleting tmppath (so pnix-hy can `run_px` the same file for
    path-resolution parity). Raises on non-JSON output."""
    import subprocess

    handle = tempfile.NamedTemporaryFile("w", suffix=".px", delete=False)
    try:
        handle.write(source)
        handle.close()
        clean_probe = _host_clean_probe([binary, handle.name], timeout=30)
        if clean_probe is None:
            proc = subprocess.run(
                [binary, handle.name], capture_output=True, text=True, timeout=30, check=False
            )
            out = proc.stdout.strip()
            err = proc.stderr.strip()
        else:
            out = clean_probe["stdout"].strip()
            err = clean_probe["stderr"].strip()
    except BaseException:
        os.unlink(handle.name)
        raise
    try:
        return json.loads(out), handle.name
    except json.JSONDecodeError as exc:
        os.unlink(handle.name)
        detail = out or err
        raise PnixError(f"original pnixc-meta produced non-JSON output: {detail!r}") from exc


def _original_is_error(value: Any) -> bool:
    """The v0 binary reports parse/eval gaps as a JSON `{status: error}` envelope."""
    return isinstance(value, dict) and value.get("status") == "error" and "error" in value


def original_oracle_report(
    cases: list[dict[str, Any]] | None = None,
    binary: str | None = None,
    include_unsupported: bool = False,
) -> dict[str, Any]:
    """Live parity against the ORIGINAL ~/pnix evaluator over the corpus.

    `ready` is True when there is no `disagree` (every case the original can
    evaluate yields the same value as pnix-hy's compiler). Returns False/available
    when the binary is missing so callers can skip without crashing."""
    binary = binary or original_pnixc_meta_path()
    cases = cases if cases is not None else SELF_TEST_CASES
    if binary is None:
        return {
            "schema": "pnix-hy.original-oracle.v0",
            "ready": False,
            "available": False,
            "cases": [
                {
                    "name": "pnixc-meta-exists",
                    "ok": False,
                    "error": "original pnixc-meta not found; set PNIX_ORIGINAL_PNIXC_META",
                }
            ],
        }
    agree = disagree = unsupported = 0
    out: list[dict[str, Any]] = []
    for case in cases:
        rec: dict[str, Any] = {"name": case["name"], "source": case["source"]}
        try:
            original, tmppath = _run_original_px(binary, case["source"])
            try:
                if _original_is_error(original):
                    unsupported += 1
                    rec.update({"status": "unsupported", "original_error": original["error"]})
                    out.append(rec)
                    continue
                actual = stable_data(run_px(tmppath))
            finally:
                os.unlink(tmppath)
            ok = actual == original
            known_divergence = ORIGINAL_V0_KNOWN_DIVERGENCES.get(case["name"])
            if not ok and known_divergence:
                unsupported += 1
                rec.update(
                    {
                        "status": "unsupported",
                        "reason": known_divergence,
                        "original": original,
                        "pnix_hy": actual,
                    }
                )
                out.append(rec)
                continue
            rec.update(
                {
                    "status": "agree" if ok else "disagree",
                    "ok": ok,
                    "original": original,
                    "pnix_hy": actual,
                }
            )
            agree += int(ok)
            disagree += int(not ok)
        except Exception as exc:  # noqa: BLE001 - report every case outcome.
            unsupported += 1
            rec.update({"status": "skipped", "error": str(exc)})
        out.append(rec)
    return {
        "schema": "pnix-hy.original-oracle.v0",
        "ready": disagree == 0,
        "available": True,
        "binary": binary,
        "total": len(cases),
        "agree": agree,
        "disagree": disagree,
        "unsupported": unsupported,
        "cases": out if include_unsupported else [c for c in out if c["status"] in ("agree", "disagree")],
    }


# ---------------------------------------------------------------------------
# Rust ground-truth corpus (P0): cases adapted from the FULL original evaluator's
# test suite `~/pnix/crates/pnix-eval/tests/eval_basics.rs`. Unlike the installed
# v0 `pnixc-meta`, the Rust `pnix-eval` tree-walker is the COMPLETE reference, so
# these {source, expect} pairs are high-fidelity ground truth that needs no binary
# to run (static). `rust_corpus_report()` checks pnix-hy's interpreter AND compiler
# against them; error cases assert pnix-hy raises (exact message text is a separate
# P0 item, so only `error_contains` is recorded, not gated).
# ---------------------------------------------------------------------------

RUST_EVAL_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-arithmetic", "source": "1 + 2", "expect": 3},
    {"name": "rs-let-in", "source": "let x = 10; in x + 5", "expect": 15},
    {"name": "rs-let-recursive", "source": "let a = b + 1; b = 2; in a", "expect": 3},
    {
        "name": "rs-self-recursive-lambda",
        "source": "let sum = xs: if (builtins.length xs) == 0 then 0 else (builtins.head xs) + sum (builtins.tail xs); in sum [1 2 3]",
        "expect": 6,
    },
    {"name": "rs-lambda-apply", "source": "(x: x + 1) 5", "expect": 6},
    {"name": "rs-curried-lambda", "source": "let add = x: y: x + y; in add 3 4", "expect": 7},
    {
        "name": "rs-attrset-pattern-default-binding",
        "source": "(args@{ x, y ? x + 1, ... }: args.x + y + args.z) { x = 3; z = 5; }",
        "expect": 12,
    },
    {
        "name": "rs-list-pattern-tail",
        "source": "([x, y, ...rest]: x + y + builtins.length rest) [1 2 3 4]",
        "expect": 5,
    },
    {"name": "rs-attrset-select", "source": "{ a = 1; b = 2; }.a", "expect": 1},
    {"name": "rs-rec-attrset", "source": "rec { a = 1; b = a + 2; }.b", "expect": 3},
    {"name": "rs-rec-forward", "source": "rec { a = b + 1; b = 2; }.a", "expect": 3},
    {"name": "rs-rec-late-binding", "source": "rec { f = x: x + seed; seed = 2; }.f 3", "expect": 5},
    {"name": "rs-inherit-let-from-throw-unused", "source": 'let s = builtins.throw "boom"; in (let inherit (s) a; in 1)', "expect": 1},
    {"name": "rs-inherit-attr-from-throw-unused", "source": 'let s = builtins.throw "boom"; in ({ inherit (s) a; }).b or 99', "expect": 99},
    {"name": "rs-inherit-from-lazy-field", "source": 'let s = { a = 1; b = builtins.throw "side"; }; in (let inherit (s) a; in a)', "expect": 1},
    {"name": "rs-inherit-multiple-independent", "source": 'let s = { a = 1; b = builtins.throw "b-side"; }; in (let inherit (s) a b; in a)', "expect": 1},
    {"name": "rs-inherit-chain-through-let", "source": 'let s = { a = 7; b = builtins.throw "side"; }; in (let inherit (s) a; in (let inherit a; in a))', "expect": 7},
    {"name": "rs-inherit-rec-outer-lazy", "source": "let x = 5; in (rec { inherit x; y = x + 1; }).y", "expect": 6},
    {"name": "rs-inherit-rec-from-scope", "source": 'let s = { a = 10; quirk = builtins.throw "side"; }; in (rec { inherit (s) a; b = a + 1; }).b', "expect": 11},
    {"name": "rs-nested-let-path", "source": "let a.b = 1; a.c = 2; in a.b + a.c", "expect": 3},
    {"name": "rs-nested-let-recursive-path", "source": "let a.b = 1; a.c = a.b + 2; in a.c", "expect": 3},
    {"name": "rs-match-guard-fallthrough", "source": "match 2 with | x if x > 2 => 9 | x if x == 2 => x + 1 | _ => 0", "expect": 3},
    {"name": "rs-match-guard-null-body", "source": "match 1 with | x if true => null | _ => 5", "expect": None},
    {"name": "rs-path-absolute-isPath", "source": "builtins.isPath /tmp/pnix-hy-path", "expect": True},
    {"name": "rs-path-home-isPath", "source": "builtins.isPath ~/pnix-hy-path", "expect": True},
    {"name": "rs-path-search-toString", "source": "builtins.toString <nixpkgs>", "expect": "<nixpkgs>"},
    {"name": "rs-path-interp-basename", "source": 'let name = "bar"; in builtins.baseNameOf ./foo/${name}', "expect": "bar"},
    {"name": "rs-curPos-line", "source": "__curPos.line", "expect": 1},
    {"name": "rs-curPos-shadow-column", "source": "let __curPos = 1; in __curPos.column", "expect": 22},
    {"name": "rs-unsafeGetAttrPos-line", "source": '(builtins.unsafeGetAttrPos "a" {\n  a = 1;\n}).line', "expect": 2},
    {"name": "rs-unsafeGetAttrPos-column", "source": '(builtins.unsafeGetAttrPos "a" {\n  a = 1;\n}).column', "expect": 3},
    {"name": "rs-unsafeGetAttrPos-nested-column", "source": '(builtins.unsafeGetAttrPos "b" ({ a.b = 1; }.a)).column', "expect": 37},
    {"name": "rs-unsafeGetAttrPos-generated-null", "source": 'builtins.unsafeGetAttrPos "a" (builtins.listToAttrs [ { name = "a"; value = 1; } ])', "expect": None},
    {"name": "rs-builtins-map", "source": "builtins.map (x: x * 2) [1 2 3]", "expect": [2, 4, 6]},
    {"name": "rs-builtins-filter", "source": "builtins.filter (x: x > 2) [1 2 3 4 5]", "expect": [3, 4, 5]},
    {"name": "rs-builtins-length", "source": "builtins.length [1 2 3 4]", "expect": 4},
    {"name": "rs-builtins-foldl", "source": "builtins.foldl' (acc: x: acc + x) 0 [1 2 3 4]", "expect": 10},
    {"name": "rs-with-expr", "source": "with { x = 42; }; x", "expect": 42},
    {"name": "rs-assert-pass", "source": "assert true; 42", "expect": 42},
    {"name": "rs-if-then-else", "source": "if true then 1 else 2", "expect": 1},
    {"name": "rs-string-interp", "source": 'let name = "world"; in "hello ${name}"', "expect": "hello world"},
    {"name": "rs-string-interp-placeholder", "source": 'let known = "x"; in "prefix ${known} ${value}"', "expect": "prefix x ${value}"},
    {"name": "rs-interp-out-path", "source": '"path=${ { outPath = "/nix/store/x"; } }"', "expect": "path=/nix/store/x"},
    {"name": "rs-interp-to-string", "source": '"greeting=${ { __toString = self: "hello"; } }"', "expect": "greeting=hello"},
    {"name": "rs-interp-to-string-self", "source": '"${ { __toString = self: self.label; label = "abc"; } }"', "expect": "abc"},
    {"name": "rs-interp-to-string-priority", "source": '"${ { __toString = _: "from-toString"; outPath = "from-outPath"; } }"', "expect": "from-toString"},
    {"name": "rs-interp-out-path-nested", "source": '"${ { outPath = { __toString = _: "deep"; }; } }"', "expect": "deep"},
    {"name": "rs-interp-explicit-to-string", "source": '"value=${builtins.toString 42}"', "expect": "value=42"},
    {"name": "rs-has-attr-true", "source": "{ a = 1; } ? a", "expect": True},
    {"name": "rs-has-attr-false", "source": "{ a = 1; } ? b", "expect": False},
    {"name": "rs-list-concat", "source": "[1 2] ++ [3 4]", "expect": [1, 2, 3, 4]},
    {"name": "rs-attrset-merge", "source": "{ a = 1; } // { b = 2; }", "expect": {"a": 1, "b": 2}},
    {"name": "rs-err-unexpected-attr", "source": "({ x }: x) { x = 1; y = 2; }", "error": True, "error_contains": "unexpected attribute"},
    {"name": "rs-err-list-arity", "source": "([x, y]: x) [1]", "error": True, "error_contains": "list pattern"},
    {"name": "rs-err-rec-cycle", "source": "rec { a = b; b = a; }.a", "error": True, "error_contains": "recursive"},
    {"name": "rs-err-assert-fail", "source": "assert false; 42", "error": True},
    {"name": "rs-err-interp-int", "source": '"value=${1}"', "error": True, "error_contains": "coerce"},
    {"name": "rs-err-interp-list", "source": '"value=${[1 2 3]}"', "error": True, "error_contains": "coerce a list"},
    {"name": "rs-err-interp-set", "source": '"value=${ { a = 1; } }"', "error": True, "error_contains": "__toString"},
    {"name": "rs-err-interp-bool", "source": '"value=${true}"', "error": True, "error_contains": "coerce a boolean"},
    {"name": "rs-err-interp-null", "source": '"value=${null}"', "error": True, "error_contains": "coerce null"},
    {"name": "rs-err-interp-lambda", "source": '"value=${ x: x }"', "error": True, "error_contains": "coerce a function"},
    {"name": "rs-err-dup-attr", "source": "{ a = 1; a = 2; }", "error": True, "error_contains": "already defined"},
    {"name": "rs-err-rec-assign-then-inherit", "source": "let x = 99; in (rec { x = 1; inherit x; }).x", "error": True, "error_contains": "already defined"},
    {"name": "rs-err-inherit-then-assign", "source": "let x = 99; in ({ inherit x; x = 1; }).x", "error": True, "error_contains": "already defined"},
    {"name": "rs-err-inherit-from-then-assign", "source": "let s = { a = 1; }; in ({ inherit (s) a; a = 99; }).a", "error": True, "error_contains": "already defined"},
    {"name": "rs-err-assign-then-inherit-from", "source": "let s = { a = 1; }; in ({ a = 99; inherit (s) a; }).a", "error": True, "error_contains": "already defined"},
    {"name": "rs-err-let-dup", "source": "let x = 1; x = 2; in x", "error": True, "error_contains": "more than once"},
    {"name": "rs-err-let-inherit-then-binding-dup", "source": "let inherit ({a=99;}) a; a = 1; in a", "error": True, "error_contains": "more than once"},
    {"name": "rs-err-let-inherit-twice-dup", "source": "let inherit ({a=1;}) a; inherit ({a=2;}) a; in a", "error": True, "error_contains": "more than once"},
    {"name": "rs-err-attr-path-conflict", "source": "{ a = 1; a.b = 2; }", "error": True, "error_contains": "attribute path conflict"},
    {"name": "rs-err-let-path-conflict", "source": "let a = 1; a.b = 2; in a", "error": True, "error_contains": "attribute path conflict"},
    {"name": "rs-err-match-guard-nonbool", "source": "match 1 with | x if 1 => x | _ => 0", "error": True, "error_contains": "match guard"},
    {"name": "rs-mod-negative-left", "source": "(-10) % 3", "expect": -1},
    {"name": "rs-mod-negative-right", "source": "10 % (-3)", "expect": 1},
    {"name": "rs-mod-float", "source": "10.5 % 3", "expect": 1.5},
    {"name": "rs-err-mod-zero", "source": "10 % 0", "error": True, "error_contains": "modulo by zero"},
    {"name": "rs-err-builtin-mod-zero", "source": "builtins.mod 10 0", "error": True, "error_contains": "builtins.mod: division by zero"},
    {"name": "rs-err-div-zero", "source": "7 / 0", "error": True, "error_contains": "division by zero"},
    {"name": "rs-err-div-float-zero", "source": "7.0 / 0.0", "error": True, "error_contains": "division by zero"},
    {"name": "rs-err-builtin-div-zero", "source": "builtins.div 7 0", "error": True, "error_contains": "division by zero"},
    {"name": "rs-err-select-null", "source": "(null).foo", "error": True, "error_contains": "select base must be an attrset"},
    {"name": "rs-list-head-singleton", "source": "builtins.head [ 7 ]", "expect": 7},
    {"name": "rs-list-tail-singleton", "source": "builtins.tail [ 7 ]", "expect": []},
    {"name": "rs-list-elemat-in-bounds", "source": "builtins.elemAt [ 10 20 30 ] 2", "expect": 30},
    {"name": "rs-err-head-empty", "source": "builtins.head []", "error": True, "error_contains": "list is empty"},
    {"name": "rs-err-tail-empty", "source": "builtins.tail []", "error": True, "error_contains": "list is empty"},
    {"name": "rs-err-elemat-negative", "source": "builtins.elemAt [ 10 20 30 ] (-1)", "error": True, "error_contains": "negative index"},
    {"name": "rs-err-elemat-out-of-bounds", "source": "builtins.elemAt [ 10 20 30 ] 3", "error": True, "error_contains": "out of bounds"},
    {"name": "rs-genlist-zero", "source": "builtins.genList (i: i) 0", "expect": []},
    {"name": "rs-genlist-three", "source": "builtins.genList (i: i * i) 3", "expect": [0, 1, 4]},
    {"name": "rs-err-genlist-negative", "source": "builtins.genList (i: i) (-1)", "error": True, "error_contains": "negative count"},
    {"name": "rs-err-genlist-too-large", "source": "builtins.genList (i: i) 9223372036854775807", "error": True, "error_contains": "exceeds maximum"},
    {"name": "rs-take-overbound", "source": "builtins.take 5 [ 1 2 3 ]", "expect": [1, 2, 3]},
    {"name": "rs-drop-overbound", "source": "builtins.drop 5 [ 1 2 3 ]", "expect": []},
    {"name": "rs-err-take-negative", "source": "builtins.take (-1) [ 1 2 3 ]", "error": True, "error_contains": "negative count"},
    {"name": "rs-err-drop-negative", "source": "builtins.drop (-1) [ 1 2 3 ]", "error": True, "error_contains": "negative count"},
    {"name": "rs-err-filter-nonbool", "source": "builtins.filter (x: x) [ 1 2 3 ]", "error": True, "error_contains": "predicate"},
    {"name": "rs-err-any-nonbool", "source": "builtins.any (x: x) [ 1 ]", "error": True, "error_contains": "predicate"},
    {"name": "rs-err-all-nonbool", "source": "builtins.all (x: 42) [ 1 ]", "error": True, "error_contains": "predicate"},
    {"name": "rs-err-sort-nonbool", "source": "builtins.sort (a: b: 42) [ 1 2 ]", "error": True, "error_contains": "comparator"},
    {"name": "rs-fromjson-i64-max", "source": 'builtins.fromJSON "9223372036854775807"', "expect": 9223372036854775807},
    {"name": "rs-fromjson-i64-min", "source": 'builtins.fromJSON "-9223372036854775808"', "expect": -9223372036854775808},
    {"name": "rs-fromjson-float-exp", "source": 'builtins.fromJSON "1e3"', "expect": 1000.0},
    {"name": "rs-fromjson-string-big-number-safe", "source": 'builtins.fromJSON "\\"999999999999999999999\\""', "expect": "999999999999999999999"},
    {"name": "rs-err-fromjson-big-int", "source": 'builtins.fromJSON "999999999999999999999"', "error": True, "error_contains": "too large"},
    {"name": "rs-err-fromjson-nested-big-int", "source": 'builtins.fromJSON "{\\"x\\":999999999999999999999}"', "error": True, "error_contains": "too large"},
    {"name": "rs-floor-large-ok", "source": "builtins.floor 1.0e18", "expect": 1000000000000000000},
    {"name": "rs-ceil-negative-ok", "source": "builtins.ceil (-3.2)", "expect": -3},
    {"name": "rs-floor-int-f64-exact", "source": "builtins.floor 9007199254740994", "expect": 9007199254740994},
    {"name": "rs-ceil-int-f64-exact", "source": "builtins.ceil (-9007199254740994)", "expect": -9007199254740994},
    {"name": "rs-floor-int-f64-precision", "source": "builtins.floor 9007199254740993", "error": True, "error_contains": "precision"},
    {"name": "rs-ceil-int-f64-range", "source": "builtins.ceil 9223372036854775807", "error": True, "error_contains": "outside i64 range"},
    {"name": "rs-err-floor-inf", "source": "builtins.floor (1.0e200 * 1.0e200)", "error": True, "error_contains": "+inf"},
    {"name": "rs-err-floor-nan", "source": "let inf = 1.0e200 * 1.0e200; in builtins.floor (inf - inf)", "error": True, "error_contains": "NaN"},
    {"name": "rs-err-floor-out-of-range", "source": "builtins.floor 1.0e200", "error": True, "error_contains": "outside i64 range"},
    {"name": "rs-err-ceil-out-of-range", "source": "builtins.ceil 1.0e200", "error": True, "error_contains": "outside i64 range"},
    {"name": "rs-pow-3-39-exact", "source": "builtins.pow 3 39", "expect": 4052555153018976267},
    {"name": "rs-pow-2-63-float-type", "source": "builtins.typeOf (builtins.pow 2 63)", "expect": "float"},
    {"name": "rs-pow-negative-exp", "source": "builtins.pow 2 (-1)", "expect": 0.5},
    {"name": "rs-err-tojson-lambda", "source": "builtins.toJSON (x: x)", "error": True, "error_contains": "cannot serialize function"},
    {"name": "rs-err-tojson-lambda-list", "source": "builtins.toJSON [ (x: x) ]", "error": True, "error_contains": "cannot serialize function"},
    {"name": "rs-err-tojson-builtin-partial", "source": "builtins.toJSON (builtins.add 1)", "error": True, "error_contains": "cannot serialize function"},
    {"name": "rs-err-tojson-inf", "source": "builtins.toJSON (1.0e200 * 1.0e200)", "error": True, "error_contains": "cannot serialize float +inf as JSON"},
    {"name": "rs-err-fromjson-nan", "source": 'builtins.fromJSON "NaN"', "error": True, "error_contains": "invalid JSON numeric constant"},
    {"name": "rs-str-substring-basic-0", "source": 'builtins.substring 0 3 "hello"', "expect": "hel"},
    {"name": "rs-str-substring-basic-1", "source": 'builtins.substring 1 3 "hello"', "expect": "ell"},
    {"name": "rs-str-substring-zero", "source": 'builtins.substring 0 0 "hello"', "expect": ""},
    {"name": "rs-str-substring-negative-length", "source": 'builtins.substring 1 (-1) "hello"', "expect": "ello"},
    {"name": "rs-str-substring-clamp", "source": 'builtins.substring 2 100 "hello"', "expect": "llo"},
    {"name": "rs-str-substring-start-past", "source": 'builtins.substring 100 3 "hello"', "expect": ""},
    {"name": "rs-str-substring-utf8-1", "source": 'builtins.substring 0 1 "héllo"', "expect": "h"},
    {"name": "rs-str-substring-utf8-2", "source": 'builtins.substring 0 3 "héllo"', "expect": "hé"},
    {"name": "rs-str-substring-korean", "source": 'builtins.substring 0 6 "빛은 뭐야?"', "expect": "빛은"},
    {"name": "rs-str-stringlength-ascii", "source": 'builtins.stringLength "hello"', "expect": 5},
    {"name": "rs-str-stringlength-utf8", "source": 'builtins.stringLength "héllo"', "expect": 6},
    {"name": "rs-str-stringlength-korean", "source": 'builtins.stringLength "안녕요"', "expect": 9},
    {"name": "rs-str-stringlength-empty", "source": 'builtins.stringLength ""', "expect": 0},
    {"name": "rs-str-concatsep-basic", "source": 'builtins.concatStringsSep ", " [ "a" "b" "c" ]', "expect": "a, b, c"},
    {"name": "rs-str-concatsep-empty", "source": 'builtins.concatStringsSep "," []', "expect": ""},
    {"name": "rs-str-concatsep-single", "source": 'builtins.concatStringsSep "," [ "x" ]', "expect": "x"},
    {"name": "rs-str-replace-basic", "source": 'builtins.replaceStrings [ "a" "c" ] [ "X" "Z" ] "abc"', "expect": "XbZ"},
    {"name": "rs-str-replace-utf8", "source": 'builtins.replaceStrings [ "$" ] [ "\\$" ] "한국어 $HOME"', "expect": "한국어 $HOME"},
    {"name": "rs-str-replace-empty-pattern-utf8", "source": 'builtins.replaceStrings [""] ["|"] "한글"', "expect": "|한|글|"},
    {"name": "rs-str-replace-no-match", "source": 'builtins.replaceStrings [ "x" ] [ "Y" ] "abc"', "expect": "abc"},
    {"name": "rs-str-replace-empty-pattern", "source": 'builtins.replaceStrings [""] ["X"] "abc"', "expect": "XaXbXcX"},
    {"name": "rs-str-err-substring-negative-start", "source": 'builtins.substring (-1) 3 "hello"', "error": True, "error_contains": "negative start position"},
    {"name": "rs-str-err-concatsep-element", "source": 'builtins.concatStringsSep "," [ "a" 1 "c" ]', "error": True, "error_contains": "index 1"},
    {"name": "rs-str-err-concatsep-separator", "source": 'builtins.concatStringsSep 42 [ "a" "b" ]', "error": True, "error_contains": "separator"},
    {"name": "rs-ts-string", "source": 'toString "hello"', "expect": "hello"},
    {"name": "rs-ts-quotes", "source": 'toString "with \\"quotes\\""', "expect": 'with "quotes"'},
    {"name": "rs-ts-int", "source": "toString 42", "expect": "42"},
    {"name": "rs-ts-int-zero", "source": "toString 0", "expect": "0"},
    {"name": "rs-ts-int-negative", "source": "toString (-7)", "expect": "-7"},
    {"name": "rs-ts-float", "source": "toString 3.5", "expect": "3.500000"},
    {"name": "rs-ts-bool-true", "source": "toString true", "expect": "1"},
    {"name": "rs-ts-bool-false", "source": "toString false", "expect": ""},
    {"name": "rs-ts-null", "source": "toString null", "expect": ""},
    {"name": "rs-ts-list", "source": "toString [ 1 2 3 ]", "expect": "1 2 3"},
    {"name": "rs-ts-list-mixed", "source": 'toString [ 1 "x" true ]', "expect": "1 x 1"},
    {"name": "rs-ts-path", "source": "toString /a/b/c", "expect": "/a/b/c"},
    {"name": "rs-ts-attr-tostring", "source": 'toString { __toString = self: "hi-" + self.label; label = "x"; }', "expect": "hi-x"},
    {"name": "rs-ts-attr-outpath", "source": 'toString { outPath = "/nix/store/x"; }', "expect": "/nix/store/x"},
    {"name": "rs-ts-priority", "source": 'toString { __toString = _: "from-toString"; outPath = "from-outPath"; }', "expect": "from-toString"},
    {"name": "rs-ts-alias", "source": "builtins.toString 42", "expect": "42"},
    {"name": "rs-ts-field", "source": "builtins.toString { __toString = self: builtins.toString self.field; field = 99; }", "expect": "99"},
    {"name": "rs-ts-err-attrset", "source": "toString { a = 1; }", "error": True, "error_contains": "__toString"},
    {"name": "rs-ts-err-lambda", "source": "toString (x: x)", "error": True, "error_contains": "function"},
    {"name": "rs-ts-err-cycle-self", "source": "let r = { __toString = self: builtins.toString self; }; in builtins.toString r", "error": True, "error_contains": "cycle"},
    {"name": "rs-ts-err-cycle-outpath", "source": "let r = { outPath = builtins.toString r; }; in builtins.toString r", "error": True, "error_contains": "cycle"},
    {"name": "rs-tostring-list-context-p1", "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.toString [ "a${./p1}" "b${./p2}" ]))', "expect": True},
    {"name": "rs-tostring-list-context-p2", "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.toString [ "a${./p1}" "b${./p2}" ]))', "expect": True},
    {"name": "rs-tostring-list-text-context", "source": 'builtins.toString [ "a" "b" ]', "expect": "a b"},
    {"name": "rs-tostring-list-no-context", "source": 'builtins.hasContext (builtins.toString [ "a" "b" "c" ])', "expect": False},
    {"name": "rs-tostring-list-path-context", "source": 'builtins.hasAttr (builtins.toString ./somepath) (builtins.getContext (builtins.toString [ "prefix" ./somepath ]))', "expect": True},
    {"name": "rs-tostring-single-path-context", "source": "builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toString ./p))", "expect": True},
    {"name": "rs-tostring-list-mixed-p1", "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.toString [ "a${./p1}" ./p2 "c" ]))', "expect": True},
    {"name": "rs-tostring-list-mixed-p2", "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.toString [ "a${./p1}" ./p2 "c" ]))', "expect": True},
    {"name": "rs-tostring-method-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toString { __toString = self: "result-${./p}"; }))', "expect": True},
    {"name": "rs-tostring-outpath-context", "source": "builtins.hasAttr (builtins.toString ./foo) (builtins.getContext (builtins.toString { outPath = ./foo; }))", "expect": True},
    {"name": "rs-concatstrings-empty", "source": "builtins.concatStrings [ ]", "expect": ""},
    {"name": "rs-concatstrings-join", "source": 'builtins.concatStrings [ "a" "b" "c" ]', "expect": "abc"},
    {"name": "rs-concatstrings-context-p1", "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.concatStrings [ "a${./p1}" "b${./p2}" ]))', "expect": True},
    {"name": "rs-concatstrings-context-p2", "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.concatStrings [ "a${./p1}" "b${./p2}" ]))', "expect": True},
    {"name": "rs-concatstrings-no-context", "source": 'builtins.hasContext (builtins.concatStrings [ "a" "b" ])', "expect": False},
    {"name": "rs-match-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.match "(.+)" "x${./p}") 0))', "expect": True},
    {"name": "rs-match-no-match", "source": 'builtins.match "x" "y"', "expect": None},
    {"name": "rs-match-no-context", "source": 'builtins.hasContext (builtins.elemAt (builtins.match "(.+)" "abc") 0)', "expect": False},
    {"name": "rs-split-first-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.split "-" "x${./p}-y") 0))', "expect": True},
    {"name": "rs-split-tail-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.split "-" "x${./p}-y") 2))', "expect": True},
    {"name": "rs-split-no-context", "source": 'builtins.hasContext (builtins.elemAt (builtins.split "-" "x-y") 0)', "expect": False},
    {"name": "rs-context-plain", "source": 'builtins.hasContext "hello"', "expect": False},
    {"name": "rs-context-path-interp", "source": 'builtins.hasContext "x=${./foo.nix}"', "expect": True},
    {"name": "rs-context-plus", "source": 'builtins.hasContext ("prefix=" + "${./a.nix}")', "expect": True},
    {"name": "rs-context-discard", "source": 'let tagged = "v=${./tag.nix}"; bare = builtins.unsafeDiscardStringContext tagged; in [ (builtins.hasContext tagged), (builtins.hasContext bare) ]', "expect": [True, False]},
    {"name": "rs-context-getcontext-path", "source": 'builtins.hasAttr (builtins.toString ./marker.nix) (builtins.getContext "x=${./marker.nix}")', "expect": True},
    {"name": "rs-context-adddrv", "source": 'builtins.hasContext (builtins.addDrvOutputDependencies "raw")', "expect": True},
    {"name": "rs-context-discard-output", "source": 'let tagged = builtins.addDrvOutputDependencies "raw"; stripped = builtins.unsafeDiscardOutputDependency tagged; in [ (builtins.hasContext tagged), (builtins.hasContext stripped) ]', "expect": [True, False]},
    {"name": "rs-context-append-names", "source": 'builtins.attrNames (builtins.getContext (builtins.appendContext "raw" { "/extra/path" = { path = true; }; }))', "expect": ["/extra/path"]},
    {"name": "rs-context-interp-two", "source": 'builtins.length (builtins.attrNames (builtins.getContext "${./a.nix}-${./b.nix}"))', "expect": 2},
    {"name": "rs-context-concat-two", "source": 'let a = "x=${./a.nix}"; b = "y=${./b.nix}"; in builtins.length (builtins.attrNames (builtins.getContext (a + b)))', "expect": 2},
    {"name": "rs-context-typeof", "source": 'builtins.typeOf "x=${./a.nix}"', "expect": "string"},
    {"name": "rs-context-placeholder", "source": 'builtins.hasContext (builtins.placeholder "out")', "expect": True},
    {"name": "rs-context-derivation-outpath", "source": 'let d = builtins.derivationStrict { name = "hello"; builder = "/bin/sh"; system = "x86_64-linux"; }; in [ d.name, d.type, (builtins.hasContext d.outPath) ]', "expect": ["hello", "derivation", True]},
    {"name": "rs-concatstrings-err-non-list", "source": "builtins.concatStrings 42", "error": True, "error_contains": "concatStrings: argument must be list, got int"},
    {"name": "rs-concatstrings-err-int", "source": 'builtins.concatStrings [ "a" 1 "b" ]', "error": True, "error_contains": "int"},
    {"name": "rs-concatstrings-err-set", "source": 'builtins.concatStrings [ "a" { x = 1; } ]', "error": True, "error_contains": "set"},
    {"name": "rs-concatstrings-err-null", "source": 'builtins.concatStrings [ "a" null ]', "error": True, "error_contains": "null"},
    {"name": "rs-prop-concatsep-p1", "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.concatStringsSep "-" [ "a${./p1}" "b${./p2}" ]))', "expect": True},
    {"name": "rs-prop-concatsep-p2", "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.concatStringsSep "-" [ "a${./p1}" "b${./p2}" ]))', "expect": True},
    {"name": "rs-prop-concatsep-sep", "source": 'builtins.hasAttr (builtins.toString ./sep) (builtins.getContext (builtins.concatStringsSep "${./sep}" [ "a" "b" ]))', "expect": True},
    {"name": "rs-prop-concatsep-noctx", "source": 'builtins.hasContext (builtins.concatStringsSep "-" [ "a" "b" "c" ])', "expect": False},
    {"name": "rs-prop-substring-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.substring 0 5 "x${./p}y"))', "expect": True},
    {"name": "rs-prop-substring-noctx", "source": 'builtins.hasContext (builtins.substring 0 3 "abc")', "expect": False},
    {"name": "rs-prop-replace-haystack", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.replaceStrings [ "x" ] [ "y" ] "x${./p}"))', "expect": True},
    {"name": "rs-prop-replace-to", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.replaceStrings [ "x" ] [ "${./p}" ] "x"))', "expect": True},
    {"name": "rs-prop-replace-unused", "source": 'builtins.hasContext (builtins.replaceStrings [ "z" ] [ "${./p}" ] "abc")', "expect": False},
    {"name": "rs-prop-replace-both-to", "source": 'builtins.hasAttr (builtins.toString ./pTo) (builtins.getContext (builtins.replaceStrings [ "x" ] [ "${./pTo}" ] "x${./pHay}"))', "expect": True},
    {"name": "rs-prop-replace-both-hay", "source": 'builtins.hasAttr (builtins.toString ./pHay) (builtins.getContext (builtins.replaceStrings [ "x" ] [ "${./pTo}" ] "x${./pHay}"))', "expect": True},
    {"name": "rs-prop-tostring-str", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toString "x${./p}"))', "expect": True},
    {"name": "rs-prop-tostring-int", "source": "builtins.hasContext (builtins.toString 42)", "expect": False},
    {"name": "rs-tojson-string-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toJSON "x${./p}"))', "expect": True},
    {"name": "rs-tojson-attr-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toJSON { a = "x${./p}"; }))', "expect": True},
    {"name": "rs-tojson-list-p1", "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.toJSON [ "x${./p1}" "y${./p2}" ]))', "expect": True},
    {"name": "rs-tojson-list-p2", "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.toJSON [ "x${./p1}" "y${./p2}" ]))', "expect": True},
    {"name": "rs-tojson-path-context", "source": "builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toJSON ./p))", "expect": True},
    {"name": "rs-tojson-noctx", "source": "builtins.hasContext (builtins.toJSON [ 1 2 3 ])", "expect": False},
    {"name": "rs-tojson-type-context-string", "source": 'builtins.typeOf (builtins.toJSON [ "x${./p}" ])', "expect": "string"},
    {"name": "rs-tojson-value-list", "source": "builtins.toJSON [ 1 2 3 ]", "expect": "[1,2,3]"},
    {"name": "rs-tojson-chain-concat", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext ("prefix:" + (builtins.toJSON [ "x${./p}" ])))', "expect": True},
    {"name": "rs-path-pathExists-context", "source": 'builtins.pathExists "x${./p}/non-existent-dir-xyz"', "expect": False},
    {"name": "rs-path-pathExists-plain", "source": 'builtins.pathExists "/non-existent-dir-xyz"', "expect": False},
    {"name": "rs-path-pathExists-path", "source": "builtins.pathExists ./non-existent-xyz", "expect": False},
    {"name": "rs-path-toPath-context", "source": 'builtins.typeOf (builtins.toPath "${/tmp}")', "expect": "string"},
    {"name": "rs-path-storePath-context", "source": 'builtins.typeOf (builtins.storePath "${./local}")', "expect": "path"},
    {"name": "rs-tofile-literal", "source": 'builtins.typeOf (builtins.toFile "name" "literal content")', "expect": "path"},
    {"name": "rs-tofile-empty", "source": 'builtins.typeOf (builtins.toFile "empty" "")', "expect": "path"},
    {"name": "rs-tofile-discard", "source": 'builtins.typeOf (builtins.toFile "n" (builtins.unsafeDiscardStringContext "x${./p}"))', "expect": "path"},
    {"name": "rs-unsafe-add-dep-exists", "source": "builtins.typeOf builtins.unsafeAddOutputDependency", "expect": "lambda"},
    {"name": "rs-unsafe-add-dep-plain", "source": 'builtins.unsafeAddOutputDependency "x"', "expect": "x"},
    {"name": "rs-unsafe-add-dep-marker", "source": 'builtins.hasAttr ("!out!" + (builtins.toString ./p)) (builtins.getContext (builtins.unsafeAddOutputDependency "x${./p}"))', "expect": True},
    {"name": "rs-unsafe-add-dep-idempotent", "source": 'builtins.length (builtins.attrNames (builtins.getContext (builtins.unsafeAddOutputDependency (builtins.unsafeAddOutputDependency "x${./p}"))))', "expect": 2},
    {"name": "rs-unsafe-add-name-exists", "source": "builtins.typeOf builtins.unsafeAddOutputName", "expect": "lambda"},
    {"name": "rs-unsafe-add-name-partial", "source": 'builtins.typeOf (builtins.unsafeAddOutputName "out")', "expect": "lambda"},
    {"name": "rs-unsafe-add-name-plain", "source": 'builtins.unsafeAddOutputName "out" "x"', "expect": "x"},
    {"name": "rs-unsafe-add-name-marker", "source": 'builtins.hasAttr ("!dev!" + (builtins.toString ./p)) (builtins.getContext (builtins.unsafeAddOutputName "dev" "x${./p}"))', "expect": True},
    {"name": "rs-unsafe-add-discard", "source": 'let v = builtins.unsafeDiscardOutputDependency (builtins.unsafeAddOutputDependency "x${./p}"); in builtins.hasAttr ("!out!" + (builtins.toString ./p)) (builtins.getContext v)', "expect": False},
    {"name": "rs-tofile-err-context", "source": 'builtins.toFile "name" "x${./p}"', "error": True, "error_contains": "context"},
    {"name": "rs-tofile-err-concat-context", "source": 'builtins.toFile "name" ("text-" + ./script)', "error": True, "error_contains": "context"},
    {"name": "rs-tofile-err-name-int", "source": 'builtins.toFile 42 "content"', "error": True, "error_contains": "int"},
    {"name": "rs-tofile-err-content-int", "source": 'builtins.toFile "n" 42', "error": True, "error_contains": "int"},
    {"name": "rs-pathExists-err-int", "source": "builtins.pathExists 42", "error": True, "error_contains": "expected path or string"},
    {"name": "rs-readFile-err-null", "source": "builtins.readFile null", "error": True, "error_contains": "expected path or string"},
    {"name": "rs-readDir-err-list", "source": "builtins.readDir [ ]", "error": True, "error_contains": "expected path or string"},
    {"name": "rs-unsafe-add-dep-err-int", "source": "builtins.unsafeAddOutputDependency 42", "error": True, "error_contains": "int"},
    {"name": "rs-unsafe-add-dep-err-null", "source": "builtins.unsafeAddOutputDependency null", "error": True, "error_contains": "null"},
    {"name": "rs-unsafe-add-name-err-int-name", "source": 'builtins.unsafeAddOutputName 42 "x"', "error": True, "error_contains": "int"},
    {"name": "rs-unsafe-add-name-err-int-value", "source": 'builtins.unsafeAddOutputName "out" 42', "error": True, "error_contains": "int"},
    {"name": "rs-ctx-eq-same", "source": '"x${./p}" == "x${./p}"', "expect": True},
    {"name": "rs-ctx-eq-diff", "source": '"x${./a}" == "x${./b}"', "expect": False},
    {"name": "rs-ctx-eq-plain-constructed", "source": '"abc" == ("a" + "b" + "c")', "expect": True},
    {"name": "rs-ctx-eq-list-same", "source": '[ "x${./p}" ] == [ "x${./p}" ]', "expect": True},
    {"name": "rs-ctx-eq-list-diff", "source": '[ "x${./a}" ] == [ "x${./b}" ]', "expect": False},
    {"name": "rs-ctx-eq-nested-list", "source": '[ [ "x${./p}" ] ] == [ [ "x${./p}" ] ]', "expect": True},
    {"name": "rs-ctx-eq-attr-same", "source": '{ a = "x${./p}"; } == { a = "x${./p}"; }', "expect": True},
    {"name": "rs-ctx-eq-attr-diff", "source": '{ a = "x${./a}"; } == { a = "x${./b}"; }', "expect": False},
    {"name": "rs-ctx-neq-same", "source": '"x${./p}" != "x${./p}"', "expect": False},
    {"name": "rs-ctx-eq-branch-empty-interp", "source": 'let str = "expected${""}"; in if str == "expected" then 1 else 0', "expect": 1},
    {"name": "rs-ctx-eq-branch-path", "source": 'let str = "x${./p}"; expected = "x${./p}"; in if str == expected then "matched" else "miss"', "expect": "matched"},
    {"name": "rs-ctx-cmp-lt-two-bool", "source": 'builtins.typeOf ("x${./a}" < "x${./b}")', "expect": "bool"},
    {"name": "rs-ctx-cmp-lt-context-plain-bool", "source": 'builtins.typeOf ("x${./a}" < "z")', "expect": "bool"},
    {"name": "rs-ctx-cmp-lt-plain-context-bool", "source": 'builtins.typeOf ("a" < "x${./p}")', "expect": "bool"},
    {"name": "rs-ctx-cmp-lt-plain", "source": '"abc" < "abd"', "expect": True},
    {"name": "rs-ctx-cmp-le-same", "source": '"x${./a}" <= "x${./a}"', "expect": True},
    {"name": "rs-ctx-cmp-gt-context-plain-bool", "source": 'builtins.typeOf ("z${./p}" > "a")', "expect": "bool"},
    {"name": "rs-ctx-cmp-ge-same", "source": '"x${./a}" >= "x${./a}"', "expect": True},
    {"name": "rs-ctx-lessThan-context-bool", "source": 'builtins.typeOf (builtins.lessThan "x${./a}" "y")', "expect": "bool"},
    {"name": "rs-ctx-lessThan-two-context-bool", "source": 'builtins.typeOf (builtins.lessThan "x${./a}" "x${./b}")', "expect": "bool"},
    {"name": "rs-ctx-sort-context-length", "source": 'builtins.length (builtins.sort (a: b: a < b) [ "b${./bp}" "a${./ap}" "c${./cp}" ])', "expect": 3},
    {"name": "rs-ctx-builtin-lt-context-bool", "source": 'builtins.typeOf (builtins.lt "x${./a}" "y")', "expect": "bool"},
    {"name": "rs-ctx-builtin-le-context", "source": 'builtins.le "x${./a}" "x${./a}"', "expect": True},
    {"name": "rs-ctx-builtin-gt-context-bool", "source": 'builtins.typeOf (builtins.gt "z${./p}" "a")', "expect": "bool"},
    {"name": "rs-ctx-builtin-ge-context", "source": 'builtins.ge "x${./a}" "x${./a}"', "expect": True},
    {"name": "rs-ctx-cmp-err-int-string", "source": '1 < "a"', "error": True, "error_contains": "cannot compare"},
    {"name": "rs-ctx-cmp-err-attrset", "source": "{ a = 1; } < { a = 2; }", "error": True, "error_contains": "cannot compare"},
    {"name": "rs-ctx-cmp-err-lambda", "source": "(x: x) < (y: y)", "error": True, "error_contains": "cannot compare"},
    {"name": "rs-ctx-getEnv-plain", "source": 'builtins.getEnv "DEFINITELY_NOT_SET_XYZ_SLICE71"', "expect": ""},
    {"name": "rs-ctx-getEnv-context", "source": 'builtins.getEnv "DEFINITELY_NOT_SET_${./marker}"', "expect": ""},
    {"name": "rs-ctx-getEnv-err-int", "source": "builtins.getEnv 42", "error": True, "error_contains": "int"},
    {"name": "rs-ctx-getEnv-err-null", "source": "builtins.getEnv null", "error": True, "error_contains": "null"},
    {"name": "rs-ctx-xmlParse-plain", "source": 'builtins.typeOf (builtins.xmlParse "<a/>")', "expect": "set"},
    {"name": "rs-ctx-xmlParse-context", "source": 'builtins.typeOf (builtins.xmlParse "<a>${./marker}</a>")', "expect": "set"},
    {"name": "rs-ctx-xmlParse-err-int", "source": "builtins.xmlParse 42", "error": True, "error_contains": "int"},
    {"name": "rs-ctx-htmlParse-plain", "source": 'builtins.typeOf (builtins.htmlParse "<p>hi</p>")', "expect": "set"},
    {"name": "rs-ctx-htmlParse-context", "source": 'builtins.typeOf (builtins.htmlParse "<p>${./body}</p>")', "expect": "set"},
    {"name": "rs-ctx-htmlParse-err-null", "source": "builtins.htmlParse null", "error": True, "error_contains": "null"},
    {"name": "rs-ctx-isString-context", "source": 'builtins.isString "x${./p}"', "expect": True},
    {"name": "rs-ctx-typeOf-context", "source": 'builtins.typeOf "x${./p}"', "expect": "string"},
    {"name": "rs-ctx-xmlEmit-attr", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p}"; }; children = []; }))', "expect": True},
    {"name": "rs-ctx-xmlEmit-text", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = {}; children = [ { kind = "text"; value = "x${./p}"; } ]; }))', "expect": True},
    {"name": "rs-ctx-xmlEmit-noctx", "source": 'builtins.hasContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = {}; children = []; })', "expect": False},
    {"name": "rs-ctx-xmlEmit-type", "source": 'builtins.typeOf (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p}"; }; children = []; })', "expect": "string"},
    {"name": "rs-ctx-xmlEmit-text-unchanged", "source": 'builtins.xmlEmit { kind = "element"; name = "a"; attrs = {}; children = []; }', "expect": "<a/>"},
    {"name": "rs-ctx-htmlEmit-text", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.htmlEmit { kind = "element"; name = "p"; attrs = {}; children = [ { kind = "text"; value = "x${./p}"; } ]; }))', "expect": True},
    {"name": "rs-ctx-htmlEmit-noctx", "source": 'builtins.hasContext (builtins.htmlEmit { kind = "element"; name = "p"; attrs = {}; children = []; })', "expect": False},
    {"name": "rs-ctx-xmlEmit-union-p1", "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p1}"; class = "y${./p2}"; }; children = [ { kind = "text"; value = "z${./p3}"; } ]; }))', "expect": True},
    {"name": "rs-ctx-xmlEmit-union-p2", "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p1}"; class = "y${./p2}"; }; children = [ { kind = "text"; value = "z${./p3}"; } ]; }))', "expect": True},
    {"name": "rs-ctx-xmlEmit-union-p3", "source": 'builtins.hasAttr (builtins.toString ./p3) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p1}"; class = "y${./p2}"; }; children = [ { kind = "text"; value = "z${./p3}"; } ]; }))', "expect": True},
    {"name": "rs-ctx-xmlEmit-path-string-attr", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { src = builtins.toString ./p; }; children = []; }))', "expect": True},
    {"name": "rs-ctx-xmlEmit-concat-keeps", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext ("<?xml?>" + (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p}"; }; children = []; })))', "expect": True},
    {"name": "rs-builtin-import-exists", "source": "builtins.typeOf builtins.import", "expect": "lambda"},
    {"name": "rs-builtin-scopedImport-exists", "source": "builtins.typeOf builtins.scopedImport", "expect": "lambda"},
    {"name": "rs-builtin-fold-sum", "source": "builtins.fold (acc: x: acc + x) 0 [ 1 2 3 4 ]", "expect": 10},
    {"name": "rs-builtin-fold-alias", "source": "fold (acc: x: acc + x) 0 [ 1 2 3 ]", "expect": 6},
    {"name": "rs-list-head-empty-err", "source": "builtins.head []", "error": True, "error_contains": "list is empty"},
    {"name": "rs-list-tail-empty-err", "source": "builtins.tail []", "error": True, "error_contains": "list is empty"},
    {"name": "rs-list-head-singleton", "source": "builtins.head [ 7 ]", "expect": 7},
    {"name": "rs-list-tail-singleton", "source": "builtins.tail [ 7 ]", "expect": []},
    {"name": "rs-list-elemat-first", "source": "builtins.elemAt [ 10 20 30 ] 0", "expect": 10},
    {"name": "rs-list-elemat-last", "source": "builtins.elemAt [ 10 20 30 ] 2", "expect": 30},
    {"name": "rs-list-elemat-neg-err", "source": "builtins.elemAt [ 10 20 30 ] (-1)", "error": True, "error_contains": "negative index"},
    {"name": "rs-list-elemat-oob-err", "source": "builtins.elemAt [ 10 20 30 ] 5", "error": True, "error_contains": "out of bounds"},
    {"name": "rs-list-elemat-len-err", "source": "builtins.elemAt [ 10 20 30 ] 3", "error": True, "error_contains": "out of bounds"},
    {"name": "rs-list-genlist-zero", "source": "builtins.genList (i: i) 0", "expect": []},
    {"name": "rs-list-genlist-three", "source": "builtins.genList (i: i * i) 3", "expect": [0, 1, 4]},
    {"name": "rs-list-genlist-neg-err", "source": "builtins.genList (i: i) (-1)", "error": True, "error_contains": "negative"},
    {"name": "rs-list-take-zero", "source": "builtins.take 0 [ 1 2 3 ]", "expect": []},
    {"name": "rs-list-take-over", "source": "builtins.take 5 [ 1 2 3 ]", "expect": [1, 2, 3]},
    {"name": "rs-list-take-neg-err", "source": "builtins.take (-1) [ 1 2 3 ]", "error": True, "error_contains": "negative"},
    {"name": "rs-list-drop-zero", "source": "builtins.drop 0 [ 1 2 3 ]", "expect": [1, 2, 3]},
    {"name": "rs-list-drop-over", "source": "builtins.drop 5 [ 1 2 3 ]", "expect": []},
    {"name": "rs-list-drop-neg-err", "source": "builtins.drop (-1) [ 1 2 3 ]", "error": True, "error_contains": "negative"},
    {"name": "rs-list-length-empty", "source": "builtins.length []", "expect": 0},
    {"name": "rs-list-length-three", "source": "builtins.length [ 1 2 3 ]", "expect": 3},
    {"name": "rs-list-elem-hit", "source": "builtins.elem 1 [ 1 2 3 ]", "expect": True},
    {"name": "rs-list-elem-miss", "source": "builtins.elem 4 [ 1 2 3 ]", "expect": False},
    {"name": "rs-list-elem-empty", "source": "builtins.elem 1 [ ]", "expect": False},
    {"name": "rs-list-elem-string", "source": 'builtins.elem "a" [ "a" "b" ]', "expect": True},
    {"name": "rs-list-elem-int-err", "source": "builtins.elem 1 42", "error": True, "error_contains": "list"},
    {"name": "rs-list-elem-null-err", "source": "builtins.elem 1 null", "error": True, "error_contains": "list"},
    {"name": "rs-list-elem-string-err", "source": 'builtins.elem 1 "abc"', "error": True, "error_contains": "list"},
    {"name": "rs-list-elem-set-err", "source": "builtins.elem 1 { a = 1; }", "error": True, "error_contains": "list"},
    {"name": "rs-list-filter-gt", "source": "builtins.filter (x: x > 1) [ 1 2 3 ]", "expect": [2, 3]},
    {"name": "rs-list-filter-false", "source": "builtins.filter (x: false) [ 1 2 3 ]", "expect": []},
    {"name": "rs-list-filter-true", "source": "builtins.filter (x: true) [ 1 2 3 ]", "expect": [1, 2, 3]},
    {"name": "rs-list-filter-empty", "source": "builtins.filter (x: x > 0) []", "expect": []},
    {"name": "rs-list-filter-int-pred-err", "source": "builtins.filter (x: x) [ 1 2 3 ]", "error": True, "error_contains": "predicate"},
    {"name": "rs-list-filter-const-int-err", "source": "builtins.filter (x: 42) [ 1 ]", "error": True, "error_contains": "predicate"},
    {"name": "rs-list-filter-string-err", "source": 'builtins.filter (x: "yes") [ 1 ]', "error": True, "error_contains": "predicate"},
    {"name": "rs-list-filter-nonlist-err", "source": "builtins.filter (x: true) 42", "error": True, "error_contains": "list"},
    {"name": "rs-list-filter-index-err", "source": "builtins.filter (x: if x == 1 then true else x) [ 1 2 ]", "error": True, "error_contains": "index 1"},
    {"name": "rs-list-toattrs-unique", "source": 'builtins.listToAttrs [ { name = "a"; value = 1; } { name = "b"; value = 2; } ]', "expect": {"a": 1, "b": 2}},
    {"name": "rs-list-toattrs-dup-first", "source": 'builtins.listToAttrs [ { name = "a"; value = 1; } { name = "a"; value = 2; } ]', "expect": {"a": 1}},
    {"name": "rs-list-toattrs-dup-three", "source": 'builtins.listToAttrs [ { name = "x"; value = 10; } { name = "x"; value = 20; } { name = "x"; value = 30; } ]', "expect": {"x": 10}},
    {"name": "rs-list-toattrs-lazy-loser", "source": 'builtins.listToAttrs [ { name = "k"; value = 1; } { name = "k"; value = throw "x"; } ]', "expect": {"k": 1}},
    {"name": "rs-seq-int", "source": "builtins.seq 1 2", "expect": 2},
    {"name": "rs-seq-null", "source": "builtins.seq null 2", "expect": 2},
    {"name": "rs-seq-list", "source": "builtins.seq [ 1 2 3 ] 99", "expect": 99},
    {"name": "rs-seq-throw-top-err", "source": 'builtins.seq (throw "top") 1', "error": True, "error_contains": "top"},
    {"name": "rs-seq-attr-lazy", "source": 'builtins.seq { a = throw "inner"; } 1', "expect": 1},
    {"name": "rs-seq-list-lazy", "source": 'builtins.seq [ (throw "inner") ] 1', "expect": 1},
    {"name": "rs-seq-nested-lazy", "source": 'builtins.seq { a = { b = throw "deep"; }; } 1', "expect": 1},
    {"name": "rs-seq-let-lazy", "source": 'let x = { a = throw "x"; }; in builtins.seq x 1', "expect": 1},
    {"name": "rs-seq-second-throw-err", "source": 'builtins.seq 1 (throw "second")', "error": True, "error_contains": "second"},
    {"name": "rs-deepseq-null", "source": "builtins.deepSeq null 1", "expect": 1},
    {"name": "rs-deepseq-int", "source": "builtins.deepSeq 42 1", "expect": 1},
    {"name": "rs-deepseq-attrs", "source": "builtins.deepSeq { a = 1; b = 2; } 99", "expect": 99},
    {"name": "rs-any-true", "source": "builtins.any (x: x > 0) [ 1 2 3 ]", "expect": True},
    {"name": "rs-any-false", "source": "builtins.any (x: x > 5) [ 1 2 3 ]", "expect": False},
    {"name": "rs-any-empty-truepred", "source": "builtins.any (x: true) []", "expect": False},
    {"name": "rs-any-empty", "source": "builtins.any (x: x > 0) []", "expect": False},
    {"name": "rs-any-int-pred-err", "source": "builtins.any (x: x) [ 1 ]", "error": True, "error_contains": "predicate"},
    {"name": "rs-any-const-int-err", "source": "builtins.any (x: 42) [ 1 ]", "error": True, "error_contains": "predicate"},
    {"name": "rs-any-index-err", "source": "builtins.any (x: if x == 1 then false else x) [ 1 2 ]", "error": True, "error_contains": "index 1"},
    {"name": "rs-any-nonlist-err", "source": "builtins.any (x: x > 0) 42", "error": True, "error_contains": "list"},
    {"name": "rs-any-short-circuit", "source": 'builtins.any (x: x > 0) [ 1 (throw "later") ]', "expect": True},
    {"name": "rs-all-true", "source": "builtins.all (x: x > 0) [ 1 2 3 ]", "expect": True},
    {"name": "rs-all-false", "source": "builtins.all (x: x > 1) [ 1 2 3 ]", "expect": False},
    {"name": "rs-all-empty-falsepred", "source": "builtins.all (x: false) []", "expect": True},
    {"name": "rs-all-empty", "source": "builtins.all (x: x > 0) []", "expect": True},
    {"name": "rs-all-int-pred-err", "source": "builtins.all (x: 42) [ 1 ]", "error": True, "error_contains": "predicate"},
    {"name": "rs-all-index-err", "source": "builtins.all (x: if x == 1 then true else x) [ 1 2 ]", "error": True, "error_contains": "index 1"},
    {"name": "rs-all-nonlist-err", "source": "builtins.all (x: x > 0) 42", "error": True, "error_contains": "list"},
    {"name": "rs-all-short-circuit", "source": 'builtins.all (x: x > 0) [ 0 (throw "later") ]', "expect": False},
    {"name": "rs-length-list", "source": "builtins.length [ 1 2 3 ]", "expect": 3},
    {"name": "rs-length-empty-list", "source": "builtins.length []", "expect": 0},
    {"name": "rs-length-string", "source": 'builtins.length "abc"', "expect": 3},
    {"name": "rs-length-utf8", "source": 'builtins.length "héllo"', "expect": 6},
    {"name": "rs-length-int-err", "source": "builtins.length 42", "error": True, "error_contains": "expected list or string"},
    {"name": "rs-length-null-err", "source": "builtins.length null", "error": True, "error_contains": "expected list or string"},
    {"name": "rs-length-bool-err", "source": "builtins.length true", "error": True, "error_contains": "expected list or string"},
    {"name": "rs-length-float-err", "source": "builtins.length 1.5", "error": True, "error_contains": "expected list or string"},
    {"name": "rs-length-set-err", "source": "builtins.length { a = 1; b = 2; }", "error": True, "error_contains": "expected list or string"},
    {"name": "rs-foldr-empty", "source": "builtins.foldr (a: b: a + b) 0 []", "expect": 0},
    {"name": "rs-foldr-sum", "source": "builtins.foldr (a: b: a + b) 0 [ 1 2 3 ]", "expect": 6},
    {"name": "rs-foldr-minus", "source": "builtins.foldr (a: b: a - b) 0 [ 1 2 3 4 ]", "expect": -2},
    {"name": "rs-foldl-minus", "source": "builtins.foldl' (a: b: a - b) 0 [ 1 2 3 4 ]", "expect": -10},
    {"name": "rs-foldr-string", "source": 'builtins.foldr (a: b: a + b) "" [ "a" "b" "c" ]', "expect": "abc"},
    {"name": "rs-foldr-build-list", "source": "builtins.foldr (x: acc: [ x ] ++ acc) [] [ 1 2 3 ]", "expect": [1, 2, 3]},
    {"name": "rs-foldr-lazy-empty", "source": 'let r = builtins.foldr (a: b: a + b) (throw "x") []; in 42', "expect": 42},
    {"name": "rs-foldr-step-throw-err", "source": 'builtins.foldr (a: b: throw "step") 0 [ 1 ]', "error": True, "error_contains": "step"},
    {"name": "rs-foldr-nonlist-err", "source": "builtins.foldr (a: b: a) 0 42", "error": True, "error_contains": "list"},
    {"name": "rs-foldr-string-err", "source": 'builtins.foldr (a: b: a) 0 "abc"', "error": True, "error_contains": "list"},
    {"name": "rs-foldl-nonlist-err", "source": "builtins.foldl' (a: b: a) 0 42", "error": True, "error_contains": "list"},
    {"name": "rs-foldl-lazy-empty", "source": 'let r = builtins.foldl\' (a: b: a + b) (throw "x") []; in 42', "expect": 42},
    {"name": "rs-foldr-arg-order", "source": 'builtins.foldr (item: acc: { i = item; a = acc; }) "Z" [ "X" ]', "expect": {"a": "Z", "i": "X"}},
    {"name": "rs-foldl-arg-order", "source": 'builtins.foldl\' (acc: item: { a = acc; i = item; }) "Z" [ "X" ]', "expect": {"a": "Z", "i": "X"}},
    {"name": "rs-lazy-map-length", "source": 'builtins.length (builtins.map (x: throw "x") [ 1 2 3 ])', "expect": 3},
    {"name": "rs-lazy-map-head", "source": 'builtins.head (builtins.map (x: x + 1) [ 1 (throw "x") 3 ])', "expect": 2},
    {"name": "rs-lazy-genlist-length", "source": 'builtins.length (builtins.genList (i: throw "x") 5)', "expect": 5},
    {"name": "rs-lazy-genlist-head", "source": 'builtins.head (builtins.genList (i: if i == 0 then 99 else throw "x") 10)', "expect": 99},
    {"name": "rs-lazy-concatlists-length", "source": 'builtins.length (builtins.concatLists [ [ 1 ] [ (throw "x") ] ])', "expect": 2},
    {"name": "rs-lazy-concatlists-head", "source": 'builtins.head (builtins.concatLists [ [ 1 ] [ (throw "x") ] ])', "expect": 1},
    {"name": "rs-lazy-attrvalues-length", "source": 'builtins.length (builtins.attrValues { a = 1; b = throw "x"; })', "expect": 2},
    {"name": "rs-lazy-values-length", "source": 'builtins.length (builtins.values { a = 1; b = throw "x"; })', "expect": 2},
    {"name": "rs-lazy-mapattrs-names", "source": 'builtins.length (builtins.attrNames (builtins.mapAttrs (k: v: throw "x") { a = 1; b = 2; }))', "expect": 2},
    {"name": "rs-lazy-catattrs-length", "source": 'builtins.length (builtins.catAttrs "a" [ { a = 1; } { a = throw "x"; } ])', "expect": 2},
    {"name": "rs-lazy-zip-names", "source": 'builtins.length (builtins.attrNames (builtins.zipAttrsWith (k: vs: throw "x") [ { a = 1; } { a = 2; b = 3; } ]))', "expect": 2},
    {"name": "rs-lazy-zip-hasattr", "source": 'let r = builtins.zipAttrsWith (k: vs: throw "x") [ { a = 1; } { a = 2; } ]; in r ? a', "expect": True},
    {"name": "rs-lazy-attrbypath-default", "source": 'builtins.attrByPath [ "a" ] (throw "default-fired") { a = 42; }', "expect": 42},
    {"name": "rs-hasattr-throw-value", "source": '{ a = throw "x"; } ? a', "expect": True},
    {"name": "rs-hasattr-path-throw-value", "source": '{ a.b = throw "x"; } ? a.b', "expect": True},
    {"name": "rs-guard-catattrs-nonlist", "source": 'builtins.catAttrs "a" 42', "error": True, "error_contains": "list"},
    {"name": "rs-guard-zipattrs-nonset", "source": 'builtins.zipAttrsWith (k: vs: vs) [ 42 ]', "error": True, "error_contains": "attrset"},
    {"name": "rs-guard-groupby-nonlist", "source": 'builtins.groupBy (x: "k") 42', "error": True, "error_contains": "list"},
    {"name": "rs-guard-getattr-missing", "source": 'builtins.getAttr "z" { a = 1; }', "error": True, "error_contains": "missing"},
    {"name": "rs-guard-mapattrs-nonset", "source": 'builtins.mapAttrs (k: v: v) 42', "error": True, "error_contains": "attrset"},
    {"name": "rs-guard-values-nonset", "source": 'builtins.values 42', "error": True, "error_contains": "attrset"},
    {"name": "rs-guard-attrbypath-nonlist", "source": 'builtins.attrByPath 42 0 { a = 1; }', "error": True, "error_contains": "list"},
    # eval_type_handling.rs remainder (arith/compare/equality not already vendored)
    {"name": "rs-th-int-plus-int", "source": "1 + 2", "expect": 3},
    {"name": "rs-th-int-div-int", "source": "10 / 3", "expect": 3},
    {"name": "rs-th-arith-chain", "source": "10 - (10 / 3) * 3", "expect": 1},
    {"name": "rs-th-cmp-ge-false", "source": "3 >= 4", "expect": False},
    {"name": "rs-th-cmp-int-float", "source": "1 < 1.5", "expect": True},
    {"name": "rs-th-eq-int-float-val", "source": "1 == 1.0", "expect": True},
    {"name": "rs-th-eq-int-str-false", "source": '1 == "1"', "expect": False},
    {"name": "rs-th-eq-nested-attr", "source": "{a={b=1;};} == {a={b=1;};}", "expect": True},
    {"name": "rs-th-list-paren-eq", "source": "[ (1 + 1) ] == [ 2 ]", "expect": True},
    {"name": "rs-th-listplus-paren", "source": "[ (1+1) ] ++ [ (2*3) ]", "expect": [2, 6]},
    {"name": "rs-th-bool-and", "source": "true && false", "expect": False},
    {"name": "rs-th-bool-not", "source": "!true", "expect": False},
    {"name": "rs-th-fn-eq-false", "source": "(x: x) == (y: y)", "expect": False},
    # eval_lambda_attrset_corners.rs — currying + listToAttrs/mapAttrs/filterAttrs guards
    {"name": "rs-lc-curry3", "source": "(a: b: c: a + b + c) 1 2 3", "expect": 6},
    {"name": "rs-lc-curry-partial-type", "source": "builtins.typeOf ((a: b: c: a + b + c) 1)", "expect": "lambda"},
    {"name": "rs-lc-closure-capture", "source": "let x = 10; f = y: x + y; in f 5", "expect": 15},
    {"name": "rs-lc-shadow-param", "source": "let x = 10; f = x: x + 1; in f 100", "expect": 101},
    {"name": "rs-lc-mapattrs-empty", "source": "builtins.mapAttrs (k: v: v) { }", "expect": {}},
    {"name": "rs-lc-filterattrs-empty", "source": "builtins.filterAttrs (k: v: true) { }", "expect": {}},
    {"name": "rs-lc-err-apply-nonfn", "source": "let f = 1; in f 2", "error": True, "error_contains": "function"},
    {"name": "rs-lc-err-listtoattrs-missing-value", "source": 'builtins.listToAttrs [ { name = "a"; } ]', "error": True, "error_contains": "value"},
    {"name": "rs-lc-err-listtoattrs-missing-name", "source": 'builtins.listToAttrs [ { value = 1; } ]', "error": True, "error_contains": "name"},
    {"name": "rs-lc-err-listtoattrs-nonset", "source": "builtins.listToAttrs [ 42 ]", "error": True, "error_contains": "attrset"},
    {"name": "rs-lc-err-mapattrs-nonset", "source": "builtins.mapAttrs (k: v: v) 42", "error": True, "error_contains": "attrset"},
    {"name": "rs-lc-err-filterattrs-nonbool", "source": 'builtins.filterAttrs (k: v: 1) { a = 1; }', "error": True, "error_contains": "bool"},
    # eval_with_lazy.rs + eval_with_priority.rs — with-chain laziness + binding priority
    {"name": "rs-wl-lazy-unused", "source": 'with (throw "boom"); 1', "expect": 1},
    {"name": "rs-wl-lazy-let", "source": 'let x = 42; in with (throw "boom"); x', "expect": 42},
    {"name": "rs-wl-merge", "source": "with { a = 1; b = 2; c = 3; }; a + b + c", "expect": 6},
    {"name": "rs-wl-lazy-outer", "source": 'with (throw "outer"); with { x = 7; }; x', "expect": 7},
    {"name": "rs-wl-int-unused", "source": "let x = 99; in with 42; x", "expect": 99},
    {"name": "rs-wl-err-consulted", "source": 'with (throw "boom"); y', "error": True, "error_contains": "boom"},
    {"name": "rs-wp-let-wins", "source": "let x = 1; in with { x = 2; }; x", "expect": 1},
    {"name": "rs-wp-lambda-wins", "source": "with { x = 2; }; (x: x) 7", "expect": 7},
    {"name": "rs-wp-basic", "source": "with { x = 2; }; x", "expect": 2},
    {"name": "rs-wp-inner-wins", "source": "with { x = 1; }; with { x = 2; }; x", "expect": 2},
    {"name": "rs-wp-two-a", "source": "with { a = 1; }; with { b = 2; }; a", "expect": 1},
    {"name": "rs-wp-let-over-with", "source": "with { x = 2; }; let x = 1; in x", "expect": 1},
    # eval_nixpkgs_lib_patterns.rs — nixpkgs/lib functional patterns (fix/override/extensible)
    {"name": "rs-nixlib-fix-counter", "source": "let fix = f: let x = f x; in x; counter = self: { value = 0; inc = self // { value = self.value + 1; }; }; c0 = fix counter; in c0.value", "expect": 0},
    {"name": "rs-nixlib-fix-factorial", "source": "let fix = f: let x = f x; in x; fact = self: n: if n <= 1 then 1 else n * self (n - 1); f = fix fact; in f 5", "expect": 120},
    {"name": "rs-nixlib-optional", "source": "let optional = cond: x: if cond then [ x ] else [ ]; in [ (builtins.length (optional true 1)) (builtins.length (optional false 2)) ]", "expect": [1, 0]},
    {"name": "rs-nixlib-optionalattrs", "source": "let optionalAttrs = cond: as: if cond then as else { }; in [ (builtins.length (builtins.attrNames (optionalAttrs true { x = 1; y = 2; }))) (builtins.length (builtins.attrNames (optionalAttrs false { x = 1; }))) ]", "expect": [2, 0]},
    {"name": "rs-nixlib-recupdate-outer", "source": "let recursiveUpdate = lhs: rhs: lhs // rhs; in builtins.attrNames (recursiveUpdate { a = 1; b = { x = 1; }; } { b = { y = 2; }; c = 3; })", "expect": ["a", "b", "c"]},
    {"name": "rs-nixlib-make-overridable", "source": 'let makeOverridable = f: origArgs: let result = f origArgs; overrideWith = newArgs: makeOverridable f (origArgs // (if builtins.isFunction newArgs then newArgs origArgs else newArgs)); in result // { override = overrideWith; }; hello = makeOverridable (a: { greeting = "hello, ${a.name}"; }) { name = "world"; }; in [ hello.greeting (hello.override { name = "pnix"; }).greeting ]', "expect": ["hello, world", "hello, pnix"]},
    {"name": "rs-nixlib-make-extensible", "source": "let fix = f: let x = f x; in x; makeExtensible = f: let self = f self // { extend = ext: makeExtensible (self_: f self_ // ext self_ (f self_)); }; in self; base = makeExtensible (self: { a = 1; b = 2; sum = self.a + self.b; }); ext = base.extend (self: super: { c = 3; sum = super.sum + self.c; }); in [ base.sum ext.sum ext.c ]", "expect": [3, 6, 3]},
    {"name": "rs-nixlib-genattrs", "source": 'let genAttrs = names: f: builtins.listToAttrs (builtins.map (name: { inherit name; value = f name; }) names); in (genAttrs [ "a" "b" "c" ] (n: "v_" + n)).c', "expect": "v_c"},
    {"name": "rs-nixlib-namevaluepair", "source": 'let nameValuePair = name: value: { inherit name value; }; pairs = builtins.map (n: nameValuePair n (n + n)) [ "a" "b" ]; in (builtins.listToAttrs pairs).a', "expect": "aa"},
    {"name": "rs-nixlib-foldattrs", "source": "let foldAttrs = op: nul: list_of_attrs: builtins.foldl' (acc: as: builtins.foldl' (acc2: name: acc2 // { ${name} = op (as.${name}) (acc2.${name} or nul); }) acc (builtins.attrNames as)) {} list_of_attrs; in (foldAttrs (item: acc: acc + item) 0 [ { x = 1; y = 10; } { x = 2; y = 20; } { x = 3; } ]).x", "expect": 6},
    {"name": "rs-nixlib-mapattrstolist", "source": 'let mapAttrsToList = f: attrs: builtins.map (name: f name (attrs.${name})) (builtins.attrNames attrs); in builtins.length (mapAttrsToList (n: v: n + "=" + (builtins.toString v)) { a = 1; b = 2; c = 3; })', "expect": 3},
    {"name": "rs-nixlib-mkoverride", "source": 'let mkOverride = priority: content: { _type = "override"; inherit priority content; }; mkDefault = mkOverride 1000; d = mkDefault "hello"; in [ d.content d.priority ]', "expect": ["hello", 1000]},
    {"name": "rs-nixlib-compose-extensions", "source": "let composeExtensions = f: g: final: prev: let r = f final prev; in g final (prev // r) // r; base = self: { a = 1; b = self.a * 2; }; addC = self: super: { c = self.a + 100; }; addD = self: super: { d = self.c + super.b; }; extension = composeExtensions addC addD; fix = f: let x = f x; in x; mk = self: let init = base self; in init // (extension self init); result = fix mk; in [ result.a result.b result.c result.d ]", "expect": [1, 2, 101, 103]},
    {"name": "rs-nixlib-hasprefix", "source": 'let hasPrefix = pref: str: builtins.substring 0 (builtins.stringLength pref) str == pref; in [ (hasPrefix "he" "hello") (hasPrefix "x" "hello") ]', "expect": [True, False]},
    # eval_force_cycle.rs / eval_cyclic_value_guards.rs / eval_interp_cycle_guard.rs
    {"name": "rs-fc-self-attr", "source": "let s = { x = s.x; }; in s.x", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-fc-rec-ab", "source": "(rec { a = b; b = a; }).a", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-fc-rec-xx", "source": "(rec { x = x; }).x", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-fc-lazy-x-unused", "source": "let x = x; in 1", "expect": 1},
    {"name": "rs-fc-deep-rec-30", "source": "let f = n: if n == 0 then 0 else f (n - 1) + 1; in f 30", "expect": 30},
    {"name": "rs-fc-x-plus-x", "source": "let x = 1; in x + x", "expect": 2},
    {"name": "rs-fc-tojson-cycle", "source": "let s = { x = s; }; in builtins.toJSON s", "error": True, "error_contains": "toJSON"},
    {"name": "rs-fc-tojson-cycle-recursion", "source": "let s = { x = s; }; in builtins.toJSON s", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-fc-deepseq-cycle", "source": "let s = { x = s; }; in builtins.deepSeq s 1", "error": True, "error_contains": "deepSeq"},
    {"name": "rs-fc-interp-cycle", "source": 'let r = { __toString = self: "${self}"; }; in "${r}"', "error": True, "error_contains": "interpolation coercion cycle"},
    {"name": "rs-fc-interp-cycle-tostring", "source": 'let r = { __toString = self: "${self}"; }; in "${r}"', "error": True, "error_contains": "__toString"},
    # eval_bool_required_positions.rs -- if-condition and boolop operands must be
    # bool; message form "<position>: expected bool, got <type>" with positions
    # "if condition" / "left|right operand of &&|||->" (vendored 2026-06-30).
    {"name": "rs-bp-if-string", "source": 'if "x" then 1 else 2', "error": True, "error_contains": "if condition: expected bool, got string"},
    {"name": "rs-bp-if-int", "source": "if 5 then 1 else 2", "error": True, "error_contains": "if condition: expected bool, got int"},
    {"name": "rs-bp-if-null", "source": "if null then 1 else 2", "error": True, "error_contains": "if condition: expected bool, got null"},
    {"name": "rs-bp-if-list", "source": "if [1] then 1 else 2", "error": True, "error_contains": "if condition: expected bool, got list"},
    {"name": "rs-bp-and-left-string", "source": '"s" && true', "error": True, "error_contains": "left operand of &&: expected bool, got string"},
    {"name": "rs-bp-and-left-int", "source": "5 && true", "error": True, "error_contains": "left operand of &&: expected bool, got int"},
    {"name": "rs-bp-and-right-int", "source": "true && 5", "error": True, "error_contains": "right operand of &&: expected bool, got int"},
    {"name": "rs-bp-or-left-int", "source": "1 || true", "error": True, "error_contains": "left operand of ||: expected bool, got int"},
    {"name": "rs-bp-or-right-int", "source": "false || 5", "error": True, "error_contains": "right operand of ||: expected bool, got int"},
    {"name": "rs-bp-impl-left-int", "source": "1 -> true", "error": True, "error_contains": "left operand of ->: expected bool, got int"},
    {"name": "rs-bp-impl-right-int", "source": "true -> 5", "error": True, "error_contains": "right operand of ->: expected bool, got int"},
    # eval_attr_concat_guards.rs -- hasAttr/removeAttrs/concatLists are hard type
    # assertions (no more silent-false / silent-empty); message form
    # "builtins.<fn>: <position> must be <type>, got <type>" with indexed element
    # checks. Lazy elements are forced before the check (vendored 2026-06-30).
    {"name": "rs-ac-hasattr-value-int", "source": 'builtins.hasAttr "a" 42', "error": True, "error_contains": "second argument must be attrset, got int"},
    {"name": "rs-ac-hasattr-value-null", "source": 'builtins.hasAttr "a" null', "error": True, "error_contains": "second argument must be attrset, got null"},
    {"name": "rs-ac-hasattr-name-int", "source": "builtins.hasAttr 42 { a = 1; }", "error": True, "error_contains": "first argument must be string, got int"},
    {"name": "rs-ac-hasattr-happy", "source": 'builtins.hasAttr "a" { a = 1; b = 2; }', "expect": True},
    {"name": "rs-ac-hasattr-happy-missing", "source": 'builtins.hasAttr "x" { a = 1; }', "expect": False},
    {"name": "rs-ac-rmattrs-first-int", "source": 'builtins.removeAttrs 42 [ "x" ]', "error": True, "error_contains": "first argument must be attrset, got int"},
    {"name": "rs-ac-rmattrs-second-int", "source": "builtins.removeAttrs { a = 1; } 42", "error": True, "error_contains": "second argument must be list of strings, got int"},
    {"name": "rs-ac-rmattrs-name-int", "source": "builtins.removeAttrs { a = 1; } [ 42 ]", "error": True, "error_contains": "name-list element at index 0 is not a string, got int"},
    {"name": "rs-ac-rmattrs-thunk-name", "source": 'builtins.removeAttrs { a = 1; b = 2; } [ ("b" + "") ]', "expect": {"a": 1}},
    {"name": "rs-ac-concatlists-arg-int", "source": "builtins.concatLists 42", "error": True, "error_contains": "argument must be list, got int"},
    {"name": "rs-ac-concatlists-elem-int", "source": "builtins.concatLists [1 2]", "error": True, "error_contains": "element at index 0 is not a list, got int"},
    {"name": "rs-ac-concatlists-elem-partial", "source": "builtins.concatLists [[1 2] 42 [3]]", "error": True, "error_contains": "element at index 1 is not a list"},
    {"name": "rs-ac-concatlists-thunk-list", "source": "builtins.concatLists [ (if true then [1] else 0) [2] ]", "expect": [1, 2]},
    # eval_abort_with_string_guards.rs -- abort is string-only (mirrors throw):
    # "builtins.abort: argument must be string, got <value>"; the "evaluation
    # aborted: " marker is only emitted after the type check passes (so tryEval
    # still re-raises a valid string abort but catches the type error). `with`
    # non-attrset source is a force-time error "with: argument must be attrset,
    # got <type>" -- raised only when a body lookup falls through to the
    # with-frame (laziness preserved) (vendored 2026-06-30).
    {"name": "rs-aw-abort-string", "source": 'builtins.abort "boom"', "error": True, "error_contains": "evaluation aborted: boom"},
    {"name": "rs-aw-abort-int", "source": "builtins.abort 42", "error": True, "error_contains": "builtins.abort: argument must be string, got 42"},
    {"name": "rs-aw-abort-global-int", "source": "abort 42", "error": True, "error_contains": "builtins.abort: argument must be string"},
    {"name": "rs-aw-abort-global-string", "source": 'abort "via-global"', "error": True, "error_contains": "evaluation aborted: via-global"},
    {"name": "rs-aw-abort-tryeval-prop", "source": 'builtins.tryEval (builtins.abort "stop")', "error": True, "error_contains": "evaluation aborted: stop"},
    {"name": "rs-aw-abort-tryeval-typecheck", "source": "(builtins.tryEval (builtins.abort 42)).success", "error": True, "error_contains": "argument must be string"},
    {"name": "rs-aw-with-int-foo", "source": "with 42; foo", "error": True, "error_contains": "with: argument must be attrset, got int"},
    {"name": "rs-aw-with-string-foo", "source": 'with "hello"; foo', "error": True, "error_contains": "with: argument must be attrset, got string"},
    {"name": "rs-aw-with-list-foo", "source": "with [ 1 2 ]; foo", "error": True, "error_contains": "with: argument must be attrset, got list"},
    {"name": "rs-aw-with-null-foo", "source": "with null; foo", "error": True, "error_contains": "with: argument must be attrset, got null"},
    {"name": "rs-aw-with-throw-src", "source": 'with (throw "src-boom"); foo', "error": True, "error_contains": "src-boom"},
    {"name": "rs-aw-with-attrset-ok", "source": "with { a = 1; b = 2; }; a + b", "expect": 3},
    {"name": "rs-aw-with-inner-wins", "source": "with 42; with { x = 99; }; x", "expect": 99},
    {"name": "rs-aw-with-int-unused", "source": "with 42; 1", "expect": 1},
    {"name": "rs-aw-with-lexical-wins", "source": "with 42; let x = 99; in x", "expect": 99},
    # eval_replace_strings_list_guards.rs -- replaceStrings non-list from/to was
    # silently coerced to [] (silent no-op); now a typed error "'from'|'to' must be
    # list, got <type>" (from checked first). The length-mismatch arm stays distinct
    # ("equal length", not a type error) so callers can tell count-wrong from
    # type-wrong; haystack non-string says "third argument"; from-element non-string
    # says "'from' element must be string" (vendored 2026-06-30).
    {"name": "rs-rsl-clean", "source": 'builtins.replaceStrings [ "a" "b" ] [ "X" "Y" ] "abc"', "expect": "XYc"},
    {"name": "rs-rsl-empty", "source": 'builtins.replaceStrings [] [] "abc"', "expect": "abc"},
    {"name": "rs-rsl-from-int", "source": 'builtins.replaceStrings 42 [ "X" ] "abc"', "error": True, "error_contains": "'from' must be list, got int"},
    {"name": "rs-rsl-to-string", "source": 'builtins.replaceStrings [ "a" ] "X" "abc"', "error": True, "error_contains": "'to' must be list, got string"},
    {"name": "rs-rsl-both-int", "source": 'builtins.replaceStrings 42 99 "abc"', "error": True, "error_contains": "'from' must be list, got int"},
    {"name": "rs-rsl-both-string", "source": 'builtins.replaceStrings "from" "to" "abc"', "error": True, "error_contains": "'from' must be list, got string"},
    {"name": "rs-rsl-both-null", "source": 'builtins.replaceStrings null null "abc"', "error": True, "error_contains": "'from' must be list, got null"},
    {"name": "rs-rsl-from-attrset", "source": 'builtins.replaceStrings { a = 1; } [ "X" ] "abc"', "error": True, "error_contains": "'from' must be list, got set"},
    {"name": "rs-rsl-length-mismatch", "source": 'builtins.replaceStrings [ "a" ] [ "X" "Y" ] "abc"', "error": True, "error_contains": "equal length"},
    {"name": "rs-rsl-haystack-int", "source": 'builtins.replaceStrings [ "a" ] [ "X" ] 42', "error": True, "error_contains": "third argument"},
    {"name": "rs-rsl-from-elem-int", "source": 'builtins.replaceStrings [ 1 ] [ "X" ] "abc"', "error": True, "error_contains": "'from' element must be string"},
    # eval_fold_groupby_guards.rs -- the pnix-only `fold` was the holdout that
    # silently returned `init` for any non-list third arg (foldl'/foldr already
    # errored); now "builtins.fold: third arg must be list, got <type>". groupBy
    # accepts context-bearing key-fn returns (string with context) and names the
    # actual returned type on failure: "key function must return string, got
    # <type>"; non-list second arg errors too (vendored 2026-06-30).
    {"name": "rs-fg-fold-sum", "source": "builtins.fold (a: b: a + b) 100 [ 1 2 3 ]", "expect": 106},
    {"name": "rs-fg-fold-empty", "source": "builtins.fold (a: b: a + b) 100 [ ]", "expect": 100},
    {"name": "rs-fg-fold-int", "source": "builtins.fold (a: b: a + b) 100 42", "error": True, "error_contains": "fold: third arg must be list, got int"},
    {"name": "rs-fg-fold-null", "source": "builtins.fold (a: b: a + b) 100 null", "error": True, "error_contains": "fold: third arg must be list, got null"},
    {"name": "rs-fg-fold-attrset", "source": "builtins.fold (a: b: a + b) 100 { x = 1; }", "error": True, "error_contains": "fold: third arg must be list, got set"},
    {"name": "rs-fg-fold-string", "source": 'builtins.fold (a: b: a + b) 100 "hi"', "error": True, "error_contains": "fold: third arg must be list, got string"},
    {"name": "rs-fg-groupby-plain", "source": 'builtins.groupBy (x: if x < 5 then "lo" else "hi") [ 1 6 2 7 ]', "expect": {"hi": [6, 7], "lo": [1, 2]}},
    {"name": "rs-fg-groupby-empty", "source": 'builtins.groupBy (x: "k") [ ]', "expect": {}},
    {"name": "rs-fg-groupby-context-key", "source": 'builtins.length (builtins.attrNames (builtins.groupBy (item: "a${./p}") [ "x" "y" ]))', "expect": 1},
    {"name": "rs-fg-groupby-key-int", "source": 'builtins.groupBy (item: 42) [ "a" ]', "error": True, "error_contains": "groupBy: key function must return string, got int"},
    {"name": "rs-fg-groupby-key-null", "source": 'builtins.groupBy (item: null) [ "a" ]', "error": True, "error_contains": "groupBy: key function must return string, got null"},
    {"name": "rs-fg-groupby-key-list", "source": 'builtins.groupBy (item: [ ]) [ "a" ]', "error": True, "error_contains": "groupBy: key function must return string, got list"},
    {"name": "rs-fg-groupby-nonlist", "source": 'builtins.groupBy (x: "k") 42', "error": True, "error_contains": "groupBy: second argument must be list, got int"},
    # eval_seq_any_all.rs / eval_filter_elem_listtoattrs.rs / eval_length_foldr.rs --
    # list-consuming builtins now name the offending argument + type instead of the
    # generic "X list must be a list": any/all/elem/filter -> "second argument must
    # be list, got <type>"; foldl'/foldl/foldr -> "third arg must be list, got
    # <type>" (matching the earlier fold change) (vendored 2026-06-30).
    {"name": "rs-la-any-nonlist", "source": "builtins.any (x: x > 0) 42", "error": True, "error_contains": "builtins.any: second argument must be list, got int"},
    {"name": "rs-la-all-nonlist", "source": "builtins.all (x: x > 0) 42", "error": True, "error_contains": "builtins.all: second argument must be list, got int"},
    {"name": "rs-la-elem-int", "source": "builtins.elem 1 42", "error": True, "error_contains": "builtins.elem: second argument must be list, got int"},
    {"name": "rs-la-elem-null", "source": "builtins.elem 1 null", "error": True, "error_contains": "second argument must be list, got null"},
    {"name": "rs-la-elem-string", "source": 'builtins.elem 1 "abc"', "error": True, "error_contains": "second argument must be list, got string"},
    {"name": "rs-la-elem-attrset", "source": "builtins.elem 1 { a = 1; }", "error": True, "error_contains": "second argument must be list, got set"},
    {"name": "rs-la-filter-nonlist", "source": "builtins.filter (x: true) 42", "error": True, "error_contains": "builtins.filter: second argument must be list, got int"},
    {"name": "rs-la-foldr-int", "source": "builtins.foldr (a: b: a) 0 42", "error": True, "error_contains": "builtins.foldr: third arg must be list, got int"},
    {"name": "rs-la-foldr-string", "source": 'builtins.foldr (a: b: a) 0 "abc"', "error": True, "error_contains": "third arg must be list, got string"},
    {"name": "rs-la-foldl-prime-int", "source": "builtins.foldl' (a: b: a) 0 42", "error": True, "error_contains": "builtins.foldl': third arg must be list, got int"},
    {"name": "rs-la-foldl-int", "source": "builtins.foldl (a: b: a) 0 42", "error": True, "error_contains": "builtins.foldl: third arg must be list, got int"},
    {"name": "rs-la-any-happy", "source": "builtins.any (x: x > 2) [ 1 2 3 ]", "expect": True},
    {"name": "rs-la-all-happy", "source": "builtins.all (x: x > 1) [ 1 2 3 ]", "expect": False},
    {"name": "rs-la-elem-happy", "source": "builtins.elem 2 [ 1 2 3 ]", "expect": True},
    {"name": "rs-la-filter-happy", "source": "builtins.filter (x: x > 1) [ 1 2 3 ]", "expect": [2, 3]},
    {"name": "rs-la-foldr-happy", "source": "builtins.foldr (a: b: a + b) 0 [ 1 2 3 ]", "expect": 6},
    {"name": "rs-la-any-short-circuit", "source": 'builtins.any (x: x > 0) [ 1 (throw "later") ]', "expect": True},
    # eval_introspection_folds.rs / eval_zipattrswith_lazy_guard.rs -- attrset- and
    # function-consuming builtins name the offending type: functionArgs on a
    # non-function errors ("expected function"); attrNames/attrValues/getAttr on a
    # non-attrset say "expected attrset, got <type>"; getAttr/getAttrs missing-key
    # name the attr in single quotes; zipAttrsWith validates its list + elements
    # (vendored 2026-06-30).
    {"name": "rs-ai-functionargs-int", "source": "builtins.functionArgs 42", "error": True, "error_contains": "builtins.functionArgs: expected function"},
    {"name": "rs-ai-functionargs-happy", "source": "builtins.functionArgs ({ a, b ? 1 }: a)", "expect": {"a": False, "b": True}},
    {"name": "rs-ai-functionargs-lambda", "source": "builtins.functionArgs (x: x)", "expect": {}},
    {"name": "rs-ai-attrnames-list", "source": "builtins.attrNames [ 1 2 ]", "error": True, "error_contains": "builtins.attrNames: expected attrset, got list"},
    {"name": "rs-ai-attrnames-string", "source": 'builtins.attrNames "hello"', "error": True, "error_contains": "attrNames: expected attrset, got string"},
    {"name": "rs-ai-attrvalues-list", "source": "builtins.attrValues [ 1 2 ]", "error": True, "error_contains": "builtins.attrValues: expected attrset, got list"},
    {"name": "rs-ai-attrvalues-int", "source": "builtins.attrValues 42", "error": True, "error_contains": "attrValues: expected attrset, got int"},
    {"name": "rs-ai-attrnames-happy", "source": "builtins.attrNames { b = 1; a = 2; }", "expect": ["a", "b"]},
    {"name": "rs-ai-getattr-missing", "source": 'builtins.getAttr "z" { a = 1; b = 2; }', "error": True, "error_contains": "builtins.getAttr: attribute 'z' missing"},
    {"name": "rs-ai-getattr-nonattrset", "source": 'builtins.getAttr "a" 42', "error": True, "error_contains": "builtins.getAttr: expected attrset, got int"},
    {"name": "rs-ai-getattr-happy", "source": 'builtins.getAttr "a" { a = 42; }', "expect": 42},
    {"name": "rs-ai-getattrs-missing", "source": 'builtins.getAttrs [ "z" ] { a = 1; }', "error": True, "error_contains": "builtins.getAttrs: attribute 'z' missing"},
    {"name": "rs-ai-getattrs-happy", "source": 'builtins.getAttrs [ "a" ] { a = 1; b = 2; }', "expect": {"a": 1}},
    {"name": "rs-ai-zip-nonlist", "source": "builtins.zipAttrsWith (k: vs: vs) 42", "error": True, "error_contains": "builtins.zipAttrsWith: second argument must be list, got int"},
    {"name": "rs-ai-zip-elem-int", "source": "builtins.zipAttrsWith (k: vs: vs) [ { a = 1; } 42 ]", "error": True, "error_contains": "builtins.zipAttrsWith: list element must be attrset, got int"},
    {"name": "rs-ai-zip-happy", "source": "builtins.zipAttrsWith (k: vs: vs) [ { a = 1; } { a = 2; } ]", "expect": {"a": [1, 2]}},
    # eval_addcontext_pos_bitops_guards.rs -- addErrorContext non-string context
    # ("context must be string, got <type>") and bitAnd/bitOr/bitXor name the
    # offending side + type ("first|second arg must be int, got <type>"). The
    # context-bearing message + lazy value behaviour is unchanged (vendored 2026-06-30).
    {"name": "rs-bx-addctx-int", "source": 'builtins.addErrorContext 42 "value"', "error": True, "error_contains": "addErrorContext: context must be string, got int"},
    {"name": "rs-bx-addctx-null", "source": 'builtins.addErrorContext null "value"', "error": True, "error_contains": "addErrorContext: context must be string, got null"},
    {"name": "rs-bx-addctx-attrset", "source": 'builtins.addErrorContext { x = 1; } "value"', "error": True, "error_contains": "addErrorContext: context must be string, got set"},
    {"name": "rs-bx-addctx-happy", "source": 'builtins.addErrorContext "ctx" 42', "expect": 42},
    {"name": "rs-bx-bitand-first-string", "source": 'builtins.bitAnd "x" 5', "error": True, "error_contains": "bitAnd: first arg must be int, got string"},
    {"name": "rs-bx-bitand-second-float", "source": "builtins.bitAnd 5 3.0", "error": True, "error_contains": "bitAnd: second arg must be int, got float"},
    {"name": "rs-bx-bitor-first-null", "source": "builtins.bitOr null 5", "error": True, "error_contains": "bitOr: first arg must be int, got null"},
    {"name": "rs-bx-bitor-second-list", "source": "builtins.bitOr 5 [ ]", "error": True, "error_contains": "bitOr: second arg must be int, got list"},
    {"name": "rs-bx-bitxor-first-attrset", "source": "builtins.bitXor { x = 1; } 5", "error": True, "error_contains": "bitXor: first arg must be int, got set"},
    {"name": "rs-bx-bitxor-second-string", "source": 'builtins.bitXor 5 "y"', "error": True, "error_contains": "bitXor: second arg must be int, got string"},
    {"name": "rs-bx-bitand-happy", "source": "builtins.bitAnd 12 10", "expect": 8},
    {"name": "rs-bx-bitor-happy", "source": "builtins.bitOr 12 10", "expect": 14},
    {"name": "rs-bx-bitxor-happy", "source": "builtins.bitXor 12 10", "expect": 6},
    # eval_appendcontext_value_shape_guard.rs -- each per-path context value must be
    # an attrset; its path/allOutputs must be bool, outputs must be a list of strings
    # (indexed element check). Messages name the path key and offending type
    # (vendored 2026-06-30). NOTE: appendContext path/allOutputs say "must be bool"
    # (this file), distinct from the generic boolop "expected bool" wording.
    {"name": "rs-acs-value-string", "source": 'builtins.appendContext "x" { "/a" = "wrong-shape"; }', "error": True, "error_contains": "context value for '/a' must be an attrset, got string"},
    {"name": "rs-acs-value-int", "source": 'builtins.appendContext "x" { "/a" = 42; }', "error": True, "error_contains": "context value for '/a' must be an attrset, got int"},
    {"name": "rs-acs-value-null", "source": 'builtins.appendContext "x" { "/a" = null; }', "error": True, "error_contains": "context value for '/a' must be an attrset, got null"},
    {"name": "rs-acs-value-list", "source": 'builtins.appendContext "x" { "/a" = [ ]; }', "error": True, "error_contains": "context value for '/a' must be an attrset, got list"},
    {"name": "rs-acs-outputs-nonlist", "source": 'builtins.appendContext "x" { "/a" = { outputs = "wrong"; }; }', "error": True, "error_contains": "'/a'.outputs must be list of strings, got string"},
    {"name": "rs-acs-outputs-elem-int", "source": 'builtins.appendContext "x" { "/a" = { outputs = [ "out" 42 ]; }; }', "error": True, "error_contains": "'/a'.outputs element at index 1 is not a string, got int"},
    {"name": "rs-acs-path-int", "source": 'builtins.appendContext "x" { "/a" = { path = 1; }; }', "error": True, "error_contains": "'/a'.path must be bool, got int"},
    {"name": "rs-acs-path-string", "source": 'builtins.appendContext "x" { "/a" = { path = "true"; }; }', "error": True, "error_contains": "'/a'.path must be bool, got string"},
    {"name": "rs-acs-alloutputs-int", "source": 'builtins.appendContext "x" { "/a" = { allOutputs = 1; }; }', "error": True, "error_contains": "'/a'.allOutputs must be bool, got int"},
    {"name": "rs-acs-outputs-empty-ok", "source": 'builtins.hasContext (builtins.appendContext "x" { "/a" = { outputs = [ ]; }; })', "expect": True},
    {"name": "rs-acs-outputs-strings-ok", "source": 'builtins.hasContext (builtins.appendContext "x" { "/a" = { outputs = [ "out" "dev" ]; }; })', "expect": True},
    {"name": "rs-acs-alloutputs-bool-ok", "source": 'builtins.hasContext (builtins.appendContext "x" { "/a" = { allOutputs = true; }; })', "expect": True},
    {"name": "rs-acs-path-bool-ok", "source": 'builtins.hasContext (builtins.appendContext "x" { "/a" = { path = true; }; })', "expect": True},
    # eval_i64_min_overflow_guards.rs -- pnix ints are i64 with CHECKED overflow.
    # i64::MIN (built via 0 - i64::MAX - 1) % -1, -(i64::MIN), builtins.neg(i64::MIN),
    # builtins.sub(i64::MIN, 1) all overflow -> "integer overflow" (pnix-hy uses
    # arbitrary-precision Python ints, so these are simulated). builtins.mod by zero
    # is "builtins.mod: division by zero" (binary % stays "modulo by zero")
    # (vendored 2026-06-30).
    {"name": "rs-ov-i64min-build", "source": "let big = 9223372036854775807; in 0 - big - 1", "expect": -9223372036854775808},
    {"name": "rs-ov-mod-min-neg1", "source": "let big = 9223372036854775807; m = 0 - big - 1; in builtins.mod m (-1)", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-ov-binmod-min-neg1", "source": "let big = 9223372036854775807; m = 0 - big - 1; in m % (-1)", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-ov-unaryneg-min", "source": "let big = 9223372036854775807; m = 0 - big - 1; in -m", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-ov-neg-min", "source": "let big = 9223372036854775807; m = 0 - big - 1; in builtins.neg m", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-ov-sub-min-1", "source": "let big = 9223372036854775807; m = 0 - big - 1; in builtins.sub m 1", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-ov-mod-zero", "source": "builtins.mod 1 0", "error": True, "error_contains": "builtins.mod: division by zero"},
    {"name": "rs-ov-mod-normal", "source": "builtins.mod 10 3", "expect": 1},
    {"name": "rs-ov-mod-neg-divisor", "source": "builtins.mod 10 (-3)", "expect": 1},
    {"name": "rs-ov-mod-negmax-neg1", "source": "let big = 9223372036854775807; m = 0 - big; in builtins.mod m (-1)", "expect": 0},
    {"name": "rs-ov-neg-max", "source": "builtins.neg 9223372036854775807", "expect": -9223372036854775807},
    {"name": "rs-ov-double-neg-max", "source": "-(-9223372036854775807)", "expect": 9223372036854775807},
    {"name": "rs-ov-neg-float", "source": "builtins.neg 1.5", "expect": -1.5},
    # eval_concat_match_split_context.rs / eval_string_ops.rs -- concatStrings names
    # the argument + type on a non-list ("concatStrings: argument must be list, got
    # <type>"); concatStringsSep names a non-string separator ("separator must be
    # string, got <type>"). Element-index checks were already correct (vendored
    # 2026-06-30).
    {"name": "rs-cs-concatstrings-nonlist", "source": "builtins.concatStrings 42", "error": True, "error_contains": "concatStrings: argument must be list, got int"},
    {"name": "rs-cs-concatstrings-elem-int", "source": 'builtins.concatStrings [ "a" 1 "b" ]', "error": True, "error_contains": "concatStrings element at index 1"},
    {"name": "rs-cs-concatstrings-elem-null", "source": 'builtins.concatStrings [ "a" null ]', "error": True, "error_contains": "element at index 1"},
    {"name": "rs-cs-concatstrings-happy", "source": 'builtins.concatStrings [ "a" "b" "c" ]', "expect": "abc"},
    {"name": "rs-cs-concatstrings-empty", "source": "builtins.concatStrings [ ]", "expect": ""},
    {"name": "rs-cs-sep-nonstring", "source": 'builtins.concatStringsSep 42 [ "a" "b" ]', "error": True, "error_contains": "concatStringsSep: separator must be string, got int"},
    {"name": "rs-cs-sep-elem-int", "source": 'builtins.concatStringsSep "," [ "a" 1 "c" ]', "error": True, "error_contains": "index 1"},
    {"name": "rs-cs-sep-happy", "source": 'builtins.concatStringsSep "," [ "a" "b" ]', "expect": "a,b"},
    {"name": "rs-cs-sep-empty", "source": 'builtins.concatStringsSep "," [ ]', "expect": ""},
    # eval_unsafe_add_output_builtins.rs -- the output-dependency string builtins now
    # name the offending type: unsafeAddOutputDependency/addDrvOutputDependencies/
    # unsafeDiscardOutputDependency say "expected string, got <type>";
    # unsafeAddOutputName names "first arg"/"second arg" + "must be string"
    # (vendored 2026-06-30).
    {"name": "rs-uo-adddep-int", "source": "builtins.unsafeAddOutputDependency 42", "error": True, "error_contains": "unsafeAddOutputDependency: expected string, got int"},
    {"name": "rs-uo-adddep-null", "source": "builtins.unsafeAddOutputDependency null", "error": True, "error_contains": "unsafeAddOutputDependency: expected string, got null"},
    {"name": "rs-uo-addname-first-int", "source": 'builtins.unsafeAddOutputName 42 "x"', "error": True, "error_contains": "unsafeAddOutputName: first arg must be string, got int"},
    {"name": "rs-uo-addname-second-int", "source": 'builtins.unsafeAddOutputName "out" 42', "error": True, "error_contains": "unsafeAddOutputName: second arg must be string, got int"},
    {"name": "rs-uo-adddrv-int", "source": "builtins.addDrvOutputDependencies 42", "error": True, "error_contains": "addDrvOutputDependencies: expected string, got int"},
    {"name": "rs-uo-adddep-happy", "source": 'builtins.unsafeAddOutputDependency "x"', "expect": "x"},
    {"name": "rs-uo-addname-happy", "source": 'builtins.unsafeAddOutputName "out" "x"', "expect": "x"},
    # eval_tostring_cycle_guard.rs / eval_derivation_builtin.rs -- already pass
    # (lock-in only; the depth/cycle guards and derivation attrset guard were in
    # place). toString/interp coercion cycles error with "...cycle..." across both
    # lanes; derivation on a non-attrset errors naming "derivation"+"attrset"
    # (vendored 2026-06-30).
    {"name": "rs-tc-self-ref", "source": "let r = { __toString = self: builtins.toString self; }; in builtins.toString r", "error": True, "error_contains": "cycle"},
    {"name": "rs-tc-alternating", "source": "let a = { __toString = self: builtins.toString b; }; b = { __toString = self: builtins.toString a; }; in builtins.toString a", "error": True, "error_contains": "cycle"},
    {"name": "rs-tc-outpath-cycle", "source": "let r = { outPath = builtins.toString r; }; in builtins.toString r", "error": True, "error_contains": "cycle"},
    {"name": "rs-tc-then-interp", "source": 'let r = { __toString = self: "wrapped:${self}"; }; in builtins.toString r', "error": True, "error_contains": "cycle"},
    {"name": "rs-tc-interp-then", "source": 'let r = { __toString = self: "${builtins.toString self}"; }; in "${r}"', "error": True, "error_contains": "cycle"},
    {"name": "rs-tc-outpath-string-ok", "source": 'builtins.toString { outPath = "/some/path"; }', "expect": "/some/path"},
    {"name": "rs-dv-nonattrset-int", "source": "builtins.derivation 42", "error": True, "error_contains": "attrset"},
    {"name": "rs-dv-nonattrset-string", "source": 'builtins.derivation "name"', "error": True, "error_contains": "derivation"},
    # eval_string_context_param_parity.rs / eval_tofile_context_guard.rs -- string-arg
    # builtins name the type: getEnv/xmlParse/htmlParse say "expected string, got
    # <type>"; toFile names "first argument"/"second argument" + "must be string" and
    # its context error points at unsafeDiscardStringContext. Context-bearing string
    # args are still accepted (vendored 2026-06-30).
    {"name": "rs-sa-getenv-int", "source": "builtins.getEnv 42", "error": True, "error_contains": "getEnv: expected string, got int"},
    {"name": "rs-sa-getenv-null", "source": "builtins.getEnv null", "error": True, "error_contains": "getEnv: expected string, got null"},
    {"name": "rs-sa-xmlparse-int", "source": "builtins.xmlParse 42", "error": True, "error_contains": "xmlParse: expected string, got int"},
    {"name": "rs-sa-htmlparse-null", "source": "builtins.htmlParse null", "error": True, "error_contains": "htmlParse: expected string, got null"},
    {"name": "rs-sa-tofile-name-int", "source": 'builtins.toFile 42 "content"', "error": True, "error_contains": "toFile: first argument must be string, got int"},
    {"name": "rs-sa-tofile-content-int", "source": 'builtins.toFile "n" 42', "error": True, "error_contains": "toFile: second argument must be string, got int"},
    {"name": "rs-sa-tofile-context", "source": 'builtins.toFile "name" "x${./p}"', "error": True, "error_contains": "unsafeDiscardStringContext"},
    {"name": "rs-sa-getenv-happy", "source": 'builtins.getEnv "PNIX_NOPE"', "expect": ""},
    {"name": "rs-sa-tofile-happy", "source": 'builtins.isString (builtins.toString (builtins.toFile "g" "hello"))', "expect": True},
    # eval_path_string_concat_context.rs -- arith operators on non-numeric operands
    # (where no +-overload like string/list/path concat applies) error with
    # "operator <op>: unsupported operand types <tl> and <tr>" (matches ~/pnix's
    # binary-op fallthrough). bool is not numeric. Overloads (str/list/attrset/path)
    # still work (vendored 2026-06-30).
    {"name": "rs-op-int-plus-string", "source": '42 + "hi"', "error": True, "error_contains": "operator +: unsupported operand types int and string"},
    {"name": "rs-op-null-plus-int", "source": "null + 1", "error": True, "error_contains": "operator +: unsupported operand types null and int"},
    {"name": "rs-op-string-plus-int", "source": '"x" + 1', "error": True, "error_contains": "operator +: unsupported operand types string and int"},
    {"name": "rs-op-int-minus-string", "source": '1 - "x"', "error": True, "error_contains": "operator -: unsupported operand types int and string"},
    {"name": "rs-op-list-div-int", "source": "[1] / 2", "error": True, "error_contains": "operator /: unsupported operand types list and int"},
    {"name": "rs-op-bool-plus-int", "source": "true + 1", "error": True, "error_contains": "operator +: unsupported operand types bool and int"},
    {"name": "rs-op-plus-int-ok", "source": "1 + 2", "expect": 3},
    {"name": "rs-op-plus-string-ok", "source": '"a" + "b"', "expect": "ab"},
    {"name": "rs-op-plus-list-ok", "source": "[1] + [2]", "expect": [1, 2]},
    {"name": "rs-op-plus-attrset-ok", "source": "{ a = 1; } + { b = 2; }", "expect": {"a": 1, "b": 2}},
    # eval_update_path_arith.rs -- toJSON of a non-finite float names which kind:
    # "cannot serialize float +inf|-inf|NaN as JSON" (vendored 2026-06-30).
    {"name": "rs-tj-posinf", "source": "builtins.toJSON (1.0e308 * 10.0)", "error": True, "error_contains": "cannot serialize float +inf as JSON"},
    {"name": "rs-tj-neginf", "source": "builtins.toJSON (-(1.0e308 * 10.0))", "error": True, "error_contains": "cannot serialize float -inf as JSON"},
    {"name": "rs-tj-nan", "source": "let inf = 1.0e308 * 10.0; in builtins.toJSON (inf - inf)", "error": True, "error_contains": "cannot serialize float NaN as JSON"},
    # eval_hashfile_builtin.rs -- hashFile(sha256/sha512) only; md5/sha1 rejected as
    # cryptographically broken, blake2 etc unsupported; non-string algo + non-path arg
    # + failed-read named (the compiler lane previously leaked a raw Python OSError on
    # a missing path -- now "builtins.hashFile: failed to read `...`"). Files are hashed
    # self-contained via builtins.toFile (no external fixtures) (vendored 2026-06-30).
    {"name": "rs-hf-exists", "source": "builtins.typeOf builtins.hashFile", "expect": "lambda"},
    {"name": "rs-hf-partial", "source": 'builtins.typeOf (builtins.hashFile "sha256")', "expect": "lambda"},
    {"name": "rs-hf-sha256-len", "source": 'builtins.stringLength (builtins.hashFile "sha256" (builtins.toFile "f" "hello"))', "expect": 64},
    {"name": "rs-hf-sha512-len", "source": 'builtins.stringLength (builtins.hashFile "sha512" (builtins.toFile "f" "hello"))', "expect": 128},
    {"name": "rs-hf-deterministic", "source": 'builtins.hashFile "sha256" (builtins.toFile "f" "hi") == builtins.hashFile "sha256" (builtins.toFile "f" "hi")', "expect": True},
    {"name": "rs-hf-algos-differ", "source": 'builtins.hashFile "sha256" (builtins.toFile "f" "hi") == builtins.hashFile "sha512" (builtins.toFile "f" "hi")', "expect": False},
    {"name": "rs-hf-md5-rejected", "source": 'builtins.hashFile "md5" (builtins.toFile "f" "x")', "error": True, "error_contains": "'md5' is not supported"},
    {"name": "rs-hf-md5-broken", "source": 'builtins.hashFile "md5" (builtins.toFile "f" "x")', "error": True, "error_contains": "cryptographically broken"},
    {"name": "rs-hf-sha1-rejected", "source": 'builtins.hashFile "sha1" (builtins.toFile "f" "x")', "error": True, "error_contains": "'sha1' is not supported"},
    {"name": "rs-hf-unsupported-algo", "source": 'builtins.hashFile "blake2" (builtins.toFile "f" "x")', "error": True, "error_contains": "unsupported algorithm 'blake2'"},
    {"name": "rs-hf-algo-int", "source": 'builtins.hashFile 42 (builtins.toFile "f" "x")', "error": True, "error_contains": "must be string"},
    {"name": "rs-hf-nonpath-arg", "source": 'builtins.hashFile "sha256" 42', "error": True, "error_contains": "expected string or path"},
    {"name": "rs-hf-missing-path", "source": 'builtins.hashFile "sha256" "/non-existent-xyz-test"', "error": True, "error_contains": "failed to read"},
    {"name": "rs-hf-missing-path-names-it", "source": 'builtins.hashFile "sha256" "/non-existent-xyz-test"', "error": True, "error_contains": "/non-existent-xyz-test"},
    {"name": "rs-hf-consistent-with-hashstring", "source": 'let f = builtins.toFile "f" "content-xyz"; in builtins.hashFile "sha256" f == builtins.hashString "sha256" (builtins.readFile f)', "expect": True},
]

# eval_basics.rs cases deliberately NOT imported yet (documented, not silently
# dropped): `builtins.ontologyLift` is a pnix domain-extension builtin (multi-arity
# ontology lifting with Accept->Candidate downgrade) -> tracked under P3 domain
# builtins, not core semantics.
RUST_EVAL_KNOWN_GAPS: list[str] = ["builtins.ontologyLift (eval_ontology_lift)"]


# Builtin parity ground truth, adapted from `~/pnix/crates/pnix-eval/tests/
# builtin_parity.rs` (the FULL evaluator). Vendored as static data so pnix-hy does
# NOT depend on ~/pnix at runtime. Note byte-based substring/stringLength (Nix
# semantics): `substring 0 6 "<2 hangul + space>"` slices 6 BYTES = 2 syllables.
RUST_BUILTIN_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-bp-substring-1", "source": 'builtins.substring 0 5 "hello world"', "expect": "hello"},
    {"name": "rs-bp-substring-2", "source": 'builtins.substring 6 5 "hello world"', "expect": "world"},
    {"name": "rs-bp-substring-3", "source": 'builtins.substring 0 6 "빛은 뭐야?"', "expect": "빛은"},
    {"name": "rs-bp-stringLength-4", "source": 'builtins.stringLength "hello"', "expect": 5},
    {"name": "rs-bp-stringLength-5", "source": 'builtins.stringLength "안녕요"', "expect": 9},
    {"name": "rs-bp-elem-6", "source": "builtins.elem 2 [1 2 3]", "expect": True},
    {"name": "rs-bp-elem-7", "source": "builtins.elem 42 [1 2 3]", "expect": False},
    {"name": "rs-bp-elem-8", "source": 'builtins.elem "b" ["a" "b" "c"]', "expect": True},
    {"name": "rs-bp-lessThan-9", "source": "builtins.lessThan 1 2", "expect": True},
    {"name": "rs-bp-lessThan-10", "source": "builtins.lessThan 5 2", "expect": False},
    {"name": "rs-bp-lessThan-11", "source": "builtins.lessThan 2 2", "expect": False},
    {"name": "rs-bp-lessThan-12", "source": "builtins.lessThan 1.5 2.0", "expect": True},
    {"name": "rs-bp-add-13", "source": "builtins.add 2 3", "expect": 5},
    {"name": "rs-bp-sub-14", "source": "builtins.sub 10 3", "expect": 7},
    {"name": "rs-bp-mul-15", "source": "builtins.mul 4 5", "expect": 20},
    {"name": "rs-bp-div-16", "source": "builtins.div 20 4", "expect": 5},
    {"name": "rs-bp-nixVersion-17", "source": "builtins.nixVersion", "expect": "2.18.0-pnix"},
    {"name": "rs-bp-storeDir-18", "source": "builtins.storeDir", "expect": "/nix/store"},
    {"name": "rs-bp-compareVersions-19", "source": 'builtins.compareVersions "1.2.3" "1.10.0"', "expect": -1},
    {"name": "rs-bp-compareVersions-20", "source": 'builtins.compareVersions "2.0" "2.0"', "expect": 0},
    {"name": "rs-bp-compareVersions-21", "source": 'builtins.compareVersions "2.0" "1.9"', "expect": 1},
    {"name": "rs-bp-getAttr-22", "source": 'builtins.getAttr "answer" { answer = 42; }', "expect": 42},
    {"name": "rs-bp-seq-23", "source": "builtins.seq 1 2", "expect": 2},
    {"name": "rs-bp-deepSeq-24", "source": 'builtins.deepSeq { a = [1 2 3]; } "done"', "expect": "done"},
    {"name": "rs-bp-match-25", "source": 'builtins.match "a(b+)" "ccc"', "expect": None},
    {"name": "rs-bp-trace-26", "source": 'builtins.trace "hello" 42', "expect": 42},
    {"name": "rs-bp-lt-27", "source": "builtins.lt 1 2", "expect": True},
    {"name": "rs-bp-le-28", "source": "builtins.le 2 2", "expect": True},
    {"name": "rs-bp-gt-29", "source": "builtins.gt 3 2", "expect": True},
    {"name": "rs-bp-ge-30", "source": "builtins.ge 3 3", "expect": True},
    {"name": "rs-bp-mod-31", "source": "builtins.mod 17 5", "expect": 2},
    {"name": "rs-bp-neg-32", "source": "builtins.neg 3", "expect": -3},
    {"name": "rs-bp-abs-33", "source": "builtins.abs (-4)", "expect": 4},
    {"name": "rs-bp-pow-34", "source": "builtins.pow 2 5", "expect": 32},
    {"name": "rs-bp-floor-35", "source": "builtins.floor 3.9", "expect": 3},
    {"name": "rs-bp-ceil-36", "source": "builtins.ceil 3.1", "expect": 4},
    {"name": "rs-bp-warn-37", "source": 'builtins.warn "hello" 7', "expect": 7},
    {"name": "rs-bp-traceVerbose-38", "source": 'builtins.traceVerbose "hello" 9', "expect": 9},
    {"name": "rs-bp-find-39", "source": "builtins.find 2 [1 2 3]", "expect": 2},
    {"name": "rs-bp-get-40", "source": 'builtins.get { x = 1; } "x"', "expect": 1},
    {"name": "rs-bp-get-41", "source": 'builtins.get { x = 1; } "y"', "expect": None},
    # structural (List/AttrSet) cases, transcribed + verified
    {"name": "rs-bp-splitVersion", "source": 'builtins.splitVersion "1.2rc1"', "expect": ["1", "2", "rc", "1"]},
    {"name": "rs-bp-concatStringsSep", "source": 'builtins.concatStringsSep "-" ["a" "b"]', "expect": "a-b"},
    {"name": "rs-bp-listToAttrs", "source": 'builtins.listToAttrs [ { name = "x"; value = 1; } { name = "y"; value = 2; } ]', "expect": {"x": 1, "y": 2}},
    {"name": "rs-bp-removeAttrs", "source": 'builtins.removeAttrs { a = 1; b = 2; c = 3; } [ "b" ]', "expect": {"a": 1, "c": 3}},
    {"name": "rs-bp-map", "source": "builtins.map (x: x * 2) [1 2 3]", "expect": [2, 4, 6]},
    {"name": "rs-bp-attrNames", "source": "builtins.attrNames { b = 1; a = 2; }", "expect": ["a", "b"]},
    {"name": "rs-bp-genList", "source": "builtins.genList (i: i * i) 4", "expect": [0, 1, 4, 9]},
    {"name": "rs-bp-sort", "source": "builtins.sort (a: b: a < b) [3 1 2]", "expect": [1, 2, 3]},
    {"name": "rs-bp-replaceStrings", "source": 'builtins.replaceStrings ["a"] ["X"] "banana"', "expect": "bXnXnX"},
    {"name": "rs-bp-toJSON", "source": "builtins.toJSON { b = 2; a = 1; }", "expect": '{"a":1,"b":2}'},
]


# Integer-overflow ground truth, adapted from `eval_arith_builtin_overflow.rs`.
# pnix is i64 with checked overflow; pnix-hy enforces the same bound (check_i64 /
# _ci / check-i64) instead of Python's silent bignum.
RUST_OVERFLOW_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-ovf-add-ok", "source": "builtins.add 1 2", "expect": 3},
    {"name": "rs-ovf-add-max", "source": "builtins.add 9223372036854775806 1", "expect": 9223372036854775807},
    {"name": "rs-ovf-sub-ok", "source": "builtins.sub 5 3", "expect": 2},
    {"name": "rs-ovf-mul-ok", "source": "builtins.mul 3 4", "expect": 12},
    {"name": "rs-ovf-div-negmax", "source": "builtins.div (-9223372036854775807) (-1)", "expect": 9223372036854775807},
    {"name": "rs-ovf-div-ok", "source": "builtins.div 10 3", "expect": 3},
    {"name": "rs-ovf-add-overflow", "source": "builtins.add 9223372036854775807 1", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-ovf-add-neg-overflow", "source": "builtins.add (-9223372036854775807) (-2)", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-ovf-sub-overflow", "source": "builtins.sub (-9223372036854775807) 2", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-ovf-mul-overflow", "source": "builtins.mul 9223372036854775807 9223372036854775807", "error": True, "error_contains": "integer overflow"},
]


RUST_FUNCTIONAL_LAZY_TYPE_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-func-curry-user", "source": "let f = a: b: a + b; in f 1 2", "expect": 3},
    {"name": "rs-func-curry-partial", "source": "let f = (a: b: a + b) 1; in f 2", "expect": 3},
    {"name": "rs-func-curry-builtin-add", "source": "let inc = builtins.add 1; in inc 41", "expect": 42},
    {"name": "rs-func-curry-builtin-lessThan", "source": "let lt5 = builtins.lessThan 5; in lt5 10", "expect": True},
    {"name": "rs-func-curry-builtin-map", "source": "let inc = builtins.add 1; in builtins.map inc [1 2 3]", "expect": [2, 3, 4]},
    {"name": "rs-func-compose-two", "source": "let compose = f: g: x: f (g x); double = x: x * 2; inc = x: x + 1; in compose double inc 5", "expect": 12},
    {"name": "rs-func-compose-let", "source": "let inc = x: x + 1; double = x: x * 2; f = x: double (inc x); in f 5", "expect": 12},
    {"name": "rs-func-compose-map-filter", "source": "let xs = [1 2 3 4 5]; even = x: builtins.mod x 2 == 0; dbl = x: x * 2; in builtins.map dbl (builtins.filter even xs)", "expect": [4, 8]},
    {"name": "rs-func-foldl-sum", "source": "builtins.foldl' (a: b: a + b) 0 [1 2 3 4]", "expect": 10},
    {"name": "rs-func-foldl-builtin-add", "source": "builtins.foldl' builtins.add 0 [1 2 3 4 5]", "expect": 15},
    {"name": "rs-func-lazy-if", "source": "if true then 42 else (1 / 0)", "expect": 42},
    {"name": "rs-func-lazy-or", "source": "true || (1 / 0 == 0)", "expect": True},
    {"name": "rs-func-lazy-and", "source": "false && (1 / 0 == 0)", "expect": False},
    {"name": "rs-func-lazy-attr-unused", "source": "let s = { used = 1; broken = 1 / 0; }; in s.used", "expect": 1},
    {"name": "rs-func-lazy-let-unused", "source": "let unused = 1 / 0; used = 7; in used", "expect": 7},
    {"name": "rs-func-lazy-list-head", "source": "builtins.head [ 1 (1 / 0) (1 / 0) ]", "expect": 1},
    {"name": "rs-func-rec-factorial", "source": "let fact = n: if n <= 1 then 1 else n * fact (n - 1); in fact 5", "expect": 120},
    {"name": "rs-func-mutual-rec", "source": "let isEven = n: if n == 0 then true else isOdd (n - 1); isOdd = n: if n == 0 then false else isEven (n - 1); in isEven 10", "expect": True},
    {"name": "rs-func-rec-attr-chain", "source": "rec { a = 1; b = a + 1; c = b + 1; }.c", "expect": 3},
    {"name": "rs-func-rec-attr-higher", "source": "rec { f = n: if n <= 0 then 0 else n + f (n - 1); total = f 4; }.total", "expect": 10},
    {"name": "rs-func-with-scope", "source": "with { a = 1; b = 2; }; a + b", "expect": 3},
    {"name": "rs-func-with-inner-wins", "source": "with { x = 1; }; with { x = 99; }; x", "expect": 99},
    {"name": "rs-func-let-overrides-with", "source": "with { x = 1; }; let x = 99; in x", "expect": 99},
    {"name": "rs-func-inherit-from-attrset", "source": "let src = { a = 1; b = 2; }; in let inherit (src) a b; in a + b", "expect": 3},
    {"name": "rs-func-attr-pattern", "source": "({a, b}: a + b) { a = 1; b = 2; }", "expect": 3},
    {"name": "rs-func-attr-default", "source": "({a, b ? 10}: a + b) { a = 1; }", "expect": 11},
    {"name": "rs-func-attr-at", "source": "(args@{a, b}: args.a + args.b + a + b) { a = 1; b = 2; }", "expect": 6},
    {"name": "rs-func-attr-ellipsis", "source": "({a, ...}: a) { a = 1; b = 2; c = 3; }", "expect": 1},
    {"name": "rs-func-gen-filter-map", "source": "builtins.map (x: x * 2) (builtins.filter (x: x > 4) (builtins.genList (i: i) 10))", "expect": [10, 12, 14, 16, 18]},
    {"name": "rs-func-concat-map", "source": "builtins.concatMap (x: [x x]) [1 2 3]", "expect": [1, 1, 2, 2, 3, 3]},
    {"name": "rs-func-all-positive", "source": "builtins.all (x: x > 0) [1 2 3]", "expect": True},
    {"name": "rs-func-any-neg", "source": "builtins.any (x: x < 0) [1 -2 3]", "expect": True},
    {"name": "rs-func-listtoattrs-select", "source": "(builtins.listToAttrs [{ name = \"a\"; value = 1; } { name = \"b\"; value = 2; }]).b", "expect": 2},
    {"name": "rs-func-shared-thunk-list", "source": "let x = 1 + 2; in [ x x x ]", "expect": [3, 3, 3]},
    {"name": "rs-func-lazy-rec-field", "source": "let s = rec { a = if cond then 1 else 2; cond = true; b = a + 10; }; in s.b", "expect": 11},
    {"name": "rs-func-lazy-default", "source": "({a, b ? a + 1}: b) { a = 5; }", "expect": 6},
    {"name": "rs-func-concatsep-map", "source": "builtins.concatStringsSep \", \" (builtins.map (x: builtins.toString x) [1 2 3])", "expect": "1, 2, 3"},
    {"name": "rs-func-interp-call-chain", "source": "let f = x: builtins.toString (x + 1); in \"v=${f 41}\"", "expect": "v=42"},
    {"name": "rs-func-substring", "source": "builtins.substring 0 3 \"hello world\"", "expect": "hel"},
    {"name": "rs-func-stringlength", "source": "builtins.stringLength \"hello\"", "expect": 5},
    {"name": "rs-func-lambda-in-list", "source": "builtins.head [ (x: x + 1) (x: x + 2) ] 10", "expect": 11},
    {"name": "rs-func-higher-order-attr", "source": "({ g = x: x * x; }.g) 5", "expect": 25},
    {"name": "rs-func-call-paren-result", "source": "(x: y: x + y) (1 + 2) (3 * 4)", "expect": 15},
    {"name": "rs-func-select-chain-apply", "source": "let m = { ops = { add = a: b: a + b; }; }; in m.ops.add 10 20", "expect": 30},
    {"name": "rs-func-tryeval-fail", "source": "(builtins.tryEval (1 / 0)).success", "error": True, "error_contains": "division by zero"},
    {"name": "rs-func-tryeval-success", "source": "(builtins.tryEval 42).value", "expect": 42},
    {"name": "rs-func-select-default-expr", "source": "{ a = 1; }.b or (1 + 2)", "expect": 3},
    {"name": "rs-func-dynamic-select", "source": "let key = \"a\"; s = { a = 42; }; in s.${key}", "expect": 42},
    {"name": "rs-type-list-paren-int", "source": "[ (1 + 2) (3 * 4) (10 - 5) ]", "expect": [3, 12, 5]},
    {"name": "rs-type-list-paren-mixed", "source": "[ (1 + 2) (\"a\" + \"b\") ([1 2] ++ [3]) ]", "expect": [3, "ab", [1, 2, 3]]},
    {"name": "rs-type-int-float-plus", "source": "1 + 1.5", "expect": 2.5},
    {"name": "rs-type-neg-paren", "source": "-(2 + 3)", "expect": -5},
    {"name": "rs-type-bool-impl", "source": "true -> false", "expect": False},
    {"name": "rs-type-eq-int-string-false", "source": "1 == \"1\"", "expect": False},
    {"name": "rs-type-lt-string", "source": "\"a\" < \"b\"", "expect": True},
    {"name": "rs-type-lt-list", "source": "[1 2] < [1 3]", "expect": True},
    {"name": "rs-type-neg-list-paren", "source": "[ (-1) (-2) (-3) ]", "expect": [-1, -2, -3]},
    {"name": "rs-type-eq-attrs-nested-a", "source": "{a={b=1;};} == {a={b=1;};}", "expect": True},
    {"name": "rs-type-eq-attrs-nested-b", "source": "{a={b=1;};} == {a={b=2;};}", "expect": False},
    {"name": "rs-type-string-plus-int-err", "source": "\"x\" + 1", "error": True, "error_contains": "unsupported operand types"},
    {"name": "rs-with-unused-throw", "source": "with (throw \"boom\"); 1", "expect": 1},
    {"name": "rs-with-lexical-skip-throw", "source": "let x = 42; in with (throw \"boom\"); x", "expect": 42},
    {"name": "rs-with-actually-used-err", "source": "with (throw \"boom\"); y", "error": True, "error_contains": "boom"},
    {"name": "rs-with-nested-outer-throw-skipped", "source": "with (throw \"outer\"); with { x = 7; }; x", "expect": 7},
    {"name": "rs-with-non-attr-unused", "source": "let x = 99; in with 42; x", "expect": 99},
    {"name": "rs-with-priority-let-outer", "source": "let x = 1; in with { x = 2; }; x", "expect": 1},
    {"name": "rs-with-priority-param", "source": "with { x = 2; }; (x: x) 7", "expect": 7},
    {"name": "rs-with-priority-inner-provides", "source": "with { a = 1; b = 2; }; with { a = 99; }; [ a b ]", "expect": [99, 2]},
    {"name": "rs-inherit-from-throw-used-err", "source": "let s = throw \"boom\"; in (let inherit (s) a; in a)", "error": True, "error_contains": "boom"},
    {"name": "rs-inherit-attr-from-throw-used-err", "source": "let s = throw \"boom\"; in ({ inherit (s) a; }).a", "error": True, "error_contains": "boom"},
    {"name": "rs-dyn-attr-basic", "source": "let name=\"a\"; in { ${name} = 1; }.a", "expect": 1},
    {"name": "rs-dyn-attr-rec-before", "source": "rec { ${name} = 1; name = \"a\"; }.a", "expect": 1},
    {"name": "rs-dyn-attr-generated-pos-null", "source": "let name=\"a\"; in builtins.unsafeGetAttrPos \"a\" { ${name} = 1; }", "expect": None},
    {"name": "rs-dyn-let-disallowed", "source": "let ${\"a\"} = 1; in a", "error": True, "error_contains": "dynamic attributes"},
    {"name": "rs-regex-posix-space-split", "source": "builtins.filter (x: builtins.isString x) (builtins.split \"[[:space:]]+\" \"  one  two\\tthree   four \")", "expect": ["", "one", "two", "three", "four", ""]},
    {"name": "rs-nixpkgs-fix-factorial", "source": "let fix = f: let x = f x; in x; fact = self: n: if n <= 1 then 1 else n * self (n - 1); f = fix fact; in f 5", "expect": 120},
    {"name": "rs-nixpkgs-make-overridable", "source": "let makeOverridable = f: origArgs: let result = f origArgs; overrideWith = newArgs: makeOverridable f (origArgs // (if builtins.isFunction newArgs then newArgs origArgs else newArgs)); in result // { override = overrideWith; overrideAttrs = overrideWith; }; hello = makeOverridable (a: { greeting = \"hello, ${a.name}\"; }) { name = \"world\"; }; in [ hello.greeting (hello.override { name = \"pnix\"; }).greeting ]", "expect": ["hello, world", "hello, pnix"]},
    {"name": "rs-nixpkgs-genAttrs", "source": "let genAttrs = names: f: builtins.listToAttrs (builtins.map (name: { inherit name; value = f name; }) names); in genAttrs [ \"a\" \"b\" \"c\" ] (n: \"v_\" + n)", "expect": {"a": "v_a", "b": "v_b", "c": "v_c"}},
    {"name": "rs-nixpkgs-foldAttrs", "source": "let foldAttrs = op: nul: list_of_attrs: builtins.foldl' (acc: as: builtins.foldl' (acc2: name: acc2 // { ${name} = op (as.${name}) (acc2.${name} or nul); }) acc (builtins.attrNames as)) {} list_of_attrs; in foldAttrs (item: acc: acc + item) 0 [ { x = 1; y = 10; } { x = 2; y = 20; } { x = 3; } ]", "expect": {"x": 6, "y": 30}},
    {"name": "rs-nixpkgs-cartesian-product-len", "source": "let cartesianProductOfSets = attrsOfLists: builtins.foldl' (listOfAttrs: name: builtins.concatMap (a: builtins.map (v: a // { ${name} = v; }) attrsOfLists.${name}) listOfAttrs) [ {} ] (builtins.attrNames attrsOfLists); in builtins.length (cartesianProductOfSets { a = [1 2]; b = [3 4]; })", "expect": 4},
]


RUST_COMPARE_VERSION_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-cmp-lt-int-string-err", "source": '1 < "a"', "error": True, "error_contains": "cannot compare"},
    {"name": "rs-cmp-lt-list-int-err", "source": "[1] < 1", "error": True, "error_contains": "cannot compare"},
    {"name": "rs-cmp-lt-null-err", "source": "null < null", "error": True, "error_contains": "cannot compare"},
    {"name": "rs-cmp-lt-int-float-a", "source": "1 < 1.5", "expect": True},
    {"name": "rs-cmp-lt-int-float-b", "source": "1.0 < 2", "expect": True},
    {"name": "rs-cmp-lt-int-float-c", "source": "2.0 < 2", "expect": False},
    {"name": "rs-cmp-lt-string-a", "source": '"a" < "b"', "expect": True},
    {"name": "rs-cmp-lt-string-b", "source": '"b" < "a"', "expect": False},
    {"name": "rs-cmp-lt-string-c", "source": '"a" < "a"', "expect": False},
    {"name": "rs-cmp-lt-list-a", "source": "[1 2] < [1 3]", "expect": True},
    {"name": "rs-cmp-lt-list-b", "source": "[1 2] < [1 2]", "expect": False},
    {"name": "rs-cmp-lt-list-prefix", "source": "[1] < [1 2]", "expect": True},
    {"name": "rs-cmp-eq-int-float-a", "source": "1 == 1.0", "expect": True},
    {"name": "rs-cmp-eq-int-float-b", "source": "1.0 == 1", "expect": True},
    {"name": "rs-cmp-eq-int-string", "source": '1 == "1"', "expect": False},
    {"name": "rs-cmp-eq-null-null", "source": "null == null", "expect": True},
    {"name": "rs-cmp-eq-null-int", "source": "null == 0", "expect": False},
    {"name": "rs-cmp-eq-null-string", "source": 'null == ""', "expect": False},
    {"name": "rs-cmp-eq-null-bool", "source": "null == false", "expect": False},
    {"name": "rs-cmp-eq-lambda-lambda", "source": "(x: x) == (x: x)", "expect": False},
    {"name": "rs-cmp-eq-lambda-self", "source": "let f = x: x; in f == f", "expect": False},
    {"name": "rs-cmp-eq-attr-lambda", "source": "{ a = 1; f = x: x; } == { a = 1; f = x: x; }", "expect": False},
    {"name": "rs-cmp-eq-list-deep", "source": "[1 [2 3]] == [1 [2 3]]", "expect": True},
    {"name": "rs-cmp-eq-list-deep-mismatch", "source": "[1 [2 3]] == [1 [2 4]]", "expect": False},
    {"name": "rs-cmp-eq-attr-deep", "source": "{ a = { b = 1; }; } == { a = { b = 1; }; }", "expect": True},
    {"name": "rs-cmp-eq-attr-deep-mismatch", "source": "{ a = { b = 1; }; } == { a = { b = 2; }; }", "expect": False},
    {"name": "rs-cmp-eq-attr-extra-left", "source": "{ a = 1; b = 2; } == { a = 1; }", "expect": False},
    {"name": "rs-cmp-eq-attr-extra-right", "source": "{ a = 1; } == { a = 1; b = 2; }", "expect": False},
    {"name": "rs-cmp-float-div-zero", "source": "0.0 / 0.0", "error": True, "error_contains": "division by zero"},
    {"name": "rs-cmp-float-mod-zero", "source": "0.0 % 0.0", "error": True, "error_contains": "modulo by zero"},
    {"name": "rs-ver-trailing-zero-a", "source": 'builtins.compareVersions "1.2" "1.2.0"', "expect": -1},
    {"name": "rs-ver-trailing-zero-b", "source": 'builtins.compareVersions "1.2.0" "1.2"', "expect": 1},
    {"name": "rs-ver-missing-a", "source": 'builtins.compareVersions "1" "1.0"', "expect": -1},
    {"name": "rs-ver-missing-b", "source": 'builtins.compareVersions "1.0" "1"', "expect": 1},
    {"name": "rs-ver-missing-c", "source": 'builtins.compareVersions "1" "1.0.0"', "expect": -1},
    {"name": "rs-ver-missing-d", "source": 'builtins.compareVersions "1.0.0" "1"', "expect": 1},
    {"name": "rs-ver-equal-a", "source": 'builtins.compareVersions "1.2" "1.2"', "expect": 0},
    {"name": "rs-ver-equal-empty", "source": 'builtins.compareVersions "" ""', "expect": 0},
    {"name": "rs-ver-equal-pre", "source": 'builtins.compareVersions "1.0pre" "1.0pre"', "expect": 0},
    {"name": "rs-ver-pre-release-a", "source": 'builtins.compareVersions "1.0pre1" "1.0"', "expect": -1},
    {"name": "rs-ver-pre-release-b", "source": 'builtins.compareVersions "1.0" "1.0pre1"', "expect": 1},
    {"name": "rs-ver-pre-release-c", "source": 'builtins.compareVersions "1.0pre" "1.0"', "expect": -1},
    {"name": "rs-ver-pre-release-d", "source": 'builtins.compareVersions "1.0" "1.0pre"', "expect": 1},
    {"name": "rs-ver-pre-number-a", "source": 'builtins.compareVersions "1.0pre1" "1.0pre2"', "expect": -1},
    {"name": "rs-ver-pre-number-b", "source": 'builtins.compareVersions "1.0pre2" "1.0pre1"', "expect": 1},
    {"name": "rs-ver-pre-alone-a", "source": 'builtins.compareVersions "1.0pre" "1.0pre1"', "expect": -1},
    {"name": "rs-ver-pre-alone-b", "source": 'builtins.compareVersions "1.0pre1" "1.0pre"', "expect": 1},
    {"name": "rs-ver-numeric-empty-a", "source": 'builtins.compareVersions "1.0a1" "1.0a"', "expect": 1},
    {"name": "rs-ver-numeric-empty-b", "source": 'builtins.compareVersions "1.0a" "1.0a1"', "expect": -1},
    {"name": "rs-ver-nonnumeric-empty-a", "source": 'builtins.compareVersions "1.0a" "1.0"', "expect": 1},
    {"name": "rs-ver-nonnumeric-empty-b", "source": 'builtins.compareVersions "1.0" "1.0a"', "expect": -1},
    {"name": "rs-ver-lex-a", "source": 'builtins.compareVersions "1.0a" "1.0b"', "expect": -1},
    {"name": "rs-ver-lex-b", "source": 'builtins.compareVersions "1.0b" "1.0a"', "expect": 1},
    {"name": "rs-ver-numeric-a", "source": 'builtins.compareVersions "2" "10"', "expect": -1},
    {"name": "rs-ver-numeric-b", "source": 'builtins.compareVersions "10" "2"', "expect": 1},
    {"name": "rs-ver-numeric-c", "source": 'builtins.compareVersions "1.10" "1.2"', "expect": 1},
    {"name": "rs-ver-numeric-d", "source": 'builtins.compareVersions "1.2" "1.10"', "expect": -1},
    {"name": "rs-ver-plus-a", "source": 'builtins.compareVersions "1.0" "1.0+rev"', "expect": -1},
    {"name": "rs-ver-plus-b", "source": 'builtins.compareVersions "1.0+rev" "1.0"', "expect": 1},
    {"name": "rs-ver-tilde-a", "source": 'builtins.compareVersions "1.0" "1.0~rc"', "expect": -1},
    {"name": "rs-ver-tilde-b", "source": 'builtins.compareVersions "1.0~rc" "1.0"', "expect": 1},
    {"name": "rs-ver-non-string", "source": 'builtins.compareVersions 1 "1.0"', "error": True, "error_contains": "expected two strings"},
    {"name": "rs-ver-split-plus", "source": 'builtins.splitVersion "1.0+rev"', "expect": ["1", "0", "+rev"]},
    {"name": "rs-ver-parse-first-digit-hyphen", "source": '(builtins.parseDrvName "a-1-b-2").name', "expect": "a"},
    {"name": "rs-cycle-eq-attrset", "source": "let r = { a = r; }; in r == r", "expect": True},
    {"name": "rs-cycle-eq-list", "source": "let r = [ r ]; in r == r", "expect": True},
    {"name": "rs-cycle-eq-independent", "source": "let r = { a = r; }; s = { a = s; }; in r == s", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-cycle-ne", "source": "let r = { a = r; }; in r != r", "expect": False},
    {"name": "rs-cycle-builtins-eq", "source": "let r = { a = r; }; in builtins.eq r r", "expect": True},
    {"name": "rs-cycle-elem", "source": "let r = { a = r; }; s = { a = s; }; in builtins.elem r [ s ]", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-cycle-find", "source": "let r = [ r ]; in builtins.find r r", "error": True, "error_contains": "recursion"},
    {"name": "rs-cycle-elem-prefix", "source": "let r = { a = r; }; in builtins.elem 1 [ 1 r ]", "expect": True},
    {"name": "rs-cycle-attrnames", "source": "let r = { a = r; b = 2; }; in builtins.attrNames r", "expect": ["a", "b"]},
    {"name": "rs-cycle-hasattr", "source": "let r = { a = r; }; in r ? a", "expect": True},
    {"name": "rs-cycle-length-list", "source": "let r = [ r ]; in builtins.length r", "expect": 1},
    {"name": "rs-cycle-with-unused", "source": "let r = { a = r; }; in with r; 1", "expect": 1},
    {"name": "rs-cycle-with-member", "source": "let r = { a = r; b = 99; }; in with r; b", "expect": 99},
    {"name": "rs-cycle-inherit-member", "source": "let r = { a = r; b = 1; }; in let inherit (r) b; in b", "expect": 1},
    {"name": "rs-cycle-eq-genlist-50", "source": "(builtins.genList (x: x) 50) == (builtins.genList (x: x) 50)", "expect": True},
    {"name": "rs-cycle-eq-dag", "source": "let x = { a = 1; }; in { p = x; q = x; } == { p = x; q = x; }", "expect": True},
    {"name": "rs-cycle-eq-concrete", "source": "{ a = 1; b = [ 2 3 ]; } == { a = 1; b = [ 2 3 ]; }", "expect": True},
    {"name": "rs-cycle-eq-concrete-diff", "source": "{ a = 1; b = 2; } == { a = 1; b = 3; }", "expect": False},
    {"name": "rs-cycle-eq-throw", "source": '{ a = throw "inner"; } == { a = 1; }', "error": True, "error_contains": "inner"},
    {"name": "rs-cycle-lt", "source": "let r = [ r ]; in r < r", "expect": False},
    {"name": "rs-cycle-le", "source": "let r = [ r ]; in r <= r", "expect": True},
    {"name": "rs-cycle-gt", "source": "let r = [ r ]; in r > r", "expect": False},
    {"name": "rs-cycle-ge", "source": "let r = [ r ]; in r >= r", "expect": True},
    {"name": "rs-cycle-builtins-lt", "source": "let r = [ r ]; in builtins.lt r r", "expect": False},
    {"name": "rs-cycle-builtins-le", "source": "let r = [ r ]; in builtins.le r r", "expect": True},
    {"name": "rs-cycle-builtins-gt", "source": "let r = [ r ]; in builtins.gt r r", "expect": False},
    {"name": "rs-cycle-builtins-ge", "source": "let r = [ r ]; in builtins.ge r r", "expect": True},
    {"name": "rs-cycle-attrset-compare", "source": "let r = { a = r; }; in r < r", "error": True, "error_contains": "set"},
    {"name": "rs-cycle-lt-genlist-50", "source": "(builtins.genList (x: x) 50) < (builtins.genList (x: x + 1) 50)", "expect": True},
    {"name": "rs-cycle-lt-strings-a", "source": '"abc" < "abd"', "expect": True},
    {"name": "rs-cycle-lt-strings-b", "source": '"abc" < "abc"', "expect": False},
    {"name": "rs-cycle-lt-strings-c", "source": '"abd" < "abc"', "expect": False},
    {"name": "rs-cycle-lt-paths-a", "source": "/a < /b", "expect": True},
    {"name": "rs-cycle-lt-paths-b", "source": "/b < /a", "expect": False},
    {"name": "rs-cycle-lt-list-length-a", "source": "[ 1 2 ] < [ 1 2 3 ]", "expect": True},
    {"name": "rs-cycle-lt-list-length-b", "source": "[ 1 2 3 ] < [ 1 2 ]", "expect": False},
    {"name": "rs-cycle-lt-dag", "source": "let x = 1; in [ [ x ] [ x ] ] < [ [ x ] [ x x ] ]", "expect": True},
    {"name": "rs-cycle-lt-throw", "source": '[ (throw "inner") ] < [ 1 ]', "error": True, "error_contains": "inner"},
    {"name": "rs-ver-parse-name-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.parseDrvName "hello-1.0${./p}").name)', "expect": True},
    {"name": "rs-ver-parse-version-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.parseDrvName "hello-1.0${./p}").version)', "expect": True},
    {"name": "rs-ver-parse-no-context", "source": 'builtins.hasContext (builtins.parseDrvName "hello-1.0").name', "expect": False},
    {"name": "rs-ver-parse-text-name", "source": '(builtins.parseDrvName "hello-1.0").name', "expect": "hello"},
    {"name": "rs-ver-parse-text-version", "source": '(builtins.parseDrvName "hello-1.0").version', "expect": "1.0"},
    {"name": "rs-ver-parse-multi-context-p1", "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.parseDrvName ("a-" + ./p1 + "-" + ./p2)).name)', "expect": True},
    {"name": "rs-ver-parse-multi-context-p2", "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.parseDrvName ("a-" + ./p1 + "-" + ./p2)).name)', "expect": True},
    {"name": "rs-ver-split-context-first", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.splitVersion "1.0${./p}") 0))', "expect": True},
    {"name": "rs-ver-split-context-second", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.splitVersion "1.0${./p}") 1))', "expect": True},
    {"name": "rs-ver-split-no-context", "source": 'builtins.hasContext (builtins.elemAt (builtins.splitVersion "1.0") 0)', "expect": False},
    {"name": "rs-ver-split-list", "source": 'builtins.splitVersion "1.2.3"', "expect": ["1", "2", "3"]},
    {"name": "rs-ver-parse-non-string", "source": "builtins.parseDrvName 42", "error": True, "error_contains": "parseDrvName"},
    {"name": "rs-ver-split-non-string", "source": "builtins.splitVersion 42", "error": True, "error_contains": "splitVersion"},
]


RUST_CYCLE_GUARD_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-force-indirect-self-cycle", "source": "let s = { x = s.x; }; in s.x", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-force-rec-attr-cycle", "source": "(rec { a = b; b = a; }).a", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-force-rec-self-cycle", "source": "(rec { x = x; }).x", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-force-unforced-self-reference", "source": "let x = x; in 1", "expect": 1},
    {"name": "rs-force-legit-recursion-lambda", "source": "let f = n: if n == 0 then 0 else f (n - 1) + 1; in f 30", "expect": 30},
    {"name": "rs-force-cycle-guard-pops", "source": "let x = 1; in x + x", "expect": 2},
    {"name": "rs-cyclic-tojson-attrset", "source": "let r = { a = r; }; in builtins.toJSON r", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-cyclic-tojson-mutual", "source": "let a = { x = b; }; b = { y = a; }; in builtins.toJSON a", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-cyclic-tojson-list", "source": "let r = [ r ]; in builtins.toJSON r", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-cyclic-tojson-dag", "source": "let x = 99; in builtins.toJSON { a = x; b = x; }", "expect": '{"a":99,"b":99}'},
    {"name": "rs-cyclic-tojson-concrete", "source": 'builtins.toJSON { a = 1; b = [ 2 3 ]; c = { d = "x"; }; }', "expect": '{"a":1,"b":[2,3],"c":{"d":"x"}}'},
    {"name": "rs-cyclic-deepseq-attrset", "source": "let r = { a = r; }; in builtins.deepSeq r 99", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-cyclic-deepseq-list", "source": "let r = [ r ]; in builtins.deepSeq r 99", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-cyclic-deepseq-mutual", "source": "let a = [ b ]; b = [ a ]; in builtins.deepSeq a 99", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-cyclic-deepseq-dag", "source": "let x = 99; in builtins.deepSeq { a = x; b = x; } 7", "expect": 7},
    {"name": "rs-cyclic-deepseq-concrete", "source": "builtins.deepSeq { a = 1; b = 2; } 99", "expect": 99},
    {"name": "rs-cyclic-deepseq-inner-throw", "source": 'builtins.deepSeq { a = throw "inner"; } 99', "error": True, "error_contains": "inner"},
    {"name": "rs-interp-tostring-chain", "source": 'let a = { __toString = self: "from-a"; }; b = { __toString = self: a; }; in "[${b}]"', "expect": "[from-a]"},
    {"name": "rs-interp-tostring-chain-deep", "source": 'let l1 = { __toString = self: "leaf"; }; l2 = { __toString = self: l1; }; l3 = { __toString = self: l2; }; l4 = { __toString = self: l3; }; l5 = { __toString = self: l4; }; l6 = { __toString = self: l5; }; l7 = { __toString = self: l6; }; l8 = { __toString = self: l7; }; in "[${l8}]"', "expect": "[leaf]"},
    {"name": "rs-interp-self-cycle", "source": 'let s = { __toString = self: "${s}"; }; in "${s}"', "error": True, "error_contains": "interpolation coercion cycle"},
    {"name": "rs-interp-tostring-self-cycle", "source": 'let s = { __toString = self: "${s}"; }; in builtins.toString s', "error": True, "error_contains": "interpolation coercion cycle"},
    {"name": "rs-interp-mutual-cycle", "source": 'let a = { __toString = self: "${b}"; }; b = { __toString = self: "${a}"; }; in "${a}"', "error": True, "error_contains": "interpolation coercion cycle"},
    {"name": "rs-interp-within-call-cycle", "source": "let s = { __toString = self: s; }; in builtins.toString s", "error": True, "error_contains": "cycle"},
    {"name": "rs-interp-outpath-ok", "source": '"[${{ outPath = "/foo"; }}]"', "expect": "[/foo]"},
    {"name": "rs-interp-tostring-non-string", "source": '"${{ __toString = self: 42; }}"', "error": True, "error_contains": "cannot coerce"},
]


RUST_JSON_TOML_DATA_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-json-tojson-int", "source": "builtins.toJSON 42", "expect": "42"},
    {"name": "rs-json-tojson-float", "source": "builtins.toJSON 3.5", "expect": "3.5"},
    {"name": "rs-json-tojson-bool-true", "source": "builtins.toJSON true", "expect": "true"},
    {"name": "rs-json-tojson-bool-false", "source": "builtins.toJSON false", "expect": "false"},
    {"name": "rs-json-tojson-null", "source": "builtins.toJSON null", "expect": "null"},
    {"name": "rs-json-tojson-string", "source": 'builtins.toJSON "hello"', "expect": '"hello"'},
    {"name": "rs-json-tojson-list", "source": "builtins.toJSON [ 1 2 3 ]", "expect": "[1,2,3]"},
    {"name": "rs-json-tojson-attrset", "source": 'builtins.toJSON { a = 1; b = "x"; }', "expect": '{"a":1,"b":"x"}'},
    {"name": "rs-json-tojson-non-id-keys", "source": 'builtins.toJSON { "not-id" = 1; "sp ace" = 2; }', "expect": '{"not-id":1,"sp ace":2}'},
    {"name": "rs-json-tojson-control-backspace", "source": 'builtins.toJSON (builtins.fromJSON "\\"a\\\\bb\\"")', "expect": '"a\\bb"'},
    {"name": "rs-json-tojson-control-formfeed", "source": 'builtins.toJSON (builtins.fromJSON "\\"a\\\\fb\\"")', "expect": '"a\\fb"'},
    {"name": "rs-json-tojson-control-u0001", "source": 'builtins.toJSON (builtins.fromJSON "\\"a\\\\u0001b\\"")', "expect": '"a\\u0001b"'},
    {"name": "rs-json-fromjson-int", "source": 'builtins.fromJSON "42"', "expect": 42},
    {"name": "rs-json-fromjson-attr-a", "source": '(builtins.fromJSON "{\\"a\\":1,\\"b\\":\\"x\\"}").a', "expect": 1},
    {"name": "rs-json-fromjson-list", "source": 'builtins.fromJSON "[1,2,3]"', "expect": [1, 2, 3]},
    {"name": "rs-json-fromjson-invalid", "source": 'builtins.fromJSON "{ not json"', "error": True, "error_contains": "parse error"},
    {
        "name": "rs-json-round-trip-attrset",
        "source": 'builtins.fromJSON (builtins.toJSON { a = 1; b = [ 2 3 ]; c = "x"; })',
        "expect": {"a": 1, "b": [2, 3], "c": "x"},
    },
    {"name": "rs-json-huge-positive-int", "source": 'builtins.fromJSON "999999999999999999999"', "error": True, "error_contains": "too large"},
    {"name": "rs-json-huge-negative-int", "source": 'builtins.fromJSON "-999999999999999999999"', "error": True, "error_contains": "too large"},
    {"name": "rs-json-i64-max-plus-one", "source": 'builtins.fromJSON "9223372036854775808"', "error": True, "error_contains": "too large"},
    {"name": "rs-json-i64-min-minus-one", "source": 'builtins.fromJSON "-9223372036854775809"', "error": True, "error_contains": "too large"},
    {"name": "rs-json-i64-max", "source": 'builtins.fromJSON "9223372036854775807"', "expect": 9223372036854775807},
    {"name": "rs-json-i64-min", "source": 'builtins.fromJSON "-9223372036854775808"', "expect": -9223372036854775808},
    {"name": "rs-json-typeof-max-int", "source": 'builtins.typeOf (builtins.fromJSON "9223372036854775807")', "expect": "int"},
    {"name": "rs-json-regular-float", "source": 'builtins.fromJSON "1.5"', "expect": 1.5},
    {"name": "rs-json-negative-float", "source": 'builtins.fromJSON "-1.5"', "expect": -1.5},
    {"name": "rs-json-huge-float", "source": 'builtins.fromJSON "1e308"', "expect": 1e308},
    {"name": "rs-json-scientific-int-mantissa", "source": 'builtins.fromJSON "1e3"', "expect": 1000.0},
    {"name": "rs-json-typeof-float", "source": 'builtins.typeOf (builtins.fromJSON "1.5")', "expect": "float"},
    {"name": "rs-json-array-overflow", "source": 'builtins.fromJSON "[1, 999999999999999999999]"', "error": True, "error_contains": "too large"},
    {"name": "rs-json-object-overflow", "source": 'builtins.fromJSON "{\\"x\\": 999999999999999999999}"', "error": True, "error_contains": "too large"},
    {
        "name": "rs-json-nested-overflow",
        "source": 'builtins.fromJSON "{\\"a\\": [{\\"b\\": 999999999999999999999}]}"',
        "error": True,
        "error_contains": "too large",
    },
    {"name": "rs-json-string-big-number-safe", "source": 'builtins.fromJSON "\\"999999999999999999999\\""', "expect": "999999999999999999999"},
    {
        "name": "rs-json-string-escape-quote-safe",
        "source": 'builtins.fromJSON "\\"a\\\\\\"b 999999999999999999999\\""',
        "expect": 'a"b 999999999999999999999',
    },
    {"name": "rs-json-array-floats", "source": 'builtins.fromJSON "[1.5, 2.5, 3.5]"', "expect": [1.5, 2.5, 3.5]},
    {"name": "rs-json-mixed-int-float", "source": 'builtins.fromJSON "[1, 2.5, 100, -50]"', "expect": [1, 2.5, 100, -50]},
    {"name": "rs-json-zero", "source": 'builtins.fromJSON "0"', "expect": 0},
    {"name": "rs-json-negative-one", "source": 'builtins.fromJSON "-1"', "expect": -1},
    {"name": "rs-json-invalid-not-number", "source": 'builtins.fromJSON "not a number"', "error": True, "error_contains": "parse error"},
    {"name": "rs-json-non-string", "source": "builtins.fromJSON 42", "error": True, "error_contains": "fromJSON"},
    {"name": "rs-json-empty-array", "source": 'builtins.fromJSON "[]"', "expect": []},
    {"name": "rs-json-empty-object", "source": 'builtins.fromJSON "{}"', "expect": {}},
    {"name": "rs-json-minus-zero-type", "source": 'builtins.typeOf (builtins.fromJSON "-0")', "expect": "float"},
    {"name": "rs-tojson-lambda", "source": "builtins.toJSON (x: x)", "error": True, "error_contains": "cannot serialize function"},
    {"name": "rs-tojson-lambda-list", "source": "builtins.toJSON [ (x: x) ]", "error": True, "error_contains": "cannot serialize function"},
    {"name": "rs-tojson-lambda-attrset", "source": "builtins.toJSON { f = (x: x); }", "error": True, "error_contains": "cannot serialize function"},
    {"name": "rs-tojson-builtin-partial", "source": "builtins.toJSON (builtins.add 1)", "error": True, "error_contains": "cannot serialize function"},
    {
        "name": "rs-tojson-context-string",
        "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toJSON "x${./p}"))',
        "expect": True,
    },
    {
        "name": "rs-tojson-context-attrset",
        "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toJSON { a = "x${./p}"; }))',
        "expect": True,
    },
    {
        "name": "rs-tojson-context-list-p1",
        "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.toJSON [ "x${./p1}" "y${./p2}" ]))',
        "expect": True,
    },
    {
        "name": "rs-tojson-context-list-p2",
        "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.toJSON [ "x${./p1}" "y${./p2}" ]))',
        "expect": True,
    },
    {"name": "rs-tojson-context-path", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toJSON ./p))', "expect": True},
    {"name": "rs-tojson-no-context", "source": "builtins.getContext (builtins.toJSON [ 1 2 3 ])", "expect": {}},
    {"name": "rs-toml-section", "source": 'builtins.fromTOML "[section]\\nkey = \\"val\\"\\nnum = 42"', "expect": {"section": {"key": "val", "num": 42}}},
    {"name": "rs-toml-top-level", "source": 'builtins.fromTOML "name = \\"pnix\\"\\nyear = 2026"', "expect": {"name": "pnix", "year": 2026}},
    {"name": "rs-toml-array", "source": 'builtins.fromTOML "vals = [1, 2, 3]"', "expect": {"vals": [1, 2, 3]}},
    {"name": "rs-toml-nested-table", "source": 'builtins.fromTOML "[a.b.c]\\nx = 1"', "expect": {"a": {"b": {"c": {"x": 1}}}}},
    {"name": "rs-toml-bool-float", "source": 'builtins.fromTOML "flag = true\\npi = 3.14"', "expect": {"flag": True, "pi": 3.14}},
    {"name": "rs-toml-invalid", "source": 'builtins.fromTOML "not [valid TOML"', "error": True, "error_contains": "parse error"},
    {"name": "rs-toml-non-string", "source": "builtins.fromTOML 42", "error": True, "error_contains": "expected string"},
    {"name": "rs-toml-empty", "source": 'builtins.fromTOML ""', "expect": {}},
    {"name": "rs-hash-sha256-hello", "source": 'builtins.hashString "sha256" "hello"', "expect": hashlib.sha256(b"hello").hexdigest()},
    {"name": "rs-hash-sha256-empty", "source": 'builtins.hashString "sha256" ""', "expect": hashlib.sha256(b"").hexdigest()},
    {"name": "rs-hash-sha256-raw-byte", "source": 'builtins.hashString "sha256" (builtins.substring 0 1 "가")', "expect": "3ad4e44a4306fb62b2df0ab7069c67b9a0f8c8eff9f1cba8e7f851199df720c9"},
    {"name": "rs-hash-sha512-length", "source": 'builtins.stringLength (builtins.hashString "sha512" "hello")', "expect": 128},
    {"name": "rs-hash-md5-hello", "source": 'builtins.hashString "md5" "hello"', "expect": "5d41402abc4b2a76b9719d911017c592"},
    {"name": "rs-hash-sha1-hello", "source": 'builtins.hashString "sha1" "hello"', "expect": "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"},
    {"name": "rs-hash-unknown", "source": 'builtins.hashString "blake42" "hello"', "error": True, "error_contains": "unsupported algorithm"},
    {"name": "rs-hash-unknown-supported-list", "source": 'builtins.hashString "blake42" "hello"', "error": True, "error_contains": "'md5', 'sha1', 'sha256', 'sha512'"},
    {"name": "rs-hash-unknown-before-data", "source": 'builtins.hashString "sha3" (throw "payload")', "error": True, "error_contains": "unsupported algorithm"},
    {"name": "rs-hash-raw-selector-before-data", "source": 'builtins.hashString (builtins.substring 0 1 "가") (throw "payload")', "error": True, "error_contains": "unsupported algorithm"},
    {"name": "rs-hash-algo-type-before-data", "source": 'builtins.hashString 1 (throw "payload")', "error": True, "error_contains": "algo"},
    {"name": "rs-hash-unknown-before-data-type", "source": 'builtins.hashString "sha3" 1', "error": True, "error_contains": "unsupported algorithm"},
    {"name": "rs-hash-data-non-string", "source": 'builtins.hashString "sha256" 42', "error": True, "error_contains": "must be string"},
    {"name": "rs-hash-algo-non-string", "source": 'builtins.hashString 42 "hello"', "error": True, "error_contains": "algo"},
    {"name": "rs-seq-success", "source": "builtins.seq 1 42", "expect": 42},
    {"name": "rs-seq-throw", "source": 'builtins.seq (throw "boom") 42', "error": True, "error_contains": "boom"},
    {"name": "rs-deepseq-list-throw", "source": 'builtins.deepSeq [ (throw "deep") ] 42', "error": True, "error_contains": "deep"},
    {"name": "rs-deepseq-list-success", "source": "builtins.deepSeq [ 1 2 [ 3 4 ] ] 99", "expect": 99},
    {
        "name": "rs-tofile-fake-path-shape",
        "source": 'builtins.match ".*pnix-nix-store.*test.txt" (builtins.toString (builtins.toFile "test.txt" "hello")) != null',
        "expect": True,
    },
    {"name": "rs-tofile-non-string-content", "source": 'builtins.toFile "test.txt" 42', "error": True, "error_contains": "must be string"},
    {"name": "rs-tryeval-success-int", "source": "builtins.tryEval 42", "expect": {"success": True, "value": 42}},
    {"name": "rs-tryeval-success-string", "source": 'builtins.tryEval "hi"', "expect": {"success": True, "value": "hi"}},
    {"name": "rs-tryeval-list-value", "source": "(builtins.tryEval [ 1 2 ]).value", "expect": [1, 2]},
    {"name": "rs-tryeval-throw", "source": 'builtins.tryEval (throw "boom")', "expect": {"success": False, "value": False}},
    {"name": "rs-tryeval-assert", "source": "builtins.tryEval (assert false; 42)", "expect": {"success": False, "value": False}},
    {"name": "rs-tryeval-div-zero", "source": "builtins.tryEval (1 / 0)", "error": True, "error_contains": "division by zero"},
    {"name": "rs-tryeval-mod-zero", "source": "builtins.tryEval (1 % 0)", "error": True, "error_contains": "modulo by zero"},
    {"name": "rs-tryeval-undefined", "source": "builtins.tryEval undefined_var", "error": True, "error_contains": "unknown variable"},
    {"name": "rs-tryeval-missing-attr", "source": "builtins.tryEval ({}.missing)", "error": True, "error_contains": "missing attr"},
    {"name": "rs-tryeval-type-error", "source": 'builtins.tryEval (1 + "x")', "error": True, "error_contains": "unsupported operand types"},
    {"name": "rs-tryeval-infinite-recursion", "source": "builtins.tryEval (let s = { x = s.x; }; in s.x)", "error": True, "error_contains": "infinite recursion"},
    {"name": "rs-tryeval-value-false-pattern", "source": 'let r = builtins.tryEval (throw "boom"); in if r.success then "ok" else "caught"', "expect": "caught"},
    {"name": "rs-tryeval-lazy-list-success", "source": '(builtins.tryEval [ (throw "side") 1 ]).success', "expect": True},
    {"name": "rs-tryeval-abort", "source": 'builtins.tryEval (abort "boom")', "error": True, "error_contains": "evaluation aborted"},
    {"name": "rs-tryeval-abort-through-attr", "source": '(builtins.tryEval (abort "deep")).success', "error": True, "error_contains": "evaluation aborted"},
    {"name": "rs-sort-ascending", "source": "builtins.sort (a: b: a < b) [ 3 1 2 ]", "expect": [1, 2, 3]},
    {"name": "rs-sort-descending", "source": "builtins.sort (a: b: a > b) [ 1 3 2 ]", "expect": [3, 2, 1]},
    {"name": "rs-sort-empty", "source": "builtins.sort (a: b: a < b) []", "expect": []},
    {"name": "rs-sort-singleton", "source": "builtins.sort (a: b: a < b) [ 42 ]", "expect": [42]},
    {"name": "rs-sort-non-bool-int", "source": "builtins.sort (a: b: 42) [ 1 2 ]", "error": True, "error_contains": "got int"},
    {"name": "rs-sort-non-bool-string", "source": 'builtins.sort (a: b: "yes") [ 1 2 ]', "error": True, "error_contains": "got string"},
    {"name": "rs-sort-non-list-int", "source": "builtins.sort (a: b: a < b) 42", "error": True, "error_contains": "second argument must be list"},
    {"name": "rs-sort-non-list-attrset", "source": "builtins.sort (a: b: a < b) { x = 1; }", "error": True, "error_contains": "got set"},
    {"name": "rs-sort-comparator-throw", "source": 'builtins.sort (a: b: throw "cmp") [ 1 2 ]', "error": True, "error_contains": "cmp"},
]


RUST_REGEX_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-regex-match-no-match", "source": 'builtins.match "abc" "xyz"', "expect": None},
    {"name": "rs-regex-match-no-captures", "source": 'builtins.match "abc" "abc"', "expect": []},
    {"name": "rs-regex-match-partial-no", "source": 'builtins.match "ab" "abc"', "expect": None},
    {"name": "rs-regex-match-dotstar", "source": 'builtins.match "ab.*" "abc"', "expect": []},
    {"name": "rs-regex-match-groups", "source": 'builtins.match "(a+)(b+)" "aaabb"', "expect": ["aaa", "bb"]},
    {"name": "rs-regex-match-optional-null", "source": 'builtins.match "(a)?(b)" "b"', "expect": [None, "b"]},
    {"name": "rs-regex-match-empty-empty", "source": 'builtins.match "" ""', "expect": []},
    {"name": "rs-regex-match-invalid", "source": 'builtins.match "[invalid" "x"', "error": True, "error_contains": "invalid regex"},
    {"name": "rs-regex-match-invalid-unclosed", "source": 'builtins.match "[invalid" "x"', "error": True, "error_contains": "unclosed"},
    {"name": "rs-regex-match-unicode-korean", "source": 'builtins.match "[가-힣]+" "안녕"', "expect": []},
    {"name": "rs-regex-match-unicode-capture", "source": 'builtins.match "([가-힣]+)-(.+)" "안녕-world"', "expect": ["안녕", "world"]},
    {"name": "rs-regex-split-no-match", "source": 'builtins.split "xyz" "abc"', "expect": ["abc"]},
    {"name": "rs-regex-split-basic", "source": 'builtins.split "[ab]" "abc"', "expect": ["", [], "", [], "c"]},
    {"name": "rs-regex-split-captures", "source": 'builtins.split "(a)(b)" "ab-ab"', "expect": ["", ["a", "b"], "-", ["a", "b"], ""]},
    {"name": "rs-regex-split-empty-pattern", "source": 'builtins.split "" "abc"', "error": True, "error_contains": "pattern cannot be empty"},
    {"name": "rs-regex-split-invalid", "source": 'builtins.split "[invalid" "x"', "error": True, "error_contains": "invalid regex"},
    {"name": "rs-regex-posix-space", "source": 'builtins.split "[[:space:]]+" "a b\\tc"', "expect": ["a", [], "b", [], "c"]},
]


# Fixtures below read a real, always-present repo file (this project's own
# todo.md) to exercise hashFile/readFile/pathExists/readDir against actual
# filesystem I/O. Resolved from this module's own location rather than a
# bare "todo.md" relative literal, which only worked when the gate happened
# to be invoked with pnix-hy/ (not the monorepo root, where these corpora
# are actually run from) as the process cwd.
_RUST_IO_FIXTURE_DIR = str(Path(__file__).resolve().parents[1])
_RUST_IO_FIXTURE_TODO = str((Path(__file__).resolve().parents[1] / "todo.md").resolve())

RUST_PATH_FS_IO_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-path-plus-path-total", "source": "builtins.typeOf (./foo + ./bar)", "expect": "path"},
    {"name": "rs-path-plus-string-basename", "source": 'builtins.baseNameOf (./foo + "/bar/baz")', "expect": "baz"},
    {
        "name": "rs-path-string-plus-path-context",
        "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext ("x" + ./p))',
        "expect": True,
    },
    {
        "name": "rs-path-context-string-plus-path-left",
        "source": 'builtins.hasAttr (builtins.toString ./a) (builtins.getContext ("x${./a}" + ./b))',
        "expect": True,
    },
    {
        "name": "rs-path-context-string-plus-path-right",
        "source": 'builtins.hasAttr (builtins.toString ./b) (builtins.getContext ("x${./a}" + ./b))',
        "expect": True,
    },
    {
        "name": "rs-path-plus-context-string-error",
        "source": './foo + "x${./p}"',
        "error": True,
        "error_contains": "unsafeDiscardStringContext",
    },
    {
        "name": "rs-path-plus-discarded-context",
        "source": 'builtins.typeOf (./foo + (builtins.unsafeDiscardStringContext "x${./p}"))',
        "expect": "path",
    },
    {"name": "rs-path-toPath-empty-error", "source": 'builtins.toPath ""', "error": True, "error_contains": "doesn't represent an absolute path"},
    {
        "name": "rs-pathExists-empty-error",
        "source": 'builtins.pathExists ""',
        "error": True,
        "error_contains": "empty string",
    },
    {
        "name": "rs-pathExists-int-old-message",
        "source": "builtins.pathExists 42",
        "error": True,
        "error_contains": "expected path or string",
    },
    {
        "name": "rs-pathExists-int-new-message",
        "source": "builtins.pathExists 42",
        "error": True,
        "error_contains": "expected string or path",
    },
    {
        "name": "rs-dirOf-string-context",
        "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.dirOf "x${./p}/file"))',
        "expect": True,
    },
    {
        "name": "rs-baseNameOf-string-context",
        "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.baseNameOf "x${./p}/file"))',
        "expect": True,
    },
    {
        "name": "rs-hashString-drops-context",
        "source": 'builtins.getContext (builtins.hashString "sha256" "x${./p}")',
        "expect": {},
    },
    {
        "name": "rs-hashString-rejects-algo-context",
        "source": 'builtins.hashString "sha256${./p}" "x"',
        "error": True,
        "error_contains": "not allowed to refer to a store path",
    },
    {
        "name": "rs-hashFile-md5-crypto",
        "source": f'builtins.hashFile "md5" "{_RUST_IO_FIXTURE_TODO}"',
        "error": True,
        "error_contains": "cryptographically broken",
    },
    {
        "name": "rs-hashFile-md5-single-quote",
        "source": f'builtins.hashFile "md5" "{_RUST_IO_FIXTURE_TODO}"',
        "error": True,
        "error_contains": "algorithm 'md5' is not supported",
    },
    {
        "name": "rs-hashFile-md5-backtick",
        "source": f'builtins.hashFile "md5" "{_RUST_IO_FIXTURE_TODO}"',
        "error": True,
        "error_contains": "`md5`",
    },
    {
        "name": "rs-hashFile-unknown",
        "source": f'builtins.hashFile "blake2" "{_RUST_IO_FIXTURE_TODO}"',
        "error": True,
        "error_contains": "unsupported algorithm 'blake2'",
    },
    {
        "name": "rs-hashFile-unknown-supported",
        "source": f'builtins.hashFile "blake2" "{_RUST_IO_FIXTURE_TODO}"',
        "error": True,
        "error_contains": "'sha256', 'sha512'",
    },
    {
        "name": "rs-hashFile-non-string-algo",
        "source": f'builtins.hashFile 42 "{_RUST_IO_FIXTURE_TODO}"',
        "error": True,
        "error_contains": "must be string",
    },
    {
        "name": "rs-hashFile-non-path-old-message",
        "source": 'builtins.hashFile "sha256" 42',
        "error": True,
        "error_contains": "expected path or string",
    },
    {
        "name": "rs-hashFile-non-path-new-message",
        "source": 'builtins.hashFile "sha256" 42',
        "error": True,
        "error_contains": "expected string or path",
    },
    {
        "name": "rs-hashFile-readFile-parity",
        "source": (
            f'builtins.hashFile "sha256" "{_RUST_IO_FIXTURE_TODO}" == '
            f'builtins.hashString "sha256" (builtins.readFile "{_RUST_IO_FIXTURE_TODO}")'
        ),
        "expect": True,
    },
]


RUST_PATH_NORMALIZATION_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-path-normalize-tostring-dotdot", "source": "builtins.toString ./a/../b", "expect": "./b"},
    {"name": "rs-path-normalize-tojson-dotdot", "source": "builtins.toJSON ./a/../b", "expect": '"./b"'},
    {"name": "rs-path-normalize-chain", "source": "builtins.toString ./a/b/c/../../d", "expect": "./a/d"},
    {"name": "rs-path-normalize-absolute", "source": "builtins.toString /abs/x/../y", "expect": "/abs/y"},
    {"name": "rs-path-normalize-unchanged", "source": "builtins.toString ./a/b", "expect": "./a/b"},
    {"name": "rs-dirOf-path-type", "source": "builtins.typeOf (builtins.dirOf ./a/b)", "expect": "path"},
    {"name": "rs-dirOf-path-normalized-root", "source": "builtins.toString (builtins.dirOf ./a/../b)", "expect": "."},
    {"name": "rs-dirOf-path-normalized-chain", "source": "builtins.toString (builtins.dirOf ./a/b/c/../../d)", "expect": "./a"},
    {"name": "rs-dirOf-path-normalized-absolute", "source": "builtins.toString (builtins.dirOf /abs/x/../y)", "expect": "/abs"},
    {"name": "rs-baseNameOf-path-normalized", "source": "builtins.baseNameOf ./a/../b", "expect": "b"},
    {"name": "rs-baseNameOf-chain-normalized", "source": "builtins.baseNameOf ./a/b/c/../../d", "expect": "d"},
    {"name": "rs-path-plus-string-normalized", "source": 'builtins.toString (./a + "/../b")', "expect": "./b"},
    {"name": "rs-dirOf-string-not-normalized", "source": 'builtins.dirOf "./a/../b"', "expect": "./a/.."},
    {"name": "rs-baseNameOf-string-double-slash", "source": 'builtins.baseNameOf "a//"', "expect": ""},
    {"name": "rs-path-eq-dotdot", "source": "./a/../b == ./b", "expect": True},
    {"name": "rs-path-eq-chain", "source": "./a/b/c/../../d == ./a/d", "expect": True},
    {"name": "rs-path-eq-multiple-dotdot", "source": "./a/../b/../c == ./c", "expect": True},
    {"name": "rs-path-eq-different", "source": "./a/../b == ./c", "expect": False},
    {"name": "rs-path-lt-normalized", "source": "./a/../b < ./c", "expect": True},
    {"name": "rs-path-le-equal-normalized", "source": "./a/../b <= ./b", "expect": True},
    {"name": "rs-path-neq-collapsed", "source": "./a/../b != ./b", "expect": False},
    {"name": "rs-path-eq-string-mismatch", "source": './foo == "./foo"', "expect": False},
    {"name": "rs-list-path-eq-normalized", "source": "[ ./a/../b ./c ] == [ ./b ./c ]", "expect": True},
    {"name": "rs-attr-path-eq-normalized", "source": "{ x = ./a/../b; } == { x = ./b; }", "expect": True},
]


RUST_PATH_CONTEXT_IO_CORPUS: list[dict[str, Any]] = [
    {
        "name": "rs-io-pathExists-context-missing",
        "source": 'builtins.pathExists "x${./p}/non-existent-dir-xyz"',
        "expect": False,
    },
    {"name": "rs-io-pathExists-abs-missing", "source": 'builtins.pathExists "/non-existent-dir-xyz"', "expect": False},
    {"name": "rs-io-pathExists-path-missing", "source": "builtins.pathExists ./non-existent-xyz", "expect": False},
    {
        "name": "rs-io-readDir-context-missing",
        "source": 'builtins.readDir "${./not-a-dir-xyz}"',
        "error": True,
        "error_contains": "builtins.readDir",
    },
    {
        "name": "rs-io-readFile-context-missing",
        "source": 'builtins.readFile "${./not-a-file-xyz}"',
        "error": True,
        "error_contains": "builtins.readFile",
    },
    {"name": "rs-io-toPath-context-type", "source": 'builtins.typeOf (builtins.toPath "${/tmp}")', "expect": "string"},
    {"name": "rs-io-storePath-context-type", "source": 'builtins.typeOf (builtins.storePath "${./local}")', "expect": "path"},
    {
        "name": "rs-io-readFileType-context-missing",
        "source": 'builtins.readFileType "${./not-a-file-xyz-zzz}"',
        "error": True,
        "error_contains": "builtins.readFileType",
    },
    {
        "name": "rs-io-readFile-null-guard",
        "source": "builtins.readFile null",
        "error": True,
        "error_contains": "expected string or path",
    },
    {
        "name": "rs-io-readDir-list-guard",
        "source": "builtins.readDir [ ]",
        "error": True,
        "error_contains": "expected string or path",
    },
    {
        "name": "rs-io-pathExists-concat-abs-missing",
        "source": 'let p = "/tmp/" + "non-existent-xyz-test-zzz"; in builtins.pathExists p',
        "expect": False,
    },
    {"name": "rs-io-string-plus-path-type", "source": 'let p = "prefix/" + ./subdir; in builtins.typeOf p', "expect": "string"},
    {
        "name": "rs-io-string-plus-path-pathExists",
        "source": 'let p = "prefix/" + ./subdir; in builtins.pathExists p',
        "expect": False,
    },
    {"name": "rs-io-pathExists-todo", "source": f'builtins.pathExists "{_RUST_IO_FIXTURE_TODO}"', "expect": True},
    {"name": "rs-io-readFileType-todo", "source": f'builtins.readFileType "{_RUST_IO_FIXTURE_TODO}"', "expect": "regular"},
    {
        "name": "rs-io-readFile-todo-prefix",
        "source": f'builtins.hasPrefix "# pnix-hy todo" (builtins.readFile "{_RUST_IO_FIXTURE_TODO}")',
        "expect": True,
    },
    {
        "name": "rs-io-readDir-current-has-todo",
        "source": f'builtins.hasAttr "todo.md" (builtins.readDir "{_RUST_IO_FIXTURE_DIR}")',
        "expect": True,
    },
    {"name": "rs-io-toFile-type", "source": 'builtins.typeOf (builtins.toFile "note.txt" "hello")', "expect": "path"},
    {"name": "rs-io-toFile-readFile", "source": 'builtins.readFile (builtins.toFile "note.txt" "hello")', "expect": "hello"},
    {"name": "rs-io-isPath-attr-lazy", "source": 'builtins.isPath { a = throw "payload"; }', "expect": False},
    {"name": "rs-io-isPath-list-lazy", "source": 'builtins.isPath [ (throw "payload") ]', "expect": False},
    {
        "name": "rs-io-isPath-top-tryeval",
        "source": '(builtins.tryEval (builtins.isPath (throw "top"))).success',
        "expect": False,
    },
]


RUST_UPDATE_PATH_ARITH_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-update-shallow-replaces", "source": "{ a = { x = 1; }; } // { a = { y = 2; }; }", "expect": {"a": {"y": 2}}},
    {"name": "rs-update-adds-key", "source": "{ a = 1; } // { b = 2; }", "expect": {"a": 1, "b": 2}},
    {"name": "rs-update-int-to-attrset", "source": "{ a = 1; } // { a = { x = 2; }; }", "expect": {"a": {"x": 2}}},
    {"name": "rs-update-attrset-to-int", "source": "{ a = { x = 1; }; } // { a = 2; }", "expect": {"a": 2}},
    {"name": "rs-update-empty-right", "source": "{ a = 1; b = 2; } // { }", "expect": {"a": 1, "b": 2}},
    {"name": "rs-update-empty-left", "source": "{ } // { a = 1; }", "expect": {"a": 1}},
    {"name": "rs-nested-path-merge-same-subtree", "source": "{ a.b = 1; a.c = 2; }.a", "expect": {"b": 1, "c": 2}},
    {"name": "rs-explicit-assign-then-path-merge", "source": "{ a = { b = 1; }; a.c = 2; }.a", "expect": {"b": 1, "c": 2}},
    {"name": "rs-two-explicit-attrsets-merge", "source": "{ a = { b = 1; }; a = { c = 2; }; }", "expect": {"a": {"b": 1, "c": 2}}},
    {"name": "rs-nested-deep-explicit-path-merge", "source": "{ a.b = { c = 1; }; a.b.d = 2; }.a.b", "expect": {"c": 1, "d": 2}},
    {"name": "rs-duplicate-leaf-path-errors", "source": "{ a.b = 1; a.b = 2; }", "error": True, "error_contains": "already defined"},
    {"name": "rs-path-conflict-non-attr-errors", "source": "{ a = 1; a.b = 2; }", "error": True, "error_contains": "non-attrset"},
    {"name": "rs-int-add-overflow", "source": "9223372036854775807 + 1", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-int-mul-overflow", "source": "10000000000 * 10000000000", "error": True, "error_contains": "integer overflow"},
    {"name": "rs-int-times-float-promotes", "source": "builtins.typeOf (9223372036854775807 * 2.0)", "expect": "float"},
    {"name": "rs-tojson-infinity-errors", "source": "builtins.toJSON (1.0e308 * 10.0)", "error": True, "error_contains": "cannot serialize"},
    {"name": "rs-tojson-infinity-in-attr-errors", "source": "builtins.toJSON { x = 1.0e308 * 10.0; }", "error": True, "error_contains": "cannot serialize"},
    {"name": "rs-tojson-finite-float", "source": "builtins.toJSON 3.5", "expect": "3.5"},
    {"name": "rs-tojson-zero-float", "source": "builtins.toJSON 0.0", "expect": "0.0"},
    {"name": "rs-isfinite-int", "source": "builtins.isFinite 42", "expect": True},
    {"name": "rs-isfinite-inf-false", "source": "builtins.isFinite (1.0e308 * 10.0)", "expect": False},
    {"name": "rs-isinf-inf-true", "source": "builtins.isInf (1.0e308 * 10.0)", "expect": True},
    {"name": "rs-isinf-int-false", "source": "builtins.isInf 42", "expect": False},
]


RUST_GUARD_MISC_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-guard-lang-version-six", "source": "builtins.langVersion", "expect": 6},
    {"name": "rs-guard-lang-version-type", "source": "builtins.typeOf builtins.langVersion", "expect": "int"},
    {
        "name": "rs-guard-lang-version-branch",
        "source": 'if builtins.langVersion >= 6 then "new" else "old"',
        "expect": "new",
    },
    {"name": "rs-guard-abort-string", "source": 'builtins.abort "boom"', "error": True, "error_contains": "evaluation aborted"},
    {"name": "rs-guard-abort-int", "source": "builtins.abort 42", "error": True, "error_contains": "builtins.abort: argument must be string, got 42"},
    {"name": "rs-guard-abort-list", "source": "builtins.abort [ 1 2 ]", "error": True, "error_contains": "builtins.abort: argument must be string"},
    {"name": "rs-guard-abort-null", "source": "builtins.abort null", "error": True, "error_contains": "builtins.abort: argument must be string"},
    {
        "name": "rs-guard-abort-attrset",
        "source": "builtins.abort { a = 1; }",
        "error": True,
        "error_contains": "builtins.abort: argument must be string",
    },
    {"name": "rs-guard-abort-int-tryeval", "source": "(builtins.tryEval (builtins.abort 42)).success", "error": True, "error_contains": "argument must be string"},
    {
        "name": "rs-guard-abort-string-tryeval-propagates",
        "source": 'builtins.tryEval (builtins.abort "stop")',
        "error": True,
        "error_contains": "evaluation aborted",
    },
    {"name": "rs-guard-with-int-unused", "source": "with 42; 1", "expect": 1},
    {"name": "rs-guard-with-string-unused", "source": 'with "hi"; 1', "expect": 1},
    {"name": "rs-guard-with-throw-unused", "source": 'with (throw "boom"); 1', "expect": 1},
    {"name": "rs-guard-with-int-lookup-error", "source": "with 42; foo", "error": True, "error_contains": "with: argument must be attrset, got int"},
    {
        "name": "rs-guard-with-outer-int-fallback-error",
        "source": "with 42; with { x = 99; }; missing",
        "error": True,
        "error_contains": "with: argument must be attrset",
    },
    {
        "name": "rs-guard-addctx-int",
        "source": 'builtins.addErrorContext 42 "value"',
        "error": True,
        "error_contains": "addErrorContext: context must be string, got int",
    },
    {
        "name": "rs-guard-addctx-null",
        "source": 'builtins.addErrorContext null "value"',
        "error": True,
        "error_contains": "addErrorContext: context must be string, got null",
    },
    {
        "name": "rs-guard-addctx-list",
        "source": 'builtins.addErrorContext [ ] "value"',
        "error": True,
        "error_contains": "addErrorContext: context must be string, got list",
    },
    {"name": "rs-guard-addctx-context-message", "source": 'builtins.addErrorContext "x${./p}" 42', "expect": 42},
    {
        "name": "rs-guard-unsafe-pos-mapattrs-null",
        "source": 'builtins.unsafeGetAttrPos "a" (builtins.mapAttrs (k: v: v) { a = 1; })',
        "expect": None,
    },
    {"name": "rs-guard-bitand-string", "source": 'builtins.bitAnd "x" 5', "error": True, "error_contains": "bitAnd"},
    {"name": "rs-guard-bitor-null", "source": "builtins.bitOr null 5", "error": True, "error_contains": "bitOr"},
    {"name": "rs-guard-bitxor-string", "source": 'builtins.bitXor 5 "y"', "error": True, "error_contains": "bitXor"},
    {
        "name": "rs-guard-bitops-happy",
        "source": "[ (builtins.bitAnd 12 10) (builtins.bitOr 12 10) (builtins.bitXor 12 10) ]",
        "expect": [8, 14, 6],
    },
    {
        "name": "rs-guard-append-value-string",
        "source": 'builtins.appendContext "x" { "/a" = "wrong"; }',
        "error": True,
        "error_contains": "attrset",
    },
    {
        "name": "rs-guard-append-value-int",
        "source": 'builtins.appendContext "x" { "/a" = 42; }',
        "error": True,
        "error_contains": "attrset",
    },
    {
        "name": "rs-guard-append-outputs-non-list",
        "source": 'builtins.appendContext "x" { "/a" = { outputs = "wrong"; }; }',
        "error": True,
        "error_contains": "list",
    },
    {
        "name": "rs-guard-append-outputs-int-element",
        "source": 'builtins.appendContext "x" { "/a" = { outputs = [ "out" 42 ]; }; }',
        "error": True,
        "error_contains": "string",
    },
    {
        "name": "rs-guard-append-path-int",
        "source": 'builtins.appendContext "x" { "/a" = { path = 1; }; }',
        "error": True,
        "error_contains": "'/a'.path must be bool, got int",
    },
    {
        "name": "rs-guard-append-alloutputs-int",
        "source": 'builtins.appendContext "x" { "/a" = { allOutputs = 1; }; }',
        "error": True,
        "error_contains": "'/a'.allOutputs must be bool, got int",
    },
    {
        "name": "rs-guard-append-unknown-lazy",
        "source": 'builtins.hasContext (builtins.appendContext "x" { "/a" = { path = true; futureField = throw "payload"; }; })',
        "expect": True,
    },
    {
        "name": "rs-guard-append-full-shape",
        "source": 'builtins.hasAttr "/a" (builtins.getContext (builtins.appendContext "x" { "/a" = { path = false; outputs = [ "out" ]; allOutputs = false; }; }))',
        "expect": True,
    },
    {"name": "rs-guard-hasattr-happy", "source": 'builtins.hasAttr "a" { a = 1; b = 2; }', "expect": True},
    {"name": "rs-guard-hasattr-value-int", "source": 'builtins.hasAttr "a" 42', "error": True, "error_contains": "attrset"},
    {"name": "rs-guard-hasattr-name-int", "source": "builtins.hasAttr 42 { a = 1; }", "error": True, "error_contains": "string"},
    {
        "name": "rs-guard-removeattrs-happy",
        "source": 'builtins.removeAttrs { a = 1; b = 2; c = 3; } [ "b" ]',
        "expect": {"a": 1, "c": 3},
    },
    {"name": "rs-guard-removeattrs-first-int", "source": 'builtins.removeAttrs 42 [ "x" ]', "error": True, "error_contains": "attrset"},
    {"name": "rs-guard-removeattrs-second-int", "source": "builtins.removeAttrs { a = 1; } 42", "error": True, "error_contains": "list"},
    {"name": "rs-guard-removeattrs-name-int", "source": "builtins.removeAttrs { a = 1; } [ 42 ]", "error": True, "error_contains": "string"},
    {"name": "rs-guard-concatlists-happy", "source": "builtins.concatLists [[1 2] [3 4] []]", "expect": [1, 2, 3, 4]},
    {"name": "rs-guard-concatlists-non-list", "source": "builtins.concatLists 42", "error": True, "error_contains": "list"},
    {
        "name": "rs-guard-concatlists-inner-non-list",
        "source": "builtins.concatLists [[1 2] 42 [3]]",
        "error": True,
        "error_contains": "list",
    },
    {"name": "rs-guard-if-string", "source": 'if "yes" then 1 else 2', "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-if-int", "source": "if 0 then 1 else 2", "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-and-left-string", "source": '"yes" && true', "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-and-right-string", "source": 'true && "yes"', "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-or-left-null", "source": "null || false", "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-impl-right-int", "source": "true -> 42", "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-not-null", "source": "!null", "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-assert-string", "source": 'assert "yes"; 42', "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-builtins-and-string", "source": 'builtins.and "yes" true', "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-builtins-or-int", "source": "builtins.or 42 false", "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-builtins-not-null", "source": "builtins.not null", "error": True, "error_contains": "expected bool"},
    {"name": "rs-guard-let-dup", "source": "let x = 1; x = 2; in x", "error": True, "error_contains": "more than once"},
    {
        "name": "rs-guard-let-inherit-dup",
        "source": "let inherit ({a=99;}) a; a = 1; in a",
        "error": True,
        "error_contains": "more than once",
    },
    {
        "name": "rs-guard-lambda-formal-dup",
        "source": "({ a, a }: a) { a = 1; }",
        "error": True,
        "error_contains": "duplicate formal",
    },
    {
        "name": "rs-guard-lambda-at-dup",
        "source": "(args@{ args }: args) { args = 1; }",
        "error": True,
        "error_contains": "duplicate formal",
    },
    {
        "name": "rs-guard-lambda-dup-before-arg",
        "source": '({ a, a }: a) (throw "should-not-fire")',
        "error": True,
        "error_contains": "duplicate formal",
    },
    {"name": "rs-guard-rec-attr-dup", "source": "rec { x = 1; x = 2; }", "error": True, "error_contains": "already defined"},
    {"name": "rs-guard-attr-literal-dup", "source": "{ a = 1; a = 2; }", "error": True, "error_contains": "already defined"},
    {
        "name": "rs-guard-inherit-assign-dup",
        "source": "let x = 99; in ({ inherit x; x = 1; }).x",
        "error": True,
        "error_contains": "already defined",
    },
]


RUST_UNSAFE_OUTPUT_DERIVATION_CORPUS: list[dict[str, Any]] = [
    {"name": "rs-unsafe-add-dep-type", "source": "builtins.typeOf builtins.unsafeAddOutputDependency", "expect": "lambda"},
    {"name": "rs-unsafe-add-dep-plain", "source": 'builtins.unsafeAddOutputDependency "x"', "expect": "x"},
    {
        "name": "rs-unsafe-add-dep-marker",
        "source": 'builtins.hasAttr ("!out!" + (builtins.toString ./p)) (builtins.getContext (builtins.unsafeAddOutputDependency "x${./p}"))',
        "expect": True,
    },
    {
        "name": "rs-unsafe-add-dep-idempotent",
        "source": 'builtins.length (builtins.attrNames (builtins.getContext (builtins.unsafeAddOutputDependency (builtins.unsafeAddOutputDependency "x${./p}"))))',
        "expect": 2,
    },
    {
        "name": "rs-unsafe-add-dep-int",
        "source": "builtins.unsafeAddOutputDependency 42",
        "error": True,
        "error_contains": "string",
    },
    {"name": "rs-unsafe-add-name-type", "source": "builtins.typeOf builtins.unsafeAddOutputName", "expect": "lambda"},
    {"name": "rs-unsafe-add-name-partial", "source": 'builtins.typeOf (builtins.unsafeAddOutputName "out")', "expect": "lambda"},
    {"name": "rs-unsafe-add-name-plain", "source": 'builtins.unsafeAddOutputName "out" "x"', "expect": "x"},
    {
        "name": "rs-unsafe-add-name-marker",
        "source": 'builtins.hasAttr ("!dev!" + (builtins.toString ./p)) (builtins.getContext (builtins.unsafeAddOutputName "dev" "x${./p}"))',
        "expect": True,
    },
    {
        "name": "rs-unsafe-add-name-int-name",
        "source": 'builtins.unsafeAddOutputName 42 "x"',
        "error": True,
        "error_contains": "string",
    },
    {
        "name": "rs-unsafe-add-name-int-str",
        "source": 'builtins.unsafeAddOutputName "out" 42',
        "error": True,
        "error_contains": "string",
    },
    {
        "name": "rs-unsafe-add-discard-round",
        "source": 'let v = builtins.unsafeDiscardOutputDependency (builtins.unsafeAddOutputDependency "x${./p}"); in builtins.hasAttr ("!out!" + (builtins.toString ./p)) (builtins.getContext v)',
        "expect": False,
    },
    {
        "name": "rs-derivation-type",
        "source": '(builtins.derivation { name = "x"; system = "x"; builder = "x"; }).type',
        "expect": "derivation",
    },
    {
        "name": "rs-derivation-name",
        "source": '(builtins.derivation { name = "myname"; system = "x"; builder = "x"; }).name',
        "expect": "myname",
    },
    {
        "name": "rs-derivation-outpath-shape",
        "source": 'builtins.match ".*derivation.*abc.*" ((builtins.derivation { name = "abc"; system = "x"; builder = "x"; }).outPath) != null',
        "expect": True,
    },
    {
        "name": "rs-derivation-attrnames-eq-strict",
        "source": 'builtins.attrNames (builtins.derivation { name = "x"; system = "x"; builder = "x"; }) == builtins.attrNames (builtins.derivationStrict { name = "x"; system = "x"; builder = "x"; })',
        "expect": True,
    },
    {
        "name": "rs-derivation-custom-field",
        "source": '(builtins.derivation { name = "x"; system = "x"; builder = "x"; myCustomField = 42; }).myCustomField',
        "expect": 42,
    },
    {
        "name": "rs-derivation-outpath-override",
        "source": '(builtins.derivation { name = "x"; system = "x"; builder = "x"; outPath = "/user/specified"; }).outPath',
        "expect": "/user/specified",
    },
    {
        "name": "rs-derivation-type-override",
        "source": '(builtins.derivation { name = "x"; system = "x"; builder = "x"; type = "custom"; }).type',
        "expect": "custom",
    },
    {
        "name": "rs-derivation-unnamed",
        "source": 'builtins.match ".*unnamed.*" ((builtins.derivation { system = "x"; builder = "x"; }).outPath) != null',
        "expect": True,
    },
    {"name": "rs-derivation-non-attr", "source": "builtins.derivation 42", "error": True, "error_contains": "attrset"},
    {
        "name": "rs-derivation-out-context",
        "source": 'builtins.hasAttr "!out!x" (builtins.getContext ((builtins.derivation { name = "x"; system = "x"; builder = "x"; }).outPath))',
        "expect": True,
    },
    {
        "name": "rs-derivation-drv-context",
        "source": 'builtins.hasAttr "!out!y" (builtins.getContext ((builtins.derivation { name = "y"; system = "x"; builder = "x"; }).drvPath))',
        "expect": True,
    },
    {
        "name": "rs-derivation-tojson-fields",
        "source": 'builtins.match ".*\\\"type\\\":\\\"derivation\\\".*" (builtins.toJSON (builtins.derivation { name = "n"; system = "s"; builder = "b"; })) != null',
        "expect": True,
    },
    {
        "name": "rs-derivation-isDerivation-missing",
        "source": 'builtins.isDerivation { type = "derivation"; }',
        "error": True,
        "error_contains": "isDerivation",
    },
    {
        "name": "rs-derivation-type-check",
        "source": 'let d = builtins.derivation { name = "x"; system = "x"; builder = "x"; }; in (d.type or null) == "derivation"',
        "expect": True,
    },
]


def rust_corpus_report(cases: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    """Run the Rust ground-truth corpus through pnix-hy's interpreter AND compiler.

    Value cases must match `expect` on both lanes; error cases must raise on both.
    `ready` is True when every case behaves as the full original evaluator does."""
    cases = (
        cases
        if cases is not None
        else RUST_EVAL_CORPUS
        + RUST_BUILTIN_CORPUS
        + RUST_OVERFLOW_CORPUS
        + RUST_FUNCTIONAL_LAZY_TYPE_CORPUS
        + RUST_COMPARE_VERSION_CORPUS
        + RUST_CYCLE_GUARD_CORPUS
        + RUST_JSON_TOML_DATA_CORPUS
        + RUST_REGEX_CORPUS
        + RUST_PATH_FS_IO_CORPUS
        + RUST_PATH_NORMALIZATION_CORPUS
        + RUST_PATH_CONTEXT_IO_CORPUS
        + RUST_UPDATE_PATH_ARITH_CORPUS
        + RUST_GUARD_MISC_CORPUS
        + RUST_UNSAFE_OUTPUT_DERIVATION_CORPUS
    )
    out: list[dict[str, Any]] = []
    agree = 0
    for case in cases:
        rec: dict[str, Any] = {"name": case["name"], "source": case["source"]}
        wants_error = bool(case.get("error"))
        interp_err = compile_err = None
        interp_val = compile_val = None
        try:
            interp_val = stable_data(eval_source(case["source"]))
        except Exception as exc:  # noqa: BLE001
            interp_err = str(exc)
        try:
            compile_val = stable_data(run_px_source(case["source"]))
        except Exception as exc:  # noqa: BLE001
            compile_err = str(exc)
        if wants_error:
            raised = interp_err is not None and compile_err is not None
            sub = case.get("error_contains")
            substr_ok = sub is None or (sub in (interp_err or "") and sub in (compile_err or ""))
            ok = raised and substr_ok
            rec.update(
                {
                    "kind": "error",
                    "ok": ok,
                    "raised": raised,
                    "error_contains_ok": substr_ok,
                    "interp_error": interp_err,
                    "compile_error": compile_err,
                }
            )
        else:
            want = stable_data(case["expect"])
            ok = interp_err is None and compile_err is None and interp_val == want and compile_val == want
            rec.update(
                {
                    "kind": "value",
                    "ok": ok,
                    "expect": want,
                    "interp": interp_val if interp_err is None else f"<error: {interp_err}>",
                    "compile": compile_val if compile_err is None else f"<error: {compile_err}>",
                }
            )
        agree += int(ok)
        out.append(rec)
    return {
        "schema": "pnix-hy.rust-corpus.v0",
        "ready": agree == len(cases),
        "source_file": "~/pnix/crates/pnix-eval/tests/eval_basics.rs",
        "count": len(cases),
        "agree": agree,
        "known_gaps": RUST_EVAL_KNOWN_GAPS,
        "cases": out,
    }


SELF_TEST_REPO_DIR = str(Path(__file__).resolve().parents[1])
SELF_TEST_TODO_PATH = str((Path(__file__).resolve().parents[1] / "todo.md").resolve())


SELF_TEST_CASES = [
    {"name": "uri-literal-basic", "source": "x:x", "expect": "x:x"},
    {"name": "uri-literal-maximal-body", "source": "a:%/?::@&=+$,-_.!~*'", "expect": "a:%/?::@&=+$,-_.!~*'"},
    {"name": "uri-literal-lambda-boundary", "source": "builtins.typeOf (x: x)", "expect": "lambda"},
    {"name": "arith", "source": "1 + 1", "expect": 2},
    {"name": "bp-if-string-fails", "source": '(builtins.tryEval (if "x" then 1 else 2)).success', "expect": False},
    {"name": "bp-and-left-int-fails", "source": "(builtins.tryEval (5 && true)).success", "expect": False},
    {"name": "bp-or-right-int-fails", "source": "(builtins.tryEval (false || 5)).success", "expect": False},
    {"name": "bp-impl-left-int-fails", "source": "(builtins.tryEval (1 -> true)).success", "expect": False},
    {"name": "ac-hasattr-value-int-fails", "source": '(builtins.tryEval (builtins.hasAttr "a" 42)).success', "expect": False},
    {"name": "ac-hasattr-name-int-fails", "source": "(builtins.tryEval (builtins.hasAttr 42 { a = 1; })).success", "expect": False},
    {"name": "ac-rmattrs-name-int-fails", "source": "(builtins.tryEval (builtins.removeAttrs { a = 1; } [ 42 ])).success", "expect": False},
    {"name": "ac-rmattrs-thunk-name-ok", "source": 'builtins.length (builtins.attrNames (builtins.removeAttrs { a = 1; b = 2; } [ ("b" + "") ]))', "expect": 1},
    {"name": "ac-concatlists-elem-int-fails", "source": "(builtins.tryEval (builtins.concatLists [1 2])).success", "expect": False},
    {"name": "ac-concatlists-thunk-list-ok", "source": "builtins.concatLists [ (if true then [1] else 0) [2] ]", "expect": [1, 2]},
    {"name": "aw-abort-int-fails", "source": "(builtins.tryEval (builtins.abort 42)).success", "expect": False},
    {"name": "aw-with-int-foo-fails", "source": "(builtins.tryEval (with 42; foo)).success", "expect": False},
    {"name": "aw-with-int-unused-ok", "source": "with 42; 1", "expect": 1},
    {"name": "aw-with-attrset-ok", "source": "with { a = 1; b = 2; }; a + b", "expect": 3},
    {"name": "aw-with-inner-wins-ok", "source": "with 42; with { x = 99; }; x", "expect": 99},
    {"name": "rsl-from-int-fails", "source": '(builtins.tryEval (builtins.replaceStrings 42 [ "X" ] "abc")).success', "expect": False},
    {"name": "rsl-to-string-fails", "source": '(builtins.tryEval (builtins.replaceStrings [ "a" ] "X" "abc")).success', "expect": False},
    {"name": "rsl-from-elem-int-fails", "source": '(builtins.tryEval (builtins.replaceStrings [ 1 ] [ "X" ] "abc")).success', "expect": False},
    {"name": "rsl-clean-ok", "source": 'builtins.replaceStrings [ "a" "b" ] [ "X" "Y" ] "abc"', "expect": "XYc"},
    {"name": "fg-fold-sum-ok", "source": "builtins.fold (a: b: a + b) 100 [ 1 2 3 ]", "expect": 106},
    {"name": "fg-fold-int-fails", "source": "(builtins.tryEval (builtins.fold (a: b: a + b) 100 42)).success", "expect": False},
    {"name": "fg-groupby-key-int-fails", "source": '(builtins.tryEval (builtins.groupBy (item: 42) [ "a" ])).success', "expect": False},
    {"name": "fg-groupby-nonlist-fails", "source": '(builtins.tryEval (builtins.groupBy (x: "k") 42)).success', "expect": False},
    {"name": "la-any-nonlist-fails", "source": "(builtins.tryEval (builtins.any (x: x > 0) 42)).success", "expect": False},
    {"name": "la-elem-nonlist-fails", "source": "(builtins.tryEval (builtins.elem 1 42)).success", "expect": False},
    {"name": "la-filter-nonlist-fails", "source": "(builtins.tryEval (builtins.filter (x: true) 42)).success", "expect": False},
    {"name": "la-foldr-nonlist-fails", "source": "(builtins.tryEval (builtins.foldr (a: b: a) 0 42)).success", "expect": False},
    {"name": "la-filter-happy-ok", "source": "builtins.filter (x: x > 1) [ 1 2 3 ]", "expect": [2, 3]},
    {"name": "ai-functionargs-int-fails", "source": "(builtins.tryEval (builtins.functionArgs 42)).success", "expect": False},
    {"name": "ai-attrnames-list-fails", "source": "(builtins.tryEval (builtins.attrNames [ 1 2 ])).success", "expect": False},
    {"name": "ai-getattr-missing-fails", "source": '(builtins.tryEval (builtins.getAttr "z" { a = 1; })).success', "expect": False},
    {"name": "ai-zip-elem-int-fails", "source": "(builtins.tryEval (builtins.zipAttrsWith (k: vs: vs) [ { a = 1; } 42 ])).success", "expect": False},
    {"name": "ai-getattr-happy-ok", "source": 'builtins.getAttr "a" { a = 42; }', "expect": 42},
    {"name": "ai-functionargs-happy-ok", "source": "builtins.length (builtins.attrNames (builtins.functionArgs ({ a, b }: a)))", "expect": 2},
    {"name": "bx-bitand-first-string-fails", "source": '(builtins.tryEval (builtins.bitAnd "x" 5)).success', "expect": False},
    {"name": "bx-addctx-int-fails", "source": '(builtins.tryEval (builtins.addErrorContext 42 "value")).success', "expect": False},
    {"name": "bx-bitxor-happy-ok", "source": "builtins.bitXor 12 10", "expect": 6},
    {"name": "bx-addctx-happy-ok", "source": 'builtins.addErrorContext "ctx" 42', "expect": 42},
    {"name": "acs-value-int-fails", "source": '(builtins.tryEval (builtins.appendContext "x" { "/a" = 42; })).success', "expect": False},
    {"name": "acs-path-int-fails", "source": '(builtins.tryEval (builtins.appendContext "x" { "/a" = { path = 1; }; })).success', "expect": False},
    {"name": "acs-outputs-elem-int-fails", "source": '(builtins.tryEval (builtins.appendContext "x" { "/a" = { outputs = [ "o" 42 ]; }; })).success', "expect": False},
    {"name": "acs-path-bool-ok", "source": 'builtins.hasContext (builtins.appendContext "x" { "/a" = { path = true; }; })', "expect": True},
    {"name": "ov-unaryneg-min-fails", "source": "(builtins.tryEval (let big = 9223372036854775807; m = 0 - big - 1; in -m)).success", "expect": False},
    {"name": "ov-neg-min-fails", "source": "(builtins.tryEval (let big = 9223372036854775807; m = 0 - big - 1; in builtins.neg m)).success", "expect": False},
    {"name": "ov-mod-min-neg1-fails", "source": "(builtins.tryEval (let big = 9223372036854775807; m = 0 - big - 1; in builtins.mod m (-1))).success", "expect": False},
    {"name": "ov-mod-zero-fails", "source": "(builtins.tryEval (builtins.mod 1 0)).success", "expect": False},
    {"name": "ov-i64min-build-ok", "source": "let big = 9223372036854775807; in 0 - big - 1", "expect": -9223372036854775808},
    {"name": "ov-neg-max-ok", "source": "builtins.neg 9223372036854775807", "expect": -9223372036854775807},
    {"name": "cs-concatstrings-nonlist-fails", "source": "(builtins.tryEval (builtins.concatStrings 42)).success", "expect": False},
    {"name": "cs-sep-nonstring-fails", "source": '(builtins.tryEval (builtins.concatStringsSep 42 [ "a" ])).success', "expect": False},
    {"name": "cs-concatstrings-happy-ok", "source": 'builtins.concatStrings [ "a" "b" "c" ]', "expect": "abc"},
    {"name": "cs-sep-happy-ok", "source": 'builtins.concatStringsSep "-" [ "a" "b" ]', "expect": "a-b"},
    {"name": "uo-adddep-int-fails", "source": "(builtins.tryEval (builtins.unsafeAddOutputDependency 42)).success", "expect": False},
    {"name": "uo-addname-first-int-fails", "source": '(builtins.tryEval (builtins.unsafeAddOutputName 42 "x")).success', "expect": False},
    {"name": "uo-adddep-happy-ok", "source": 'builtins.unsafeAddOutputDependency "x"', "expect": "x"},
    {"name": "uo-addname-happy-ok", "source": 'builtins.unsafeAddOutputName "out" "x"', "expect": "x"},
    {"name": "tc-self-ref-fails", "source": "(builtins.tryEval (let r = { __toString = self: builtins.toString self; }; in builtins.toString r)).success", "expect": False},
    {"name": "tc-outpath-string-ok", "source": 'builtins.toString { outPath = "/some/path"; }', "expect": "/some/path"},
    {"name": "dv-nonattrset-fails", "source": "(builtins.tryEval (builtins.derivation 42)).success", "expect": False},
    {"name": "sa-getenv-int-fails", "source": "(builtins.tryEval (builtins.getEnv 42)).success", "expect": False},
    {"name": "sa-tofile-name-int-fails", "source": '(builtins.tryEval (builtins.toFile 42 "x")).success', "expect": False},
    {"name": "sa-getenv-happy-ok", "source": 'builtins.getEnv "PNIX_NOPE"', "expect": ""},
    {"name": "op-int-plus-string-fails", "source": '(builtins.tryEval (42 + "hi")).success', "expect": False},
    {"name": "op-null-plus-int-fails", "source": "(builtins.tryEval (null + 1)).success", "expect": False},
    {"name": "op-plus-int-ok", "source": "1 + 2", "expect": 3},
    {"name": "op-plus-list-ok", "source": "[1] + [2]", "expect": [1, 2]},
    {"name": "tj-posinf-fails", "source": "(builtins.tryEval (builtins.toJSON (1.0e308 * 10.0))).success", "expect": False},
    {"name": "tj-nan-fails", "source": "(builtins.tryEval (let inf = 1.0e308 * 10.0; in builtins.toJSON (inf - inf))).success", "expect": False},
    {"name": "hf-sha256-len-ok", "source": 'builtins.stringLength (builtins.hashFile "sha256" (builtins.toFile "f" "hello"))', "expect": 64},
    {"name": "hf-md5-rejected-fails", "source": '(builtins.tryEval (builtins.hashFile "md5" (builtins.toFile "f" "x"))).success', "expect": False},
    {"name": "hf-missing-path-fails", "source": '(builtins.tryEval (builtins.hashFile "sha256" "/non-existent-xyz-test")).success', "expect": False},
    {"name": "let-recursive", "source": "let x = y + 1; y = 41; in x", "expect": 42},
    {"name": "lambda", "source": "(x: x + 1) 41", "expect": 42},
    {"name": "lambda-attr-pattern", "source": "({x}: x + 1) { x = 2; }", "expect": 3},
    {"name": "lambda-attr-default", "source": "({x, y ? 4}: x + y) { x = 3; }", "expect": 7},
    {"name": "lambda-attr-as-left", "source": "(a@{x}: a.x + x) { x = 5; }", "expect": 10},
    {"name": "lambda-attr-as-right", "source": "({x}@a: a.x + x) { x = 6; }", "expect": 12},
    {"name": "lambda-list-pattern", "source": "([x y]: x + y) [1 2]", "expect": 3},
    {"name": "lambda-list-rest-pattern", "source": "([x, y, ...rest]: x + y + builtins.length rest) [1 2 3 4]", "expect": 5},
    {"name": "attr-select", "source": "{ a = 1; b.c = 2; }.b.c", "expect": 2},
    {"name": "rec-attr", "source": "rec { x = 1; y = x + 41; }.y", "expect": 42},
    {"name": "rec-forward", "source": "rec { x = y + 1; y = 41; }.x", "expect": 42},
    {"name": "list-builtin", "source": "builtins.elemAt [ 10 20 30 ] 1", "expect": 20},
    {"name": "bool", "source": "if true && false then 1 else 2", "expect": 2},
    {"name": "implication-true-true", "source": "true -> true", "expect": True},
    {"name": "implication-true-false", "source": "true -> false", "expect": False},
    {"name": "implication-false-lazy", "source": "false -> builtins.elemAt [] 0", "expect": True},
    {"name": "merge", "source": "({ a = 1; } // { b = 2; }).b", "expect": 2},
    {"name": "merge-null-left", "source": "(null // { a = 1; }).a", "expect": 1},
    {"name": "merge-null-right", "source": "({ a = 1; } // null).a", "expect": 1},
    {"name": "merge-null-left-scalar", "source": "null // 42", "expect": 42},
    {"name": "has-attr", "source": "{ a = 1; } ? a", "expect": True},
    {"name": "plus-string", "source": '"a" + "b"', "expect": "ab"},
    {"name": "plus-list", "source": "[ 1 ] + [ 2 ]", "expect": [1, 2]},
    {"name": "plus-attrset", "source": "({ a = 1; } + { b = 2; }).b", "expect": 2},
    {"name": "compare-string", "source": '"a" < "b"', "expect": True},
    {"name": "compare-list", "source": "[ 1 2 ] < [ 1 3 ]", "expect": True},
    {"name": "float-plus", "source": "1 + 1.5", "expect": 2.5},
    {"name": "float-compare", "source": "1.0 < 1.5", "expect": True},
    {"name": "float-eq-int", "source": "2.0 == 2", "expect": True},
    {"name": "float-eq-int-f64-rounding", "source": "9007199254740993 == 9007199254740992.0", "expect": True},
    {"name": "float-eq-int-f64-nested", "source": "{ a = [ 9007199254740993 ]; } == { a = [ 9007199254740992.0 ]; }", "expect": True},
    {"name": "float-nan-scalar-not-equal", "source": "let n = (1.0e308 * 10.0) - (1.0e308 * 10.0); in n == n", "expect": False},
    {"name": "float-nan-shared-list-equal", "source": "let n = (1.0e308 * 10.0) - (1.0e308 * 10.0); in [ n ] == [ n ]", "expect": True},
    {"name": "float-nan-shared-attr-equal", "source": "let n = (1.0e308 * 10.0) - (1.0e308 * 10.0); in { a = n; } == { a = n; }", "expect": True},
    {"name": "float-toString-fixed", "source": "builtins.toString 1.23456789", "expect": "1.234568"},
    {"name": "float-exponent-integer", "source": "builtins.toString 1.0e3", "expect": "1000.000000"},
    {"name": "float-exponent-negative", "source": "builtins.toString 1.25e-3", "expect": "0.001250"},
    {"name": "float-exponent-empty-fraction", "source": "builtins.toString 1.e2", "expect": "100.000000"},
    {"name": "float-leading-dot", "source": "builtins.toString .5", "expect": "0.500000"},
    {"name": "float-leading-dot-exponent", "source": "builtins.toString .5e2", "expect": "50.000000"},
    {"name": "float-exponent-zero-underflow", "source": "builtins.toString 0.0e-400", "expect": "0.000000"},
    {"name": "float-exponent-underflow-error", "source": "1.0e-400", "error": True, "parse_error": True, "error_contains": "invalid float"},
    {"name": "float-exponent-subnormal-error", "source": "1.0e-308", "error": True, "parse_error": True, "error_contains": "invalid float"},
    {"name": "float-exponent-overflow-error", "source": "1.0e400", "error": True, "parse_error": True, "error_contains": "invalid float"},
    {"name": "float-toString-negative-zero", "source": "builtins.toString (-0.0)", "expect": "0.000000"},
    {"name": "float-toString-small-negative", "source": "builtins.toString (-0.0000001)", "expect": "-0.000000"},
    {"name": "float-toString-div-negative-zero", "source": "builtins.toString (0.0 / (-1.0))", "expect": "-0.000000"},
    {"name": "float-toString-mul-negative-zero", "source": "builtins.toString ((-1.0) * 0.0)", "expect": "-0.000000"},
    {"name": "float-toString-inf", "source": "builtins.toString (1.0e308 * 10.0)", "expect": "inf"},
    {"name": "float-toString-neg-inf", "source": "builtins.toString (-(1.0e308 * 10.0))", "expect": "-inf"},
    {"name": "float-toString-nan", "source": "let inf = 1.0e308 * 10.0; in builtins.toString (inf - inf)", "expect": "nan"},
    {"name": "float-mixed-builtin-add", "source": "builtins.add 9007199254740993 0.0", "expect": 9007199254740992.0},
    {"name": "float-mixed-builtin-sub", "source": "builtins.sub 9007199254740993 1.0", "expect": 9007199254740991.0},
    {"name": "float-mixed-builtin-mul", "source": "builtins.mul 9007199254740993 1.0", "expect": 9007199254740992.0},
    {"name": "float-mixed-builtin-div", "source": "builtins.div 9007199254740993 1.0", "expect": 9007199254740992.0},
    {"name": "float-mixed-builtin-less-rounding-left", "source": "builtins.lessThan 9007199254740993 9007199254740993.0", "expect": False},
    {"name": "float-mixed-builtin-less-rounding-right", "source": "builtins.lessThan 9007199254740992.0 9007199254740993", "expect": False},
    {"name": "compare-eq-list-shared-lambda", "source": "let f = x: x; in [ f ] == [ f ]", "expect": True},
    {"name": "compare-eq-attr-shared-lambda", "source": "let f = x: x; in { a = f; } == { a = f; }", "expect": True},
    {"name": "compare-elem-shared-lambda", "source": "let f = x: x; in builtins.elem f [ f ]", "expect": True},
    {"name": "compare-eq-same-list-lambda", "source": "let l = [ (x: x) ]; in l == l", "expect": True},
    {"name": "compare-eq-same-attr-lambda", "source": "let a = { f = x: x; }; in a == a", "expect": True},
    {"name": "compare-eq-distinct-list-lambda", "source": "[ (x: x) ] == [ (x: x) ]", "expect": False},
    {"name": "compare-eq-same-list-throw", "source": "let l = [ (throw \"x\") ]; in (builtins.tryEval (l == l)).success", "expect": False},
    {"name": "compare-eq-same-attr-throw", "source": "let a = { f = throw \"x\"; }; in (builtins.tryEval (a == a)).success", "expect": False},
    {"name": "compare-eq-nested-same-attr-throw", "source": "let a = { f = throw \"x\"; }; in [ a ] == [ a ]", "expect": True},
    {"name": "compare-eq-alias-chain", "source": "let f = x: x; g = h: [ h ]; in (g f) == (g f)", "expect": True},
    {"name": "compare-less-alias-chain", "source": "let f = x: x; g = h: [ h 0 ]; in (g f) < (g f)", "expect": False},
    {"name": "compare-less-list-shared-lambda", "source": "let f = x: x; in [ f ] < [ f ]", "expect": False},
    {"name": "compare-less-list-shared-nan", "source": "let n = (1.0e308 * 10.0) - (1.0e308 * 10.0); in [ n ] < [ n ]", "expect": False},
    {"name": "compare-less-list-distinct-nan", "source": "let inf = 1.0e308 * 10.0; a = inf - inf; b = inf - inf; in [ a 0 ] < [ b 1 ]", "expect": False},
    {"name": "compare-le-list-distinct-nan", "source": "let inf = 1.0e308 * 10.0; a = inf - inf; b = inf - inf; in [ a 0 ] <= [ b (-1) ]", "expect": True},
    {"name": "compare-eq-lambda-self", "source": "let f = x: x; in f == f", "expect": False},
    {"name": "compare-eq-attr-lambda", "source": "{ a = 1; f = x: x; } == { a = 1; f = x: x; }", "expect": False},
    {"name": "compare-version-pre-release", "source": 'builtins.compareVersions "1.0pre1" "1.0"', "expect": -1},
    {"name": "compare-version-plus-revision", "source": 'builtins.compareVersions "1.0" "1.0+rev"', "expect": -1},
    {"name": "split-version-plus-revision", "source": 'builtins.splitVersion "1.0+rev"', "expect": ["1", "0", "+rev"]},
    {"name": "parse-drv-first-hyphen", "source": '(builtins.parseDrvName "a-1-b-2").name', "expect": "a"},
    {"name": "version-split-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.splitVersion "1.0${./p}") 0))', "expect": True},
    {"name": "version-parse-context", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.parseDrvName "hello-1.0${./p}").version)', "expect": True},
    {"name": "eq-cycle-tryeval", "source": "let r = { a = r; }; in (builtins.tryEval (r == r)).success", "expect": True},
    {"name": "lt-cycle-tryeval", "source": "let r = [ r ]; in (builtins.tryEval (r < r)).success", "expect": True},
    {"name": "elem-cycle-tryeval", "source": "let r = { a = r; }; s = { a = s; }; in (builtins.tryEval (builtins.elem r [ s ])).success", "expect": False},
    {"name": "compare-version-guard", "source": '(builtins.tryEval (builtins.compareVersions 1 "1.0")).success', "expect": False},
    {"name": "force-cycle-tryeval", "source": "let s = { x = s.x; }; in (builtins.tryEval s.x).success", "expect": False},
    {"name": "tojson-cycle-tryeval", "source": "let r = { a = r; }; in (builtins.tryEval (builtins.toJSON r)).success", "expect": False},
    {"name": "deepseq-cycle-tryeval", "source": "let r = [ r ]; in (builtins.tryEval (builtins.deepSeq r 99)).success", "expect": False},
    {"name": "interp-cycle-tryeval", "source": 'let s = { __toString = self: "${s}"; }; in (builtins.tryEval "${s}").success', "expect": False},
    {"name": "interp-tostring-chain", "source": 'let a = { __toString = self: "from-a"; }; b = { __toString = self: a; }; in "[${b}]"', "expect": "[from-a]"},
    {"name": "json-fromjson-overflow-tryeval", "source": '(builtins.tryEval (builtins.fromJSON "9223372036854775808")).success', "expect": False},
    {"name": "json-minus-zero-type", "source": 'builtins.typeOf (builtins.fromJSON "-0")', "expect": "float"},
    {"name": "tojson-lambda-tryeval", "source": "(builtins.tryEval (builtins.toJSON [ (x: x) ])).success", "expect": False},
    {"name": "tojson-context-list", "source": 'builtins.hasAttr (builtins.toString ./p2) (builtins.getContext (builtins.toJSON [ "x${./p1}" "y${./p2}" ]))', "expect": True},
    {"name": "tojson-non-id-keys", "source": 'builtins.toJSON { "not-id" = 1; "sp ace" = 2; }', "expect": '{"not-id":1,"sp ace":2}'},
    {"name": "tojson-control-backspace", "source": 'builtins.toJSON (builtins.fromJSON "\\"a\\\\bb\\"")', "expect": '"a\\bb"'},
    {"name": "tojson-control-formfeed", "source": 'builtins.toJSON (builtins.fromJSON "\\"a\\\\fb\\"")', "expect": '"a\\fb"'},
    {"name": "tojson-control-u0001", "source": 'builtins.toJSON (builtins.fromJSON "\\"a\\\\u0001b\\"")', "expect": '"a\\u0001b"'},
    {"name": "toml-invalid-tryeval", "source": '(builtins.tryEval (builtins.fromTOML "not [valid TOML")).success', "expect": False},
    {"name": "hash-md5-tryeval", "source": '(builtins.tryEval (builtins.hashString "md5" "hello")).success', "expect": True},
    {"name": "hash-md5-digest", "source": 'builtins.hashString "md5" "hello"', "expect": "5d41402abc4b2a76b9719d911017c592"},
    {"name": "hash-sha1-digest", "source": 'builtins.hashString "sha1" "hello"', "expect": "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"},
    {"name": "hash-sha256-raw-byte", "source": 'builtins.hashString "sha256" (builtins.substring 0 1 "가")', "expect": "3ad4e44a4306fb62b2df0ab7069c67b9a0f8c8eff9f1cba8e7f851199df720c9"},
    {"name": "hash-unknown-before-data", "source": 'builtins.hashString "sha3" (throw "payload")', "error": True, "error_contains": "unsupported algorithm"},
    {"name": "hash-raw-selector-before-data", "source": 'builtins.hashString (builtins.substring 0 1 "가") (throw "payload")', "error": True, "error_contains": "unsupported algorithm"},
    {"name": "hash-algo-type-before-data", "source": 'builtins.hashString 1 (throw "payload")', "error": True, "error_contains": "algo"},
    {"name": "hash-unknown-before-data-type", "source": 'builtins.hashString "sha3" 1', "error": True, "error_contains": "unsupported algorithm"},
    {"name": "hash-sha512-length", "source": 'builtins.stringLength (builtins.hashString "sha512" "hello")', "expect": 128},
    {"name": "sort-second-arg-tryeval", "source": "(builtins.tryEval (builtins.sort (a: b: a < b) 42)).success", "expect": False},
    {"name": "sort-nonbool-tryeval", "source": "(builtins.tryEval (builtins.sort (a: b: 42) [ 1 2 ])).success", "expect": False},
    {"name": "tryeval-lazy-list-success", "source": '(builtins.tryEval [ (throw "side") 1 ]).success', "expect": True},
    {"name": "tofile-fake-path-shape", "source": 'builtins.match ".*pnix-nix-store.*test.txt" (builtins.toString (builtins.toFile "test.txt" "hello")) != null', "expect": True},
    {"name": "regex-invalid-tryeval", "source": '(builtins.tryEval (builtins.match "[invalid" "x")).success', "expect": False},
    {"name": "regex-unicode-capture", "source": 'builtins.match "([가-힣]+)-(.+)" "안녕-world"', "expect": ["안녕", "world"]},
    {"name": "regex-split-adjacent", "source": 'builtins.split "[ab]" "abc"', "expect": ["", [], "", [], "c"]},
    {"name": "path-plus-path-total", "source": "builtins.typeOf (./foo + ./bar)", "expect": "path"},
    {"name": "path-plus-context-string-tryeval", "source": '(builtins.tryEval (./foo + "x${./p}")).success', "expect": False},
    {
        "name": "string-plus-path-context",
        "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext ("x" + ./p))',
        "expect": True,
    },
    {
        "name": "dirOf-string-context",
        "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.dirOf "x${./p}/file"))',
        "expect": True,
    },
    {
        "name": "baseNameOf-string-context",
        "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.baseNameOf "x${./p}/file"))',
        "expect": True,
    },
    {
        "name": "hashString-drops-context",
        "source": 'builtins.getContext (builtins.hashString "sha256" "x${./p}")',
        "expect": {},
    },
    {"name": "hashString-rejects-algo-context", "source": 'builtins.hashString "sha256${./p}" "x"', "error": True, "error_contains": "not allowed to refer to a store path"},
    {
        "name": "hashFile-readFile-parity-abs",
        "source": f'builtins.hashFile "sha256" "{SELF_TEST_TODO_PATH}" == builtins.hashString "sha256" (builtins.readFile "{SELF_TEST_TODO_PATH}")',
        "expect": True,
    },
    {"name": "path-empty-tryeval", "source": '(builtins.tryEval (builtins.pathExists "")).success', "expect": False},
    {
        "name": "hashFile-md5-tryeval",
        "source": f'(builtins.tryEval (builtins.hashFile "md5" "{SELF_TEST_TODO_PATH}")).success',
        "expect": False,
    },
    {
        "name": "path-context-pathExists-missing",
        "source": 'builtins.pathExists "x${./p}/non-existent-dir-xyz"',
        "expect": False,
    },
    {"name": "path-context-toPath-type", "source": 'builtins.typeOf (builtins.toPath "${/tmp}")', "expect": "string"},
    {
        "name": "io-readFile-missing-tryeval",
        "source": '(builtins.tryEval (builtins.readFile "${./not-a-file-xyz}")).success',
        "expect": False,
    },
    {
        "name": "io-readDir-missing-tryeval",
        "source": '(builtins.tryEval (builtins.readDir "${./not-a-dir-xyz}")).success',
        "expect": False,
    },
    {
        "name": "io-readFileType-missing-tryeval",
        "source": '(builtins.tryEval (builtins.readFileType "${./not-a-file-xyz-zzz}")).success',
        "expect": False,
    },
    {
        "name": "io-readDir-repo-has-todo",
        "source": f'builtins.hasAttr "todo.md" (builtins.readDir "{SELF_TEST_REPO_DIR}")',
        "expect": True,
    },
    {"name": "io-readFileType-todo-abs", "source": f'builtins.readFileType "{SELF_TEST_TODO_PATH}"', "expect": "regular"},
    {
        "name": "io-readFile-todo-prefix-abs",
        "source": f'builtins.hasPrefix "# pnix-hy todo" (builtins.readFile "{SELF_TEST_TODO_PATH}")',
        "expect": True,
    },
    {
        "name": "io-toFile-readFile-roundtrip",
        "source": 'builtins.readFile (builtins.toFile "note.txt" "hello")',
        "expect": "hello",
    },
    {"name": "isPath-attr-lazy", "source": 'builtins.isPath { a = throw "payload"; }', "expect": False},
    {"name": "isPath-list-lazy", "source": 'builtins.isPath [ (throw "payload") ]', "expect": False},
    {"name": "isPath-top-tryeval", "source": '(builtins.tryEval (builtins.isPath (throw "top"))).success', "expect": False},
    {"name": "guard-langVersion-type", "source": "builtins.typeOf builtins.langVersion", "expect": "int"},
    {"name": "guard-abort-int-tryeval", "source": "(builtins.tryEval (builtins.abort 42)).success", "expect": False},
    {"name": "guard-with-int-lookup-tryeval", "source": "(builtins.tryEval (with 42; foo)).success", "expect": False},
    {"name": "guard-addctx-int-tryeval", "source": '(builtins.tryEval (builtins.addErrorContext 42 "value")).success', "expect": False},
    {
        "name": "guard-unsafe-pos-mapattrs-null",
        "source": 'builtins.unsafeGetAttrPos "a" (builtins.mapAttrs (k: v: v) { a = 1; })',
        "expect": None,
    },
    {
        "name": "guard-bitops-happy",
        "source": "[ (builtins.bitAnd 12 10) (builtins.bitOr 12 10) (builtins.bitXor 12 10) ]",
        "expect": [8, 14, 6],
    },
    {
        "name": "guard-append-path-int-tryeval",
        "source": '(builtins.tryEval (builtins.appendContext "x" { "/a" = { path = 1; }; })).success',
        "expect": False,
    },
    {
        "name": "guard-append-unknown-lazy",
        "source": 'builtins.hasContext (builtins.appendContext "x" { "/a" = { path = true; futureField = throw "payload"; }; })',
        "expect": True,
    },
    {"name": "guard-hasAttr-value-int-tryeval", "source": '(builtins.tryEval (builtins.hasAttr "a" 42)).success', "expect": False},
    {"name": "guard-hasAttr-name-int-tryeval", "source": "(builtins.tryEval (builtins.hasAttr 42 { a = 1; })).success", "expect": False},
    {
        "name": "guard-removeAttrs-second-int-tryeval",
        "source": "(builtins.tryEval (builtins.removeAttrs { a = 1; } 42)).success",
        "expect": False,
    },
    {
        "name": "guard-concatLists-inner-int-tryeval",
        "source": "(builtins.tryEval (builtins.concatLists [[1 2] 42 [3]])).success",
        "expect": False,
    },
    {"name": "guard-if-string-tryeval", "source": '(builtins.tryEval (if "yes" then 1 else 2)).success', "expect": False},
    {"name": "guard-builtins-and-string-tryeval", "source": '(builtins.tryEval (builtins.and "yes" true)).success', "expect": False},
    {
        "name": "guard-lambda-formal-dup-tryeval",
        "source": "(builtins.tryEval (({ a, a }: a) { a = 1; })).success",
        "expect": False,
    },
    {
        "name": "guard-lambda-at-dup-tryeval",
        "source": "(builtins.tryEval ((args@{ args }: args) { args = 1; })).success",
        "expect": False,
    },
    {
        "name": "guard-lambda-dup-before-arg-tryeval",
        "source": '(builtins.tryEval (({ a, a }: a) (throw "should-not-fire"))).success',
        "expect": False,
    },
    {
        "name": "unsafe-add-output-dep-marker",
        "source": 'builtins.hasAttr ("!out!" + (builtins.toString ./p)) (builtins.getContext (builtins.unsafeAddOutputDependency "x${./p}"))',
        "expect": True,
    },
    {
        "name": "unsafe-add-output-name-marker",
        "source": 'builtins.hasAttr ("!dev!" + (builtins.toString ./p)) (builtins.getContext (builtins.unsafeAddOutputName "dev" "x${./p}"))',
        "expect": True,
    },
    {
        "name": "unsafe-add-output-name-int-tryeval",
        "source": '(builtins.tryEval (builtins.unsafeAddOutputName 42 "x")).success',
        "expect": False,
    },
    {
        "name": "unsafe-add-output-discard-roundtrip",
        "source": 'let v = builtins.unsafeDiscardOutputDependency (builtins.unsafeAddOutputDependency "x${./p}"); in builtins.hasAttr ("!out!" + (builtins.toString ./p)) (builtins.getContext v)',
        "expect": False,
    },
    {
        "name": "derivation-attrnames-eq-strict",
        "source": 'builtins.attrNames (builtins.derivation { name = "x"; system = "x"; builder = "x"; }) == builtins.attrNames (builtins.derivationStrict { name = "x"; system = "x"; builder = "x"; })',
        "expect": True,
    },
    {
        "name": "derivation-type-override",
        "source": '(builtins.derivation { name = "x"; system = "x"; builder = "x"; type = "custom"; }).type',
        "expect": "custom",
    },
    {
        "name": "derivation-non-attr-tryeval",
        "source": "(builtins.tryEval (builtins.derivation 42)).success",
        "expect": False,
    },
    {
        "name": "derivation-outpath-context",
        "source": 'builtins.hasAttr "!out!x" (builtins.getContext ((builtins.derivation { name = "x"; system = "x"; builder = "x"; }).outPath))',
        "expect": True,
    },
    {
        "name": "derivation-type-check",
        "source": 'let d = builtins.derivation { name = "x"; system = "x"; builder = "x"; }; in (d.type or null) == "derivation"',
        "expect": True,
    },
    {"name": "path-normalize-tostring", "source": "builtins.toString ./a/../b", "expect": "./b"},
    {"name": "path-normalize-tojson", "source": "builtins.toJSON ./a/../b", "expect": '"./b"'},
    {"name": "path-dirOf-type", "source": "builtins.typeOf (builtins.dirOf ./a/b)", "expect": "path"},
    {"name": "path-dirOf-normalized", "source": "builtins.toString (builtins.dirOf ./a/../b)", "expect": "."},
    {"name": "path-eq-normalized", "source": "./a/../b == ./b", "expect": True},
    {"name": "path-lt-normalized", "source": "./a/../b < ./c", "expect": True},
    {"name": "path-string-mismatch", "source": './foo == "./foo"', "expect": False},
    {"name": "path-string-dirOf-not-normalized", "source": 'builtins.dirOf "./a/../b"', "expect": "./a/.."},
    {"name": "path-baseName-double-slash", "source": 'builtins.baseNameOf "a//"', "expect": ""},
    {"name": "attr-explicit-then-path-merge", "source": "{ a = { b = 1; }; a.c = 2; }.a", "expect": {"b": 1, "c": 2}},
    {"name": "attr-two-explicit-merge", "source": "{ a = { b = 1; }; a = { c = 2; }; }", "expect": {"a": {"b": 1, "c": 2}}},
    {"name": "attr-duplicate-leaf-tryeval", "source": "(builtins.tryEval { a.b = 1; a.b = 2; }).success", "expect": False},
    {"name": "attr-nonattr-path-conflict-tryeval", "source": "(builtins.tryEval { a = 1; a.b = 2; }).success", "expect": False},
    {"name": "tojson-inf-tryeval", "source": "(builtins.tryEval (builtins.toJSON (1.0e308 * 10.0))).success", "expect": False},
    {"name": "builtin-typeOf-float", "source": "builtins.typeOf 1.5", "expect": "float"},
    {"name": "builtin-isFloat", "source": "builtins.isFloat 1.5", "expect": True},
    {"name": "with-basic", "source": "with { x = 42; }; x", "expect": 42},
    {"name": "with-nested-shadow", "source": "with { x = 1; }; with { x = 99; }; x", "expect": 99},
    {"name": "with-let-wins", "source": "with { x = 1; }; let x = 99; in x", "expect": 99},
    {"name": "with-lexical-wins", "source": "let x = 2; in with { x = 1; }; x", "expect": 2},
    {"name": "assert-pass", "source": "assert 1 < 2; 99", "expect": 99},
    {"name": "block-comment-whitespace", "source": "1 /* ignored */ + 2", "expect": 3},
    {"name": "block-comment-first-close", "source": "1 /*/*/ + 2", "expect": 3},
    {"name": "select-default-missing", "source": "{ a = 1; }.b or 99", "expect": 99},
    {"name": "select-default-expr", "source": "{ a = 1; }.b or (1 + 2)", "expect": 3},
    {"name": "select-default-chain", "source": '(123).bla or null.foo or "xyzzy"', "expect": "xyzzy"},
    {"name": "index-list", "source": "[1 2 3][1]", "expect": 2},
    {"name": "index-attrset", "source": '{ a = 7; }["a"]', "expect": 7},
    {"name": "path-type", "source": "builtins.typeOf ./foo", "expect": "path"},
    {"name": "path-isPath", "source": "builtins.isPath ./foo", "expect": True},
    {"name": "path-value", "source": "./foo", "expect": "./foo"},
    {"name": "path-absolute-isPath", "source": "builtins.isPath /tmp/pnix-hy-path", "expect": True},
    {"name": "path-home-isPath", "source": "builtins.isPath ~/pnix-hy-path", "expect": True},
    {"name": "path-search-type", "source": "builtins.typeOf <nixpkgs>", "expect": "path"},
    {"name": "path-search-toString", "source": "builtins.toString <nixpkgs>", "expect": "<nixpkgs>"},
    {"name": "path-interp-isPath", "source": 'let name = "bar"; in builtins.isPath ./foo/${name}', "expect": True},
    {"name": "path-interp-baseNameOf", "source": 'let name = "bar"; in builtins.baseNameOf ./foo/${name}', "expect": "bar"},
    {"name": "dynamic-select", "source": 'let k = "a"; in { a = 1; }.${k}', "expect": 1},
    {"name": "dynamic-select-default", "source": 'let k = "b"; in { a = 1; }.${k} or 9', "expect": 9},
    {"name": "dynamic-attr-key", "source": 'let name = "a"; in { ${name} = 1; }.a', "expect": 1},
    {"name": "dynamic-rec-attr-key", "source": 'rec { ${name} = 1; name = "a"; }.a', "expect": 1},
    {"name": "dynamic-attr-key-pos-null", "source": 'let name = "a"; in builtins.unsafeGetAttrPos "a" { ${name} = 1; }', "expect": None},
    {"name": "list-items-no-apply", "source": "let x = 3; in [ x x x ]", "expect": [3, 3, 3]},
    {"name": "concatMap-list-items-no-apply", "source": "builtins.concatMap (x: [x x]) [1 2 3]", "expect": [1, 1, 2, 2, 3, 3]},
    {"name": "regex-posix-space-split", "source": 'builtins.filter (x: builtins.isString x) (builtins.split "[[:space:]]+" "  one  two\\tthree   four ")', "expect": ["", "one", "two", "three", "four", ""]},
    {"name": "nixpkgs-foldAttrs-dynamic-key", "source": "let foldAttrs = op: nul: list_of_attrs: builtins.foldl' (acc: as: builtins.foldl' (acc2: name: acc2 // { ${name} = op (as.${name}) (acc2.${name} or nul); }) acc (builtins.attrNames as)) {} list_of_attrs; in foldAttrs (item: acc: acc + item) 0 [ { x = 1; y = 10; } { x = 2; y = 20; } { x = 3; } ]", "expect": {"x": 6, "y": 30}},
    {"name": "dynamic-hasattr", "source": 'let k = "a"; in { a = 1; } ? ${k}', "expect": True},
    {"name": "dynamic-hasattr-path", "source": 'let k = "b"; in { "a.b.c" = 2; } ? a.${k}.c', "expect": False},
    {"name": "dynamic-hasattr-number", "source": 'let k = 1; in { "a.1" = 2; } ? a.${k}', "expect": False},
    {"name": "hasattr-quoted-dot", "source": 'let a = { "x.y" = 1; }; in a ? "x.y"', "expect": True},
    {"name": "quoted-dot-key-select", "source": '{ "x.y" = 1; }."x.y"', "expect": 1},
    {"name": "quoted-dot-key-names", "source": 'builtins.attrNames { "x.y" = 1; }', "expect": ["x.y"]},
    {"name": "quoted-dot-key-not-nested", "source": '{ "a.b.c" = 2; } ? a.b.c', "expect": False},
    {"name": "inherit-plain-attrset", "source": "let x = 5; in { inherit x; }.x", "expect": 5},
    {"name": "inherit-scope-attrset", "source": "let s = { a = 1; b = 2; }; in ({ inherit (s) a b; }).b", "expect": 2},
    {"name": "inherit-scope-quoted-key", "source": 'let s = { "weird key" = 9; }; in ({ inherit (s) "weird key"; })."weird key"', "expect": 9},
    {"name": "inherit-dynamic-literal-name", "source": 'let x = 4; in { inherit ${"x"}; }.x', "expect": 4},
    {"name": "inherit-scope-lazy-unused", "source": 'let s = builtins.throw "boom"; in ({ inherit (s) a; }).b or 99', "expect": 99},
    {"name": "inherit-let", "source": "let s = { a = 7; }; inherit (s) a; in a", "expect": 7},
    {"name": "inherit-chain-let", "source": 'let s = { a = 7; b = builtins.throw "side"; }; in (let inherit (s) a; in (let inherit a; in a))', "expect": 7},
    {"name": "inherit-rec-outer", "source": "let x = 5; in (rec { inherit x; y = x + 1; }).y", "expect": 6},
    {"name": "inherit-rec-scope", "source": 'let s = { a = 10; quirk = builtins.throw "side"; }; in (rec { inherit (s) a; b = a + 1; }).b', "expect": 11},
    {"name": "nested-let-path", "source": "let a.b = 1; a.c = 2; in a.b + a.c", "expect": 3},
    {"name": "nested-let-recursive-path", "source": "let a.b = 1; a.c = a.b + 2; in a.c", "expect": 3},
    {"name": "top-level-builtin-alias", "source": "let ys = map (x: x * 2) [1 2 3]; in elemAt ys 1", "expect": 4},
    {"name": "top-level-list-builtins", "source": "let ys = append (cons 0 [1 2]) [3]; in foldl (acc: x: acc + x) 0 ys", "expect": 6},
    {"name": "match-literal", "source": "match 1 with | 0 => 2 | _ => 3", "expect": 3},
    {"name": "match-list", "source": "match [1 2] with | [x, y] => x + y | _ => 0", "expect": 3},
    {"name": "match-attrset", "source": "match { a = 1; } with | { a = x } => x | _ => 0", "expect": 1},
    {"name": "match-constructor", "source": "match Some(1) with | Some(x) => x | _ => 0", "expect": 1},
    {"name": "match-guard-fallthrough", "source": "match 2 with | x if x > 2 => 9 | x if x == 2 => x + 1 | _ => 0", "expect": 3},
    {"name": "match-guard-attrset", "source": "match { a = 3; } with | { a = x } if x > 2 => x | _ => 0", "expect": 3},
    {"name": "match-guard-false-default", "source": "match 1 with | x if false => null | _ => 5", "expect": 5},
    {"name": "match-guard-null-body", "source": "match 1 with | x if true => null | _ => 5", "expect": None},
    {"name": "str-plain", "source": '"hello"', "expect": "hello"},
    {"name": "str-escape", "source": '"a\\tb"', "expect": "a\tb"},
    {"name": "str-dollar-literal", "source": '"price $5"', "expect": "price $5"},
    {"name": "str-indented-basic", "source": "''hello\nworld''", "expect": "hello\nworld"},
    {"name": "str-indented-strip", "source": "''\n  hello\n  world\n''", "expect": "hello\nworld\n"},
    {"name": "str-indented-leading-whitespace-only", "source": "''  \n  hello\n''", "expect": "hello\n"},
    {"name": "str-indented-quote-escape", "source": "''hello''''world''", "expect": "hello'''world"},
    {"name": "str-indented-dollar-escape", "source": "''literal ''${notvar}''", "expect": "literal ${notvar}"},
    {"name": "str-indented-backslash-escapes", "source": "''a''\\tb''\\n''", "expect": "a\tb\n"},
    {"name": "str-indented-interp", "source": "let name = \"world\"; in ''\n  hi ${name}\n''", "expect": "hi world\n"},
    {"name": "str-interp", "source": 'let name = "world"; in "hi ${name}!"', "expect": "hi world!"},
    {"name": "str-interp-concat", "source": 'let a = "x"; b = "y"; in "${a}-${b}"', "expect": "x-y"},
    {"name": "str-interp-rec", "source": 'rec { who = "pnix"; msg = "hi ${who}"; }.msg', "expect": "hi pnix"},
    {"name": "str-interp-nested", "source": 'let x = "a"; in "[${ "<${x}>" }]"', "expect": "[<a>]"},
    {"name": "str-interp-nested-indented", "source": "\"before ${''inner } content''} after\"", "expect": "before inner } content after"},
    {"name": "str-interp-block-comment", "source": '"value ${builtins.toString (1 /* } */ + 2)}"', "expect": "value 3"},
    {"name": "str-interp-toString", "source": '"n=${builtins.toString (1 + 1)}"', "expect": "n=2"},
    {"name": "str-placeholder", "source": '"session ${sid} ctx ${cid}"', "expect": "session ${sid} ctx ${cid}"},
    {"name": "str-escape-interp", "source": '"literal \\${notvar}"', "expect": "literal ${notvar}"},
    {"name": "str-interp-tostring-attr", "source": '"greeting=${ { __toString = self: "hello"; } }"', "expect": "greeting=hello"},
    {"name": "str-interp-tostring-self", "source": '"${ { __toString = self: self.label; label = "abc"; } }"', "expect": "abc"},
    {"name": "str-interp-outpath-attr", "source": '"path=${ { outPath = "/nix/store/x"; } }"', "expect": "path=/nix/store/x"},
    {"name": "str-interp-tostring-priority", "source": '"${ { __toString = _: "from-toString"; outPath = "from-outPath"; } }"', "expect": "from-toString"},
    {"name": "str-interp-outpath-nested", "source": '"${ { outPath = { __toString = _: "deep"; }; } }"', "expect": "deep"},
    {"name": "builtin-elem-true", "source": "builtins.elem 2 [ 1 2 3 ]", "expect": True},
    {"name": "builtin-elem-false", "source": "builtins.elem 5 [ 1 2 3 ]", "expect": False},
    {"name": "builtin-any", "source": "builtins.any (x: x > 2) [ 1 2 3 ]", "expect": True},
    {"name": "builtin-all-true", "source": "builtins.all (x: x > 0) [ 1 2 3 ]", "expect": True},
    {"name": "builtin-all-false", "source": "builtins.all (x: x > 1) [ 1 2 3 ]", "expect": False},
    {"name": "builtin-concatLists", "source": "builtins.length (builtins.concatLists [ [ 1 2 ] [ 3 4 ] ])", "expect": 4},
    {"name": "builtin-concatMap", "source": "builtins.length (builtins.concatMap (n: builtins.genList (i: n) n) [ 1 2 3 ])", "expect": 6},
    {"name": "builtin-genList", "source": "builtins.elemAt (builtins.genList (i: i * i) 4) 3", "expect": 9},
    {"name": "builtin-genList-zero", "source": "builtins.genList (i: i) 0", "expect": []},
    {"name": "builtin-attrValues", "source": "builtins.elemAt (builtins.attrValues { a = 1; b = 2; c = 3; }) 1", "expect": 2},
    {"name": "builtin-mapAttrs", "source": "(builtins.mapAttrs (n: v: v + 1) { a = 1; b = 2; }).b", "expect": 3},
    {"name": "builtin-sort", "source": "builtins.elemAt (builtins.sort (a: b: a < b) [ 3 1 2 ]) 0", "expect": 1},
    {"name": "builtin-stringLength", "source": 'builtins.stringLength "hello"', "expect": 5},
    {"name": "builtin-stringLength-utf8", "source": 'builtins.stringLength "é"', "expect": 2},
    {"name": "builtin-substring", "source": 'builtins.substring 1 3 "hello"', "expect": "ell"},
    {"name": "builtin-substring-utf8", "source": 'builtins.substring 0 3 "héllo"', "expect": "hé"},
    {"name": "builtin-hasPrefix", "source": 'builtins.hasPrefix "he" "hello"', "expect": True},
    {"name": "builtin-hasSuffix", "source": 'builtins.hasSuffix "lo" "hello"', "expect": True},
    {"name": "builtin-replaceStrings", "source": 'builtins.replaceStrings [ "a" "c" ] [ "X" "Z" ] "abc"', "expect": "XbZ"},
    {"name": "builtin-replaceStrings-empty", "source": 'builtins.replaceStrings [""] ["X"] "abc"', "expect": "XaXbXcX"},
    {"name": "builtin-concatStringsSep", "source": 'builtins.concatStringsSep "-" [ "a" "b" "c" ]', "expect": "a-b-c"},
    {"name": "builtin-concatStrings-empty", "source": 'builtins.concatStrings []', "expect": ""},
    {"name": "builtin-concatStrings", "source": 'builtins.concatStrings [ "a" "b" "c" ]', "expect": "abc"},
    {"name": "builtin-listToAttrs", "source": '(builtins.listToAttrs [ { name = "a"; value = 1; } { name = "a"; value = 2; } ]).a', "expect": 1},
    {"name": "builtin-removeAttrs", "source": 'builtins.removeAttrs { a = 1; b = 2; c = 3; } ["b"]', "expect": {"a": 1, "c": 3}},
    {"name": "builtin-attrByPath-hit", "source": 'builtins.attrByPath ["a" "b"] 9 { a.b = 3; }', "expect": 3},
    {"name": "builtin-attrByPath-miss", "source": 'builtins.attrByPath ["a" "z"] 9 { a.b = 3; }', "expect": 9},
    {"name": "builtin-getAttrs", "source": 'builtins.getAttrs ["b" "a"] { a = 1; b = 2; c = 3; }', "expect": {"a": 1, "b": 2}},
    {"name": "builtin-filterAttrs", "source": '(builtins.filterAttrs (name: value: name == "keep" && value > 0) { keep = 1; drop = 0; }).keep', "expect": 1},
    {"name": "builtin-intersectAttrs", "source": '(builtins.intersectAttrs { a = 1; b = 2; } { b = 9; c = 3; }).b', "expect": 9},
    {"name": "builtin-catAttrs", "source": 'builtins.catAttrs "x" [ { x = 1; } { y = 2; } { x = 3; } ]', "expect": [1, 3]},
    {"name": "builtin-groupBy", "source": 'builtins.length ((builtins.groupBy (x: if x < 3 then "small" else "big") [1 2 3 4]).small)', "expect": 2},
    {"name": "builtin-partition", "source": "builtins.length ((builtins.partition (x: x < 3) [1 2 3 4]).right)", "expect": 2},
    {"name": "builtin-genericClosure-chain", "source": "builtins.map (x: x.key) (builtins.genericClosure { startSet = [ { key = 0; } ]; operator = item: if item.key < 2 then [ { key = item.key + 1; } ] else [ ]; })", "expect": [0, 1, 2]},
    {"name": "builtin-genericClosure-dedupe", "source": "builtins.length (builtins.genericClosure { startSet = [ { key = 0; } { key = 0; } ]; operator = item: if item.key < 2 then [ { key = item.key + 1; } ] else [ ]; })", "expect": 3},
    {"name": "builtin-genericClosure-composite-key", "source": 'builtins.length (builtins.genericClosure { startSet = [ { key = { a = 1; }; value = "x"; } { key = { a = 1; }; value = "y"; } ]; operator = item: [ ]; })', "expect": 1},
    {"name": "builtin-functionArgs", "source": "builtins.functionArgs ({ x, y ? 1, ... }: x + y)", "expect": {"x": False, "y": True}},
    {"name": "builtin-functionArgs-native", "source": "builtins.functionArgs builtins.map", "expect": {}},
    {"name": "builtin-zipAttrsWith", "source": 'let r = builtins.zipAttrsWith (name: values: name + ":" + (builtins.toString (builtins.length values))) [ { a = 1; b = 2; } { a = 3; c = 4; } ]; in r.a', "expect": "a:2"},
    {"name": "builtin-match", "source": 'builtins.match "a(b+)" "abbb"', "expect": ["bbb"]},
    {"name": "builtin-match-miss", "source": 'builtins.match "a(b+)" "ccc"', "expect": None},
    {"name": "builtin-split", "source": 'builtins.split "," "a,b,c"', "expect": ["a", [], "b", [], "c"]},
    {"name": "builtin-fromJSON", "source": '(builtins.fromJSON "{\\"a\\":1,\\"b\\":[true,null]}").b', "expect": [True, None]},
    {"name": "builtin-fromJSON-i64-max", "source": 'builtins.fromJSON "9223372036854775807"', "expect": 9223372036854775807},
    {"name": "builtin-fromJSON-exp-float", "source": 'builtins.fromJSON "1e3"', "expect": 1000.0},
    {"name": "builtin-fromTOML-simple", "source": '(builtins.fromTOML "name = \\"pnix\\"\\nyear = 2026").name', "expect": "pnix"},
    {"name": "builtin-fromTOML-int", "source": '(builtins.fromTOML "name = \\"pnix\\"\\nyear = 2026").year', "expect": 2026},
    {"name": "builtin-fromTOML-section", "source": '(builtins.fromTOML "[section]\\nkey = \\"val\\"\\nnum = 42").section.num', "expect": 42},
    {"name": "builtin-fromTOML-array", "source": '(builtins.fromTOML "vals = [1, 2, 3]").vals', "expect": [1, 2, 3]},
    {"name": "builtin-fromTOML-nested-section", "source": '(builtins.fromTOML "[a.b.c]\\nx = 1").a.b.c.x', "expect": 1},
    {"name": "builtin-fromTOML-bool", "source": '(builtins.fromTOML "flag = true\\npi = 3.14").flag', "expect": True},
    {"name": "builtin-fromTOML-float", "source": '(builtins.fromTOML "flag = true\\npi = 3.14").pi', "expect": 3.14},
    {"name": "builtin-fromTOML-empty", "source": 'builtins.fromTOML ""', "expect": {}},
    {"name": "builtin-schemaNormalize-default", "source": 'builtins.schemaNormalize (rec { string = { kind = "string"; }; root = { kind = "record"; fields = { name = string; enabled = { kind = "bool"; default = true; }; }; optional = [ "enabled" ]; }; }) { name = "demo"; }', "expect": {"enabled": True, "name": "demo"}},
    {"name": "builtin-schemaValidate-ok", "source": '(builtins.schemaValidate { kind = "record"; fields = { name = { kind = "string"; }; }; } { name = "ok"; }).success', "expect": True},
    {"name": "builtin-schemaValidate-nested-root", "source": '(builtins.schemaValidate { kind = "list"; elem = { root = { kind = "string"; }; }; } [ "a" "b" ]).ok', "expect": True},
    {"name": "builtin-schemaValidate-string-fail", "source": '(builtins.schemaValidate { kind = "string"; } 42).ok', "expect": False},
    {"name": "builtin-schemaExplain-string", "source": 'builtins.hasPrefix "root.name: type: expected string" (builtins.schemaExplain { kind = "record"; fields = { name = { kind = "string"; }; }; } { name = 1; })', "expect": True},
    {"name": "builtin-xmlParse-kind", "source": '(builtins.xmlParse "<root a=\\"1\\"><child>text</child></root>").kind', "expect": "element"},
    {"name": "builtin-xmlParse-name", "source": '(builtins.xmlParse "<root a=\\"1\\"><child>text</child></root>").name', "expect": "root"},
    {"name": "builtin-xmlEmit-roundtrip", "source": 'builtins.xmlEmit { kind = "element"; name = "root"; attrs = { a = "1"; }; children = [ { kind = "element"; name = "child"; children = [ { kind = "text"; value = "text"; } ]; } ]; }', "expect": '<root a="1"><child>text</child></root>'},
    {"name": "builtin-xmlEmit-empty", "source": 'builtins.xmlEmit { kind = "element"; name = "a"; attrs = {}; children = []; }', "expect": "<a/>"},
    {"name": "builtin-htmlParse-document", "source": '(builtins.htmlParse "<div class=\\"test\\">Hello</div>").kind', "expect": "document"},
    {"name": "builtin-htmlEmit-roundtrip", "source": 'builtins.htmlEmit (builtins.htmlParse "<div class=\\"test\\">Hello</div>")', "expect": '<div class="test">Hello</div>'},
    {"name": "builtin-tryEval-ok", "source": "builtins.tryEval (1 + 2)", "expect": {"success": True, "value": 3}},
    {"name": "builtin-tryEval-fail", "source": 'builtins.tryEval (builtins.throw "boom")', "expect": {"success": False, "value": False}},
    {"name": "builtin-derivation-type", "source": '(builtins.derivation { name = "x"; system = "x"; builder = "x"; }).type', "expect": "derivation"},
    {"name": "builtin-derivation-name", "source": '(builtins.derivation { name = "myname"; system = "x"; builder = "x"; }).name', "expect": "myname"},
    {"name": "builtin-derivation-outPath", "source": '(builtins.derivation { name = "abc"; system = "x"; builder = "x"; }).outPath', "expect": "/pnix-placeholder/derivation/abc"},
    {"name": "builtin-derivation-override", "source": '(builtins.derivation { name = "x"; outPath = "/user/specified"; }).outPath', "expect": "/user/specified"},
    {"name": "builtin-derivationStrict-unnamed", "source": '(builtins.derivationStrict { system = "x"; builder = "x"; }).outPath', "expect": "/pnix-placeholder/derivation/unnamed"},
    {"name": "builtin-pathExists-true", "source": f'builtins.pathExists "{SELF_TEST_TODO_PATH}"', "expect": True},
    {"name": "builtin-pathExists-false", "source": f'builtins.pathExists "{SELF_TEST_TODO_PATH}.missing"', "expect": False},
    {"name": "builtin-readFile", "source": f'builtins.hasPrefix "# pnix-hy todo" (builtins.readFile "{SELF_TEST_TODO_PATH}")', "expect": True},
    {"name": "builtin-readFileType", "source": f'builtins.readFileType "{SELF_TEST_TODO_PATH}"', "expect": "regular"},
    {"name": "builtin-readDir", "source": f'(builtins.readDir "{SELF_TEST_REPO_DIR}")."todo.md"', "expect": "regular"},
    {"name": "builtin-toFile-readFile", "source": 'builtins.readFile (builtins.toFile "pnix-hy-test.txt" "hello world")', "expect": "hello world"},
    {"name": "builtin-hashString-sha256", "source": 'builtins.hashString "sha256" "hello"', "expect": hashlib.sha256(b"hello").hexdigest()},
    {"name": "builtin-hashFile-readFile-parity", "source": f'builtins.hashFile "sha256" "{SELF_TEST_TODO_PATH}" == builtins.hashString "sha256" (builtins.readFile "{SELF_TEST_TODO_PATH}")', "expect": True},
    {"name": "builtin-baseNameOf", "source": f'builtins.baseNameOf "{SELF_TEST_TODO_PATH}"', "expect": "todo.md"},
    {"name": "builtin-dirOf", "source": f'builtins.dirOf "{SELF_TEST_TODO_PATH}"', "expect": str(Path(SELF_TEST_TODO_PATH).parent)},
    {"name": "builtin-toPath-isPath", "source": f'builtins.isPath (builtins.toPath "{SELF_TEST_TODO_PATH}")', "expect": False},
    {"name": "builtin-storePath-isPath", "source": f'builtins.isPath (builtins.storePath "{SELF_TEST_TODO_PATH}")', "expect": True},
    {"name": "builtin-getEnv-missing", "source": 'builtins.getEnv "PNIX_HY_SELF_TEST_SHOULD_NOT_EXIST_9C4C2E6D"', "expect": ""},
    {"name": "builtin-placeholder", "source": 'builtins.placeholder "out"', "expect": "/pnix-placeholder/out"},
    {"name": "builtin-break", "source": "builtins.break 42", "expect": 42},
    {"name": "builtin-warn", "source": 'builtins.warn "hello" 7', "expect": 7},
    {"name": "builtin-traceVerbose", "source": 'builtins.traceVerbose "quiet" 9', "expect": 9},
    {"name": "builtin-addErrorContext", "source": 'builtins.addErrorContext "ctx" 42', "expect": 42},
    {"name": "builtin-unsafeGetAttrPos-null", "source": 'builtins.unsafeGetAttrPos "z" { a = 1; }', "expect": None},
    {"name": "builtin-curPos-line", "source": "__curPos.line", "expect": 1},
    {"name": "builtin-curPos-shadow-column", "source": "let __curPos = 1; in __curPos.column", "expect": 22},
    {"name": "builtin-unsafeGetAttrPos-line", "source": '(builtins.unsafeGetAttrPos "a" {\n  a = 1;\n}).line', "expect": 2},
    {"name": "builtin-unsafeGetAttrPos-column", "source": '(builtins.unsafeGetAttrPos "a" {\n  a = 1;\n}).column', "expect": 3},
    {"name": "builtin-unsafeGetAttrPos-nested-column", "source": '(builtins.unsafeGetAttrPos "b" ({ a.b = 1; }.a)).column', "expect": 37},
    {"name": "builtin-unsafeGetAttrPos-generated-null", "source": 'builtins.unsafeGetAttrPos "a" (builtins.listToAttrs [ { name = "a"; value = 1; } ])', "expect": None},
    {"name": "builtin-bool-aliases", "source": "builtins.and true (builtins.not false) && builtins.or false true", "expect": True},
    {"name": "builtin-comparison-aliases", "source": 'builtins.eq "ab" ("a" + "b") && builtins.le 2 2 && builtins.gt 3 2 && builtins.ge 3 3 && builtins.lt 1 2', "expect": True},
    {"name": "builtin-mod", "source": "builtins.mod 17 5", "expect": 2},
    {"name": "builtin-mod-negative-left", "source": "(-10) % 3", "expect": -1},
    {"name": "builtin-mod-negative-right", "source": "10 % (-3)", "expect": 1},
    {"name": "builtin-mod-float", "source": "10.5 % 3", "expect": 1.5},
    {"name": "builtin-neg", "source": "builtins.neg 3", "expect": -3},
    {"name": "builtin-abs", "source": "builtins.abs (-4)", "expect": 4},
    {"name": "builtin-bitAnd", "source": "builtins.bitAnd 12 10", "expect": 8},
    {"name": "builtin-bitOr", "source": "builtins.bitOr 12 10", "expect": 14},
    {"name": "builtin-bitXor", "source": "builtins.bitXor 12 10", "expect": 6},
    {"name": "builtin-pow", "source": "builtins.pow 2 5", "expect": 32},
    {"name": "builtin-pow-3-39-exact", "source": "builtins.pow 3 39", "expect": 4052555153018976267},
    {"name": "builtin-pow-overflow-float-type", "source": "builtins.typeOf (builtins.pow 2 63)", "expect": "float"},
    {"name": "builtin-pow-negative-exp", "source": "builtins.pow 2 (-1)", "expect": 0.5},
    {"name": "builtin-sqrt", "source": "builtins.sqrt 25", "expect": 5.0},
    {"name": "builtin-floor", "source": "builtins.floor 3.9", "expect": 3},
    {"name": "builtin-ceil", "source": "builtins.ceil 3.1", "expect": 4},
    {"name": "builtin-floor-large", "source": "builtins.floor 1.0e18", "expect": 1000000000000000000},
    {"name": "builtin-ceil-negative", "source": "builtins.ceil (-3.2)", "expect": -3},
    {"name": "builtin-floor-int-f64-exact", "source": "builtins.floor 9007199254740994", "expect": 9007199254740994},
    {"name": "builtin-ceil-int-f64-exact", "source": "builtins.ceil (-9007199254740994)", "expect": -9007199254740994},
    {"name": "builtin-floor-int-f64-precision", "source": "builtins.floor 9007199254740993", "error": True, "error_contains": "precision"},
    {"name": "builtin-ceil-int-f64-range", "source": "builtins.ceil 9223372036854775807", "error": True, "error_contains": "outside i64 range"},
    {"name": "builtin-exp", "source": "builtins.exp 0", "expect": 1.0},
    {"name": "builtin-ln", "source": "builtins.ln 1", "expect": 0.0},
    {"name": "builtin-log", "source": "builtins.log 1", "expect": 0.0},
    {"name": "builtin-sin", "source": "builtins.sin 0", "expect": 0.0},
    {"name": "builtin-cos", "source": "builtins.cos 0", "expect": 1.0},
    {"name": "builtin-tan", "source": "builtins.tan 0", "expect": 0.0},
    {"name": "builtin-atan2", "source": "builtins.atan2 0 1", "expect": 0.0},
    {"name": "builtin-lessThan", "source": "builtins.lessThan 1.5 2.0", "expect": True},
    {"name": "builtin-add", "source": "builtins.add 1 2.5", "expect": 3.5},
    {"name": "builtin-sub", "source": "builtins.sub 10 3", "expect": 7},
    {"name": "builtin-mul", "source": "builtins.mul 4 5", "expect": 20},
    {"name": "builtin-div", "source": "builtins.div 20 4", "expect": 5},
    {"name": "builtin-compareVersions", "source": 'builtins.compareVersions "1.2.3" "1.10.0"', "expect": -1},
    {"name": "builtin-splitVersion", "source": 'builtins.splitVersion "1.2-rc1"', "expect": ["1", "2", "rc", "1"]},
    {"name": "builtin-parseDrvName", "source": '(builtins.parseDrvName "hello-1.2.3").version', "expect": "1.2.3"},
    {"name": "builtin-nixVersion", "source": "builtins.nixVersion", "expect": "2.18.0-pnix"},
    {"name": "builtin-storeDir", "source": "builtins.storeDir", "expect": "/nix/store"},
    {"name": "builtin-typeOf-list", "source": "builtins.typeOf [ 1 ]", "expect": "list"},
    {"name": "builtin-typeOf-set", "source": "builtins.typeOf { a = 1; }", "expect": "set"},
    {"name": "builtin-typeOf-int", "source": "builtins.typeOf 7", "expect": "int"},
    {"name": "builtin-isAttrs", "source": "builtins.isAttrs { a = 1; }", "expect": True},
    {"name": "builtin-isList", "source": "builtins.isList [ 1 ]", "expect": True},
    {"name": "builtin-isFunction", "source": "builtins.isFunction (x: x)", "expect": True},
    {"name": "builtin-isNull", "source": "builtins.isNull null", "expect": True},
    {"name": "builtin-isString-false", "source": "builtins.isString 1", "expect": False},
    {"name": "builtin-attrNames", "source": "builtins.attrNames { b = 2; a = 1; }", "expect": ["a", "b"]},
    {"name": "builtin-hasAttr-true", "source": 'builtins.hasAttr "a" { a = 1; }', "expect": True},
    {"name": "builtin-hasAttr-false", "source": 'builtins.hasAttr "z" { a = 1; }', "expect": False},
    {"name": "builtin-getAttr", "source": 'builtins.getAttr "a" { a = 5; }', "expect": 5},
    {"name": "builtin-head", "source": "builtins.head [ 10 20 30 ]", "expect": 10},
    {"name": "builtin-tail", "source": "builtins.tail [ 10 20 30 ]", "expect": [20, 30]},
    {"name": "builtin-map", "source": "builtins.map (x: x + 1) [ 1 2 3 ]", "expect": [2, 3, 4]},
    {"name": "builtin-filter", "source": "builtins.filter (x: x > 1) [ 1 2 3 ]", "expect": [2, 3]},
    {"name": "builtin-foldl", "source": "builtins.foldl' (acc: x: acc + x) 0 [ 1 2 3 ]", "expect": 6},
    {"name": "builtin-foldr", "source": "builtins.foldr (x: acc: x - acc) 0 [ 1 2 3 ]", "expect": 2},
    {"name": "builtin-take", "source": "builtins.take 2 [ 1 2 3 ]", "expect": [1, 2]},
    {"name": "builtin-drop", "source": "builtins.drop 1 [ 1 2 3 ]", "expect": [2, 3]},
    {"name": "builtin-take-overbound", "source": "builtins.take 5 [ 1 2 3 ]", "expect": [1, 2, 3]},
    {"name": "builtin-drop-overbound", "source": "builtins.drop 5 [ 1 2 3 ]", "expect": []},
    {"name": "builtin-reverse-alias", "source": "builtins.reverse [ 1 2 3 ]", "expect": [3, 2, 1]},
    {"name": "builtin-reverseList", "source": "builtins.reverseList [ 1 2 3 ]", "expect": [3, 2, 1]},
    {"name": "builtin-zip", "source": 'builtins.zip [ 1 2 ] [ "a" "b" "c" ]', "expect": [[1, "a"], [2, "b"]]},
    {"name": "builtin-flatten", "source": "builtins.flatten [ 1 [ 2 [ 3 ] ] ]", "expect": [1, 2, 3]},
    {"name": "builtin-find-hit", "source": "builtins.find 2 [ 1 2 3 ]", "expect": 2},
    {"name": "builtin-find-miss", "source": "builtins.find 9 [ 1 2 3 ]", "expect": None},
    {"name": "builtin-get-alias-hit", "source": 'builtins.get { x = 1; } "x"', "expect": 1},
    {"name": "builtin-get-alias-miss", "source": 'builtins.get { x = 1; } "y"', "expect": None},
    {"name": "builtin-mapGet-alias", "source": 'builtins.mapGet { x = 1; } "x"', "expect": 1},
    {"name": "builtin-set-alias", "source": 'builtins.set { x = 1; } "y" 2', "expect": {"x": 1, "y": 2}},
    {"name": "builtin-mapSet-alias", "source": 'builtins.mapSet { x = 1; } "y" 2', "expect": {"x": 1, "y": 2}},
    {"name": "builtin-keys-alias", "source": "builtins.keys { b = 2; a = 1; }", "expect": ["a", "b"]},
    {"name": "builtin-mapKeys-alias", "source": "builtins.mapKeys { b = 2; a = 1; }", "expect": ["a", "b"]},
    {"name": "builtin-values-alias", "source": "builtins.values { b = 2; a = 1; }", "expect": [1, 2]},
    {"name": "builtin-mapValues-alias", "source": "builtins.mapValues { b = 2; a = 1; }", "expect": [1, 2]},
    {"name": "builtin-merge-alias", "source": "builtins.merge { a = 1; } { b = 2; }", "expect": {"a": 1, "b": 2}},
    {"name": "builtin-mapMerge-alias", "source": "builtins.mapMerge { a = 1; } { b = 2; }", "expect": {"a": 1, "b": 2}},
    {"name": "builtin-isInt-true", "source": "builtins.isInt 5", "expect": True},
    {"name": "builtin-isInt-false", "source": 'builtins.isInt "x"', "expect": False},
    {"name": "builtin-isBool-true", "source": "builtins.isBool true", "expect": True},
    {"name": "builtin-isBool-false", "source": "builtins.isBool 1", "expect": False},
    {"name": "builtin-isFinite-int", "source": "builtins.isFinite 42", "expect": True},
    {"name": "builtin-isFinite-float", "source": "builtins.isFinite 3.5", "expect": True},
    {"name": "builtin-isFinite-inf", "source": "builtins.isFinite (1.0e308 * 10.0)", "expect": False},
    {"name": "builtin-isInf", "source": "builtins.isInf (1.0e308 * 10.0)", "expect": True},
    {"name": "builtin-isNaN-false", "source": "builtins.isNaN 1.0", "expect": False},
    {"name": "builtin-getContext-plain", "source": 'builtins.getContext "hello"', "expect": {}},
    {"name": "builtin-appendContext-empty", "source": 'builtins.appendContext "hello" {}', "expect": "hello"},
    {"name": "builtin-appendContext-getContext", "source": 'builtins.getContext (builtins.appendContext "x" { "/a" = { path = true; }; })', "expect": {"/a": {"path": True}}},
    {"name": "builtin-context-concat-union", "source": 'builtins.getContext ((builtins.appendContext "a" { "/a" = { path = true; }; }) + (builtins.appendContext "b" { "/b" = { path = true; }; }))', "expect": {"/a": {"path": True}, "/b": {"path": True}}},
    {"name": "builtin-context-interp", "source": 'let s = builtins.appendContext "x" { "/a" = { path = true; }; }; in builtins.hasAttr "/a" (builtins.getContext "pre${s}")', "expect": True},
    {"name": "builtin-context-toJSON", "source": 'builtins.hasAttr "/a" (builtins.getContext (builtins.toJSON [ (builtins.appendContext "x" { "/a" = { path = true; }; }) ]))', "expect": True},
    {"name": "builtin-context-concatStrings", "source": 'builtins.hasAttr (builtins.toString ./p1) (builtins.getContext (builtins.concatStrings [ "a${./p1}" "b${./p2}" ]))', "expect": True},
    {"name": "builtin-context-match", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.match "(.+)" "x${./p}") 0))', "expect": True},
    {"name": "builtin-context-split-head", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.split "-" "x${./p}-y") 0))', "expect": True},
    {"name": "builtin-context-split-tail", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.elemAt (builtins.split "-" "x${./p}-y") 2))', "expect": True},
    {"name": "builtin-context-toJSON-path", "source": "builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.toJSON ./p))", "expect": True},
    {"name": "builtin-context-toFile-guard", "source": '(builtins.tryEval (builtins.toFile "name" "x${./p}")).success', "expect": False},
    {"name": "builtin-context-toPath", "source": 'builtins.typeOf (builtins.toPath "${/tmp}")', "expect": "string"},
    {"name": "builtin-context-eq-same", "source": '"x${./p}" == "x${./p}"', "expect": True},
    {"name": "builtin-context-sort-strings", "source": 'builtins.length (builtins.sort (a: b: a < b) [ "b${./bp}" "a${./ap}" "c${./cp}" ])', "expect": 3},
    {"name": "builtin-context-getEnv", "source": 'builtins.getEnv "DEFINITELY_NOT_SET_${./marker}"', "expect": ""},
    {"name": "builtin-context-xmlParse", "source": 'builtins.typeOf (builtins.xmlParse "<a>${./marker}</a>")', "expect": "set"},
    {"name": "builtin-context-htmlParse", "source": 'builtins.typeOf (builtins.htmlParse "<p>${./body}</p>")', "expect": "set"},
    {"name": "builtin-context-xmlEmit-attr", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p}"; }; children = []; }))', "expect": True},
    {"name": "builtin-context-xmlEmit-text", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = {}; children = [ { kind = "text"; value = "x${./p}"; } ]; }))', "expect": True},
    {"name": "builtin-context-htmlEmit-text", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.htmlEmit { kind = "element"; name = "p"; attrs = {}; children = [ { kind = "text"; value = "x${./p}"; } ]; }))', "expect": True},
    {"name": "builtin-context-xmlEmit-union", "source": 'builtins.length (builtins.attrNames (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p1}"; class = "y${./p2}"; }; children = [ { kind = "text"; value = "z${./p3}"; } ]; })))', "expect": 3},
    {"name": "builtin-context-xmlEmit-path-attr", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { src = builtins.toString ./p; }; children = []; }))', "expect": True},
    {"name": "builtin-context-xmlEmit-concat", "source": 'builtins.hasAttr (builtins.toString ./p) (builtins.getContext ("<?xml?>" + (builtins.xmlEmit { kind = "element"; name = "a"; attrs = { id = "x${./p}"; }; children = []; })))', "expect": True},
    {"name": "builtin-import-exists", "source": "builtins.typeOf builtins.import", "expect": "lambda"},
    {"name": "builtin-scopedImport-exists", "source": "builtins.typeOf builtins.scopedImport", "expect": "lambda"},
    {"name": "builtin-fold-alias", "source": "builtins.fold (acc: x: acc + x) 0 [ 1 2 3 4 ]", "expect": 10},
    {"name": "builtin-length-string-byte", "source": 'builtins.length "héllo"', "expect": 6},
    {"name": "builtin-length-error", "source": "(builtins.tryEval (builtins.length { a = 1; })).success", "expect": False},
    {"name": "builtin-seq-shallow-attr", "source": 'builtins.seq { a = throw "inner"; } 1', "expect": 1},
    {"name": "builtin-seq-top-throw", "source": '(builtins.tryEval (builtins.seq (throw "top") 1)).success', "expect": False},
    {"name": "builtin-foldr-minus", "source": "builtins.foldr (a: b: a - b) 0 [ 1 2 3 4 ]", "expect": -2},
    {"name": "builtin-elem-nonlist-guard", "source": "(builtins.tryEval (builtins.elem 1 42)).success", "expect": False},
    {"name": "builtin-lazy-map-length", "source": 'builtins.length (builtins.map (x: throw "x") [ 1 2 3 ])', "expect": 3},
    {"name": "builtin-lazy-genlist-head", "source": 'builtins.head (builtins.genList (i: if i == 0 then 99 else throw "x") 10)', "expect": 99},
    {"name": "builtin-lazy-concatlists-head", "source": 'builtins.head (builtins.concatLists [ [ 1 ] [ (throw "x") ] ])', "expect": 1},
    {"name": "builtin-lazy-attrvalues-length", "source": 'builtins.length (builtins.attrValues { a = 1; b = throw "x"; })', "expect": 2},
    {"name": "builtin-lazy-mapattrs-names", "source": 'builtins.length (builtins.attrNames (builtins.mapAttrs (k: v: throw "x") { a = 1; b = 2; }))', "expect": 2},
    {"name": "builtin-lazy-zip-hasattr", "source": 'let r = builtins.zipAttrsWith (k: vs: throw "x") [ { a = 1; } { a = 2; } ]; in r ? a', "expect": True},
    {"name": "builtin-lazy-attrbypath-default", "source": 'builtins.attrByPath [ "a" ] (throw "default-fired") { a = 42; }', "expect": 42},
    {"name": "builtin-hasattr-throw-value", "source": '{ a = throw "x"; } ? a', "expect": True},
    {"name": "builtin-guard-getattr-missing", "source": '(builtins.tryEval (builtins.getAttr "z" { a = 1; })).success', "expect": False},
    {"name": "builtin-guard-catattrs-nonlist", "source": '(builtins.tryEval (builtins.catAttrs "a" 42)).success', "expect": False},
    {"name": "builtin-curry3", "source": "(a: b: c: a + b + c) 1 2 3", "expect": 6},
    {"name": "builtin-closure-capture", "source": "let x = 10; f = y: x + y; in f 5", "expect": 15},
    {"name": "builtin-with-lazy-unused", "source": 'with (throw "boom"); 1', "expect": 1},
    {"name": "builtin-with-priority-let", "source": "let x = 1; in with { x = 2; }; x", "expect": 1},
    {"name": "builtin-with-priority-inner", "source": "with { x = 1; }; with { x = 2; }; x", "expect": 2},
    {"name": "builtin-fn-eq-false", "source": "(x: x) == (y: y)", "expect": False},
    {"name": "builtin-guard-apply-nonfn", "source": "(builtins.tryEval (let f = 1; in f 2)).success", "expect": False},
    {"name": "builtin-guard-listtoattrs-missing-value", "source": '(builtins.tryEval (builtins.listToAttrs [ { name = "a"; } ])).success', "expect": False},
    {"name": "builtin-nixlib-fix-factorial", "source": "let fix = f: let x = f x; in x; fact = self: n: if n <= 1 then 1 else n * self (n - 1); f = fix fact; in f 5", "expect": 120},
    {"name": "builtin-nixlib-make-extensible", "source": "let fix = f: let x = f x; in x; makeExtensible = f: let self = f self // { extend = ext: makeExtensible (self_: f self_ // ext self_ (f self_)); }; in self; base = makeExtensible (self: { a = 1; b = 2; sum = self.a + self.b; }); ext = base.extend (self: super: { c = 3; sum = super.sum + self.c; }); in ext.sum", "expect": 6},
    {"name": "builtin-nixlib-genattrs", "source": 'let genAttrs = names: f: builtins.listToAttrs (builtins.map (name: { inherit name; value = f name; }) names); in (genAttrs [ "a" "b" "c" ] (n: "v_" + n)).c', "expect": "v_c"},
    {"name": "builtin-nixlib-compose", "source": "let composeExtensions = f: g: final: prev: let r = f final prev; in g final (prev // r) // r; base = self: { a = 1; b = self.a * 2; }; addC = self: super: { c = self.a + 100; }; addD = self: super: { d = self.c + super.b; }; extension = composeExtensions addC addD; fix = f: let x = f x; in x; mk = self: let init = base self; in init // (extension self init); result = fix mk; in result.d", "expect": 103},
    {"name": "builtin-fc-deep-rec", "source": "let f = n: if n == 0 then 0 else f (n - 1) + 1; in f 30", "expect": 30},
    {"name": "builtin-fc-lazy-unused", "source": "let x = x; in 1", "expect": 1},
    {"name": "builtin-fc-self-cycle-guard", "source": "(builtins.tryEval (let s = { x = s.x; }; in s.x)).success", "expect": False},
    {"name": "builtin-fc-tojson-cycle-guard", "source": "(builtins.tryEval (let s = { x = s; }; in builtins.toJSON s)).success", "expect": False},
    {"name": "builtin-toString-path-context", "source": "builtins.hasContext (builtins.toString ./foo)", "expect": True},
    {"name": "builtin-toString-path-context-key", "source": "let s = builtins.toString ./foo; in builtins.hasAttr s (builtins.getContext s)", "expect": True},
    {"name": "builtin-toString-list-context", "source": 'builtins.hasAttr "/a" (builtins.getContext (builtins.toString [ (builtins.appendContext "x" { "/a" = { path = true; }; }) ]))', "expect": True},
    {"name": "builtin-addDrvOutputDependencies", "source": 'builtins.addDrvOutputDependencies "raw"', "expect": "raw"},
    {"name": "builtin-addDrvOutputDependencies-context", "source": 'builtins.hasAttr "!out!raw" (builtins.getContext (builtins.addDrvOutputDependencies "raw"))', "expect": True},
    {"name": "builtin-unsafeDiscardOutputDependency", "source": 'builtins.unsafeDiscardOutputDependency "raw"', "expect": "raw"},
    {"name": "builtin-unsafeAddOutputDependency", "source": 'builtins.unsafeAddOutputDependency "raw"', "expect": "raw"},
    {"name": "builtin-unsafeAddOutputDependency-context", "source": 'builtins.hasAttr "!out!/a" (builtins.getContext (builtins.unsafeAddOutputDependency (builtins.appendContext "x" { "/a" = { path = true; }; })))', "expect": True},
    {"name": "builtin-unsafeAddOutputName", "source": 'builtins.unsafeAddOutputName "out" "raw"', "expect": "raw"},
    {"name": "builtin-unsafeAddOutputName-context", "source": 'builtins.hasAttr "!dev!/a" (builtins.getContext (builtins.unsafeAddOutputName "dev" (builtins.appendContext "x" { "/a" = { path = true; }; })))', "expect": True},
    {"name": "builtin-derivation-outPath-context", "source": 'builtins.hasAttr "!out!demo" (builtins.getContext (builtins.derivation { name = "demo"; }).outPath)', "expect": True},
    {"name": "builtin-unsafeDiscardStringContext-context", "source": 'builtins.hasContext (builtins.unsafeDiscardStringContext (builtins.appendContext "x" { "/a" = { path = true; }; }))', "expect": False},
    {"name": "builtin-toString-list", "source": "builtins.toString [ 1 2 3 ]", "expect": "1 2 3"},
    {"name": "builtin-toString-true", "source": "builtins.toString true", "expect": "1"},
    {"name": "builtin-toString-null", "source": "builtins.toString null", "expect": ""},
    {"name": "builtin-toString-attr-tostring", "source": 'builtins.toString { __toString = self: "hi-" + self.label; label = "x"; }', "expect": "hi-x"},
    {"name": "builtin-toString-attr-outPath", "source": 'builtins.toString { outPath = "/nix/store/x"; }', "expect": "/nix/store/x"},
    {"name": "builtin-toString-attr-priority", "source": 'builtins.toString { __toString = _: "from-toString"; outPath = "from-outPath"; }', "expect": "from-toString"},
    {"name": "builtin-toString-cycle-guard", "source": 'let r = { __toString = self: builtins.toString self; }; in (builtins.tryEval (builtins.toString r)).success', "expect": False},
    {"name": "whole-attrset", "source": "{ a = 1; b = 2; c = 3; }", "expect": {"a": 1, "b": 2, "c": 3}},
    {"name": "whole-rec", "source": "rec { x = 1; y = x + 1; z = y + 1; }", "expect": {"x": 1, "y": 2, "z": 3}},
    {"name": "whole-nested", "source": "{ a = { b = 1; c = 2; }; d = 3; }", "expect": {"a": {"b": 1, "c": 2}, "d": 3}},
    {"name": "builtin-toJSON", "source": "builtins.toJSON { b = 2; a = 1; }", "expect": '{"a":1,"b":2}'},
    {"name": "builtin-toJSON-nested", "source": "builtins.toJSON { b = [ 1 2 ]; a = { y = 2; x = 1; }; }", "expect": '{"a":{"x":1,"y":2},"b":[1,2]}'},
    {"name": "builtin-toJSON-str", "source": 'builtins.toJSON "hi"', "expect": '"hi"'},
    {"name": "oracle-eval-let", "source": "let x = 10; in x + 5", "expect": 15},
    {"name": "oracle-eval-let-recursive", "source": "let a = b + 1; b = 2; in a", "expect": 3},
    {
        "name": "oracle-eval-self-recursive-lambda",
        "source": "let sum = xs: if (builtins.length xs) == 0 then 0 else (builtins.head xs) + sum (builtins.tail xs); in sum [ 1 2 3 ]",
        "expect": 6,
    },
    {"name": "oracle-eval-curried-lambda", "source": "let add = x: y: x + y; in add 3 4", "expect": 7},
    {"name": "oracle-eval-rec-late-binding", "source": "rec { f = x: x + seed; seed = 2; }.f 3", "expect": 5},
    {"name": "oracle-eval-foldl", "source": "builtins.foldl' (acc: x: acc + x) 0 [ 1 2 3 4 ]", "expect": 10},
]


# Nix tryEval catches explicit throw/assert only. Older pnix-hy tests encoded a
# catch-all policy by expecting `.success == false` for ordinary evaluator,
# type, IO, and recursion errors. Reclassify those receipts as propagated-error
# cases while retaining the two explicit-throw probes as catchable values.
_TRYEVAL_FALSE_IS_CATCHABLE = {
    "isPath-top-tryeval",
    "builtin-seq-top-throw",
    "compare-eq-same-attr-throw",
    "compare-eq-same-list-throw",
}
for _case in SELF_TEST_CASES:
    if (
        _case.get("expect") is False
        and "builtins.tryEval" in _case.get("source", "")
        and _case.get("name") not in _TRYEVAL_FALSE_IS_CATCHABLE
    ):
        _case.pop("expect")
        _case["error"] = True


HY_RUNTIME_CORE_CASES = {
    "arith",
    "bp-if-string-fails",
    "bp-and-left-int-fails",
    "bp-or-right-int-fails",
    "bp-impl-left-int-fails",
    "ac-hasattr-value-int-fails",
    "ac-hasattr-name-int-fails",
    "ac-rmattrs-name-int-fails",
    "ac-rmattrs-thunk-name-ok",
    "ac-concatlists-elem-int-fails",
    "ac-concatlists-thunk-list-ok",
    "aw-abort-int-fails",
    "aw-with-int-foo-fails",
    "aw-with-int-unused-ok",
    "aw-with-attrset-ok",
    "aw-with-inner-wins-ok",
    "rsl-from-int-fails",
    "rsl-to-string-fails",
    "rsl-from-elem-int-fails",
    "rsl-clean-ok",
    "fg-fold-sum-ok",
    "fg-fold-int-fails",
    "fg-groupby-key-int-fails",
    "fg-groupby-nonlist-fails",
    "la-any-nonlist-fails",
    "la-elem-nonlist-fails",
    "la-filter-nonlist-fails",
    "la-foldr-nonlist-fails",
    "la-filter-happy-ok",
    "ai-functionargs-int-fails",
    "ai-attrnames-list-fails",
    "ai-getattr-missing-fails",
    "ai-zip-elem-int-fails",
    "ai-getattr-happy-ok",
    "ai-functionargs-happy-ok",
    "bx-bitand-first-string-fails",
    "bx-addctx-int-fails",
    "bx-bitxor-happy-ok",
    "bx-addctx-happy-ok",
    "acs-value-int-fails",
    "acs-path-int-fails",
    "acs-outputs-elem-int-fails",
    "acs-path-bool-ok",
    "ov-unaryneg-min-fails",
    "ov-neg-min-fails",
    "ov-mod-min-neg1-fails",
    "ov-mod-zero-fails",
    "ov-i64min-build-ok",
    "ov-neg-max-ok",
    "cs-concatstrings-nonlist-fails",
    "cs-sep-nonstring-fails",
    "cs-concatstrings-happy-ok",
    "cs-sep-happy-ok",
    "uo-adddep-int-fails",
    "uo-addname-first-int-fails",
    "uo-adddep-happy-ok",
    "uo-addname-happy-ok",
    "tc-self-ref-fails",
    "tc-outpath-string-ok",
    "dv-nonattrset-fails",
    "sa-getenv-int-fails",
    "sa-tofile-name-int-fails",
    "sa-getenv-happy-ok",
    "op-int-plus-string-fails",
    "op-null-plus-int-fails",
    "op-plus-int-ok",
    "op-plus-list-ok",
    "tj-posinf-fails",
    "tj-nan-fails",
    "hf-sha256-len-ok",
    "hf-md5-rejected-fails",
    "hf-missing-path-fails",
    "let-recursive",
    "lambda",
    "lambda-attr-pattern",
    "lambda-attr-default",
    "lambda-attr-as-left",
    "lambda-attr-as-right",
    "lambda-list-pattern",
    "lambda-list-rest-pattern",
    "attr-select",
    "rec-attr",
    "rec-forward",
    "bool",
    "implication-true-true",
    "implication-true-false",
    "implication-false-lazy",
    "merge",
    "merge-null-left",
    "merge-null-right",
    "merge-null-left-scalar",
    "has-attr",
    "plus-string",
    "plus-list",
    "plus-attrset",
    "compare-string",
    "compare-list",
    "float-plus",
    "float-compare",
    "float-eq-int",
    "float-eq-int-f64-rounding",
    "float-eq-int-f64-nested",
    "float-nan-scalar-not-equal",
    "float-nan-shared-list-equal",
    "float-nan-shared-attr-equal",
    "float-toString-fixed",
    "float-exponent-integer",
    "float-exponent-negative",
    "float-exponent-empty-fraction",
    "float-leading-dot",
    "float-leading-dot-exponent",
    "float-exponent-zero-underflow",
    "float-toString-negative-zero",
    "float-toString-small-negative",
    "float-toString-div-negative-zero",
    "float-toString-mul-negative-zero",
    "float-toString-inf",
    "float-toString-neg-inf",
    "float-toString-nan",
    "float-mixed-builtin-add",
    "float-mixed-builtin-sub",
    "float-mixed-builtin-mul",
    "float-mixed-builtin-div",
    "float-mixed-builtin-less-rounding-left",
    "float-mixed-builtin-less-rounding-right",
    "compare-eq-list-shared-lambda",
    "compare-eq-attr-shared-lambda",
    "compare-elem-shared-lambda",
    "compare-eq-same-list-lambda",
    "compare-eq-same-attr-lambda",
    "compare-eq-distinct-list-lambda",
    "compare-eq-same-list-throw",
    "compare-eq-same-attr-throw",
    "compare-eq-nested-same-attr-throw",
    "compare-eq-alias-chain",
    "compare-less-alias-chain",
    "compare-less-list-shared-lambda",
    "compare-less-list-shared-nan",
    "compare-less-list-distinct-nan",
    "compare-le-list-distinct-nan",
    "compare-eq-lambda-self",
    "compare-eq-attr-lambda",
    "compare-version-pre-release",
    "compare-version-plus-revision",
    "split-version-plus-revision",
    "parse-drv-first-hyphen",
    "version-split-context",
    "version-parse-context",
    "eq-cycle-tryeval",
    "lt-cycle-tryeval",
    "elem-cycle-tryeval",
    "compare-version-guard",
    "force-cycle-tryeval",
    "tojson-cycle-tryeval",
    "deepseq-cycle-tryeval",
    "interp-cycle-tryeval",
    "interp-tostring-chain",
    "json-fromjson-overflow-tryeval",
    "json-minus-zero-type",
    "tojson-lambda-tryeval",
    "tojson-context-list",
    "toml-invalid-tryeval",
    "hash-md5-tryeval",
    "hash-md5-digest",
    "hash-sha1-digest",
    "hash-sha256-raw-byte",
    "hash-sha512-length",
    "sort-second-arg-tryeval",
    "sort-nonbool-tryeval",
    "tryeval-lazy-list-success",
    "tofile-fake-path-shape",
    "regex-invalid-tryeval",
    "regex-unicode-capture",
    "regex-split-adjacent",
    "path-plus-path-total",
    "path-plus-context-string-tryeval",
    "string-plus-path-context",
    "dirOf-string-context",
    "baseNameOf-string-context",
    "hashString-drops-context",
    "hashFile-readFile-parity-abs",
    "path-empty-tryeval",
    "hashFile-md5-tryeval",
    "path-context-pathExists-missing",
    "path-context-toPath-type",
    "io-readFile-missing-tryeval",
    "io-readDir-missing-tryeval",
    "io-readFileType-missing-tryeval",
    "io-readDir-repo-has-todo",
    "io-readFileType-todo-abs",
    "io-readFile-todo-prefix-abs",
    "io-toFile-readFile-roundtrip",
    "isPath-attr-lazy",
    "isPath-list-lazy",
    "isPath-top-tryeval",
    "guard-langVersion-type",
    "guard-abort-int-tryeval",
    "guard-with-int-lookup-tryeval",
    "guard-addctx-int-tryeval",
    "guard-unsafe-pos-mapattrs-null",
    "guard-bitops-happy",
    "guard-append-path-int-tryeval",
    "guard-append-unknown-lazy",
    "guard-hasAttr-value-int-tryeval",
    "guard-hasAttr-name-int-tryeval",
    "guard-removeAttrs-second-int-tryeval",
    "guard-concatLists-inner-int-tryeval",
    "guard-if-string-tryeval",
    "guard-builtins-and-string-tryeval",
    "guard-lambda-formal-dup-tryeval",
    "guard-lambda-at-dup-tryeval",
    "guard-lambda-dup-before-arg-tryeval",
    "unsafe-add-output-dep-marker",
    "unsafe-add-output-name-marker",
    "unsafe-add-output-name-int-tryeval",
    "unsafe-add-output-discard-roundtrip",
    "derivation-attrnames-eq-strict",
    "derivation-type-override",
    "derivation-non-attr-tryeval",
    "derivation-outpath-context",
    "derivation-type-check",
    "path-normalize-tostring",
    "path-normalize-tojson",
    "path-dirOf-type",
    "path-dirOf-normalized",
    "path-eq-normalized",
    "path-lt-normalized",
    "path-string-mismatch",
    "path-string-dirOf-not-normalized",
    "path-baseName-double-slash",
    "attr-explicit-then-path-merge",
    "attr-two-explicit-merge",
    "attr-duplicate-leaf-tryeval",
    "attr-nonattr-path-conflict-tryeval",
    "tojson-inf-tryeval",
    "builtin-typeOf-float",
    "builtin-isFloat",
    "with-basic",
    "with-nested-shadow",
    "with-let-wins",
    "with-lexical-wins",
    "assert-pass",
    "block-comment-whitespace",
    "block-comment-first-close",
    "select-default-missing",
    "select-default-expr",
    "select-default-chain",
    "index-list",
    "index-attrset",
    "path-type",
    "path-isPath",
    "path-absolute-isPath",
    "path-home-isPath",
    "path-search-type",
    "path-search-toString",
    "path-interp-isPath",
    "path-interp-baseNameOf",
    "dynamic-select",
    "dynamic-select-default",
    "dynamic-attr-key",
    "dynamic-rec-attr-key",
    "dynamic-attr-key-pos-null",
    "list-items-no-apply",
    "concatMap-list-items-no-apply",
    "regex-posix-space-split",
    "nixpkgs-foldAttrs-dynamic-key",
    "dynamic-hasattr",
    "dynamic-hasattr-path",
    "dynamic-hasattr-number",
    "hasattr-quoted-dot",
    "quoted-dot-key-select",
    "quoted-dot-key-names",
    "quoted-dot-key-not-nested",
    "inherit-plain-attrset",
    "inherit-scope-attrset",
    "inherit-scope-quoted-key",
    "inherit-dynamic-literal-name",
    "inherit-scope-lazy-unused",
    "inherit-let",
    "inherit-chain-let",
    "inherit-rec-outer",
    "inherit-rec-scope",
    "nested-let-path",
    "nested-let-recursive-path",
    "top-level-builtin-alias",
    "top-level-list-builtins",
    "match-literal",
    "match-list",
    "match-attrset",
    "match-constructor",
    "match-guard-fallthrough",
    "match-guard-attrset",
    "match-guard-false-default",
    "match-guard-null-body",
    "str-plain",
    "str-escape",
    "str-dollar-literal",
    "str-indented-basic",
    "str-indented-strip",
    "str-indented-leading-whitespace-only",
    "str-indented-quote-escape",
    "str-indented-dollar-escape",
    "str-indented-backslash-escapes",
    "str-indented-interp",
    "str-interp",
    "str-interp-concat",
    "str-interp-rec",
    "str-interp-nested",
    "str-interp-nested-indented",
    "str-interp-block-comment",
    "str-interp-toString",
    "str-placeholder",
    "str-escape-interp",
    "str-interp-tostring-attr",
    "str-interp-tostring-self",
    "str-interp-outpath-attr",
    "str-interp-tostring-priority",
    "str-interp-outpath-nested",
    "list-builtin",
    "builtin-elem-true",
    "builtin-elem-false",
    "builtin-any",
    "builtin-all-true",
    "builtin-all-false",
    "builtin-concatLists",
    "builtin-concatMap",
    "builtin-genList",
    "builtin-genList-zero",
    "builtin-attrValues",
    "builtin-mapAttrs",
    "builtin-sort",
    "builtin-stringLength",
    "builtin-stringLength-utf8",
    "builtin-substring",
    "builtin-substring-utf8",
    "builtin-hasPrefix",
    "builtin-hasSuffix",
    "builtin-replaceStrings",
    "builtin-replaceStrings-empty",
    "builtin-concatStringsSep",
    "builtin-concatStrings-empty",
    "builtin-concatStrings",
    "builtin-listToAttrs",
    "builtin-removeAttrs",
    "builtin-attrByPath-hit",
    "builtin-attrByPath-miss",
    "builtin-getAttrs",
    "builtin-filterAttrs",
    "builtin-intersectAttrs",
    "builtin-catAttrs",
    "builtin-groupBy",
    "builtin-partition",
    "builtin-genericClosure-chain",
    "builtin-genericClosure-dedupe",
    "builtin-genericClosure-composite-key",
    "builtin-functionArgs",
    "builtin-functionArgs-native",
    "builtin-zipAttrsWith",
    "builtin-match",
    "builtin-match-miss",
    "builtin-split",
    "builtin-fromJSON",
    "builtin-fromJSON-i64-max",
    "builtin-fromJSON-exp-float",
    "builtin-fromTOML-simple",
    "builtin-fromTOML-int",
    "builtin-fromTOML-section",
    "builtin-fromTOML-array",
    "builtin-fromTOML-nested-section",
    "builtin-fromTOML-bool",
    "builtin-fromTOML-float",
    "builtin-fromTOML-empty",
    "builtin-schemaNormalize-default",
    "builtin-schemaValidate-ok",
    "builtin-schemaValidate-nested-root",
    "builtin-schemaValidate-string-fail",
    "builtin-schemaExplain-string",
    "builtin-xmlParse-kind",
    "builtin-xmlParse-name",
    "builtin-xmlEmit-roundtrip",
    "builtin-xmlEmit-empty",
    "builtin-htmlParse-document",
    "builtin-htmlEmit-roundtrip",
    "builtin-tryEval-ok",
    "builtin-tryEval-fail",
    "builtin-derivation-type",
    "builtin-derivation-name",
    "builtin-derivation-outPath",
    "builtin-derivation-override",
    "builtin-derivationStrict-unnamed",
    "builtin-pathExists-true",
    "builtin-pathExists-false",
    "builtin-readFile",
    "builtin-readFileType",
    "builtin-readDir",
    "builtin-toFile-readFile",
    "builtin-hashString-sha256",
    "builtin-hashFile-readFile-parity",
    "builtin-baseNameOf",
    "builtin-dirOf",
    "builtin-toPath-isPath",
    "builtin-storePath-isPath",
    "builtin-getEnv-missing",
    "builtin-placeholder",
    "builtin-break",
    "builtin-warn",
    "builtin-traceVerbose",
    "builtin-addErrorContext",
    "builtin-unsafeGetAttrPos-null",
    "builtin-bool-aliases",
    "builtin-comparison-aliases",
    "builtin-mod",
    "builtin-mod-negative-left",
    "builtin-mod-negative-right",
    "builtin-mod-float",
    "builtin-neg",
    "builtin-abs",
    "builtin-bitAnd",
    "builtin-bitOr",
    "builtin-bitXor",
    "builtin-pow",
    "builtin-pow-3-39-exact",
    "builtin-pow-overflow-float-type",
    "builtin-pow-negative-exp",
    "builtin-sqrt",
    "builtin-floor",
    "builtin-ceil",
    "builtin-floor-large",
    "builtin-ceil-negative",
    "builtin-floor-int-f64-exact",
    "builtin-ceil-int-f64-exact",
    "builtin-exp",
    "builtin-ln",
    "builtin-log",
    "builtin-sin",
    "builtin-cos",
    "builtin-tan",
    "builtin-atan2",
    "builtin-lessThan",
    "builtin-add",
    "builtin-sub",
    "builtin-mul",
    "builtin-div",
    "builtin-compareVersions",
    "builtin-splitVersion",
    "builtin-parseDrvName",
    "builtin-nixVersion",
    "builtin-storeDir",
    "builtin-typeOf-list",
    "builtin-typeOf-set",
    "builtin-typeOf-int",
    "builtin-isAttrs",
    "builtin-isList",
    "builtin-isFunction",
    "builtin-isNull",
    "builtin-isString-false",
    "builtin-attrNames",
    "builtin-hasAttr-true",
    "builtin-hasAttr-false",
    "builtin-getAttr",
    "builtin-head",
    "builtin-tail",
    "builtin-map",
    "builtin-filter",
    "builtin-foldl",
    "builtin-foldr",
    "builtin-take",
    "builtin-drop",
    "builtin-take-overbound",
    "builtin-drop-overbound",
    "builtin-reverse-alias",
    "builtin-reverseList",
    "builtin-zip",
    "builtin-flatten",
    "builtin-find-hit",
    "builtin-find-miss",
    "builtin-get-alias-hit",
    "builtin-get-alias-miss",
    "builtin-mapGet-alias",
    "builtin-set-alias",
    "builtin-mapSet-alias",
    "builtin-keys-alias",
    "builtin-mapKeys-alias",
    "builtin-values-alias",
    "builtin-mapValues-alias",
    "builtin-merge-alias",
    "builtin-mapMerge-alias",
    "builtin-isInt-true",
    "builtin-isInt-false",
    "builtin-isBool-true",
    "builtin-isBool-false",
    "builtin-isFinite-int",
    "builtin-isFinite-float",
    "builtin-isFinite-inf",
    "builtin-isInf",
    "builtin-isNaN-false",
    "builtin-getContext-plain",
    "builtin-appendContext-empty",
    "builtin-appendContext-getContext",
    "builtin-context-concat-union",
    "builtin-context-interp",
    "builtin-context-toJSON",
    "builtin-context-concatStrings",
    "builtin-context-match",
    "builtin-context-split-head",
    "builtin-context-split-tail",
    "builtin-context-toJSON-path",
    "builtin-context-toFile-guard",
    "builtin-context-toPath",
    "builtin-context-eq-same",
    "builtin-context-sort-strings",
    "builtin-context-getEnv",
    "builtin-context-xmlParse",
    "builtin-context-htmlParse",
    "builtin-context-xmlEmit-attr",
    "builtin-context-xmlEmit-text",
    "builtin-context-htmlEmit-text",
    "builtin-context-xmlEmit-union",
    "builtin-context-xmlEmit-path-attr",
    "builtin-context-xmlEmit-concat",
    "builtin-import-exists",
    "builtin-scopedImport-exists",
    "builtin-fold-alias",
    "builtin-length-string-byte",
    "builtin-length-error",
    "builtin-seq-shallow-attr",
    "builtin-seq-top-throw",
    "builtin-foldr-minus",
    "builtin-elem-nonlist-guard",
    "builtin-lazy-map-length",
    "builtin-lazy-genlist-head",
    "builtin-lazy-concatlists-head",
    "builtin-lazy-attrvalues-length",
    "builtin-lazy-mapattrs-names",
    "builtin-lazy-zip-hasattr",
    "builtin-lazy-attrbypath-default",
    "builtin-hasattr-throw-value",
    "builtin-guard-getattr-missing",
    "builtin-guard-catattrs-nonlist",
    "builtin-curry3",
    "builtin-closure-capture",
    "builtin-with-lazy-unused",
    "builtin-with-priority-let",
    "builtin-with-priority-inner",
    "builtin-fn-eq-false",
    "builtin-guard-apply-nonfn",
    "builtin-guard-listtoattrs-missing-value",
    "builtin-nixlib-fix-factorial",
    "builtin-nixlib-make-extensible",
    "builtin-nixlib-genattrs",
    "builtin-nixlib-compose",
    "builtin-fc-deep-rec",
    "builtin-fc-lazy-unused",
    "builtin-fc-self-cycle-guard",
    "builtin-fc-tojson-cycle-guard",
    "builtin-toString-path-context",
    "builtin-toString-path-context-key",
    "builtin-toString-list-context",
    "builtin-addDrvOutputDependencies",
    "builtin-addDrvOutputDependencies-context",
    "builtin-unsafeDiscardOutputDependency",
    "builtin-unsafeAddOutputDependency",
    "builtin-unsafeAddOutputDependency-context",
    "builtin-unsafeAddOutputName",
    "builtin-unsafeAddOutputName-context",
    "builtin-derivation-outPath-context",
    "builtin-unsafeDiscardStringContext-context",
    "builtin-toString-list",
    "builtin-toString-true",
    "builtin-toString-null",
    "builtin-toString-attr-tostring",
    "builtin-toString-attr-outPath",
    "builtin-toString-attr-priority",
    "builtin-toString-cycle-guard",
    "whole-attrset",
    "whole-rec",
    "whole-nested",
    "builtin-toJSON",
    "builtin-toJSON-nested",
    "builtin-toJSON-str",
    "tojson-non-id-keys",
    "tojson-control-backspace",
    "tojson-control-formfeed",
    "tojson-control-u0001",
    "oracle-eval-let",
    "oracle-eval-let-recursive",
    "oracle-eval-self-recursive-lambda",
    "oracle-eval-curried-lambda",
    "oracle-eval-rec-late-binding",
    "oracle-eval-foldl",
}

HY_RUNTIME_CORPUS = [
    case
    for case in SELF_TEST_CASES
    if case["name"] in HY_RUNTIME_CORE_CASES and not case.get("error")
]


def self_test_report() -> dict[str, Any]:
    eval_cases = []
    for case in SELF_TEST_CASES:
        try:
            actual = eval_source(case["source"])
            expected = case.get("expect")
            eval_cases.append(
                {
                    "name": case["name"],
                    "source": case["source"],
                    "expect": expected,
                    "actual": actual,
                    "ok": not case.get("error") and actual == expected,
                }
            )
        except Exception as exc:  # noqa: BLE001 - report all failures in the smoke payload.
            expected_error = bool(case.get("error"))
            contains = case.get("error_contains")
            eval_cases.append(
                {
                    "name": case["name"],
                    "source": case["source"],
                    "expect": case.get("expect"),
                    "error": str(exc),
                    "ok": expected_error and (not contains or contains in str(exc)),
                }
            )
    emit_cases = []
    for case in SELF_TEST_CASES:
        try:
            ast = parse(case["source"])
            emitted = emit_source(ast)
            reparsed = parse(emitted)
            emit_cases.append(
                {
                    "name": "emit-round-trip-" + case["name"],
                    "source": case["source"],
                    "emitted": emitted,
                    "ok": ast_hash(ast) == ast_hash(reparsed),
                }
            )
        except Exception as exc:  # noqa: BLE001
            emit_cases.append(
                {
                    "name": "emit-round-trip-" + case["name"],
                    "source": case["source"],
                    "error": str(exc),
                    "ok": bool(case.get("parse_error"))
                    and (not case.get("error_contains") or case["error_contains"] in str(exc)),
                }
            )
    import_cases = import_self_test_cases()
    cases = eval_cases + emit_cases + import_cases
    return {
        "schema": "pnix-hy.runtime.self-test.v0",
        "runtime": RUNTIME_SCHEMA,
        "ready": all(case["ok"] for case in cases),
        "cases": cases,
    }
