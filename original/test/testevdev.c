/*
  Copyright (C) 1997-2024 Sam Lantinga <slouken@libsdl.org>
  Copyright (C) 2020-2022 Collabora Ltd.

  This software is provided 'as-is', without any express or implied
  warranty.  In no event will the authors be held liable for any damages
  arising from the use of this software.

  Permission is granted to anyone to use this software for any purpose,
  including commercial applications, and to alter it and redistribute it
  freely.
*/

#include "SDL.h"

#if defined(__linux__) && defined(HAVE_LINUX_INPUT_H)

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <linux/input.h>
#include <linux/joystick.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>

#define BITS_PER_LONG    (sizeof(unsigned long) * 8)
#define NBITS(x)         ((((x)-1) / BITS_PER_LONG) + 1)
#define SET_BIT(a, bit)  ((a)[(bit) / BITS_PER_LONG] |= (1UL << ((bit) % BITS_PER_LONG)))

#define TEST_GAMEPAD_NAME "SDL Fake evdev Gamepad"
#define TEST_USB_VENDOR_SONY 0x054c
#define TEST_USB_PRODUCT_DUALSHOCK4 0x09cc

typedef enum
{
    FAKE_DEVICE_NONE,
    FAKE_DEVICE_JOYSTICK
} FakeDeviceKind;

typedef struct
{
    FakeDeviceKind kind;
    const char *name;
    struct input_id input_id;
    unsigned long evbit[NBITS(EV_MAX)];
    unsigned long keybit[NBITS(KEY_MAX)];
    unsigned long relbit[NBITS(REL_MAX)];
    unsigned long absbit[NBITS(ABS_MAX)];
    unsigned long ffbit[NBITS(FF_MAX)];
    unsigned long keystate[NBITS(KEY_MAX)];
    struct input_absinfo absinfo[ABS_CNT];
} FakeDevice;

typedef struct
{
    int fd;
    FakeDeviceKind kind;
} FakeFd;

static char fake_dir_template[] = "/tmp/sdl-testevdev-XXXXXX";
static char joystick_path[PATH_MAX];
static FakeDevice joystick_device;
static FakeFd fake_fds[16];

int __real_open(const char *pathname, int flags, ...);
int __real_close(int fd);
ssize_t __real_read(int fd, void *buf, size_t count);
int __real_ioctl(int fd, unsigned long request, ...);

static int
fail_sdl(const char *operation)
{
    SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "%s: %s", operation, SDL_GetError());
    return 1;
}

static void
clear_fake_fd(int fd)
{
    int i;

    for (i = 0; i < SDL_arraysize(fake_fds); ++i) {
        if (fake_fds[i].fd == fd) {
            fake_fds[i].fd = -1;
            fake_fds[i].kind = FAKE_DEVICE_NONE;
            return;
        }
    }
}

static void
register_fake_fd(int fd, FakeDeviceKind kind)
{
    int i;

    clear_fake_fd(fd);

    for (i = 0; i < SDL_arraysize(fake_fds); ++i) {
        if (fake_fds[i].fd < 0) {
            fake_fds[i].fd = fd;
            fake_fds[i].kind = kind;
            return;
        }
    }
}

static FakeDeviceKind
get_fake_fd_kind(int fd)
{
    int i;

    for (i = 0; i < SDL_arraysize(fake_fds); ++i) {
        if (fake_fds[i].fd == fd) {
            return fake_fds[i].kind;
        }
    }
    return FAKE_DEVICE_NONE;
}

static FakeDevice *
get_fake_device_for_fd(int fd)
{
    switch (get_fake_fd_kind(fd)) {
    case FAKE_DEVICE_JOYSTICK:
        return &joystick_device;
    default:
        return NULL;
    }
}

static void
copy_ioctl_data(unsigned long request, const void *src, size_t src_len, void *dst)
{
    const size_t dst_len = _IOC_SIZE(request);
    const size_t copy_len = SDL_min(dst_len, src_len);

    SDL_memset(dst, 0, dst_len);
    SDL_memcpy(dst, src, copy_len);
}

static void
copy_ioctl_string(unsigned long request, const char *src, char *dst)
{
    const size_t dst_len = _IOC_SIZE(request);

    if (dst_len == 0) {
        return;
    }

    SDL_strlcpy(dst, src, dst_len);
}

static void
set_absinfo(struct input_absinfo *info,
            int minimum, int maximum, int fuzz, int flat, int resolution, int value)
{
    SDL_zero(*info);
    info->minimum = minimum;
    info->maximum = maximum;
    info->fuzz = fuzz;
    info->flat = flat;
    info->resolution = resolution;
    info->value = value;
}

static void
init_fake_gamepad(FakeDevice *device)
{
    SDL_zero(*device);
    device->kind = FAKE_DEVICE_JOYSTICK;
    device->name = TEST_GAMEPAD_NAME;
    device->input_id.bustype = BUS_USB;
    device->input_id.vendor = TEST_USB_VENDOR_SONY;
    device->input_id.product = TEST_USB_PRODUCT_DUALSHOCK4;
    device->input_id.version = 0x0001;

    SET_BIT(device->evbit, EV_KEY);
    SET_BIT(device->evbit, EV_ABS);

    SET_BIT(device->keybit, BTN_A);
    SET_BIT(device->keybit, BTN_B);
    SET_BIT(device->keybit, BTN_X);
    SET_BIT(device->keybit, BTN_Y);
    SET_BIT(device->keybit, BTN_TL);
    SET_BIT(device->keybit, BTN_TR);
    SET_BIT(device->keybit, BTN_TL2);
    SET_BIT(device->keybit, BTN_TR2);
    SET_BIT(device->keybit, BTN_SELECT);
    SET_BIT(device->keybit, BTN_START);
    SET_BIT(device->keybit, BTN_MODE);
    SET_BIT(device->keybit, BTN_THUMBL);
    SET_BIT(device->keybit, BTN_THUMBR);
    SET_BIT(device->keystate, BTN_A);

    SET_BIT(device->absbit, ABS_X);
    SET_BIT(device->absbit, ABS_Y);
    SET_BIT(device->absbit, ABS_Z);
    SET_BIT(device->absbit, ABS_RX);
    SET_BIT(device->absbit, ABS_RY);
    SET_BIT(device->absbit, ABS_RZ);
    SET_BIT(device->absbit, ABS_HAT0X);
    SET_BIT(device->absbit, ABS_HAT0Y);

    set_absinfo(&device->absinfo[ABS_X], -32768, 32767, 16, 128, 256, 16384);
    set_absinfo(&device->absinfo[ABS_Y], -32768, 32767, 16, 128, 256, 0);
    set_absinfo(&device->absinfo[ABS_Z], 0, 255, 0, 0, 1, 0);
    set_absinfo(&device->absinfo[ABS_RX], -32768, 32767, 16, 128, 256, 0);
    set_absinfo(&device->absinfo[ABS_RY], -32768, 32767, 16, 128, 256, 0);
    set_absinfo(&device->absinfo[ABS_RZ], 0, 255, 0, 0, 1, 0);
    set_absinfo(&device->absinfo[ABS_HAT0X], -1, 1, 0, 0, 0, 1);
    set_absinfo(&device->absinfo[ABS_HAT0Y], -1, 1, 0, 0, 0, -1);
}

static int
create_fake_device_paths(void)
{
    int fd;

    if (!mkdtemp(fake_dir_template)) {
        SDL_SetError("mkdtemp failed: %s", strerror(errno));
        return -1;
    }

    if (SDL_snprintf(joystick_path, sizeof(joystick_path), "%s/joystick-event0", fake_dir_template) >= (int)sizeof(joystick_path)) {
        SDL_SetError("Fake device path too long");
        return -1;
    }

    fd = __real_open(joystick_path, O_CREAT | O_EXCL | O_RDWR | O_CLOEXEC, 0600);
    if (fd < 0) {
        SDL_SetError("open(%s) failed: %s", joystick_path, strerror(errno));
        return -1;
    }
    __real_close(fd);

    init_fake_gamepad(&joystick_device);
    return 0;
}

static void
destroy_fake_device_paths(void)
{
    unlink(joystick_path);
    rmdir(fake_dir_template);
    joystick_path[0] = '\0';
}

static int
path_matches(const char *path, const char *expected)
{
    return expected[0] != '\0' && SDL_strcmp(path, expected) == 0;
}

int
__wrap_open(const char *pathname, int flags, ...)
{
    mode_t mode = 0;
    FakeDevice *device = NULL;
    int fd;

    if (flags & O_CREAT) {
        va_list ap;

        va_start(ap, flags);
        mode = va_arg(ap, mode_t);
        va_end(ap);
    }

    if (path_matches(pathname, joystick_path)) {
        device = &joystick_device;
    }

    if (!device) {
        if (flags & O_CREAT) {
            return __real_open(pathname, flags, mode);
        }
        return __real_open(pathname, flags);
    }

    if (flags & O_CREAT) {
        fd = __real_open(pathname, flags, mode);
    } else {
        fd = __real_open(pathname, flags);
    }
    if (fd >= 0) {
        register_fake_fd(fd, device->kind);
    }
    return fd;
}

int
__wrap_close(int fd)
{
    clear_fake_fd(fd);
    return __real_close(fd);
}

ssize_t
__wrap_read(int fd, void *buf, size_t count)
{
    if (get_fake_fd_kind(fd) != FAKE_DEVICE_NONE) {
        (void)buf;
        (void)count;
        errno = EAGAIN;
        return -1;
    }
    return __real_read(fd, buf, count);
}

int
__wrap_ioctl(int fd, unsigned long request, ...)
{
    FakeDevice *device = get_fake_device_for_fd(fd);
    void *arg;

    va_list ap;
    va_start(ap, request);
    arg = va_arg(ap, void *);
    va_end(ap);

    if (!device) {
        return __real_ioctl(fd, request, arg);
    }

    if (request == JSIOCGNAME(128)) {
        errno = ENOTTY;
        return -1;
    }

    if (request == EVIOCGID) {
        copy_ioctl_data(request, &device->input_id, sizeof(device->input_id), arg);
        return 0;
    }
    if (request == EVIOCGNAME(128)) {
        copy_ioctl_string(request, device->name, (char *)arg);
        return 0;
    }

    if (request == EVIOCGBIT(0, sizeof(device->evbit))) {
        copy_ioctl_data(request, device->evbit, sizeof(device->evbit), arg);
        return 0;
    }
    if (request == EVIOCGBIT(EV_KEY, sizeof(device->keybit))) {
        copy_ioctl_data(request, device->keybit, sizeof(device->keybit), arg);
        return 0;
    }
    if (request == EVIOCGBIT(EV_REL, sizeof(device->relbit))) {
        copy_ioctl_data(request, device->relbit, sizeof(device->relbit), arg);
        return 0;
    }
    if (request == EVIOCGBIT(EV_ABS, sizeof(device->absbit))) {
        copy_ioctl_data(request, device->absbit, sizeof(device->absbit), arg);
        return 0;
    }
    if (request == EVIOCGBIT(EV_FF, sizeof(device->ffbit))) {
        copy_ioctl_data(request, device->ffbit, sizeof(device->ffbit), arg);
        return 0;
    }
    if (request == EVIOCGKEY(sizeof(device->keystate))) {
        copy_ioctl_data(request, device->keystate, sizeof(device->keystate), arg);
        return 0;
    }

    if (request == EVIOCGABS(ABS_X) || request == EVIOCGABS(ABS_Y) ||
        request == EVIOCGABS(ABS_Z) || request == EVIOCGABS(ABS_RX) ||
        request == EVIOCGABS(ABS_RY) || request == EVIOCGABS(ABS_RZ) ||
        request == EVIOCGABS(ABS_HAT0X) || request == EVIOCGABS(ABS_HAT0Y)) {
        int code;

        if (request == EVIOCGABS(ABS_X)) {
            code = ABS_X;
        } else if (request == EVIOCGABS(ABS_Y)) {
            code = ABS_Y;
        } else if (request == EVIOCGABS(ABS_Z)) {
            code = ABS_Z;
        } else if (request == EVIOCGABS(ABS_RX)) {
            code = ABS_RX;
        } else if (request == EVIOCGABS(ABS_RY)) {
            code = ABS_RY;
        } else if (request == EVIOCGABS(ABS_RZ)) {
            code = ABS_RZ;
        } else if (request == EVIOCGABS(ABS_HAT0X)) {
            code = ABS_HAT0X;
        } else {
            code = ABS_HAT0Y;
        }
        copy_ioctl_data(request, &device->absinfo[code], sizeof(device->absinfo[code]), arg);
        return 0;
    }

    errno = EINVAL;
    return -1;
}

static int
run_test(void)
{
    SDL_GameController *controller = NULL;
    const char *device_name = NULL;
    const char *device_path = NULL;
    int device_index = -1;
    int num_joysticks;
    int i;
    int result = 1;
    char guid_string[33];
    char mapping[512];
    Sint16 leftx;

    SDL_memset(fake_fds, 0xff, sizeof(fake_fds));

    if (create_fake_device_paths() < 0) {
        return fail_sdl("create_fake_device_paths");
    }

    SDL_SetHint(SDL_HINT_JOYSTICK_DEVICE, joystick_path);

    if (SDL_Init(SDL_INIT_GAMECONTROLLER) < 0) {
        result = fail_sdl("SDL_Init(SDL_INIT_GAMECONTROLLER)");
        goto done;
    }

    num_joysticks = SDL_NumJoysticks();
    if (num_joysticks < 1) {
        SDL_SetError("Expected at least 1 joystick, got %d", num_joysticks);
        result = fail_sdl("SDL_NumJoysticks");
        goto done;
    }

    for (i = 0; i < num_joysticks; ++i) {
        device_path = SDL_JoystickPathForIndex(i);
        if (device_path && SDL_strcmp(device_path, joystick_path) == 0) {
            device_index = i;
            break;
        }
    }
    if (device_index < 0) {
        SDL_SetError("Couldn't find synthetic joystick path \"%s\" among %d devices",
                     joystick_path, num_joysticks);
        result = fail_sdl("SDL_JoystickPathForIndex");
        goto done;
    }

    device_name = SDL_JoystickNameForIndex(device_index);
    if (!device_name || !*device_name) {
        SDL_SetError("SDL_JoystickNameForIndex(%d) returned no name", device_index);
        result = fail_sdl("SDL_JoystickNameForIndex");
        goto done;
    }

    device_path = SDL_JoystickPathForIndex(device_index);
    if (!device_path || SDL_strcmp(device_path, joystick_path) != 0) {
        SDL_SetError("SDL_JoystickPathForIndex(%d) returned \"%s\", expected \"%s\"",
                     device_index, device_path ? device_path : "(null)", joystick_path);
        result = fail_sdl("SDL_JoystickPathForIndex");
        goto done;
    }

    if (SDL_JoystickGetDeviceVendor(device_index) != TEST_USB_VENDOR_SONY ||
        SDL_JoystickGetDeviceProduct(device_index) != TEST_USB_PRODUCT_DUALSHOCK4) {
        SDL_SetError("Unexpected vendor/product 0x%04x/0x%04x",
                     SDL_JoystickGetDeviceVendor(device_index),
                     SDL_JoystickGetDeviceProduct(device_index));
        result = fail_sdl("SDL_JoystickGetDeviceVendor/Product");
        goto done;
    }

    SDL_JoystickGetGUIDString(SDL_JoystickGetDeviceGUID(device_index), guid_string, sizeof(guid_string));
    SDL_snprintf(mapping, sizeof(mapping),
                 "%s,%s,a:b0,b:b1,back:b8,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,dpup:h0.1,guide:b10,leftshoulder:b4,leftstick:b11,lefttrigger:a2,leftx:a0,lefty:a1,rightshoulder:b5,rightstick:b12,righttrigger:a5,rightx:a3,righty:a4,start:b9,x:b2,y:b3,",
                 guid_string, device_name);
    if (SDL_GameControllerAddMapping(mapping) < 0) {
        result = fail_sdl("SDL_GameControllerAddMapping");
        goto done;
    }

    if (!SDL_IsGameController(device_index)) {
        SDL_SetError("SDL_IsGameController(%d) returned false", device_index);
        result = fail_sdl("SDL_IsGameController");
        goto done;
    }

    controller = SDL_GameControllerOpen(device_index);
    if (!controller) {
        result = fail_sdl("SDL_GameControllerOpen");
        goto done;
    }

    SDL_GameControllerUpdate();

    if (SDL_GameControllerGetButton(controller, SDL_CONTROLLER_BUTTON_A) != SDL_PRESSED) {
        SDL_SetError("SDL_GameControllerGetButton(SDL_CONTROLLER_BUTTON_A) did not return SDL_PRESSED");
        result = fail_sdl("SDL_GameControllerGetButton");
        goto done;
    }

    if (SDL_GameControllerGetButton(controller, SDL_CONTROLLER_BUTTON_DPAD_RIGHT) != SDL_PRESSED ||
        SDL_GameControllerGetButton(controller, SDL_CONTROLLER_BUTTON_DPAD_UP) != SDL_PRESSED ||
        SDL_GameControllerGetButton(controller, SDL_CONTROLLER_BUTTON_DPAD_LEFT) != SDL_RELEASED ||
        SDL_GameControllerGetButton(controller, SDL_CONTROLLER_BUTTON_DPAD_DOWN) != SDL_RELEASED) {
        SDL_SetError("SDL_GameControllerGetButton() did not expose the expected hat state");
        result = fail_sdl("SDL_GameControllerGetButton(dpad)");
        goto done;
    }

    leftx = SDL_GameControllerGetAxis(controller, SDL_CONTROLLER_AXIS_LEFTX);
    if (leftx < 12000) {
        SDL_SetError("SDL_GameControllerGetAxis(SDL_CONTROLLER_AXIS_LEFTX) returned %d", leftx);
        result = fail_sdl("SDL_GameControllerGetAxis");
        goto done;
    }

    result = 0;

done:
    if (controller) {
        SDL_GameControllerClose(controller);
    }
    if (SDL_WasInit(SDL_INIT_GAMECONTROLLER)) {
        SDL_Quit();
    }
    SDL_SetHint(SDL_HINT_JOYSTICK_DEVICE, "");
    destroy_fake_device_paths();
    return result;
}

#else

static int
run_test(void)
{
    SDL_Log("Skipping evdev test on this platform");
    return 0;
}

#endif

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;
    return run_test();
}
