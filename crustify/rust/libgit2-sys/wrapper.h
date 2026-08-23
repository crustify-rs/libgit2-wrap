#include <git2.h>

/* Bindgen shims for libgit2's header-inline allocator primitives. */
void *crustify_git__malloc(size_t len);
void crustify_git__free(void *ptr);
