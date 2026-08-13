#ifndef PNIX_RS_H
#define PNIX_RS_H

/*
 * C ABI for the embeddable pnix-rs library (host-bound; not multi-host .px).
 *
 * Versioning (P3.4 plan — keep in sync with pnix_rs::PNIX_RS_ABI_VERSION):
 *   1 = initial: pnix_rs_abi_version, pnix_rs_eval, pnix_rs_string_free
 * Bump the constant in src/lib.rs AND document the change here on any
 * signature / semantics break. Do not remove exports without a major bump.
 */

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
  PNIX_RS_OK = 0,
  PNIX_RS_EVAL_FAILED = 1,
  PNIX_RS_NULL_ARGUMENT = -1,
  PNIX_RS_INVALID_UTF8 = -2,
  PNIX_RS_INTERIOR_NUL = -3
};

/* Current ABI version is 1 (see src/lib.rs PNIX_RS_ABI_VERSION). */
uint32_t pnix_rs_abi_version(void);
int32_t pnix_rs_eval(const char *source, char **out);
void pnix_rs_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif
