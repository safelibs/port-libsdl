#define _GNU_SOURCE

#include <dlfcn.h>
#include <pthread.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static pthread_once_t safe_sdl_system_once = PTHREAD_ONCE_INIT;
static void *safe_sdl_system_handle_value = NULL;

static void safe_sdl_load_failure(const char *message) {
    fprintf(stderr, "safe-sdl phase-2 variadic forwarding error: %s\n", message);
    abort();
}

static void safe_sdl_open_system_runtime(void) {
    const char *override_path = getenv("SAFE_SDL_SYSTEM_LIBSDL2");
    const char *fallbacks[] = {
        "/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0",
        "/lib/x86_64-linux-gnu/libSDL2-2.0.so.0",
    };

    if (override_path && override_path[0] != '\0') {
        safe_sdl_system_handle_value = dlopen(override_path, RTLD_NOW | RTLD_LOCAL);
        if (safe_sdl_system_handle_value != NULL) {
            return;
        }
    }

    for (size_t i = 0; i < (sizeof(fallbacks) / sizeof(fallbacks[0])); i++) {
        safe_sdl_system_handle_value = dlopen(fallbacks[i], RTLD_NOW | RTLD_LOCAL);
        if (safe_sdl_system_handle_value != NULL) {
            return;
        }
    }

    safe_sdl_load_failure(dlerror() ? dlerror() : "unable to open the system SDL runtime");
}

static void *safe_sdl_symbol(const char *name) {
    pthread_once(&safe_sdl_system_once, safe_sdl_open_system_runtime);
    void *symbol = dlsym(safe_sdl_system_handle_value, name);
    if (symbol == NULL) {
        safe_sdl_load_failure(name);
    }
    return symbol;
}

void safe_sdl_set_error_message(const char *message) {
    typedef int (*set_error_fn)(const char *fmt, ...);
    set_error_fn real = (set_error_fn)safe_sdl_symbol("SDL_SetError");
    real("%s", message ? message : "");
}

int SDL_SetError(const char *fmt, ...) {
    int len = 0;
    char *buffer = NULL;
    va_list ap;
    va_list ap_copy;

    if (fmt == NULL) {
        safe_sdl_set_error_message("");
        return -1;
    }

    va_start(ap, fmt);
    va_copy(ap_copy, ap);
    len = vsnprintf(NULL, 0, fmt, ap_copy);
    va_end(ap_copy);

    if (len < 0) {
        va_end(ap);
        safe_sdl_set_error_message("SDL_SetError formatting failed");
        return -1;
    }

    buffer = (char *)malloc((size_t)len + 1);
    if (buffer == NULL) {
        va_end(ap);
        safe_sdl_set_error_message("out of memory");
        return -1;
    }

    vsnprintf(buffer, (size_t)len + 1, fmt, ap);
    va_end(ap);
    safe_sdl_set_error_message(buffer);
    free(buffer);
    return -1;
}

int SDL_vsnprintf(char *text, size_t maxlen, const char *fmt, va_list ap) {
    typedef int (*vsnprintf_fn)(char *text, size_t maxlen, const char *fmt, va_list ap);
    vsnprintf_fn real = (vsnprintf_fn)safe_sdl_symbol("SDL_vsnprintf");
    return real(text, maxlen, fmt, ap);
}

int SDL_snprintf(char *text, size_t maxlen, const char *fmt, ...) {
    int result;
    va_list ap;

    va_start(ap, fmt);
    result = SDL_vsnprintf(text, maxlen, fmt, ap);
    va_end(ap);
    return result;
}

int SDL_vasprintf(char **strp, const char *fmt, va_list ap) {
    typedef int (*vasprintf_fn)(char **strp, const char *fmt, va_list ap);
    vasprintf_fn real = (vasprintf_fn)safe_sdl_symbol("SDL_vasprintf");
    return real(strp, fmt, ap);
}

int SDL_asprintf(char **strp, const char *fmt, ...) {
    int result;
    va_list ap;

    va_start(ap, fmt);
    result = SDL_vasprintf(strp, fmt, ap);
    va_end(ap);
    return result;
}

int SDL_vsscanf(const char *text, const char *fmt, va_list ap) {
    typedef int (*vsscanf_fn)(const char *text, const char *fmt, va_list ap);
    vsscanf_fn real = (vsscanf_fn)safe_sdl_symbol("SDL_vsscanf");
    return real(text, fmt, ap);
}

int SDL_sscanf(const char *text, const char *fmt, ...) {
    int result;
    va_list ap;

    va_start(ap, fmt);
    result = SDL_vsscanf(text, fmt, ap);
    va_end(ap);
    return result;
}

void SDL_LogMessageV(int category, unsigned int priority, const char *fmt, va_list ap) {
    typedef void (*log_message_v_fn)(int category, unsigned int priority, const char *fmt, va_list ap);
    log_message_v_fn real = (log_message_v_fn)safe_sdl_symbol("SDL_LogMessageV");
    real(category, priority, fmt, ap);
}

void SDL_LogMessage(int category, unsigned int priority, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, priority, fmt, ap);
    va_end(ap);
}

void SDL_Log(const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(0, 3, fmt, ap);
    va_end(ap);
}

void SDL_LogVerbose(int category, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 1, fmt, ap);
    va_end(ap);
}

void SDL_LogDebug(int category, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 2, fmt, ap);
    va_end(ap);
}

void SDL_LogInfo(int category, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 3, fmt, ap);
    va_end(ap);
}

void SDL_LogWarn(int category, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 4, fmt, ap);
    va_end(ap);
}

void SDL_LogError(int category, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 5, fmt, ap);
    va_end(ap);
}

void SDL_LogCritical(int category, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 6, fmt, ap);
    va_end(ap);
}
