#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#define SDLTEST_MAX_LOGMESSAGE_LENGTH 3584

typedef struct SDLTest_TextWindow SDLTest_TextWindow;

void SDLTest_LogFromBuffer(const char *message);
void SDLTest_LogErrorFromBuffer(const char *message);
void SDLTest_AssertFromBuffer(int assertCondition, const char *message);
int SDLTest_AssertCheckFromBuffer(int assertCondition, const char *message);
void SDLTest_AssertPassFromBuffer(const char *message);
void SDLTest_TextWindowAddTextFromBuffer(SDLTest_TextWindow *textwin, const char *text);

static void sdltest_format(char *buffer, size_t buffer_len, const char *fmt, va_list ap)
{
    if (!buffer || buffer_len == 0) {
        return;
    }
    buffer[0] = '\0';
    if (!fmt) {
        return;
    }
    (void)vsnprintf(buffer, buffer_len, fmt, ap);
}

void SDLTest_Log(const char *fmt, ...)
{
    char buffer[SDLTEST_MAX_LOGMESSAGE_LENGTH];
    va_list ap;
    va_start(ap, fmt);
    sdltest_format(buffer, sizeof(buffer), fmt, ap);
    va_end(ap);
    SDLTest_LogFromBuffer(buffer);
}

void SDLTest_LogError(const char *fmt, ...)
{
    char buffer[SDLTEST_MAX_LOGMESSAGE_LENGTH];
    va_list ap;
    va_start(ap, fmt);
    sdltest_format(buffer, sizeof(buffer), fmt, ap);
    va_end(ap);
    SDLTest_LogErrorFromBuffer(buffer);
}

void SDLTest_Assert(int assertCondition, const char *assertDescription, ...)
{
    char buffer[SDLTEST_MAX_LOGMESSAGE_LENGTH];
    va_list ap;
    va_start(ap, assertDescription);
    sdltest_format(buffer, sizeof(buffer), assertDescription, ap);
    va_end(ap);
    SDLTest_AssertFromBuffer(assertCondition, buffer);
}

int SDLTest_AssertCheck(int assertCondition, const char *assertDescription, ...)
{
    char buffer[SDLTEST_MAX_LOGMESSAGE_LENGTH];
    va_list ap;
    va_start(ap, assertDescription);
    sdltest_format(buffer, sizeof(buffer), assertDescription, ap);
    va_end(ap);
    return SDLTest_AssertCheckFromBuffer(assertCondition, buffer);
}

void SDLTest_AssertPass(const char *assertDescription, ...)
{
    char buffer[SDLTEST_MAX_LOGMESSAGE_LENGTH];
    va_list ap;
    va_start(ap, assertDescription);
    sdltest_format(buffer, sizeof(buffer), assertDescription, ap);
    va_end(ap);
    SDLTest_AssertPassFromBuffer(buffer);
}

void SDLTest_TextWindowAddText(SDLTest_TextWindow *textwin, const char *fmt, ...)
{
    char buffer[1024];
    va_list ap;
    va_start(ap, fmt);
    sdltest_format(buffer, sizeof(buffer), fmt, ap);
    va_end(ap);
    SDLTest_TextWindowAddTextFromBuffer(textwin, buffer);
}
