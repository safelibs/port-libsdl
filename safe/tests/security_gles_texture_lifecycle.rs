use std::sync::{Mutex, OnceLock};

use safe_sdl::render::gles::{
    reset_texture_lifecycle_counters, simulate_texture_creation_for_test,
    texture_lifecycle_counters, TestTextureKind,
};

fn serial_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn successful_streaming_yuv_texture_creation_releases_every_resource_on_drop() {
    let _serial = serial_lock();
    reset_texture_lifecycle_counters();

    {
        let allocation =
            simulate_texture_creation_for_test(TestTextureKind::YuvPlanar, true, None)
                .expect("YUV texture allocation should succeed");
        let counters = texture_lifecycle_counters();
        assert_eq!(counters.created_textures, 3);
        assert_eq!(counters.destroyed_textures, 0);
        assert_eq!(counters.allocated_pixel_buffers, 1);
        assert_eq!(counters.freed_pixel_buffers, 0);
        drop(allocation);
    }

    let counters = texture_lifecycle_counters();
    assert_eq!(counters.created_textures, counters.destroyed_textures);
    assert_eq!(
        counters.allocated_pixel_buffers,
        counters.freed_pixel_buffers
    );
}

#[test]
fn injected_failures_do_not_leak_partially_created_gles_resources() {
    let _serial = serial_lock();
    let scenarios = [
        (TestTextureKind::Rgba, true, 2usize),
        (TestTextureKind::YuvPlanar, true, 4usize),
        (TestTextureKind::Nv12, false, 2usize),
        (TestTextureKind::ExternalOes, false, 1usize),
    ];

    for (kind, streaming, max_step) in scenarios {
        for fail_after in 1..=max_step {
            reset_texture_lifecycle_counters();
            let result = simulate_texture_creation_for_test(kind, streaming, Some(fail_after));
            assert!(
                result.is_err(),
                "expected failure at step {fail_after} for {kind:?}"
            );

            let counters = texture_lifecycle_counters();
            assert_eq!(
                counters.created_textures, counters.destroyed_textures,
                "texture leak detected for {kind:?} at step {fail_after}: {counters:?}"
            );
            assert_eq!(
                counters.allocated_pixel_buffers, counters.freed_pixel_buffers,
                "pixel-buffer leak detected for {kind:?} at step {fail_after}: {counters:?}"
            );
        }
    }
}
