#define _GNU_SOURCE

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern unsigned int SDL_LogGetPriority(int category);
extern void SDL_LogGetOutputFunction(void (**callback)(void *, int, unsigned int, const char *), void **userdata);

static _Thread_local char *safe_sdl_error_buffer = NULL;
static _Thread_local int safe_sdl_error_active = 0;

__attribute__((visibility("hidden")))
void safe_sdl_store_error_message(const char *message)
{
    const char *source = message ? message : "";
    size_t len = strlen(source) + 1;
    char *buffer = (char *)realloc(safe_sdl_error_buffer, len);
    if (buffer == NULL) {
        return;
    }
    memcpy(buffer, source, len);
    safe_sdl_error_buffer = buffer;
    safe_sdl_error_active = 1;
}

__attribute__((visibility("hidden")))
const char *safe_sdl_get_error_message(void)
{
    if (safe_sdl_error_active && safe_sdl_error_buffer != NULL) {
        return safe_sdl_error_buffer;
    }
    return "";
}

__attribute__((visibility("hidden")))
void safe_sdl_clear_error_message(void)
{
    safe_sdl_error_active = 0;
    if (safe_sdl_error_buffer != NULL) {
        safe_sdl_error_buffer[0] = '\0';
    }
}

__attribute__((visibility("hidden")))
int safe_sdl_error_is_active(void)
{
    return safe_sdl_error_active;
}

static int safe_sdl_format_message(char **buffer, const char *fmt, va_list ap)
{
    int len;
    va_list ap_copy;

    if (fmt == NULL) {
        *buffer = NULL;
        return 0;
    }

    va_copy(ap_copy, ap);
    len = vsnprintf(NULL, 0, fmt, ap_copy);
    va_end(ap_copy);
    if (len < 0) {
        *buffer = NULL;
        return len;
    }

    *buffer = (char *)malloc((size_t)len + 1);
    if (*buffer == NULL) {
        return -1;
    }

    vsnprintf(*buffer, (size_t)len + 1, fmt, ap);
    return len;
}

int SDL_SetError(const char *fmt, ...)
{
    char *buffer = NULL;
    va_list ap;

    if (fmt == NULL) {
        safe_sdl_store_error_message("");
        return -1;
    }

    va_start(ap, fmt);
    if (safe_sdl_format_message(&buffer, fmt, ap) < 0) {
        va_end(ap);
        safe_sdl_store_error_message("SDL_SetError formatting failed");
        return -1;
    }
    va_end(ap);

    safe_sdl_store_error_message(buffer ? buffer : "");
    if (SDL_LogGetPriority(1) <= 2 && buffer != NULL) {
        SDL_LogDebug(1, "%s", buffer);
    }
    free(buffer);
    return -1;
}

int SDL_vsnprintf(char *text, size_t maxlen, const char *fmt, va_list ap)
{
    return vsnprintf(text, maxlen, fmt, ap);
}

int SDL_snprintf(char *text, size_t maxlen, const char *fmt, ...)
{
    int result;
    va_list ap;

    va_start(ap, fmt);
    result = SDL_vsnprintf(text, maxlen, fmt, ap);
    va_end(ap);
    return result;
}

int SDL_vasprintf(char **strp, const char *fmt, va_list ap)
{
    return vasprintf(strp, fmt, ap);
}

int SDL_asprintf(char **strp, const char *fmt, ...)
{
    int result;
    va_list ap;

    va_start(ap, fmt);
    result = SDL_vasprintf(strp, fmt, ap);
    va_end(ap);
    return result;
}

int SDL_vsscanf(const char *text, const char *fmt, va_list ap)
{
    return vsscanf(text, fmt, ap);
}

int SDL_sscanf(const char *text, const char *fmt, ...)
{
    int result;
    va_list ap;

    va_start(ap, fmt);
    result = SDL_vsscanf(text, fmt, ap);
    va_end(ap);
    return result;
}

void SDL_LogMessageV(int category, unsigned int priority, const char *fmt, va_list ap)
{
    void (*callback)(void *, int, unsigned int, const char *) = NULL;
    void *userdata = NULL;
    char *buffer = NULL;

    if (fmt == NULL || priority == 0 || priority >= 7) {
        return;
    }
    if (priority < SDL_LogGetPriority(category)) {
        return;
    }
    if (safe_sdl_format_message(&buffer, fmt, ap) < 0) {
        safe_sdl_store_error_message("SDL_LogMessageV formatting failed");
        return;
    }

    SDL_LogGetOutputFunction(&callback, &userdata);
    if (callback != NULL) {
        callback(userdata, category, priority, buffer ? buffer : "");
    }
    free(buffer);
}

void SDL_LogMessage(int category, unsigned int priority, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, priority, fmt, ap);
    va_end(ap);
}

void SDL_Log(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(0, 3, fmt, ap);
    va_end(ap);
}

void SDL_LogVerbose(int category, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 1, fmt, ap);
    va_end(ap);
}

void SDL_LogDebug(int category, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 2, fmt, ap);
    va_end(ap);
}

void SDL_LogInfo(int category, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 3, fmt, ap);
    va_end(ap);
}

void SDL_LogWarn(int category, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 4, fmt, ap);
    va_end(ap);
}

void SDL_LogError(int category, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 5, fmt, ap);
    va_end(ap);
}

void SDL_LogCritical(int category, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    SDL_LogMessageV(category, 6, fmt, ap);
    va_end(ap);
}
