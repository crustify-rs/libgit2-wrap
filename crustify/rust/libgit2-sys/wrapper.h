#include <git2.h>
#include <git2/sys/credential.h>
#include "util.h"

/* Allocator primitives needed by the Rust bindings. */
void *crustify_git__malloc(size_t len);
char *crustify_git__strdup(const char *str);
void crustify_git__free(void *ptr);
