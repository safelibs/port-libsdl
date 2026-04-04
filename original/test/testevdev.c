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

#define TEST_USB_VENDOR_NVIDIA 0x0955
#define TEST_USB_PRODUCT_NVIDIA_SHIELD_CONTROLLER_V104 0x7214

static int
run_test(void)
{
    SDL_VirtualJoystickDesc desc;
    SDL_Joystick *joystick = NULL;
    const char *name = NULL;
    int initial_count = -1;
    int device_index = -1;
    int success = 0;

    if (SDL_InitSubSystem(SDL_INIT_JOYSTICK | SDL_INIT_GAMECONTROLLER) < 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_InitSubSystem(SDL_INIT_JOYSTICK | SDL_INIT_GAMECONTROLLER): %s",
                     SDL_GetError());
        goto done;
    }

    initial_count = SDL_NumJoysticks();
    if (initial_count < 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "SDL_NumJoysticks(): %s", SDL_GetError());
        goto done;
    }

    SDL_zero(desc);
    desc.version = SDL_VIRTUAL_JOYSTICK_DESC_VERSION;
    desc.type = SDL_JOYSTICK_TYPE_GAMECONTROLLER;
    desc.naxes = SDL_CONTROLLER_AXIS_MAX;
    desc.nbuttons = SDL_CONTROLLER_BUTTON_MAX;
    desc.vendor_id = TEST_USB_VENDOR_NVIDIA;
    desc.product_id = TEST_USB_PRODUCT_NVIDIA_SHIELD_CONTROLLER_V104;
    desc.name = "Virtual NVIDIA SHIELD Controller";

    device_index = SDL_JoystickAttachVirtualEx(&desc);
    if (device_index < 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "SDL_JoystickAttachVirtualEx(): %s", SDL_GetError());
        goto done;
    }

    if (!SDL_JoystickIsVirtual(device_index)) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "SDL_JoystickIsVirtual(%d) returned false", device_index);
        goto done;
    }

    if (SDL_NumJoysticks() != (initial_count + 1)) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "Expected joystick count %d after attaching a virtual joystick, got %d",
                     initial_count + 1, SDL_NumJoysticks());
        goto done;
    }

    name = SDL_JoystickNameForIndex(device_index);
    if (!name || SDL_strcmp(name, desc.name) != 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickNameForIndex(%d) returned '%s'",
                     device_index, name ? name : "(null)");
        goto done;
    }

    if (SDL_JoystickGetDeviceVendor(device_index) != desc.vendor_id) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickGetDeviceVendor(%d) returned 0x%04x",
                     device_index, SDL_JoystickGetDeviceVendor(device_index));
        goto done;
    }

    if (SDL_JoystickGetDeviceProduct(device_index) != desc.product_id) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickGetDeviceProduct(%d) returned 0x%04x",
                     device_index, SDL_JoystickGetDeviceProduct(device_index));
        goto done;
    }

    if (SDL_JoystickGetDeviceProductVersion(device_index) != 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickGetDeviceProductVersion(%d) returned 0x%04x",
                     device_index, SDL_JoystickGetDeviceProductVersion(device_index));
        goto done;
    }

    if (SDL_JoystickGetDeviceType(device_index) != desc.type) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickGetDeviceType(%d) returned %d",
                     device_index, (int)SDL_JoystickGetDeviceType(device_index));
        goto done;
    }

    if (SDL_JoystickGetDeviceInstanceID(device_index) < 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickGetDeviceInstanceID(%d): %s",
                     device_index, SDL_GetError());
        goto done;
    }

    joystick = SDL_JoystickOpen(device_index);
    if (!joystick) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "SDL_JoystickOpen(%d): %s", device_index, SDL_GetError());
        goto done;
    }

    if (SDL_JoystickNumAxes(joystick) != desc.naxes) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickNumAxes() returned %d",
                     SDL_JoystickNumAxes(joystick));
        goto done;
    }

    if (SDL_JoystickNumButtons(joystick) != desc.nbuttons) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickNumButtons() returned %d",
                     SDL_JoystickNumButtons(joystick));
        goto done;
    }

    if (SDL_JoystickSetVirtualButton(joystick, SDL_CONTROLLER_BUTTON_A, SDL_PRESSED) < 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickSetVirtualButton(SDL_CONTROLLER_BUTTON_A, SDL_PRESSED): %s",
                     SDL_GetError());
        goto done;
    }
    SDL_JoystickUpdate();
    if (SDL_JoystickGetButton(joystick, SDL_CONTROLLER_BUTTON_A) != SDL_PRESSED) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "SDL_JoystickGetButton(SDL_CONTROLLER_BUTTON_A) != SDL_PRESSED");
        goto done;
    }

    if (SDL_JoystickSetVirtualButton(joystick, SDL_CONTROLLER_BUTTON_A, SDL_RELEASED) < 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_JoystickSetVirtualButton(SDL_CONTROLLER_BUTTON_A, SDL_RELEASED): %s",
                     SDL_GetError());
        goto done;
    }
    SDL_JoystickUpdate();
    if (SDL_JoystickGetButton(joystick, SDL_CONTROLLER_BUTTON_A) != SDL_RELEASED) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "SDL_JoystickGetButton(SDL_CONTROLLER_BUTTON_A) != SDL_RELEASED");
        goto done;
    }

    success = 1;

done:
    if (joystick) {
        SDL_JoystickClose(joystick);
    }
    if (device_index >= 0) {
        if (SDL_JoystickDetachVirtual(device_index) < 0) {
            SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "SDL_JoystickDetachVirtual(%d): %s", device_index, SDL_GetError());
            success = 0;
        } else if (initial_count >= 0 && SDL_NumJoysticks() != initial_count) {
            SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                         "Expected joystick count %d after detaching the virtual joystick, got %d",
                         initial_count, SDL_NumJoysticks());
            success = 0;
        }
    }
    SDL_QuitSubSystem(SDL_INIT_JOYSTICK | SDL_INIT_GAMECONTROLLER);
    return success;
}

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;
    return run_test() ? 0 : 1;
}
