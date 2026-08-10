"""pnix_hy.compartment -- SES-Compartment-style guest isolation for pnix evaluation (proposal 0021).

A Compartment is an evaluation scope with its OWN persistent environment (bindings accumulate,
REPL-style) and its OWN module table (a hook-controlled mini loader: modules are registered as
pnix sources and materialize lazily as attrset bindings), while the pure intrinsics (pnix
builtins) are shared with every other compartment -- exactly the SES shape: own globalThis +
own module system, shared frozen primordials.

No new evaluator: every evaluation goes through the one sacred runtime (`rt.eval_source_raw`).
Compartments are isolation BOOKKEEPING, not a second VM.
"""

from __future__ import annotations

from typing import Any

from . import pnix_runtime as rt


class Compartment:
    def __init__(self, *, granted: tuple[str, ...] = ()) -> None:
        self.granted = tuple(granted)
        self._ctx = rt.runtime_context(None)
        self._env: dict[str, Any] = {}
        self._modules: dict[str, str] = {}
        self._ctx["env"] = self._env

    # --- own global namespace ---
    def bind(self, name: str, source: str) -> None:
        """Bind `name` to the (lazy) value of a pnix source fragment in THIS compartment only."""
        value = rt.eval_source_raw(source, self._ctx, realize=False)
        self._env[name] = value

    def eval(self, source: str) -> Any:
        """Evaluate pnix source against this compartment's accumulated environment."""
        raw = rt.eval_source_raw(source, self._ctx, realize=False)
        return rt.realize_value(rt.force_value(raw))

    def names(self) -> list[str]:
        return sorted(self._env)

    # --- own hook-controlled module table ---
    def register_module(self, name: str, source: str) -> None:
        """Register a module (a pnix source, conventionally an attrset) under `name`. It
        materializes LAZILY into the environment on first use -- the compartment's loader
        hook, not a global one."""
        self._modules[name] = source
        compartment_ctx = self._ctx
        self._env[name] = rt.Thunk(lambda src=source: rt.force_value(
            rt.eval_source_raw(src, compartment_ctx, realize=False)))

    def modules(self) -> list[str]:
        return sorted(self._modules)


def compartment_report() -> dict[str, Any]:
    """Self-check (proposal 0021): compartments have isolated bindings and module tables,
    state persists WITHIN a compartment, and the shared pure intrinsics behave identically."""
    try:
        a, b = Compartment(), Compartment()
        a.bind("secret", "41 + 1")
        a.register_module("mathx", "{ double = x: x * 2; }")

        persists = a.eval("secret") == 42 and a.eval("secret + 1") == 43
        module_ok = a.eval("mathx.double 21") == 42

        # isolation: b sees neither a's binding nor a's module
        try:
            b.eval("secret")
            bind_isolated = False
        except Exception:  # noqa: BLE001 - unknown variable in b is exactly the point
            bind_isolated = True
        try:
            b.eval("mathx.double 1")
            module_isolated = False
        except Exception:  # noqa: BLE001
            module_isolated = True

        # shared intrinsics: both compartments agree on pure builtins
        intrinsics_shared = (a.eval("builtins.length [1 2 3]")
                             == b.eval("builtins.length [1 2 3]") == 3)

        # b's own state does not leak back into a
        b.bind("secret", '"other"')
        no_backleak = a.eval("secret") == 42 and b.eval("secret") == "other"

        ready = bool(persists and module_ok and bind_isolated and module_isolated
                     and intrinsics_shared and no_backleak)
        return {"schema": "pnix-hy.compartment.report.v0", "ready": ready, "available": True,
                "state_persists": persists, "module_loader": module_ok,
                "binding_isolated": bind_isolated, "module_isolated": module_isolated,
                "intrinsics_shared": intrinsics_shared, "no_backleak": no_backleak}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.compartment.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


__all__ = ["Compartment", "compartment_report"]
