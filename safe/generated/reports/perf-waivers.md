# Performance Waivers

Phase: `impl_phase_09_performance`.

- Default max median CPU regression: 20%.
- Default max peak allocation regression: 25%.
- Allocation guard uses per-workload peak RSS because each workload runs in its own process.

## `audio_pure_rust_decode_resample`

- Workload: `audio_stream_convert_resample_wave`.
- Reason: The safe build keeps checked Rust implementations for MS ADPCM decode and sample-rate conversion; after buffer reuse and resample-order tuning the remaining CPU gap is accepted to preserve memory safety and deterministic behavior without hand-written unsafe SIMD.
- Allowed CPU ratio: 1.900.
- Allowed allocation ratio: 1.250.
- Measured CPU ratio: 1.838; measured allocation ratio: 1.000.

