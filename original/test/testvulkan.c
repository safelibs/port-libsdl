/*
  Copyright (C) 1997-2024 Sam Lantinga <slouken@libsdl.org>

  This software is provided 'as-is', without any express or implied
  warranty.  In no event will the authors be held liable for any damages
  arising from the use of this software.

  Permission is granted to anyone to use this software for any purpose,
  including commercial applications, and to alter it and redistribute it
  freely.
*/
#include <stdio.h>

#include "SDL.h"

#define VK_NO_PROTOTYPES
#ifndef SDL_PUBLIC_VULKAN_HEADER
#error "testvulkan requires an explicitly configured public Vulkan header"
#endif
#include SDL_PUBLIC_VULKAN_HEADER

#include "SDL_vulkan.h"

#if defined(__ANDROID__) && defined(__ARM_EABI__) && !defined(__ARM_ARCH_7A__)

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;
    SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "No Vulkan support on this system");
    return 1;
}

#else

static int
fail_sdl(const char *operation)
{
    SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "%s: %s", operation, SDL_GetError());
    return 1;
}

static int
fail_vk(const char *operation, VkResult result)
{
    SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "%s: VkResult %d", operation, (int)result);
    return 1;
}

static int
fail_message(const char *message)
{
    SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "%s", message);
    return 1;
}

static int
load_vulkan_loader(void **loader_handle)
{
#if defined(_WIN32)
    static const char *library_names[] = { "vulkan-1.dll" };
#elif defined(__APPLE__)
    static const char *library_names[] = {
        "vulkan.framework/vulkan",
        "libvulkan.1.dylib",
        "MoltenVK.framework/MoltenVK",
        "libMoltenVK.dylib"
    };
#else
    static const char *library_names[] = {
        "libvulkan.so.1",
        "libvulkan.so"
    };
#endif
    int i;

    *loader_handle = NULL;

    for (i = 0; i < SDL_arraysize(library_names); ++i) {
        *loader_handle = SDL_LoadObject(library_names[i]);
        if (*loader_handle) {
            return 0;
        }
    }

    SDL_SetError("Could not load the public Vulkan loader library");
    return -1;
}

int main(int argc, char *argv[])
{
    SDL_Window *window = NULL;
    const char **extensions = NULL;
    void *loader_handle = NULL;
    PFN_vkCreateInstance vkCreateInstance = NULL;
    PFN_vkDestroyInstance vkDestroyInstance = NULL;
    PFN_vkDestroySurfaceKHR vkDestroySurfaceKHR = NULL;
    PFN_vkEnumeratePhysicalDevices vkEnumeratePhysicalDevices = NULL;
    VkApplicationInfo appInfo;
    VkInstanceCreateInfo instanceCreateInfo;
    VkInstance instance = VK_NULL_HANDLE;
    VkSurfaceKHR surface = VK_NULL_HANDLE;
    VkResult vk_result = VK_SUCCESS;
    unsigned int extensionCount = 0;
    Uint32 physicalDeviceCount = 0;
    int drawableW = 0;
    int drawableH = 0;
    int result = 1;

    (void)argc;
    (void)argv;

    SDL_zero(appInfo);
    SDL_zero(instanceCreateInfo);

    if (SDL_Init(SDL_INIT_VIDEO) < 0) {
        return fail_sdl("SDL_Init(SDL_INIT_VIDEO)");
    }

    window = SDL_CreateWindow("SDL Vulkan Public API Test",
                              SDL_WINDOWPOS_UNDEFINED, SDL_WINDOWPOS_UNDEFINED,
                              128, 128,
                              SDL_WINDOW_VULKAN | SDL_WINDOW_HIDDEN);
    if (!window) {
        result = fail_sdl("SDL_CreateWindow(SDL_WINDOW_VULKAN)");
        goto done;
    }

    if (!SDL_Vulkan_GetInstanceExtensions(window, &extensionCount, NULL)) {
        result = fail_sdl("SDL_Vulkan_GetInstanceExtensions(count)");
        goto done;
    }
    if (extensionCount == 0) {
        result = fail_message("SDL_Vulkan_GetInstanceExtensions returned no extensions");
        goto done;
    }

    extensions = (const char **)SDL_calloc(extensionCount, sizeof(*extensions));
    if (!extensions) {
        result = fail_sdl("SDL_calloc");
        goto done;
    }

    if (!SDL_Vulkan_GetInstanceExtensions(window, &extensionCount, extensions)) {
        result = fail_sdl("SDL_Vulkan_GetInstanceExtensions(names)");
        goto done;
    }

    if (load_vulkan_loader(&loader_handle) < 0) {
        result = fail_sdl("load_vulkan_loader");
        goto done;
    }

    vkCreateInstance = (PFN_vkCreateInstance)SDL_LoadFunction(loader_handle, "vkCreateInstance");
    vkDestroyInstance = (PFN_vkDestroyInstance)SDL_LoadFunction(loader_handle, "vkDestroyInstance");
    if (!vkCreateInstance || !vkDestroyInstance) {
        result = fail_message("SDL_LoadFunction failed to resolve core Vulkan entry points");
        goto done;
    }

    appInfo.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    appInfo.pApplicationName = "SDL Vulkan Public API Test";
    appInfo.pEngineName = "SDL";
    appInfo.apiVersion = VK_API_VERSION_1_0;

    instanceCreateInfo.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    instanceCreateInfo.pApplicationInfo = &appInfo;
    instanceCreateInfo.enabledExtensionCount = extensionCount;
    instanceCreateInfo.ppEnabledExtensionNames = extensions;

    vk_result = vkCreateInstance(&instanceCreateInfo, NULL, &instance);
    if (vk_result != VK_SUCCESS) {
        result = fail_vk("vkCreateInstance", vk_result);
        goto done;
    }

    vkDestroySurfaceKHR = (PFN_vkDestroySurfaceKHR)SDL_LoadFunction(loader_handle, "vkDestroySurfaceKHR");
    vkEnumeratePhysicalDevices = (PFN_vkEnumeratePhysicalDevices)SDL_LoadFunction(loader_handle, "vkEnumeratePhysicalDevices");
    if (!vkDestroySurfaceKHR || !vkEnumeratePhysicalDevices) {
        result = fail_message("SDL_LoadFunction failed to resolve instance Vulkan entry points");
        goto done;
    }

    vk_result = vkEnumeratePhysicalDevices(instance, &physicalDeviceCount, NULL);
    if (vk_result != VK_SUCCESS) {
        result = fail_vk("vkEnumeratePhysicalDevices", vk_result);
        goto done;
    }
    if (physicalDeviceCount == 0) {
        result = fail_message("vkEnumeratePhysicalDevices reported no devices");
        goto done;
    }

    if (!SDL_Vulkan_CreateSurface(window, instance, &surface)) {
        result = fail_sdl("SDL_Vulkan_CreateSurface");
        goto done;
    }

    SDL_Vulkan_GetDrawableSize(window, &drawableW, &drawableH);
    if (drawableW <= 0 || drawableH <= 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION,
                     "SDL_Vulkan_GetDrawableSize returned %dx%d",
                     drawableW, drawableH);
        result = 1;
        goto done;
    }

    result = 0;

done:
    if (surface != VK_NULL_HANDLE && vkDestroySurfaceKHR) {
        vkDestroySurfaceKHR(instance, surface, NULL);
    }
    if (instance != VK_NULL_HANDLE && vkDestroyInstance) {
        vkDestroyInstance(instance, NULL);
    }
    SDL_free(extensions);
    if (window) {
        SDL_DestroyWindow(window);
    }
    if (loader_handle) {
        SDL_UnloadObject(loader_handle);
    }
    SDL_Vulkan_UnloadLibrary();
    SDL_Quit();
    return result;
}

#endif
