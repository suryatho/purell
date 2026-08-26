#include "imprt.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

// ---------------------------------------------------------------------------
// Arena
// ---------------------------------------------------------------------------

// Chunks are linked, never realloc'd. Growing by realloc would move live
// closures and invalidate every pointer already handed to compiled code.
typedef struct imp_chunk {
  struct imp_chunk *next;
  uint64_t cap;
  uint64_t head;
} imp_chunk_t;

#define IMP_CHUNK_MIN (64u * 1024u)

static imp_chunk_t *imp_chunks = NULL;

void *imp_alloc(uint64_t size) {
  size = (size + 7u) & ~(uint64_t)7u;

  if (imp_chunks == NULL || imp_chunks->head + size > imp_chunks->cap) {
    uint64_t cap = IMP_CHUNK_MIN;
    while (cap < size) {
      cap <<= 1;
    }
    imp_chunk_t *chunk = malloc(sizeof(imp_chunk_t) + cap);
    if (chunk == NULL) {
      imp_panic("out of memory");
    }
    chunk->next = imp_chunks;
    chunk->cap = cap;
    chunk->head = 0;
    imp_chunks = chunk;
  }

  void *ptr = (unsigned char *)(imp_chunks + 1) + imp_chunks->head;
  imp_chunks->head += size;
  return ptr;
}

imp_clo_t *imp_make(imp_fn_t fn, int64_t nfree) {
  imp_clo_t *clo =
      imp_alloc(sizeof(imp_clo_t) + (uint64_t)nfree * sizeof(imp_value_t));
  clo->fn = fn;
  clo->nfree = nfree;
  return clo;
}

void imp_arena_release(void) {
  imp_chunk_t *chunk = imp_chunks;
  while (chunk != NULL) {
    imp_chunk_t *next = chunk->next;
    free(chunk);
    chunk = next;
  }
  imp_chunks = NULL;
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

_Noreturn void imp_panic(const char *msg) {
  fflush(stdout);
  fprintf(stderr, "impurell: %s\n", msg);
  exit(1);
}

_Noreturn void imp_not_a_function(imp_value_t v) {
  fflush(stdout);
  fprintf(stderr, "impurell: applied a non-function: %" PRId64 "\n",
          IMP_UNTAG_NUM(v));
  exit(1);
}

_Noreturn void imp_type_error(const char *prim, imp_value_t v) {
  fflush(stdout);
  if (IMP_IS_NUM(v)) {
    fprintf(stderr, "impurell: %s expected a number, got %" PRId64 "\n", prim,
            IMP_UNTAG_NUM(v));
  } else {
    fprintf(stderr, "impurell: %s expected a number, got a function\n", prim);
  }
  exit(1);
}

void imp_print_value(imp_value_t v) {
  if (IMP_IS_NUM(v)) {
    printf("%" PRId64, IMP_UNTAG_NUM(v));
  } else {
    printf("<function>");
  }
}

void imp_show_expr(const char *src, imp_value_t result) {
  printf("Expr: %s\nResult: ", src);
  imp_print_value(result);
  printf("\n\n");
}

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------
//
// Every primitive is curried the same way a compiled lambda is: stage 1 takes
// the first argument and returns a heap closure capturing it, stage 2 does the
// work. That way the compiler treats `+` exactly like any other value.

#define IMP_BINARY(global, sym, result_expr)                                   \
  static imp_value_t global##_2(imp_clo_t *self, imp_value_t y) {              \
    imp_value_t x = self->env[0];                                              \
    if (!IMP_IS_NUM(x))                                                        \
      imp_type_error(sym, x);                                                  \
    if (!IMP_IS_NUM(y))                                                        \
      imp_type_error(sym, y);                                                  \
    int64_t a = IMP_UNTAG_NUM(x);                                              \
    int64_t b = IMP_UNTAG_NUM(y);                                              \
    (void)a;                                                                   \
    (void)b;                                                                   \
    return (result_expr);                                                      \
  }                                                                            \
  static imp_value_t global##_1(imp_clo_t *self, imp_value_t x) {              \
    (void)self;                                                                \
    imp_clo_t *clo = imp_make(global##_2, 1);                                  \
    clo->env[0] = x;                                                           \
    return IMP_FROM_CLO(clo);                                                  \
  }                                                                            \
  const imp_clo_t global = {global##_1, 0}

// Church booleans, so comparisons stay inside the lambda calculus:
//   true  = \x.\y.x     false = \x.\y.y
static imp_value_t church_true_2(imp_clo_t *self, imp_value_t y) {
  (void)y;
  return self->env[0];
}

static imp_value_t church_true_1(imp_clo_t *self, imp_value_t x) {
  (void)self;
  imp_clo_t *clo = imp_make(church_true_2, 1);
  clo->env[0] = x;
  return IMP_FROM_CLO(clo);
}

static imp_value_t church_false_2(imp_clo_t *self, imp_value_t y) {
  (void)self;
  return y;
}

// `false` discards both arguments, so its second stage captures nothing and
// can be a single static object rather than a fresh allocation.
static const imp_clo_t church_false_2_clo = {church_false_2, 0};

static imp_value_t church_false_1(imp_clo_t *self, imp_value_t x) {
  (void)self;
  (void)x;
  return IMP_FROM_CLO(&church_false_2_clo);
}

const imp_clo_t imp_prim_true = {church_true_1, 0};
const imp_clo_t imp_prim_false = {church_false_1, 0};

#define IMP_BOOL(cond)                                                         \
  ((cond) ? IMP_FROM_CLO(&imp_prim_true) : IMP_FROM_CLO(&imp_prim_false))

static int64_t imp_checked_div(const char *sym, int64_t a, int64_t b) {
  if (b == 0) {
    fflush(stdout);
    fprintf(stderr, "impurell: %s by zero\n", sym);
    exit(1);
  }
  return a / b;
}

static int64_t imp_checked_rem(const char *sym, int64_t a, int64_t b) {
  if (b == 0) {
    fflush(stdout);
    fprintf(stderr, "impurell: %s by zero\n", sym);
    exit(1);
  }
  return a % b;
}

// Arithmetic wraps within the 63-bit payload; there is no overflow trap.
IMP_BINARY(imp_prim_add, "+", IMP_TAG_NUM(a + b));
IMP_BINARY(imp_prim_sub, "-", IMP_TAG_NUM(a - b));
IMP_BINARY(imp_prim_mul, "*", IMP_TAG_NUM(a *b));
IMP_BINARY(imp_prim_div, "/", IMP_TAG_NUM(imp_checked_div("/", a, b)));
IMP_BINARY(imp_prim_rem, "%", IMP_TAG_NUM(imp_checked_rem("%", a, b)));
IMP_BINARY(imp_prim_lt, "<", IMP_BOOL(a < b));
IMP_BINARY(imp_prim_gt, ">", IMP_BOOL(a > b));
IMP_BINARY(imp_prim_le, "<=", IMP_BOOL(a <= b));
IMP_BINARY(imp_prim_ge, ">=", IMP_BOOL(a >= b));
IMP_BINARY(imp_prim_eq, "=", IMP_BOOL(a == b));
IMP_BINARY(imp_prim_ne, "/=", IMP_BOOL(a != b));

static imp_value_t prim_print_1(imp_clo_t *self, imp_value_t x) {
  (void)self;
  imp_print_value(x);
  printf("\n");
  return x;
}

const imp_clo_t imp_prim_print = {prim_print_1, 0};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

int main(void) {
  imp_start();
  imp_arena_release();
  fflush(stdout);
  return 0;
}
