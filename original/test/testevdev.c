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
#include <linux/input.h>
#include <linux/uinput.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/types.h>
#include <unistd.h>

#define TEST_USB_VENDOR_SONY 0x054c
#define TEST_USB_PRODUCT_DUALSHOCK4 0x09cc
#define TEST_CONTROLLER_NAME "SDL Public evdev Gamepad"
#define TEST_SENSOR_NAME "SDL Public evdev Accelerometer"

typedef struct
{
    int code;
    int minimum;
    int maximum;
    int fuzz;
    int flat;
    int resolution;
    int value;
} UInputAxisDesc;

typedef struct
{
    int fd;
} UInputDevice;

static const int gamepad_keys[] = {
    BTN_A, BTN_B, BTN_X, BTN_Y,
    BTN_TL, BTN_TR, BTN_TL2, BTN_TR2,
    BTN_SELECT, BTN_START, BTN_MODE,
    BTN_THUMBL, BTN_THUMBR
};

static const UInputAxisDesc gamepad_axes[] = {
    { ABS_X, -32768, 32767, 16, 128, 256, 0 },
    { ABS_Y, -32768, 32767, 16, 128, 256, 0 },
    { ABS_Z, 0, 255, 0, 0, 1, 0 },
    { ABS_RX, -32768, 32767, 16, 128, 256, 0 },
    { ABS_RY, -32768, 32767, 16, 128, 256, 0 },
    { ABS_RZ, 0, 255, 0, 0, 1, 0 },
    { ABS_HAT0X, -1, 1, 0, 0, 0, 0 },
    { ABS_HAT0Y, -1, 1, 0, 0, 0, 0 }
};

static const UInputAxisDesc accelerometer_axes[] = {
    { ABS_X, -4096, 4096, 0, 0, 256, 0 },
    { ABS_Y, -4096, 4096, 0, 0, 256, 0 },
    { ABS_Z, -4096, 4096, 0, 0, 256, 0 }
};

static int
skip_test(const char *message)
{
    SDL_Log("%s", message);
    return 0;
}

static int
fail_sdl(const char *operation)
{
    SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "%s: %s", operation, SDL_GetError());
    return 1;
}

static int
set_uinput_bit(int fd, unsigned long request, int bit, const char *what)
{
    if (ioctl(fd, request, bit) < 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "%s(%d): %s", what, bit, strerror(errno));
        return -1;
    }
    return 0;
}

static int
open_uinput_device(void)
{
    static const char *paths[] = { "/dev/uinput", "/dev/input/uinput" };
    int i;

    for (i = 0; i < SDL_arraysize(paths); ++i) {
        int fd = open(paths[i], O_WRONLY | O_NONBLOCK | O_CLOEXEC);
        if (fd >= 0) {
            return fd;
        }
        if (errno != ENOENT) {
            break;
        }
    }
    return -1;
}

static int
configure_abs_axis(int fd, const UInputAxisDesc *axis)
{
    struct uinput_abs_setup setup;

    SDL_zero(setup);
    setup.code = (Uint16)axis->code;
    setup.absinfo.minimum = axis->minimum;
    setup.absinfo.maximum = axis->maximum;
    setup.absinfo.fuzz = axis->fuzz;
    setup.absinfo.flat = axis->flat;
    setup.absinfo.resolution = axis->resolution;
    setup.absinfo.value = axis->value;

    if (ioctl(fd, UI_ABS_SETUP, &setup) < 0) {
        SDL_SetError("UI_ABS_SETUP(%d) failed: %s", axis->code, strerror(errno));
        return -1;
    }
    return 0;
}

static int
create_uinput_device(UInputDevice *device,
                     const char *name,
                     const int *keys,
                     int num_keys,
                     const UInputAxisDesc *axes,
                     int num_axes)
{
    struct uinput_setup setup;
    int i;
    int fd = -1;

    device->fd = -1;

    fd = open_uinput_device();
    if (fd < 0) {
        SDL_SetError("Couldn't open /dev/uinput: %s", strerror(errno));
        return -1;
    }

    if (set_uinput_bit(fd, UI_SET_EVBIT, EV_SYN, "UI_SET_EVBIT") < 0) {
        goto error;
    }

    if (num_keys > 0) {
        if (set_uinput_bit(fd, UI_SET_EVBIT, EV_KEY, "UI_SET_EVBIT") < 0) {
            goto error;
        }
        for (i = 0; i < num_keys; ++i) {
            if (set_uinput_bit(fd, UI_SET_KEYBIT, keys[i], "UI_SET_KEYBIT") < 0) {
                goto error;
            }
        }
    }

    if (num_axes > 0) {
        if (set_uinput_bit(fd, UI_SET_EVBIT, EV_ABS, "UI_SET_EVBIT") < 0) {
            goto error;
        }
        for (i = 0; i < num_axes; ++i) {
            if (set_uinput_bit(fd, UI_SET_ABSBIT, axes[i].code, "UI_SET_ABSBIT") < 0) {
                goto error;
            }
        }
    }

    SDL_zero(setup);
    SDL_strlcpy(setup.name, name, sizeof(setup.name));
    setup.id.bustype = BUS_USB;
    setup.id.vendor = TEST_USB_VENDOR_SONY;
    setup.id.product = TEST_USB_PRODUCT_DUALSHOCK4;
    setup.id.version = 0;
    if (ioctl(fd, UI_DEV_SETUP, &setup) < 0) {
        SDL_SetError("UI_DEV_SETUP failed: %s", strerror(errno));
        goto error;
    }

    for (i = 0; i < num_axes; ++i) {
        if (configure_abs_axis(fd, &axes[i]) < 0) {
            goto error;
        }
    }

    if (ioctl(fd, UI_DEV_CREATE) < 0) {
        SDL_SetError("UI_DEV_CREATE failed: %s", strerror(errno));
        goto error;
    }

    device->fd = fd;
    SDL_Delay(100);
    return 0;

error:
    close(fd);
    return -1;
}

static void
destroy_uinput_device(UInputDevice *device)
{
    if (device->fd >= 0) {
        ioctl(device->fd, UI_DEV_DESTROY);
        close(device->fd);
        device->fd = -1;
        SDL_Delay(100);
    }
}

static int
write_input_event(UInputDevice *device, Uint16 type, Uint16 code, Sint32 value)
{
    struct input_event event;
    ssize_t written;

    SDL_zero(event);
    event.type = type;
    event.code = code;
    event.value = value;

    written = write(device->fd, &event, sizeof(event));
    if (written != (ssize_t)sizeof(event)) {
        SDL_SetError("write(input_event %u/%u) failed: %s", (unsigned)type, (unsigned)code, strerror(errno));
        return -1;
    }
    return 0;
}

static int
sync_input_device(UInputDevice *device)
{
    return write_input_event(device, EV_SYN, SYN_REPORT, 0);
}

static int
find_joystick_index(const char *name)
{
    int i;

    for (i = 0; i < SDL_NumJoysticks(); ++i) {
        const char *device_name = SDL_JoystickNameForIndex(i);
        if (device_name && SDL_strcmp(device_name, name) == 0) {
            return i;
        }
    }
    return -1;
}

static int
wait_for_joystick(const char *name, int timeout_ms)
{
    const Uint32 deadline = SDL_GetTicks() + (Uint32)timeout_ms;

    do {
        const int index = find_joystick_index(name);
        if (index >= 0) {
            return index;
        }
        SDL_GameControllerUpdate();
        SDL_Delay(20);
    } while (!SDL_TICKS_PASSED(SDL_GetTicks(), deadline));

    return -1;
}

static int
wait_for_button_state(SDL_Joystick *joystick, int button, Uint8 expected_state, int timeout_ms)
{
    const Uint32 deadline = SDL_GetTicks() + (Uint32)timeout_ms;

    do {
        SDL_GameControllerUpdate();
        if (SDL_JoystickGetButton(joystick, button) == expected_state) {
            return 0;
        }
        SDL_Delay(20);
    } while (!SDL_TICKS_PASSED(SDL_GetTicks(), deadline));

    SDL_SetError("Timed out waiting for button %d to become %u", button, (unsigned)expected_state);
    return -1;
}

static SDL_bool
sensor_matches(const float *data, const float *expected, float tolerance)
{
    int i;

    for (i = 0; i < 3; ++i) {
        if (SDL_fabsf(data[i] - expected[i]) > tolerance) {
            return SDL_FALSE;
        }
    }
    return SDL_TRUE;
}

static int
wait_for_sensor_data(SDL_GameController *controller, const float *expected, float tolerance, int timeout_ms)
{
    const Uint32 deadline = SDL_GetTicks() + (Uint32)timeout_ms;
    float data[3];

    do {
        SDL_GameControllerUpdate();
        if (SDL_GameControllerGetSensorData(controller, SDL_SENSOR_ACCEL, data, 3) == 0 &&
            sensor_matches(data, expected, tolerance)) {
            return 0;
        }
        SDL_Delay(20);
    } while (!SDL_TICKS_PASSED(SDL_GetTicks(), deadline));

    SDL_SetError("Timed out waiting for accelerometer sample");
    return -1;
}

static int
run_test(void)
{
    UInputDevice gamepad;
    UInputDevice accelerometer;
    SDL_GameController *controller = NULL;
    SDL_Joystick *joystick = NULL;
    int controller_index = -1;
    int result = 1;
    char guid_string[33];
    char mapping[512];
    float expected_sensor[3];

    gamepad.fd = -1;
    accelerometer.fd = -1;

#ifndef UI_DEV_SETUP
    return skip_test("Skipping evdev test because this system's public uinput API is too old");
#else
#ifndef UI_ABS_SETUP
    return skip_test("Skipping evdev test because this system's public uinput axis API is too old");
#else
    if (create_uinput_device(&gamepad,
                             TEST_CONTROLLER_NAME,
                             gamepad_keys, SDL_arraysize(gamepad_keys),
                             gamepad_axes, SDL_arraysize(gamepad_axes)) < 0) {
        if (errno == EACCES || errno == EPERM || errno == ENOENT) {
            result = skip_test(SDL_GetError());
        } else {
            result = fail_sdl("create_uinput_device(gamepad)");
        }
        goto done;
    }

    if (create_uinput_device(&accelerometer,
                             TEST_SENSOR_NAME,
                             NULL, 0,
                             accelerometer_axes, SDL_arraysize(accelerometer_axes)) < 0) {
        if (errno == EACCES || errno == EPERM || errno == ENOENT) {
            result = skip_test(SDL_GetError());
        } else {
            result = fail_sdl("create_uinput_device(accelerometer)");
        }
        goto done;
    }

    if (SDL_InitSubSystem(SDL_INIT_JOYSTICK | SDL_INIT_GAMECONTROLLER) < 0) {
        result = fail_sdl("SDL_InitSubSystem(SDL_INIT_JOYSTICK | SDL_INIT_GAMECONTROLLER)");
        goto done;
    }

    controller_index = wait_for_joystick(TEST_CONTROLLER_NAME, 3000);
    if (controller_index < 0) {
        result = fail_sdl("wait_for_joystick(TEST_CONTROLLER_NAME)");
        goto done;
    }

    if (find_joystick_index(TEST_SENSOR_NAME) >= 0) {
        SDL_SetError("The accelerometer device was exposed as a joystick");
        result = fail_sdl("find_joystick_index(TEST_SENSOR_NAME)");
        goto done;
    }

    SDL_JoystickGetGUIDString(SDL_JoystickGetDeviceGUID(controller_index), guid_string, sizeof(guid_string));
    SDL_snprintf(mapping, sizeof(mapping),
                 "%s,%s,a:b0,b:b1,back:b8,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,dpup:h0.1,guide:b10,leftshoulder:b4,leftstick:b11,lefttrigger:a2,leftx:a0,lefty:a1,rightshoulder:b5,rightstick:b12,righttrigger:a5,rightx:a3,righty:a4,start:b9,x:b2,y:b3,",
                 guid_string, TEST_CONTROLLER_NAME);
    if (SDL_GameControllerAddMapping(mapping) < 0) {
        result = fail_sdl("SDL_GameControllerAddMapping");
        goto done;
    }

    if (SDL_JoystickGetDeviceType(controller_index) != SDL_JOYSTICK_TYPE_GAMECONTROLLER) {
        SDL_SetError("SDL_JoystickGetDeviceType(%d) returned %d",
                     controller_index, (int)SDL_JoystickGetDeviceType(controller_index));
        result = fail_sdl("SDL_JoystickGetDeviceType");
        goto done;
    }

    if (!SDL_IsGameController(controller_index)) {
        SDL_SetError("SDL_IsGameController(%d) returned false", controller_index);
        result = fail_sdl("SDL_IsGameController");
        goto done;
    }

    controller = SDL_GameControllerOpen(controller_index);
    if (!controller) {
        result = fail_sdl("SDL_GameControllerOpen");
        goto done;
    }

    joystick = SDL_GameControllerGetJoystick(controller);
    if (!joystick) {
        result = fail_sdl("SDL_GameControllerGetJoystick");
        goto done;
    }

    if (!SDL_GameControllerHasSensor(controller, SDL_SENSOR_ACCEL)) {
        SDL_SetError("SDL_GameControllerHasSensor(SDL_SENSOR_ACCEL) returned false");
        result = fail_sdl("SDL_GameControllerHasSensor");
        goto done;
    }

    if (SDL_GameControllerSetSensorEnabled(controller, SDL_SENSOR_ACCEL, SDL_TRUE) < 0) {
        result = fail_sdl("SDL_GameControllerSetSensorEnabled(SDL_SENSOR_ACCEL, SDL_TRUE)");
        goto done;
    }

    if (!SDL_GameControllerIsSensorEnabled(controller, SDL_SENSOR_ACCEL)) {
        SDL_SetError("SDL_GameControllerIsSensorEnabled(SDL_SENSOR_ACCEL) returned false");
        result = fail_sdl("SDL_GameControllerIsSensorEnabled");
        goto done;
    }

    if (write_input_event(&gamepad, EV_KEY, BTN_A, 1) < 0 ||
        sync_input_device(&gamepad) < 0) {
        result = fail_sdl("write_input_event(gamepad button press)");
        goto done;
    }

    if (wait_for_button_state(joystick, 0, SDL_PRESSED, 2000) < 0) {
        result = fail_sdl("wait_for_button_state(SDL_PRESSED)");
        goto done;
    }

    if (write_input_event(&gamepad, EV_KEY, BTN_A, 0) < 0 ||
        sync_input_device(&gamepad) < 0) {
        result = fail_sdl("write_input_event(gamepad button release)");
        goto done;
    }

    if (wait_for_button_state(joystick, 0, SDL_RELEASED, 2000) < 0) {
        result = fail_sdl("wait_for_button_state(SDL_RELEASED)");
        goto done;
    }

    if (write_input_event(&accelerometer, EV_ABS, ABS_X, 256) < 0 ||
        write_input_event(&accelerometer, EV_ABS, ABS_Y, -512) < 0 ||
        write_input_event(&accelerometer, EV_ABS, ABS_Z, 128) < 0 ||
        sync_input_device(&accelerometer) < 0) {
        result = fail_sdl("write_input_event(accelerometer)");
        goto done;
    }

    expected_sensor[0] = SDL_STANDARD_GRAVITY;
    expected_sensor[1] = -2.0f * SDL_STANDARD_GRAVITY;
    expected_sensor[2] = 0.5f * SDL_STANDARD_GRAVITY;
    if (wait_for_sensor_data(controller, expected_sensor, 0.25f, 2000) < 0) {
        result = fail_sdl("wait_for_sensor_data");
        goto done;
    }

    result = 0;

done:
    if (controller) {
        SDL_GameControllerClose(controller);
    }
    if (SDL_WasInit(SDL_INIT_GAMECONTROLLER | SDL_INIT_JOYSTICK)) {
        SDL_QuitSubSystem(SDL_INIT_GAMECONTROLLER | SDL_INIT_JOYSTICK);
    }
    destroy_uinput_device(&accelerometer);
    destroy_uinput_device(&gamepad);
    return result;
#endif
#endif
}

#else

static int
run_test(void)
{
    return skip_test("Skipping evdev test on this platform");
}

#endif

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;
    return run_test();
}
