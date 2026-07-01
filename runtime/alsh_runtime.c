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

void *alsh_arena_alloc(size_t size) {
    if (!current_arena) {
        return malloc(size);
    }
    size_t aligned_off = align_up(current_arena->off, 8);
    if (aligned_off + size > current_arena->cap) {
        return malloc(size); // TODO: overflow allocations currently untracked/leaked
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

