use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use tempfile::{Builder, TempDir};

use crate::contracts::UBUNTU_MULTIARCH;
use crate::stage_install::{stage_install, StageInstallArgs, StageInstallMode};

pub const PHASE_09_ID: &str = "impl_phase_09_performance";
pub const DEFAULT_ORIGINAL_BUILD_DIR: &str = "build-phase9-original-reference";
pub const DEFAULT_ORIGINAL_PREFIX: &str = "build-phase9-original-prefix";
pub const DEFAULT_SAFE_STAGE_ROOT: &str = "build-phase9-safe-stage";
pub const DEFAULT_PERF_RUNNER_DIR: &str = "build-phase9-perf";
pub const DEFAULT_PERF_REPORT: &str = "safe/generated/reports/perf-baseline-vs-safe.json";
pub const DEFAULT_PERF_WAIVERS: &str = "safe/generated/reports/perf-waivers.md";
pub const DEFAULT_PERF_MANIFEST: &str = "safe/generated/perf_workload_manifest.json";
pub const DEFAULT_PERF_THRESHOLDS: &str = "safe/generated/perf_thresholds.json";

const PERF_RUNNER_SOURCE: &str = r#"
#include <errno.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <time.h>

#include "SDL.h"

static const char *last_error(void) {
    const char *message = SDL_GetError();
    return (message != NULL && message[0] != '\0') ? message : "unknown SDL error";
}

static const char *resource_at(
    const char *workload,
    const char **resources,
    int resource_count,
    int index
) {
    if (index >= resource_count) {
        fprintf(stderr, "%s missing resource %d\n", workload, index);
        return NULL;
    }
    return resources[index];
}

static uint64_t timespec_to_us(struct timespec value) {
    return ((uint64_t) value.tv_sec * 1000000ULL) + ((uint64_t) value.tv_nsec / 1000ULL);
}

static uint64_t timeval_to_us(struct timeval value) {
    return ((uint64_t) value.tv_sec * 1000000ULL) + (uint64_t) value.tv_usec;
}

static uint64_t rusage_cpu_us(const struct rusage *value) {
    return timeval_to_us(value->ru_utime) + timeval_to_us(value->ru_stime);
}

static uint64_t buffer_checksum(const Uint8 *bytes, size_t len) {
    if (bytes == NULL || len == 0) {
        return 0;
    }

    uint64_t checksum = (uint64_t) len;
    checksum += bytes[0];
    checksum += bytes[len / 3];
    checksum += bytes[(len * 2) / 3];
    checksum += bytes[len - 1];
    return checksum;
}

static uint64_t surface_checksum(SDL_Surface *surface) {
    if (surface == NULL || surface->pixels == NULL || surface->pitch <= 0 || surface->h <= 0) {
        return 0;
    }

    const int must_unlock = SDL_MUSTLOCK(surface);
    if (must_unlock && SDL_LockSurface(surface) != 0) {
        return 0;
    }

    size_t len = (size_t) surface->pitch * (size_t) surface->h;
    uint64_t checksum = buffer_checksum((const Uint8 *) surface->pixels, len);

    if (must_unlock) {
        SDL_UnlockSurface(surface);
    }
    return checksum;
}

static unsigned char *load_file_bytes(const char *path, size_t *size_out) {
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        fprintf(stderr, "open %s failed: %s\n", path, strerror(errno));
        return NULL;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        return NULL;
    }
    long size = ftell(file);
    if (size < 0) {
        fclose(file);
        return NULL;
    }
    if (fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }

    unsigned char *bytes = (unsigned char *) malloc((size_t) size);
    if (bytes == NULL) {
        fclose(file);
        return NULL;
    }

    size_t read_len = fread(bytes, 1, (size_t) size, file);
    fclose(file);
    if (read_len != (size_t) size) {
        free(bytes);
        return NULL;
    }

    *size_out = (size_t) size;
    return bytes;
}

static int run_surface_workload(
    int loops,
    const char **resources,
    int resource_count,
    uint64_t *checksum
) {
    const char *sample_path = resource_at("surface_create_fill_convert_blit", resources, resource_count, 0);
    const char *axis_path = resource_at("surface_create_fill_convert_blit", resources, resource_count, 1);
    const char *button_path = resource_at("surface_create_fill_convert_blit", resources, resource_count, 2);
    if (sample_path == NULL || axis_path == NULL || button_path == NULL) {
        return 1;
    }

    SDL_Surface *sample = SDL_LoadBMP(sample_path);
    SDL_Surface *axis = SDL_LoadBMP(axis_path);
    SDL_Surface *button = SDL_LoadBMP(button_path);
    SDL_Surface *sample_argb = NULL;
    SDL_Surface *axis_argb = NULL;
    SDL_Surface *button_argb = NULL;
    if (sample == NULL || axis == NULL || button == NULL) {
        fprintf(stderr, "surface workload resource load failed: %s\n", last_error());
        SDL_FreeSurface(sample);
        SDL_FreeSurface(axis);
        SDL_FreeSurface(button);
        return 1;
    }

    sample_argb = SDL_ConvertSurfaceFormat(sample, SDL_PIXELFORMAT_ARGB8888, 0);
    axis_argb = SDL_ConvertSurfaceFormat(axis, SDL_PIXELFORMAT_ARGB8888, 0);
    button_argb = SDL_ConvertSurfaceFormat(button, SDL_PIXELFORMAT_ARGB8888, 0);
    SDL_FreeSurface(sample);
    SDL_FreeSurface(axis);
    SDL_FreeSurface(button);
    if (sample_argb == NULL || axis_argb == NULL || button_argb == NULL) {
        fprintf(stderr, "surface workload conversion failed: %s\n", last_error());
        SDL_FreeSurface(sample_argb);
        SDL_FreeSurface(axis_argb);
        SDL_FreeSurface(button_argb);
        return 1;
    }

    for (int i = 0; i < loops; ++i) {
        SDL_Surface *canvas = SDL_CreateRGBSurfaceWithFormat(
            0,
            640,
            480,
            32,
            SDL_PIXELFORMAT_ARGB8888
        );
        SDL_Surface *converted = SDL_ConvertSurface(sample_argb, canvas->format, 0);
        if (canvas == NULL || converted == NULL) {
            fprintf(stderr, "surface workload surface creation failed: %s\n", last_error());
            SDL_FreeSurface(canvas);
            SDL_FreeSurface(converted);
            SDL_FreeSurface(sample_argb);
            SDL_FreeSurface(axis_argb);
            SDL_FreeSurface(button_argb);
            return 1;
        }

        Uint32 clear_color = SDL_MapRGB(canvas->format, 0x18, 0x2c, 0x44);
        Uint32 fill_color = SDL_MapRGB(canvas->format, (Uint8) (i * 17), 0x94, (Uint8) (255 - i));
        SDL_Rect fill = { (i * 13) % 400, (i * 7) % 280, 128, 96 };
        SDL_Rect axis_dst = { 16 + (i * 5) % 160, 24 + (i * 3) % 120, axis_argb->w, axis_argb->h };
        SDL_Rect button_dst = { 196 + (i * 11) % 220, 140 + (i * 7) % 160, button_argb->w * 2, button_argb->h * 2 };

        if (SDL_FillRect(canvas, NULL, clear_color) != 0 ||
            SDL_FillRect(canvas, &fill, fill_color) != 0 ||
            SDL_BlitSurface(converted, NULL, canvas, NULL) != 0 ||
            SDL_BlitSurface(axis_argb, NULL, canvas, &axis_dst) != 0 ||
            SDL_BlitScaled(button_argb, NULL, canvas, &button_dst) != 0) {
            fprintf(stderr, "surface workload blit/fill failed: %s\n", last_error());
            SDL_FreeSurface(canvas);
            SDL_FreeSurface(converted);
            SDL_FreeSurface(sample_argb);
            SDL_FreeSurface(axis_argb);
            SDL_FreeSurface(button_argb);
            return 1;
        }

        *checksum += surface_checksum(canvas);
        SDL_FreeSurface(converted);
        SDL_FreeSurface(canvas);
    }

    SDL_FreeSurface(sample_argb);
    SDL_FreeSurface(axis_argb);
    SDL_FreeSurface(button_argb);
    return 0;
}

static int run_renderer_workload(
    int loops,
    const char **resources,
    int resource_count,
    uint64_t *checksum
) {
    const char *sample_path = resource_at("renderer_queue_copy_texture_upload", resources, resource_count, 0);
    const char *normal_path = resource_at("renderer_queue_copy_texture_upload", resources, resource_count, 1);
    if (sample_path == NULL || normal_path == NULL) {
        return 1;
    }

    SDL_Surface *sample = SDL_LoadBMP(sample_path);
    SDL_Surface *normal = SDL_LoadBMP(normal_path);
    SDL_Surface *target = SDL_CreateRGBSurfaceWithFormat(0, 800, 600, 32, SDL_PIXELFORMAT_ARGB8888);
    if (sample == NULL || normal == NULL || target == NULL) {
        fprintf(stderr, "renderer workload setup failed: %s\n", last_error());
        SDL_FreeSurface(sample);
        SDL_FreeSurface(normal);
        SDL_FreeSurface(target);
        return 1;
    }

    SDL_Renderer *renderer = SDL_CreateSoftwareRenderer(target);
    if (renderer == NULL) {
        fprintf(stderr, "renderer workload renderer create failed: %s\n", last_error());
        SDL_FreeSurface(sample);
        SDL_FreeSurface(normal);
        SDL_FreeSurface(target);
        return 1;
    }

    SDL_Texture *sample_tex = SDL_CreateTextureFromSurface(renderer, sample);
    SDL_Texture *normal_tex = SDL_CreateTextureFromSurface(renderer, normal);
    SDL_Texture *streaming = SDL_CreateTexture(
        renderer,
        SDL_PIXELFORMAT_ARGB8888,
        SDL_TEXTUREACCESS_STREAMING,
        128,
        128
    );
    size_t pixel_len = 128U * 128U * 4U;
    Uint8 *pixels = (Uint8 *) malloc(pixel_len);
    if (sample_tex == NULL || normal_tex == NULL || streaming == NULL || pixels == NULL) {
        fprintf(stderr, "renderer workload texture setup failed: %s\n", last_error());
        SDL_DestroyTexture(sample_tex);
        SDL_DestroyTexture(normal_tex);
        SDL_DestroyTexture(streaming);
        SDL_DestroyRenderer(renderer);
        SDL_FreeSurface(sample);
        SDL_FreeSurface(normal);
        SDL_FreeSurface(target);
        free(pixels);
        return 1;
    }

    for (size_t i = 0; i < pixel_len; i += 4) {
        pixels[i + 0] = (Uint8) (i & 0xff);
        pixels[i + 1] = (Uint8) ((i / 2) & 0xff);
        pixels[i + 2] = (Uint8) ((255 - i) & 0xff);
        pixels[i + 3] = 0xff;
    }

    for (int i = 0; i < loops; ++i) {
        for (int j = 0; j < 256; ++j) {
            size_t offset = (size_t) ((j * 17) % (128 * 128)) * 4U;
            pixels[offset + 0] ^= (Uint8) (i + j);
            pixels[offset + 1] += (Uint8) (i * 3);
            pixels[offset + 2] ^= (Uint8) (j * 5);
        }

        if (SDL_UpdateTexture(streaming, NULL, pixels, 128 * 4) != 0 ||
            SDL_SetRenderDrawColor(renderer, (Uint8) (16 + i), 0x55, 0x88, 0xff) != 0 ||
            SDL_RenderClear(renderer) != 0) {
            fprintf(stderr, "renderer workload update failed: %s\n", last_error());
            SDL_DestroyTexture(sample_tex);
            SDL_DestroyTexture(normal_tex);
            SDL_DestroyTexture(streaming);
            SDL_DestroyRenderer(renderer);
            SDL_FreeSurface(sample);
            SDL_FreeSurface(normal);
            SDL_FreeSurface(target);
            free(pixels);
            return 1;
        }

        for (int j = 0; j < 96; ++j) {
            SDL_Rect dst0 = { (j * 17) % 700, (j * 11) % 500, 64, 64 };
            SDL_Rect dst1 = { (j * 19 + 33) % 700, (j * 7 + 21) % 500, 96, 96 };
            if (SDL_RenderCopy(renderer, sample_tex, NULL, &dst0) != 0 ||
                SDL_RenderCopy(renderer, streaming, NULL, &dst1) != 0) {
                fprintf(stderr, "renderer workload copy failed: %s\n", last_error());
                SDL_DestroyTexture(sample_tex);
                SDL_DestroyTexture(normal_tex);
                SDL_DestroyTexture(streaming);
                SDL_DestroyRenderer(renderer);
                SDL_FreeSurface(sample);
                SDL_FreeSurface(normal);
                SDL_FreeSurface(target);
                free(pixels);
                return 1;
            }
            if ((j % 3) == 0) {
                SDL_Rect dst2 = { (j * 23 + 9) % 700, (j * 5 + 71) % 500, 48, 48 };
                if (SDL_RenderCopy(renderer, normal_tex, NULL, &dst2) != 0) {
                    fprintf(stderr, "renderer workload alt copy failed: %s\n", last_error());
                    SDL_DestroyTexture(sample_tex);
                    SDL_DestroyTexture(normal_tex);
                    SDL_DestroyTexture(streaming);
                    SDL_DestroyRenderer(renderer);
                    SDL_FreeSurface(sample);
                    SDL_FreeSurface(normal);
                    SDL_FreeSurface(target);
                    free(pixels);
                    return 1;
                }
            }
        }

        SDL_RenderPresent(renderer);
        *checksum += surface_checksum(target);
    }

    free(pixels);
    SDL_DestroyTexture(sample_tex);
    SDL_DestroyTexture(normal_tex);
    SDL_DestroyTexture(streaming);
    SDL_DestroyRenderer(renderer);
    SDL_FreeSurface(sample);
    SDL_FreeSurface(normal);
    SDL_FreeSurface(target);
    return 0;
}

static int run_audio_workload(
    int loops,
    const char **resources,
    int resource_count,
    uint64_t *checksum
) {
    const char *wav_path = resource_at("audio_stream_convert_resample_wave", resources, resource_count, 0);
    if (wav_path == NULL) {
        return 1;
    }

    size_t wav_size = 0;
    unsigned char *wav_bytes = load_file_bytes(wav_path, &wav_size);
    if (wav_bytes == NULL) {
        fprintf(stderr, "audio workload could not read %s\n", wav_path);
        return 1;
    }

    for (int i = 0; i < loops; ++i) {
        SDL_RWops *rw = SDL_RWFromConstMem(wav_bytes, (int) wav_size);
        SDL_AudioSpec spec;
        Uint8 *decoded = NULL;
        Uint32 decoded_len = 0;
        if (rw == NULL || SDL_LoadWAV_RW(rw, 1, &spec, &decoded, &decoded_len) == NULL) {
            fprintf(stderr, "audio workload decode failed: %s\n", last_error());
            free(wav_bytes);
            return 1;
        }

        int source_frame = (SDL_AUDIO_BITSIZE(spec.format) / 8) * spec.channels;
        int target_rate = (spec.freq >= 22050) ? (spec.freq / 2) : (spec.freq * 2);
        SDL_AudioStream *stream = SDL_NewAudioStream(
            spec.format,
            spec.channels,
            spec.freq,
            AUDIO_F32SYS,
            2,
            target_rate
        );
        if (stream == NULL) {
            fprintf(stderr, "audio workload stream create failed: %s\n", last_error());
            SDL_FreeWAV(decoded);
            free(wav_bytes);
            return 1;
        }

        int chunk = ((int) decoded_len / 5 / source_frame) * source_frame;
        if (chunk <= 0) {
            chunk = source_frame;
        }

        for (Uint32 offset = 0; offset < decoded_len; offset += (Uint32) chunk) {
            int len = chunk;
            if (offset + (Uint32) len > decoded_len) {
                len = (int) (decoded_len - offset);
            }
            if (SDL_AudioStreamPut(stream, decoded + offset, len) != 0) {
                fprintf(stderr, "audio workload put failed: %s\n", last_error());
                SDL_FreeAudioStream(stream);
                SDL_FreeWAV(decoded);
                free(wav_bytes);
                return 1;
            }
        }

        if (SDL_AudioStreamFlush(stream) != 0) {
            fprintf(stderr, "audio workload flush failed: %s\n", last_error());
            SDL_FreeAudioStream(stream);
            SDL_FreeWAV(decoded);
            free(wav_bytes);
            return 1;
        }

        int out_capacity = ((int) decoded_len * 8) + 4096;
        Uint8 *output = (Uint8 *) malloc((size_t) out_capacity);
        if (output == NULL) {
            SDL_FreeAudioStream(stream);
            SDL_FreeWAV(decoded);
            free(wav_bytes);
            return 1;
        }

        for (;;) {
            int got = SDL_AudioStreamGet(stream, output, out_capacity);
            if (got < 0) {
                fprintf(stderr, "audio workload get failed: %s\n", last_error());
                free(output);
                SDL_FreeAudioStream(stream);
                SDL_FreeWAV(decoded);
                free(wav_bytes);
                return 1;
            }
            if (got == 0) {
                break;
            }
            *checksum += buffer_checksum(output, (size_t) got);
        }

        free(output);
        SDL_FreeAudioStream(stream);
        SDL_FreeWAV(decoded);
    }

    free(wav_bytes);
    return 0;
}

static int run_event_workload(
    int loops,
    const char **resources,
    int resource_count,
    uint64_t *checksum
) {
    (void) resources;
    (void) resource_count;

    if (SDL_InitSubSystem(SDL_INIT_EVENTS) != 0) {
        fprintf(stderr, "event workload init failed: %s\n", last_error());
        return 1;
    }

    Uint32 event_type = SDL_RegisterEvents(1);
    if (event_type == ((Uint32) -1)) {
        fprintf(stderr, "event workload register failed: %s\n", last_error());
        SDL_QuitSubSystem(SDL_INIT_EVENTS);
        return 1;
    }
    SDL_FlushEvents(SDL_FIRSTEVENT, SDL_LASTEVENT);

    for (int i = 0; i < loops; ++i) {
        const int batch = 1024;
        SDL_Event event;
        SDL_Event drained[128];
        SDL_zero(event);
        event.type = event_type;
        for (int j = 0; j < batch; ++j) {
            event.user.code = (i * batch) + j;
            int pushed = SDL_PushEvent(&event);
            if (pushed != 1) {
                fprintf(
                    stderr,
                    "event workload push failed: rc=%d %s\n",
                    pushed,
                    last_error()
                );
                SDL_QuitSubSystem(SDL_INIT_EVENTS);
                return 1;
            }
        }

        int seen = 0;
        while (seen < batch) {
            int wanted = batch - seen;
            if (wanted > (int) SDL_arraysize(drained)) {
                wanted = (int) SDL_arraysize(drained);
            }
            int got = SDL_PeepEvents(drained, wanted, SDL_GETEVENT, event_type, event_type);
            if (got < 0) {
                fprintf(stderr, "event workload peep failed: %s\n", last_error());
                SDL_QuitSubSystem(SDL_INIT_EVENTS);
                return 1;
            }
            if (got == 0) {
                fprintf(stderr, "event workload poll underflow\n");
                SDL_QuitSubSystem(SDL_INIT_EVENTS);
                return 1;
            }
            for (int k = 0; k < got; ++k) {
                *checksum += (uint64_t) drained[k].user.code;
            }
            seen += got;
        }
    }

    SDL_QuitSubSystem(SDL_INIT_EVENTS);
    return 0;
}

static void make_guid_string(int index, char out[33]) {
    const Uint16 bus = 0x0003;
    const Uint16 vendor = (Uint16) (0x1000 + (index & 0x0fff));
    const Uint16 product = (Uint16) (0x2000 + (index & 0x0fff));
    const Uint16 version = (Uint16) (0x0100 + (index & 0x00ff));
    const Uint8 guid_bytes[16] = {
        (Uint8) (bus & 0xff), (Uint8) (bus >> 8),
        0, 0,
        (Uint8) (vendor & 0xff), (Uint8) (vendor >> 8),
        0, 0,
        (Uint8) (product & 0xff), (Uint8) (product >> 8),
        0, 0,
        (Uint8) (version & 0xff), (Uint8) (version >> 8),
        0, 0
    };
    for (int byte_index = 0; byte_index < (int) SDL_arraysize(guid_bytes); ++byte_index) {
        snprintf(out + (byte_index * 2), 3, "%02x", guid_bytes[byte_index]);
    }
    out[(int) SDL_arraysize(guid_bytes) * 2] = '\0';
}

static int run_controller_workload(
    int loops,
    const char **resources,
    int resource_count,
    uint64_t *checksum
) {
    (void) resources;
    (void) resource_count;

    if (SDL_InitSubSystem(SDL_INIT_GAMECONTROLLER | SDL_INIT_JOYSTICK) != 0) {
        fprintf(stderr, "controller workload init failed: %s\n", last_error());
        return 1;
    }

    enum { mapping_count = 256 };
    SDL_JoystickGUID guids[mapping_count];
    for (int i = 0; i < mapping_count; ++i) {
        char guid_text[33];
        char mapping[512];
        make_guid_string(i + 1, guid_text);
        snprintf(
            mapping,
            sizeof(mapping),
            "%s,Perf Controller %03d,a:b0,b:b1,x:b2,y:b3,back:b4,guide:b5,start:b6,leftstick:b7,rightstick:b8,leftshoulder:b9,rightshoulder:b10,dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,leftx:a0,lefty:a1,rightx:a2,righty:a3,lefttrigger:a4,righttrigger:a5,platform:Linux",
            guid_text,
            i
        );
        if (SDL_GameControllerAddMapping(mapping) < 0) {
            fprintf(stderr, "controller workload add mapping failed: %s\n", last_error());
            SDL_QuitSubSystem(SDL_INIT_GAMECONTROLLER | SDL_INIT_JOYSTICK);
            return 1;
        }
        guids[i] = SDL_JoystickGetGUIDFromString(guid_text);
    }
    int lookup_base = SDL_GameControllerNumMappings() - mapping_count;
    if (lookup_base < 0) {
        fprintf(stderr, "controller workload missing added mappings\n");
        SDL_QuitSubSystem(SDL_INIT_GAMECONTROLLER | SDL_INIT_JOYSTICK);
        return 1;
    }

    for (int i = 0; i < loops; ++i) {
        for (int mapping_index = 0; mapping_index < mapping_count; ++mapping_index) {
            char guid_buffer[33];
            char *mapping = SDL_GameControllerMappingForIndex(lookup_base + mapping_index);
            if (mapping == NULL) {
                fprintf(stderr, "controller workload lookup failed: %s\n", last_error());
                SDL_QuitSubSystem(SDL_INIT_GAMECONTROLLER | SDL_INIT_JOYSTICK);
                return 1;
            }

            *checksum += buffer_checksum((const Uint8 *) mapping, strlen(mapping));
            SDL_free(mapping);
            SDL_GUIDToString(guids[mapping_index], guid_buffer, (int) sizeof(guid_buffer));
            *checksum += buffer_checksum((const Uint8 *) guid_buffer, strlen(guid_buffer));
        }
    }

    SDL_QuitSubSystem(SDL_INIT_GAMECONTROLLER | SDL_INIT_JOYSTICK);
    return 0;
}

static int run_workload(
    const char *workload_id,
    int loops,
    const char **resources,
    int resource_count,
    uint64_t *checksum
) {
    if (strcmp(workload_id, "surface_create_fill_convert_blit") == 0) {
        return run_surface_workload(loops, resources, resource_count, checksum);
    }
    if (strcmp(workload_id, "renderer_queue_copy_texture_upload") == 0) {
        return run_renderer_workload(loops, resources, resource_count, checksum);
    }
    if (strcmp(workload_id, "audio_stream_convert_resample_wave") == 0) {
        return run_audio_workload(loops, resources, resource_count, checksum);
    }
    if (strcmp(workload_id, "event_queue_throughput") == 0) {
        return run_event_workload(loops, resources, resource_count, checksum);
    }
    if (strcmp(workload_id, "controller_mapping_guid") == 0) {
        return run_controller_workload(loops, resources, resource_count, checksum);
    }

    fprintf(stderr, "unknown workload %s\n", workload_id);
    return 1;
}

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: perf_runner <workload-id> <warmup-loops> <timed-loops> [resource ...]\n");
        return 2;
    }

    const char *workload_id = argv[1];
    int warmup_loops = atoi(argv[2]);
    int timed_loops = atoi(argv[3]);
    const char **resources = (const char **) &argv[4];
    int resource_count = argc - 4;

    SDL_SetHint(SDL_HINT_RENDER_DRIVER, "software");
    SDL_SetHint(SDL_HINT_RENDER_SCALE_QUALITY, "0");

    uint64_t checksum = 0;
    if (warmup_loops > 0 && run_workload(workload_id, warmup_loops, resources, resource_count, &checksum) != 0) {
        return 1;
    }

    struct rusage before_usage;
    struct rusage after_usage;
    struct timespec before_wall;
    struct timespec after_wall;
    if (clock_gettime(CLOCK_MONOTONIC_RAW, &before_wall) != 0 || getrusage(RUSAGE_SELF, &before_usage) != 0) {
        fprintf(stderr, "metric capture setup failed\n");
        return 1;
    }

    if (run_workload(workload_id, timed_loops, resources, resource_count, &checksum) != 0) {
        return 1;
    }

    if (clock_gettime(CLOCK_MONOTONIC_RAW, &after_wall) != 0 || getrusage(RUSAGE_SELF, &after_usage) != 0) {
        fprintf(stderr, "metric capture teardown failed\n");
        return 1;
    }

    uint64_t cpu_time_us = rusage_cpu_us(&after_usage) - rusage_cpu_us(&before_usage);
    uint64_t wall_time_us = timespec_to_us(after_wall) - timespec_to_us(before_wall);
    printf(
        "{\"workload_id\":\"%s\",\"loops\":%d,\"cpu_time_us\":%" PRIu64 ",\"wall_time_us\":%" PRIu64 ",\"max_rss_kib\":%ld,\"checksum\":%" PRIu64 "}\n",
        workload_id,
        timed_loops,
        cpu_time_us,
        wall_time_us,
        after_usage.ru_maxrss,
        checksum
    );
    return 0;
}
"#;

#[derive(Debug, Clone)]
pub struct BuildOriginalReferenceArgs {
    pub repo_root: PathBuf,
    pub original_dir: PathBuf,
    pub build_dir: PathBuf,
    pub prefix_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PerfCaptureArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub original_dir: PathBuf,
    pub original_prefix_dir: PathBuf,
    pub safe_stage_root: PathBuf,
    pub runner_dir: PathBuf,
    pub workload_manifest: PathBuf,
    pub thresholds_path: PathBuf,
    pub report_path: PathBuf,
    pub waivers_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PerfAssertArgs {
    pub repo_root: PathBuf,
    pub thresholds_path: PathBuf,
    pub report_path: PathBuf,
    pub waivers_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfWorkloadManifest {
    pub schema_version: u32,
    pub phase_id: String,
    pub workloads: Vec<PerfWorkload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfWorkload {
    pub workload_id: String,
    pub subsystem: String,
    pub driver_sources: Vec<String>,
    pub resource_paths: Vec<String>,
    pub warmup_loops: u32,
    pub timed_loops: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfThresholds {
    pub schema_version: u32,
    pub phase_id: String,
    pub default_policy: PerfDefaultPolicy,
    pub workloads: Vec<PerfWorkloadThreshold>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfDefaultPolicy {
    pub samples_per_workload: usize,
    pub max_median_cpu_regression_ratio: f64,
    pub max_peak_allocation_regression_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfWorkloadThreshold {
    pub workload_id: String,
    pub max_median_cpu_regression_ratio: Option<f64>,
    pub max_peak_allocation_regression_ratio: Option<f64>,
    pub waiver_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfReport {
    pub schema_version: u32,
    pub phase_id: String,
    pub methodology: PerfMethodology,
    pub artifacts: PerfArtifacts,
    pub workloads: Vec<PerfWorkloadReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfMethodology {
    pub samples_per_workload: usize,
    pub cpu_metric: String,
    pub allocation_metric: String,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfArtifacts {
    pub original_reference_prefix: String,
    pub safe_stage_root: String,
    pub benchmark_runner: String,
    pub original_real_sdl_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfWorkloadReport {
    pub workload_id: String,
    pub subsystem: String,
    pub description: String,
    pub resource_paths: Vec<String>,
    pub original: PerfSubjectStats,
    pub safe: PerfSubjectStats,
    pub regression: PerfRegression,
    pub thresholds: PerfAppliedThresholds,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfSubjectStats {
    pub median_cpu_time_us: u64,
    pub median_wall_time_us: u64,
    pub peak_rss_kib: u64,
    pub samples: Vec<PerfRunnerSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfRegression {
    pub median_cpu_ratio: f64,
    pub median_wall_ratio: f64,
    pub peak_allocation_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfAppliedThresholds {
    pub max_median_cpu_regression_ratio: f64,
    pub max_peak_allocation_regression_ratio: f64,
    pub waiver_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfRunnerSample {
    pub workload_id: String,
    pub loops: u32,
    pub cpu_time_us: u64,
    pub wall_time_us: u64,
    pub max_rss_kib: u64,
    pub checksum: u64,
}

struct PreparedBuildDir {
    path: PathBuf,
    _guard: Option<TempDir>,
}

pub fn build_original_reference(args: BuildOriginalReferenceArgs) -> Result<()> {
    let original_dir = absolutize(&args.repo_root, &args.original_dir);
    let build_dir = absolutize(&args.repo_root, &args.build_dir);
    let prefix_dir = absolutize(&args.repo_root, &args.prefix_dir);

    install_original_build_dependencies(&args.repo_root, &original_dir)?;

    let build_dir = prepare_reference_build_dir(&build_dir)?;
    if prefix_dir.exists() {
        fs::remove_dir_all(&prefix_dir)
            .with_context(|| format!("remove {}", prefix_dir.display()))?;
    }
    fs::create_dir_all(&prefix_dir)?;

    let mut configure = Command::new("cmake");
    configure
        .current_dir(&args.repo_root)
        .arg("-S")
        .arg(&original_dir)
        .arg("-B")
        .arg(&build_dir.path)
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", prefix_dir.display()))
        .arg("-DSDL_SHARED=ON")
        .arg("-DSDL_STATIC=OFF")
        .arg("-DSDL_TEST=OFF")
        .arg("-DSDL_TESTS=OFF")
        .arg("-DSDL_INSTALL_TESTS=OFF");
    run_command(&mut configure, "configure original performance reference")?;

    let mut build = Command::new("cmake");
    build
        .current_dir(&args.repo_root)
        .arg("--build")
        .arg(&build_dir.path)
        .arg("--parallel")
        .arg(parallelism().to_string());
    run_command(&mut build, "build original performance reference")?;

    let mut install = Command::new("cmake");
    install
        .current_dir(&args.repo_root)
        .arg("--install")
        .arg(&build_dir.path);
    run_command(&mut install, "install original performance reference")
}

fn prepare_reference_build_dir(requested: &Path) -> Result<PreparedBuildDir> {
    if requested.exists() {
        match fs::remove_dir_all(requested) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                let tempdir = Builder::new()
                    .prefix("libsdl-original-reference-")
                    .tempdir()
                    .context("create fallback original reference build dir")?;
                eprintln!(
                    "xtask: using fallback original reference build dir {} because {} could not be removed: {}",
                    tempdir.path().display(),
                    requested.display(),
                    error
                );
                return Ok(PreparedBuildDir {
                    path: tempdir.path().to_path_buf(),
                    _guard: Some(tempdir),
                });
            }
            Err(error) => {
                return Err(error).with_context(|| format!("remove {}", requested.display()));
            }
        }
    }

    Ok(PreparedBuildDir {
        path: requested.to_path_buf(),
        _guard: None,
    })
}

pub fn perf_capture(args: PerfCaptureArgs) -> Result<()> {
    let workload_manifest =
        load_perf_workload_manifest(&absolutize(&args.repo_root, &args.workload_manifest))?;
    let thresholds = load_perf_thresholds(&absolutize(&args.repo_root, &args.thresholds_path))?;
    validate_threshold_coverage(&workload_manifest, &thresholds)?;

    let original_prefix_dir = absolutize(&args.repo_root, &args.original_prefix_dir);
    if !original_prefix_dir.exists() {
        bail!(
            "original reference {} is missing; run build-original-reference first",
            original_prefix_dir.display()
        );
    }

    let safe_stage_root = absolutize(&args.repo_root, &args.safe_stage_root);
    stage_install(StageInstallArgs {
        repo_root: args.repo_root.clone(),
        generated_dir: args.generated_dir.clone(),
        original_dir: args.original_dir.clone(),
        stage_root: safe_stage_root.clone(),
        library_path: None,
        mode: StageInstallMode::Full,
    })?;

    let runner_dir = absolutize(&args.repo_root, &args.runner_dir);
    let runner_path = build_benchmark_runner(&args.repo_root, &original_prefix_dir, &runner_dir)?;
    let original_libdir = detect_sdl_library_dir(&original_prefix_dir)?;
    let safe_libdir = detect_sdl_library_dir(&safe_stage_root)?;
    let original_real_sdl_path = detect_real_sdl_library(&original_prefix_dir)?;

    let threshold_lookup = thresholds
        .workloads
        .iter()
        .map(|threshold| (threshold.workload_id.as_str(), threshold))
        .collect::<BTreeMap<_, _>>();

    let workloads = workload_manifest
        .workloads
        .iter()
        .map(|workload| {
            let threshold = threshold_lookup
                .get(workload.workload_id.as_str())
                .ok_or_else(|| anyhow!("missing threshold for {}", workload.workload_id))?;
            let original = collect_subject_samples(
                &args.repo_root,
                &runner_path,
                workload,
                thresholds.default_policy.samples_per_workload,
                &original_libdir,
                None,
            )?;
            let safe = collect_subject_samples(
                &args.repo_root,
                &runner_path,
                workload,
                thresholds.default_policy.samples_per_workload,
                &safe_libdir,
                Some(&original_real_sdl_path),
            )?;

            let regression = PerfRegression {
                median_cpu_ratio: ratio(safe.median_cpu_time_us, original.median_cpu_time_us),
                median_wall_ratio: ratio(safe.median_wall_time_us, original.median_wall_time_us),
                peak_allocation_ratio: ratio(safe.peak_rss_kib, original.peak_rss_kib),
            };
            let applied = applied_thresholds(&thresholds.default_policy, threshold);
            let status = workload_status(&thresholds.default_policy, &applied, &regression)
                .to_string();
            Ok(PerfWorkloadReport {
                workload_id: workload.workload_id.clone(),
                subsystem: workload.subsystem.clone(),
                description: workload.description.clone(),
                resource_paths: workload.resource_paths.clone(),
                original,
                safe,
                regression,
                thresholds: applied,
                status,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let report = PerfReport {
        schema_version: 1,
        phase_id: PHASE_09_ID.to_string(),
        methodology: PerfMethodology {
            samples_per_workload: thresholds.default_policy.samples_per_workload,
            cpu_metric: "per-process user+system CPU time from getrusage(RUSAGE_SELF) delta"
                .to_string(),
            allocation_metric: "per-process peak RSS in KiB from getrusage(RUSAGE_SELF).ru_maxrss"
                .to_string(),
            environment: BTreeMap::from([
                ("SDL_AUDIODRIVER".to_string(), "dummy".to_string()),
                ("SDL_RENDER_DRIVER".to_string(), "software".to_string()),
                ("SDL_VIDEODRIVER".to_string(), "dummy".to_string()),
            ]),
        },
        artifacts: PerfArtifacts {
            original_reference_prefix: original_prefix_dir.display().to_string(),
            safe_stage_root: safe_stage_root.display().to_string(),
            benchmark_runner: runner_path.display().to_string(),
            original_real_sdl_path: original_real_sdl_path.display().to_string(),
        },
        workloads,
    };

    let report_path = absolutize(&args.repo_root, &args.report_path);
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report).context("serialize performance report")?,
    )
    .with_context(|| format!("write {}", report_path.display()))?;

    let waivers_path = absolutize(&args.repo_root, &args.waivers_path);
    if let Some(parent) = waivers_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &waivers_path,
        render_waivers_markdown(&thresholds, &report)?,
    )
    .with_context(|| format!("write {}", waivers_path.display()))?;

    Ok(())
}

pub fn perf_assert(args: PerfAssertArgs) -> Result<()> {
    let thresholds = load_perf_thresholds(&absolutize(&args.repo_root, &args.thresholds_path))?;
    let report = load_perf_report(&absolutize(&args.repo_root, &args.report_path))?;
    let waivers = fs::read_to_string(absolutize(&args.repo_root, &args.waivers_path))
        .context("read perf waiver report")?;

    let threshold_lookup = thresholds
        .workloads
        .iter()
        .map(|threshold| (threshold.workload_id.as_str(), threshold))
        .collect::<BTreeMap<_, _>>();

    for workload in &report.workloads {
        let threshold = threshold_lookup
            .get(workload.workload_id.as_str())
            .ok_or_else(|| anyhow!("missing threshold for {}", workload.workload_id))?;
        let applied = applied_thresholds(&thresholds.default_policy, threshold);
        if workload.thresholds.max_median_cpu_regression_ratio
            != applied.max_median_cpu_regression_ratio
            || workload.thresholds.max_peak_allocation_regression_ratio
                != applied.max_peak_allocation_regression_ratio
            || workload.thresholds.waiver_id != applied.waiver_id
        {
            bail!(
                "{} report thresholds do not match {}",
                workload.workload_id,
                absolutize(&args.repo_root, &args.thresholds_path).display()
            );
        }
        let expected_status = workload_status(
            &thresholds.default_policy,
            &applied,
            &workload.regression,
        );
        if workload.status != expected_status {
            bail!(
                "{} report status {} does not match expected {}",
                workload.workload_id,
                workload.status,
                expected_status
            );
        }
        if workload.regression.median_cpu_ratio > applied.max_median_cpu_regression_ratio {
            bail!(
                "{} CPU regression {:.3} exceeds {:.3}",
                workload.workload_id,
                workload.regression.median_cpu_ratio,
                applied.max_median_cpu_regression_ratio
            );
        }
        if workload.regression.peak_allocation_ratio > applied.max_peak_allocation_regression_ratio
        {
            bail!(
                "{} allocation regression {:.3} exceeds {:.3}",
                workload.workload_id,
                workload.regression.peak_allocation_ratio,
                applied.max_peak_allocation_regression_ratio
            );
        }
        if let Some(waiver_id) = &applied.waiver_id {
            if !waivers.contains(&format!("`{waiver_id}`")) {
                bail!(
                    "{} uses waiver {} but {} does not document it",
                    workload.workload_id,
                    waiver_id,
                    absolutize(&args.repo_root, &args.waivers_path).display()
                );
            }
        } else if workload.regression.median_cpu_ratio
            > thresholds.default_policy.max_median_cpu_regression_ratio
            || workload.regression.peak_allocation_ratio
                > thresholds
                    .default_policy
                    .max_peak_allocation_regression_ratio
        {
            bail!(
                "{} exceeds the default performance policy without a waiver",
                workload.workload_id
            );
        }
    }

    Ok(())
}

pub fn load_perf_workload_manifest(path: &Path) -> Result<PerfWorkloadManifest> {
    serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
        .with_context(|| format!("parse {}", path.display()))
}

pub fn load_perf_thresholds(path: &Path) -> Result<PerfThresholds> {
    serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
        .with_context(|| format!("parse {}", path.display()))
}

pub fn load_perf_report(path: &Path) -> Result<PerfReport> {
    serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
        .with_context(|| format!("parse {}", path.display()))
}

fn install_original_build_dependencies(repo_root: &Path, original_dir: &Path) -> Result<()> {
    let original_build_dep_path = original_dir
        .strip_prefix(repo_root)
        .unwrap_or(original_dir)
        .to_str()
        .ok_or_else(|| anyhow!("non-utf8 original directory"))?;
    let original_build_dep_path = if original_build_dep_path.starts_with("./") {
        original_build_dep_path.to_string()
    } else {
        format!("./{original_build_dep_path}")
    };
    for args in [
        vec![
            "-n",
            "env",
            "DEBIAN_FRONTEND=noninteractive",
            "apt-get",
            "update",
        ],
        vec![
            "-n",
            "env",
            "DEBIAN_FRONTEND=noninteractive",
            "apt-get",
            "install",
            "-y",
            "build-essential",
            "cmake",
            "pkg-config",
        ],
        vec![
            "-n",
            "env",
            "DEBIAN_FRONTEND=noninteractive",
            "apt-get",
            "build-dep",
            "-y",
            original_build_dep_path.as_str(),
        ],
    ] {
        let mut command = Command::new("sudo");
        command.current_dir(repo_root).args(args);
        run_command(
            &mut command,
            "install original performance build dependencies",
        )?;
    }
    Ok(())
}

fn build_benchmark_runner(
    repo_root: &Path,
    original_prefix_dir: &Path,
    runner_dir: &Path,
) -> Result<PathBuf> {
    let runner_dir = prepare_perf_runner_dir(runner_dir)?;

    let source_path = runner_dir.join("perf_runner.c");
    let binary_path = runner_dir.join("perf_runner");
    fs::write(&source_path, PERF_RUNNER_SOURCE)
        .with_context(|| format!("write {}", source_path.display()))?;

    let sdl2_config = detect_sdl2_config(original_prefix_dir)?;
    let cflags = split_shell_words(&command_stdout(
        Command::new(&sdl2_config).arg("--cflags"),
        "query sdl2-config --cflags",
    )?);
    let libs = split_shell_words(&command_stdout(
        Command::new(&sdl2_config).arg("--libs"),
        "query sdl2-config --libs",
    )?);

    let mut command = Command::new("cc");
    command
        .current_dir(repo_root)
        .arg("-O3")
        .arg("-DNDEBUG")
        .arg("-std=c11")
        .arg(&source_path)
        .arg("-o")
        .arg(&binary_path)
        .args(cflags)
        .args(libs);
    run_command(&mut command, "compile benchmark runner")?;
    Ok(binary_path)
}

fn prepare_perf_runner_dir(requested: &Path) -> Result<PathBuf> {
    if requested.exists() {
        match fs::remove_dir_all(requested) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                let fallback = unique_temp_dir("libsdl-perf-runner")?;
                eprintln!(
                    "xtask: using fallback perf runner dir {} because {} could not be removed: {}",
                    fallback.display(),
                    requested.display(),
                    error
                );
                return Ok(fallback);
            }
            Err(error) => {
                return Err(error).with_context(|| format!("remove {}", requested.display()));
            }
        }
    }

    fs::create_dir_all(requested).with_context(|| format!("create {}", requested.display()))?;
    Ok(requested.to_path_buf())
}

fn unique_temp_dir(prefix: &str) -> Result<PathBuf> {
    for attempt in 0..16 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let candidate = env::temp_dir().join(format!("{prefix}-{nonce}-{attempt}"));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("create {}", candidate.display()));
            }
        }
    }

    bail!("unable to create unique temp dir for {prefix}")
}

fn collect_subject_samples(
    repo_root: &Path,
    runner_path: &Path,
    workload: &PerfWorkload,
    sample_count: usize,
    library_dir: &Path,
    real_sdl_path: Option<&Path>,
) -> Result<PerfSubjectStats> {
    let resource_paths = workload
        .resource_paths
        .iter()
        .map(|path| absolutize(repo_root, Path::new(path)))
        .collect::<Vec<_>>();
    let mut samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let mut command = Command::new(runner_path);
        command
            .current_dir(repo_root)
            .arg(&workload.workload_id)
            .arg(workload.warmup_loops.to_string())
            .arg(workload.timed_loops.to_string())
            .args(resource_paths.iter().map(|path| path.as_os_str()));
        apply_perf_env(&mut command, library_dir, real_sdl_path)?;
        let output = command
            .output()
            .with_context(|| format!("run perf workload {}", workload.workload_id))?;
        if !output.status.success() {
            bail!(
                "perf workload {} failed:\n{}",
                workload.workload_id,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        samples.push(
            serde_json::from_slice::<PerfRunnerSample>(&output.stdout)
                .with_context(|| format!("parse perf output for {}", workload.workload_id))?,
        );
    }

    Ok(PerfSubjectStats {
        median_cpu_time_us: median(samples.iter().map(|sample| sample.cpu_time_us)),
        median_wall_time_us: median(samples.iter().map(|sample| sample.wall_time_us)),
        peak_rss_kib: samples
            .iter()
            .map(|sample| sample.max_rss_kib)
            .max()
            .unwrap_or_default(),
        samples,
    })
}

fn render_waivers_markdown(thresholds: &PerfThresholds, report: &PerfReport) -> Result<String> {
    let mut markdown = String::from("# Performance Waivers\n\n");
    markdown.push_str(&format!("Phase: `{}`.\n\n", PHASE_09_ID));
    markdown.push_str("- Default max median CPU regression: 20%.\n");
    markdown.push_str("- Default max peak allocation regression: 25%.\n");
    markdown.push_str(
        "- Allocation guard uses per-workload peak RSS because each workload runs in its own process.\n\n",
    );

    let waived = thresholds
        .workloads
        .iter()
        .filter(|threshold| threshold.waiver_id.is_some())
        .collect::<Vec<_>>();
    if waived.is_empty() {
        markdown.push_str("No active waivers.\n");
        return Ok(markdown);
    }

    for threshold in waived {
        let waiver_id = threshold
            .waiver_id
            .as_ref()
            .ok_or_else(|| anyhow!("waiver entry missing waiver_id"))?;
        let reason = threshold
            .reason
            .as_deref()
            .unwrap_or("Compatibility or safety requirements justify the measured regression.");
        let workload = report
            .workloads
            .iter()
            .find(|entry| entry.workload_id == threshold.workload_id);
        markdown.push_str(&format!("## `{waiver_id}`\n\n"));
        markdown.push_str(&format!("- Workload: `{}`.\n", threshold.workload_id));
        markdown.push_str(&format!("- Reason: {}.\n", reason));
        markdown.push_str(&format!(
            "- Allowed CPU ratio: {:.3}.\n",
            threshold
                .max_median_cpu_regression_ratio
                .unwrap_or(thresholds.default_policy.max_median_cpu_regression_ratio)
        ));
        markdown.push_str(&format!(
            "- Allowed allocation ratio: {:.3}.\n",
            threshold.max_peak_allocation_regression_ratio.unwrap_or(
                thresholds
                    .default_policy
                    .max_peak_allocation_regression_ratio
            )
        ));
        if let Some(workload) = workload {
            markdown.push_str(&format!("- Current report status: `{}`.\n", workload.status));
            markdown.push_str(&format!(
                "- Measured CPU ratio: {:.3}; measured wall ratio: {:.3}; measured allocation ratio: {:.3}.\n",
                workload.regression.median_cpu_ratio,
                workload.regression.median_wall_ratio,
                workload.regression.peak_allocation_ratio
            ));
        }
        markdown.push('\n');
    }

    Ok(markdown)
}

fn validate_threshold_coverage(
    workload_manifest: &PerfWorkloadManifest,
    thresholds: &PerfThresholds,
) -> Result<()> {
    if workload_manifest.phase_id != PHASE_09_ID {
        bail!(
            "perf workload manifest phase_id must be {}, found {}",
            PHASE_09_ID,
            workload_manifest.phase_id
        );
    }
    if thresholds.phase_id != PHASE_09_ID {
        bail!(
            "perf thresholds phase_id must be {}, found {}",
            PHASE_09_ID,
            thresholds.phase_id
        );
    }
    let workload_ids = workload_manifest
        .workloads
        .iter()
        .map(|workload| workload.workload_id.as_str())
        .collect::<BTreeSet<_>>();
    let threshold_ids = thresholds
        .workloads
        .iter()
        .map(|workload| workload.workload_id.as_str())
        .collect::<BTreeSet<_>>();
    if workload_ids != threshold_ids {
        bail!(
            "perf workload/threshold coverage mismatch\nworkloads: {:?}\nthresholds: {:?}",
            workload_ids,
            threshold_ids
        );
    }
    for workload in &thresholds.workloads {
        if workload.waiver_id.is_some()
            && workload
                .reason
                .as_deref()
                .map(str::trim)
                .filter(|reason| !reason.is_empty())
                .is_none()
        {
            bail!(
                "waived workload {} is missing a reason",
                workload.workload_id
            );
        }
    }
    Ok(())
}

fn applied_thresholds(
    default_policy: &PerfDefaultPolicy,
    threshold: &PerfWorkloadThreshold,
) -> PerfAppliedThresholds {
    PerfAppliedThresholds {
        max_median_cpu_regression_ratio: threshold
            .max_median_cpu_regression_ratio
            .unwrap_or(default_policy.max_median_cpu_regression_ratio),
        max_peak_allocation_regression_ratio: threshold
            .max_peak_allocation_regression_ratio
            .unwrap_or(default_policy.max_peak_allocation_regression_ratio),
        waiver_id: threshold.waiver_id.clone(),
    }
}

fn workload_status(
    default_policy: &PerfDefaultPolicy,
    applied: &PerfAppliedThresholds,
    regression: &PerfRegression,
) -> &'static str {
    let within_default = regression.median_cpu_ratio
        <= default_policy.max_median_cpu_regression_ratio
        && regression.peak_allocation_ratio
            <= default_policy.max_peak_allocation_regression_ratio;
    let within_threshold = regression.median_cpu_ratio
        <= applied.max_median_cpu_regression_ratio
        && regression.peak_allocation_ratio <= applied.max_peak_allocation_regression_ratio;

    if !within_threshold {
        "fail"
    } else if !within_default && applied.waiver_id.is_some() {
        "pass_with_waiver"
    } else {
        "pass"
    }
}

fn apply_perf_env(
    command: &mut Command,
    library_dir: &Path,
    real_sdl_path: Option<&Path>,
) -> Result<()> {
    command
        .env("SDL_AUDIODRIVER", "dummy")
        .env("SDL_RENDER_DRIVER", "software")
        .env("SDL_VIDEODRIVER", "dummy");

    if let Some(path) = real_sdl_path {
        command.env("SAFE_SDL_REAL_SDL_PATH", path);
    } else {
        command.env_remove("SAFE_SDL_REAL_SDL_PATH");
    }
    command.env(
        "LD_LIBRARY_PATH",
        joined_env_path(library_dir, env::var_os("LD_LIBRARY_PATH"))?,
    );
    Ok(())
}

fn detect_sdl2_config(root: &Path) -> Result<PathBuf> {
    for candidate in [
        root.join("bin/sdl2-config"),
        root.join("usr/bin/sdl2-config"),
    ] {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!("unable to locate sdl2-config under {}", root.display());
}

fn detect_sdl_library_dir(root: &Path) -> Result<PathBuf> {
    for base in [root.to_path_buf(), root.join("usr")] {
        for candidate in [
            base.join(format!("lib/{UBUNTU_MULTIARCH}")),
            base.join("lib"),
        ] {
            if candidate.join("libSDL2.so").exists()
                || candidate.join("libSDL2-2.0.so.0").exists()
                || candidate.join("libSDL2-2.0.so.0.0.0").exists()
            {
                return Ok(candidate);
            }
        }
    }
    bail!("unable to locate libSDL2 under {}", root.display());
}

fn detect_real_sdl_library(root: &Path) -> Result<PathBuf> {
    let libdir = detect_sdl_library_dir(root)?;
    for candidate in [
        libdir.join("libSDL2-2.0.so.0.0.0"),
        libdir.join("libSDL2-2.0.so.0"),
        libdir.join("libSDL2.so"),
    ] {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!(
        "unable to locate the real SDL shared object under {}",
        root.display()
    );
}

fn split_shell_words(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_string).collect()
}

fn command_stdout(command: &mut Command, description: &str) -> Result<String> {
    let output = command.output().with_context(|| description.to_string())?;
    if !output.status.success() {
        bail!(
            "{description} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn joined_env_path(
    first: &Path,
    existing: Option<std::ffi::OsString>,
) -> Result<std::ffi::OsString> {
    let mut entries = vec![first.to_path_buf()];
    if let Some(existing) = existing {
        entries.extend(env::split_paths(&existing));
    }
    env::join_paths(entries).context("join environment search path")
}

fn median<I>(values: I) -> u64
where
    I: IntoIterator<Item = u64>,
{
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or_default()
}

fn ratio(candidate: u64, baseline: u64) -> f64 {
    if baseline == 0 {
        if candidate == 0 {
            1.0
        } else {
            f64::INFINITY
        }
    } else {
        candidate as f64 / baseline as f64
    }
}

fn parallelism() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

fn run_command(command: &mut Command, description: &str) -> Result<()> {
    let status = command.status().with_context(|| description.to_string())?;
    if !status.success() {
        bail!("{description} failed with status {status}");
    }
    Ok(())
}

fn absolutize(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}
