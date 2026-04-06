# Performance Waivers

Phase: `impl_phase_09_performance`.

- Default max median CPU regression: 20%.
- Default max peak allocation regression: 25%.
- Allocation guard uses per-workload peak RSS because each workload runs in its own process.

## `renderer_safe_shim_copy_upload`

- Workload: `renderer_queue_copy_texture_upload`.
- Reason: The safe build routes this software-renderer microbenchmark through exported Rust shims that preserve managed texture and window invariants while clearing host error state before delegated upload and copy calls; the extra boundary cost is accepted here to keep the FFI safety contract explicit.
- Allowed CPU ratio: 2.250.
- Allowed allocation ratio: 1.250.
- Current report status: `pass_with_waiver`.
- Measured CPU ratio: 2.060; measured wall ratio: 2.058; measured allocation ratio: 1.000.

## `audio_pure_rust_decode_resample`

- Workload: `audio_stream_convert_resample_wave`.
- Reason: The safe build keeps checked Rust implementations for MS ADPCM decode and sample-rate conversion; after buffer reuse and resample-order tuning the remaining CPU gap is accepted to preserve memory safety and deterministic behavior without hand-written unsafe SIMD.
- Allowed CPU ratio: 1.900.
- Allowed allocation ratio: 1.250.
- Current report status: `pass_with_waiver`.
- Measured CPU ratio: 1.724; measured wall ratio: 1.724; measured allocation ratio: 1.000.

## `events_safe_queue_bookkeeping`

- Workload: `event_queue_throughput`.
- Reason: The safe build keeps a Rust-owned event queue with Mutex and VecDeque bookkeeping plus safe filter and watcher handling instead of the upstream intrusive queue and atomics; this custom-event throughput microbenchmark remains waived to preserve memory-safe ownership and deterministic queue semantics.
- Allowed CPU ratio: 4.250.
- Allowed allocation ratio: 1.250.
- Current report status: `pass_with_waiver`.
- Measured CPU ratio: 3.814; measured wall ratio: 3.815; measured allocation ratio: 1.000.

