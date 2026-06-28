// Minimal ALSH runtime: arena and string helpers (simplified)
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

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
        // fallback to malloc
        return malloc(size);
    }
    size_t need = size;
    if (current_arena->off + need > current_arena->cap) {
        // not enough space, fallback to malloc for simplicity
        return malloc(size);
    }
    void *ptr = current_arena->mem + current_arena->off;
    current_arena->off += need;
    return ptr;
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
