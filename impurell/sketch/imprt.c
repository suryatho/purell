#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  uint8_t *buf;
  size_t cap;
  size_t head;
} heap_t;

heap_t heap_init() { return (heap_t){.buf = NULL, .cap = 128, .head = 0}; }

void heap_reset(heap_t *heap) {
  memset(heap->buf, 0, heap->cap);
  heap->head = 0;
}

void heap_free(heap_t *heap) { free(heap->buf); }

void *heap_alloc(heap_t *heap, size_t size) {
  if (heap->buf == NULL) {
    heap->buf = malloc(heap->cap);
  }
  if (heap->head + size >= heap->cap) {
    heap->cap <<= 1;
    heap->buf = realloc(heap->buf, heap->cap);
  }
  void *ret = &heap->buf[heap->head];
  heap->head += size;
  return ret;
}

// LSB of pointer is one if value is a number
#define IS_FUN(imp_ptr) (((size_t)(imp_ptr) & 1) == 0)
#define IS_NUM(imp_ptr) (((size_t)(imp_ptr) & 1) == 1)

// Declare types
typedef union imp_value_t imp_value_t;
typedef imp_value_t *(*imp_fun_t)(imp_value_t *);
union imp_value_t {
  imp_fun_t as_fun;
  uint64_t as_num;
};

imp_value_t *value_from_ptr(void *imp_ptr) {
  uintptr_t addr = (uintptr_t)imp_ptr;
  addr &= ~(uintptr_t)0b1;
  return (imp_value_t *)addr;
}

// Tries to show the imp_value_t struct
void imp_show(void *imp_ptr) {
  imp_value_t *imp_value = value_from_ptr(imp_ptr);
  if (IS_FUN(imp_ptr)) {
    printf("ERROR: Could not evaluate ended with %p\n", imp_value->as_fun);
  } else {
    printf("Result: %llu", imp_value->as_num);
  }
}

void imp_showexpr(const char *expr, int num) {
  printf("Expr %d: %s\n", num, expr);
}

static heap_t *heap = NULL;

// Create a imp_value out of a 64 bit integer
void *imp_num(uint64_t value) {
  assert(heap);
  imp_value_t *num = heap_alloc(heap, sizeof(imp_value_t));
  num->as_num = value;
  uintptr_t addr = (uintptr_t)num;
  addr |= 0b1;
  return (void *)addr;
}

// Native methods for impurell:
// Returns a new function that does the op onto the imp_value
void *imp_add(imp_value_t *imp_value);
void *imp_sub(imp_value_t *imp_value);
void *imp_mul(imp_value_t *imp_value);
void *imp_div(imp_value_t *imp_value);

static imp_value_t* cap_st[] = {};
static int cap_st_head = 0;

void cap_push(imp_value_t  *imp_value) {
    cap_st[++cap_st_head] = imp_value;
}
void cap_clear();
imp_value_t *cap_get(int i);

extern int imp_start(void);
int main(void) { return imp_start(); }
