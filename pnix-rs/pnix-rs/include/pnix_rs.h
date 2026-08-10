#ifndef PNIX_RS_H
#define PNIX_RS_H

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

uint32_t pnix_rs_abi_version(void);
int32_t pnix_rs_eval(const char *source, char **out);
void pnix_rs_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif
