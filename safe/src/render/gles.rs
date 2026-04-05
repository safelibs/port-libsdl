use std::sync::{Mutex, OnceLock};

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestTextureKind {
    Rgba,
    YuvPlanar,
    Nv12,
    ExternalOes,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextureLifecycleCounters {
    pub created_textures: u32,
    pub destroyed_textures: u32,
    pub allocated_pixel_buffers: u32,
    pub freed_pixel_buffers: u32,
}

#[derive(Debug, Default)]
struct TrackerState {
    next_texture_id: u32,
    counters: TextureLifecycleCounters,
}

fn tracker() -> &'static Mutex<TrackerState> {
    static TRACKER: OnceLock<Mutex<TrackerState>> = OnceLock::new();
    TRACKER.get_or_init(|| Mutex::new(TrackerState::default()))
}

struct FakeTexture {
    active: bool,
}

impl FakeTexture {
    fn create(fail_after: Option<usize>, step: &mut usize) -> Result<Self, String> {
        *step += 1;
        if fail_after == Some(*step) {
            return Err(format!("simulated GLES texture allocation failure at step {}", *step));
        }

        let mut state = tracker().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.next_texture_id += 1;
        state.counters.created_textures += 1;
        Ok(Self { active: true })
    }
}

impl Drop for FakeTexture {
    fn drop(&mut self) {
        if self.active {
            let mut state = tracker().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            state.counters.destroyed_textures += 1;
        }
    }
}

struct PixelBuffer {
    active: bool,
}

impl PixelBuffer {
    fn allocate(fail_after: Option<usize>, step: &mut usize) -> Result<Self, String> {
        *step += 1;
        if fail_after == Some(*step) {
            return Err(format!(
                "simulated GLES pixel buffer allocation failure at step {}",
                *step
            ));
        }

        let mut state = tracker().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.counters.allocated_pixel_buffers += 1;
        Ok(Self { active: true })
    }
}

impl Drop for PixelBuffer {
    fn drop(&mut self) {
        if self.active {
            let mut state = tracker().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            state.counters.freed_pixel_buffers += 1;
        }
    }
}

#[doc(hidden)]
pub struct TestTextureAllocation {
    main: Option<FakeTexture>,
    u_plane: Option<FakeTexture>,
    v_plane: Option<FakeTexture>,
    pixel_buffer: Option<PixelBuffer>,
}

impl TestTextureAllocation {
    fn new() -> Self {
        Self {
            main: None,
            u_plane: None,
            v_plane: None,
            pixel_buffer: None,
        }
    }
}

#[doc(hidden)]
pub fn reset_texture_lifecycle_counters() {
    let mut state = tracker().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *state = TrackerState::default();
}

#[doc(hidden)]
pub fn texture_lifecycle_counters() -> TextureLifecycleCounters {
    tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .counters
}

#[doc(hidden)]
pub fn simulate_texture_creation_for_test(
    kind: TestTextureKind,
    streaming: bool,
    fail_after: Option<usize>,
) -> Result<TestTextureAllocation, String> {
    let mut allocation = TestTextureAllocation::new();
    let mut step = 0usize;

    if streaming {
        allocation.pixel_buffer = Some(PixelBuffer::allocate(fail_after, &mut step)?);
    }

    allocation.main = Some(FakeTexture::create(fail_after, &mut step)?);

    match kind {
        TestTextureKind::YuvPlanar => {
            allocation.u_plane = Some(FakeTexture::create(fail_after, &mut step)?);
            allocation.v_plane = Some(FakeTexture::create(fail_after, &mut step)?);
        }
        TestTextureKind::Nv12 => {
            allocation.u_plane = Some(FakeTexture::create(fail_after, &mut step)?);
        }
        TestTextureKind::ExternalOes | TestTextureKind::Rgba => {}
    }

    Ok(allocation)
}
