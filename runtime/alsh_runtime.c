// Minimal ALSH runtime: arena and string helpers (simplified)
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

typedef struct alsh_str {
    int64_t len;
    int64_t cap;
    char *data;
} alsh_str;

// A simple stack of arenas (very small, single current arena)
typedef struct Arena {
    char *mem;
    size_t cap;
    size_t off;
    struct Arena *prev;
} Arena;

static Arena *current_arena = NULL;
// Track fallback malloc'd blocks for allocations that couldn't fit in arenas
typedef struct FallbackAlloc {
    void *ptr;
    struct FallbackAlloc *next;
} FallbackAlloc;

static FallbackAlloc *fallback_head = NULL;

static size_t align_up(size_t n, size_t align) {
    return (n + align - 1) & ~(align - 1);
}

void alsh_arena_push(size_t initial_cap) {
    Arena *a = (Arena*)malloc(sizeof(Arena));
    a->cap = initial_cap > 0 ? initial_cap : 65536;
    a->mem = (char*)malloc(a->cap);
    a->off = 0;
    a->prev = current_arena;
    current_arena = a;
}

void alsh_arena_pop(void) {
    if (!current_arena) return;
    Arena *prev = current_arena->prev;
    free(current_arena->mem);
    free(current_arena);
    current_arena = prev;
}

void alsh_arena_free_all(void) {
    while (current_arena) {
        Arena *prev = current_arena->prev;
        free(current_arena->mem);
        free(current_arena);
        current_arena = prev;
    }
    // free any fallback malloc'd blocks
    FallbackAlloc *f = fallback_head;
    while (f) {
        FallbackAlloc *n = f->next;
        free(f->ptr);
        free(f);
        f = n;
    }
    fallback_head = NULL;
}

// Register cleanup at process exit to free any remaining arenas allocated
static void alsh_runtime_register_cleanup(void) __attribute__((constructor));
static void alsh_runtime_register_cleanup(void) {
    /* Ensure there is at least one arena so small allocations
       use arena memory (and are freed at exit) instead of falling
       back to malloc which would not be tracked here. */
    alsh_arena_push(0);
    atexit(alsh_arena_free_all);
}

void *alsh_arena_alloc(size_t size) {
    if (!current_arena) {
        void *p = malloc(size);
        if (p) {
            FallbackAlloc *fa = (FallbackAlloc*)malloc(sizeof(FallbackAlloc));
            if (fa) {
                fa->ptr = p;
                fa->next = fallback_head;
                fallback_head = fa;
            }
        }
        return p;
    }
    size_t aligned_off = align_up(current_arena->off, 8);
    if (aligned_off + size > current_arena->cap) {
        void *p = malloc(size); // overflow allocations tracked so they can be freed
        if (p) {
            FallbackAlloc *fa = (FallbackAlloc*)malloc(sizeof(FallbackAlloc));
            if (fa) {
                fa->ptr = p;
                fa->next = fallback_head;
                fallback_head = fa;
            }
        }
        return p;
    }
    void *ptr = current_arena->mem + aligned_off;
    current_arena->off = aligned_off + size;
    return ptr;
}

// Concatenation
alsh_str *alsh_str_concat(alsh_str *a, alsh_str *b) {
    int64_t total_len = a->len + b->len;
    alsh_str *s = (alsh_str*)alsh_arena_alloc(sizeof(alsh_str));
    char *buf = (char*)alsh_arena_alloc((size_t)total_len + 1);
    if (!s || !buf) return NULL;
    memcpy(buf, a->data, (size_t)a->len);
    memcpy(buf + a->len, b->data, (size_t)b->len);
    buf[total_len] = '\0';
    s->len = total_len;
    s->cap = total_len;
    s->data = buf;
    return s;
}

// Create an alsh_str that points at existing data (used for static literals)
alsh_str *alsh_make_static_str(const char *data, int64_t len) {
    alsh_str *s = (alsh_str*)malloc(sizeof(alsh_str));
    s->len = len;
    s->cap = len;
    s->data = (char*)data;
    return s;
}

// Create an alsh_str in the current arena and copy data into it
alsh_str *alsh_make_heap_str(const char *data, int64_t len) {
    alsh_str *s = (alsh_str*)alsh_arena_alloc(sizeof(alsh_str));
    char *buf = (char*)alsh_arena_alloc((size_t)len + 1);
    if (!s || !buf) return NULL;
    memcpy(buf, data, (size_t)len);
    buf[len] = '\0';
    s->len = len;
    s->cap = len;
    s->data = buf;
    return s;
}


// Numeric -> alsh_str, for interpolating $numbers
alsh_str *alsh_int_to_str(int64_t n) {
    char tmp[32];
    int len = snprintf(tmp, sizeof(tmp), "%lld", (long long)n);
    return alsh_make_heap_str(tmp, len);
}

alsh_str *alsh_float_to_str(double f) {
    char tmp[64];
    int len = snprintf(tmp, sizeof(tmp), "%g", f);
    return alsh_make_heap_str(tmp, len);
}

// Varargs array helpers (allocate and store) for supported element types
void *alsh_make_varargs_array_i32(size_t count) {
    return alsh_arena_alloc(sizeof(int32_t) * count);
}

void alsh_varargs_store_i32(void *base, size_t idx, int32_t v) {
    int32_t *arr = (int32_t*)base;
    arr[idx] = v;
}

void *alsh_make_varargs_array_i64(size_t count) {
    return alsh_arena_alloc(sizeof(int64_t) * count);
}

void alsh_varargs_store_i64(void *base, size_t idx, int64_t v) {
    int64_t *arr = (int64_t*)base;
    arr[idx] = v;
}

void *alsh_make_varargs_array_f64(size_t count) {
    return alsh_arena_alloc(sizeof(double) * count);
}

void alsh_varargs_store_f64(void *base, size_t idx, double v) {
    double *arr = (double*)base;
    arr[idx] = v;
}

void *alsh_make_varargs_array_ptr(size_t count) {
    return alsh_arena_alloc(sizeof(void*) * count);
}

void alsh_varargs_store_ptr(void *base, size_t idx, void *v) {
    void **arr = (void**)base;
    arr[idx] = v;
}

int32_t alsh_varargs_get_i32(void *base, size_t idx) {
    int32_t *arr = (int32_t*)base;
    return arr[idx];
}

int64_t alsh_varargs_get_i64(void *base, size_t idx) {
    int64_t *arr = (int64_t*)base;
    return arr[idx];
}

double alsh_varargs_get_f64(void *base, size_t idx) {
    double *arr = (double*)base;
    return arr[idx];
}

void *alsh_varargs_get_ptr(void *base, size_t idx) {
    void **arr = (void**)base;
    return arr[idx];
}

