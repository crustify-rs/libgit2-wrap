#include <git2.h>
#include <git2/sys/credential.h>
#include <git2/sys/config.h>
#include <git2/sys/odb_backend.h>
#include <git2/sys/refdb_backend.h>
#include <git2/sys/repository.h>
#include <git2/sys/transport.h>
#include "util.h"
#include "cache.h"

/* Allocator primitives needed by the Rust bindings. */
void *crustify_git__malloc(size_t len);
char *crustify_git__strdup(const char *str);
void crustify_git__free(void *ptr);
