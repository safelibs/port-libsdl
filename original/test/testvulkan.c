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
#include "SDL_vulkan.h"

#if defined(__ANDROID__) && defined(__ARM_EABI__) && !defined(__ARM_ARCH_7A__)

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;
    SDL_Log("Skipping Vulkan test on this system");
    return 0;
}

#else

#if defined(_WIN32)
#define VKAPI_CALL __stdcall
#else
#define VKAPI_CALL
#endif
#define VKAPI_PTR VKAPI_CALL *

typedef Sint32 VkResult;
typedef Uint32 VkFlags;
typedef Uint32 VkStructureType;
typedef struct VkAllocationCallbacks VkAllocationCallbacks;
typedef struct VkApplicationInfo
{
    VkStructureType sType;
    const void *pNext;
    const char *pApplicationName;
    Uint32 applicationVersion;
    const char *pEngineName;
    Uint32 engineVersion;
    Uint32 apiVersion;
} VkApplicationInfo;

typedef struct VkInstanceCreateInfo
{
    VkStructureType sType;
    const void *pNext;
    VkFlags flags;
    const VkApplicationInfo *pApplicationInfo;
    Uint32 enabledLayerCount;
    const char *const *ppEnabledLayerNames;
    Uint32 enabledExtensionCount;
    const char *const *ppEnabledExtensionNames;
} VkInstanceCreateInfo;

typedef void (*PFN_vkVoidFunction)(void);
typedef PFN_vkVoidFunction(VKAPI_PTR PFN_vkGetInstanceProcAddr)(VkInstance instance, const char *pName);
typedef VkResult(VKAPI_PTR PFN_vkCreateInstance)(const VkInstanceCreateInfo *pCreateInfo, const VkAllocationCallbacks *pAllocator, VkInstance *pInstance);
typedef void(VKAPI_PTR PFN_vkDestroyInstance)(VkInstance instance, const VkAllocationCallbacks *pAllocator);
typedef void(VKAPI_PTR PFN_vkDestroySurfaceKHR)(VkInstance instance, VkSurfaceKHR surface, const VkAllocationCallbacks *pAllocator);

#define VK_SUCCESS 0
#define VK_STRUCTURE_TYPE_APPLICATION_INFO 0
#define VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO 1
#define VK_API_VERSION_1_0 ((Uint32)(1u << 22))
#define VK_NULL_HANDLE 0

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
fail_vk(const char *operation, VkResult result)
{
    SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "%s: VkResult %d", operation, (int)result);
    return 1;
}

int main(int argc, char *argv[])
{
    SDL_Window *window = NULL;
    const char **extensions = NULL;
    PFN_vkGetInstanceProcAddr vkGetInstanceProcAddr = NULL;
    PFN_vkCreateInstance vkCreateInstance = NULL;
    PFN_vkDestroyInstance vkDestroyInstance = NULL;
    PFN_vkDestroySurfaceKHR vkDestroySurfaceKHR = NULL;
    VkApplicationInfo appInfo;
    VkInstanceCreateInfo instanceCreateInfo;
    VkInstance instance = VK_NULL_HANDLE;
    VkSurfaceKHR surface = VK_NULL_HANDLE;
    VkResult vk_result = VK_SUCCESS;
    unsigned int extensionCount = 0;
    int drawableW = 0;
    int drawableH = 0;
    SDL_bool instance_created = SDL_FALSE;
    SDL_bool surface_created = SDL_FALSE;
    int result = 1;

    (void)argc;
    (void)argv;

    SDL_zero(appInfo);
    SDL_zero(instanceCreateInfo);

    if (SDL_Init(SDL_INIT_VIDEO) < 0) {
        return fail_sdl("SDL_Init(SDL_INIT_VIDEO)");
    }

    if (SDL_Vulkan_LoadLibrary(NULL) < 0) {
        result = skip_test("Skipping Vulkan test because no public Vulkan loader is available");
        goto done;
    }

    window = SDL_CreateWindow("SDL Vulkan Public API Test",
                              SDL_WINDOWPOS_UNDEFINED, SDL_WINDOWPOS_UNDEFINED,
                              128, 128,
                              SDL_WINDOW_VULKAN | SDL_WINDOW_HIDDEN);
    if (!window) {
        result = skip_test("Skipping Vulkan test because the active video driver cannot create Vulkan windows");
        goto done;
    }

    if (!SDL_Vulkan_GetInstanceExtensions(window, &extensionCount, NULL)) {
        result = fail_sdl("SDL_Vulkan_GetInstanceExtensions(count)");
        goto done;
    }
    if (extensionCount == 0) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "SDL_Vulkan_GetInstanceExtensions returned no extensions");
        result = 1;
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

    vkGetInstanceProcAddr = (PFN_vkGetInstanceProcAddr)SDL_Vulkan_GetVkGetInstanceProcAddr();
    if (!vkGetInstanceProcAddr) {
        result = fail_sdl("SDL_Vulkan_GetVkGetInstanceProcAddr");
        goto done;
    }

    vkCreateInstance = (PFN_vkCreateInstance)vkGetInstanceProcAddr(VK_NULL_HANDLE, "vkCreateInstance");
    vkDestroyInstance = (PFN_vkDestroyInstance)vkGetInstanceProcAddr(VK_NULL_HANDLE, "vkDestroyInstance");
    if (!vkCreateInstance || !vkDestroyInstance) {
        result = skip_test("Skipping Vulkan test because the loaded Vulkan loader is missing core entry points");
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
    instance_created = SDL_TRUE;

    vkDestroySurfaceKHR = (PFN_vkDestroySurfaceKHR)vkGetInstanceProcAddr(instance, "vkDestroySurfaceKHR");
    if (!vkDestroySurfaceKHR) {
        SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, "vkGetInstanceProcAddr(vkDestroySurfaceKHR) returned NULL");
        result = 1;
        goto done;
    }

    if (!SDL_Vulkan_CreateSurface(window, instance, &surface)) {
        result = fail_sdl("SDL_Vulkan_CreateSurface");
        goto done;
    }
    surface_created = SDL_TRUE;

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
    if (surface_created && vkDestroySurfaceKHR) {
        vkDestroySurfaceKHR(instance, surface, NULL);
    }
    if (instance_created && vkDestroyInstance) {
        vkDestroyInstance(instance, NULL);
    }
    SDL_free(extensions);
    if (window) {
        SDL_DestroyWindow(window);
    }
    SDL_Vulkan_UnloadLibrary();
    SDL_Quit();
    return result;
}

#endif
