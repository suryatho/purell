// impurell runtime — value representation and ABI.
//
// A purell value is exactly one machine word:
//
//   bit 0 == 1   immediate integer, value is (n << 1) | 1  (63-bit signed)
//   bit 0 == 0   pointer to an imp_clo_t (closures are 8-aligned, so bit 0
//                is naturally clear)
//
// Numbers never allocate. Closures do, out of a chunked bump arena that is
// released in one shot when the program exits.

#ifndef IMPURELL_RT_H
#define IMPURELL_RT_H

#include <stddef.h>
#include <stdint.h>

typedef uint64_t imp_value_t;
typedef struct imp_clo imp_clo_t;

// Every compiled lambda has this signature. `self` is the closure the
// function was reached through, so captured values live at self->env[i].
typedef imp_value_t (*imp_fn_t)(imp_clo_t *self, imp_value_t arg);

struct imp_clo {
  imp_fn_t fn;
  int64_t nfree;
  imp_value_t env[];
};

#define IMP_IS_NUM(v) (((imp_value_t)(v) & 1u) == 1u)
#define IMP_IS_CLO(v) (((imp_value_t)(v) & 1u) == 0u)
#define IMP_TAG_NUM(n) ((imp_value_t)(((uint64_t)(int64_t)(n) << 1) | 1u))
#define IMP_UNTAG_NUM(v) ((int64_t)((int64_t)(imp_value_t)(v) >> 1))
#define IMP_AS_CLO(v) ((imp_clo_t *)(uintptr_t)(imp_value_t)(v))
#define IMP_FROM_CLO(c) ((imp_value_t)(uintptr_t)(const void *)(c))

// Bump arena. Chunks are linked and never moved, so pointers handed out stay
// valid for the life of the program.
void *imp_alloc(uint64_t size);
imp_clo_t *imp_make(imp_fn_t fn, int64_t nfree);
void imp_arena_release(void);

// Diagnostics, all of which exit non-zero.
_Noreturn void imp_not_a_function(imp_value_t v);
_Noreturn void imp_type_error(const char *prim, imp_value_t v);
_Noreturn void imp_panic(const char *msg);

void imp_print_value(imp_value_t v);
void imp_show_expr(const char *src, imp_value_t result);

// Primitives, referenced from generated IR as address-taken globals.
extern const imp_clo_t imp_prim_add;
extern const imp_clo_t imp_prim_sub;
extern const imp_clo_t imp_prim_mul;
extern const imp_clo_t imp_prim_div;
extern const imp_clo_t imp_prim_rem;
extern const imp_clo_t imp_prim_lt;
extern const imp_clo_t imp_prim_gt;
extern const imp_clo_t imp_prim_le;
extern const imp_clo_t imp_prim_ge;
extern const imp_clo_t imp_prim_eq;
extern const imp_clo_t imp_prim_ne;
extern const imp_clo_t imp_prim_true;
extern const imp_clo_t imp_prim_false;
extern const imp_clo_t imp_prim_print;

// Emitted by the compiler: runs every top-level expression in order.
void imp_start(void);

#endif // IMPURELL_RT_H
