//! Retex D3D9 hook (32-bit).
//!
//! An inline detour on `Direct3DCreate9` leads to vtable hooks on
//! `CreateDevice`, `CreateTexture`, `SetTexture`, and the texture's
//! `LockRect`/`UnlockRect`/`Release`. Textures are identified with TexMod's
//! hash and swapped for the pack's replacement when bound.
//!
//! Work is kept off the frame as far as possible:
//!
//! - A texture is hashed as soon as the game finishes writing it (usually
//!   behind a loading screen), not when it is first drawn. Textures filled any
//!   other way are hashed on first bind instead.
//! - Replacements are read, decrypted and inflated by background threads, in
//!   parallel with the game's own loading; only the D3D texture creation runs
//!   on the game's thread.
//! - After that, a bind is one hash-map lookup.
//!
//! Load as an ASI plugin (export `InitializeASI`) or inject with
//! `retex-inject` (export `Initialize`). Only the first copy loaded into a
//! process activates.

#![cfg_attr(not(all(windows, target_arch = "x86")), allow(unused))]

#[cfg(all(windows, target_arch = "x86"))]
mod imp {
    #![allow(clippy::manual_c_str_literals)]
    use minhook::MinHook;
    use retex_core::{bytes_per_pixel, texmod_crc, PackageIndex};
    use std::cell::Cell;
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::ffi::{c_void, CString, OsString};
    use std::hash::{BuildHasherDefault, Hasher};
    use std::io::Write;
    use std::mem;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard, Once, OnceLock};
    use std::time::Instant;

    const PAGE_EXECUTE_READWRITE: u32 = 0x40;
    const ERROR_ALREADY_EXISTS: u32 = 183;
    const D3DERR_INVALIDCALL: i32 = 0x8876_086Cu32 as i32;

    const IUNKNOWN_RELEASE: usize = 2;
    const IDIRECT3DRESOURCE9_GET_DEVICE: usize = 3;
    const IDIRECT3D9_CREATE_DEVICE: usize = 16;
    const IDIRECT3DDEVICE9_CREATE_TEXTURE: usize = 23;
    const IDIRECT3DDEVICE9_SET_TEXTURE: usize = 65;

    const IDIRECT3DBASETEXTURE9_GET_TYPE: usize = 10;
    const IDIRECT3DTEXTURE9_GET_LEVEL_DESC: usize = 17;
    const IDIRECT3DTEXTURE9_LOCK_RECT: usize = 19;
    const IDIRECT3DTEXTURE9_UNLOCK_RECT: usize = 20;

    const D3DRTYPE_TEXTURE: u32 = 3;
    const D3DUSAGE_RENDERTARGET: u32 = 0x0000_0001;
    const D3DUSAGE_DEPTHSTENCIL: u32 = 0x0000_0002;
    const D3DUSAGE_DYNAMIC: u32 = 0x0000_0200;
    const D3DFMT_A8R8G8B8: u32 = 0x15;
    const D3DPOOL_MANAGED: u32 = 3;
    const D3DLOCK_READONLY: u32 = 0x0000_0010;
    const MAX_TRACKED_TEXTURES: usize = 200_000;
    /// Decoded replacements waiting for texture creation; background decoding
    /// pauses above this so a 32-bit game is not flooded.
    const MAX_READY_BYTES: usize = 256 << 20;

    type Direct3DCreate9Fn = unsafe extern "system" fn(u32) -> *mut c_void;
    type CreateDeviceFn = unsafe extern "system" fn(
        *mut c_void,
        u32,
        u32,
        *mut c_void,
        u32,
        *mut c_void,
        *mut *mut c_void,
    ) -> i32;
    type CreateTextureFn = unsafe extern "system" fn(
        *mut c_void,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        *mut *mut c_void,
        *mut c_void,
    ) -> i32;
    type SetTextureFn = unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> i32;
    type ReleaseFn = unsafe extern "system" fn(*mut c_void) -> u32;
    type GetDeviceFn = unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32;
    type GetTypeFn = unsafe extern "system" fn(*mut c_void) -> u32;
    type GetLevelDescFn = unsafe extern "system" fn(*mut c_void, u32, *mut D3DSurfaceDesc) -> i32;
    type LockRectFn =
        unsafe extern "system" fn(*mut c_void, u32, *mut D3DLockedRect, *const c_void, u32) -> i32;
    type UnlockRectFn = unsafe extern "system" fn(*mut c_void, u32) -> i32;

    #[repr(C)]
    #[derive(Default)]
    struct D3DLockedRect {
        pitch: i32,
        bits: *mut c_void,
    }

    #[repr(C)]
    #[derive(Default)]
    struct D3DSurfaceDesc {
        format: u32,
        resource_type: u32,
        usage: u32,
        pool: u32,
        multisample_type: u32,
        multisample_quality: u32,
        width: u32,
        height: u32,
    }

    struct Replacement {
        width: u32,
        height: u32,
        /// BGRA, top-down, tightly packed.
        pixels: Vec<u8>,
    }

    /// Multiply-rotate hasher for pointer and texture-hash keys. The default
    /// SipHash is built to resist adversarial keys, which these are not, and
    /// it would run on every texture bind.
    #[derive(Default, Clone, Copy)]
    struct FastHasher(u64);

    impl FastHasher {
        #[inline]
        fn add(&mut self, value: u64) {
            self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x517C_C1B7_2722_0A95);
        }
    }

    impl Hasher for FastHasher {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.add(b as u64);
            }
        }
        fn write_u32(&mut self, value: u32) {
            self.add(value as u64);
        }
        fn write_usize(&mut self, value: usize) {
            self.add(value as u64);
        }
    }

    type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;
    type FastSet<K> = HashSet<K, BuildHasherDefault<FastHasher>>;

    static ORIG_D3D9: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static ORIG_CREATE_DEVICE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static ORIG_CREATE_TEXTURE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static ORIG_SET_TEXTURE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    /// Per-texture logging is off unless `RETEX_DEBUG` is set: a log write
    /// opens and closes a file, which is far too slow for per-bind checks.
    static VERBOSE: AtomicBool = AtomicBool::new(false);

    fn verbose() -> bool {
        VERBOSE.load(Ordering::Relaxed)
    }

    /// What a game texture is bound as.
    #[derive(Clone, Copy)]
    enum Tex {
        /// Not in any pack (or its replacement failed): bind as is.
        Original,
        /// In a pack; the replacement has not been created yet.
        Replace { hash: u32, format: u32 },
        /// Bind this replacement texture instead.
        Ready(usize),
    }

    /// Game texture pointer -> how to bind it. Entries are evicted when the
    /// texture is released, so a recycled pointer never inherits a stale
    /// answer, and refreshed when the game rewrites the texture.
    static TEXTURES: LazyLock<Mutex<FastMap<usize, Tex>>> = LazyLock::new(Default::default);

    /// Textures locked for writing at level 0 and not yet unlocked.
    static WRITE_LOCKED: LazyLock<Mutex<FastSet<usize>>> = LazyLock::new(Default::default);

    /// How often the game has rewritten each texture. One rewritten over and
    /// over (a minimap, a scrolling sign) is left alone rather than re-hashed
    /// on every update.
    static REWRITES: LazyLock<Mutex<FastMap<usize, u32>>> = LazyLock::new(Default::default);
    const MAX_REWRITES: u32 = 8;

    /// Original texture methods, per vtable: ReShade or another wrapper can
    /// expose more than one implementation, and each needs its own original
    /// to forward to. `lock`/`unlock` are 0 for non-2D textures.
    #[derive(Clone, Copy)]
    struct VtableOriginals {
        release: usize,
        lock: usize,
        unlock: usize,
    }
    static VTABLES: LazyLock<Mutex<FastMap<usize, VtableOriginals>>> =
        LazyLock::new(Default::default);

    /// `(hash, format)` -> replacement texture, or 0 if creating it failed (so a
    /// broken entry is not retried on every bind). Cleared when a device is
    /// created.
    type ReplacementKey = (u32, u32);
    static REPLACEMENTS: LazyLock<Mutex<FastMap<ReplacementKey, usize>>> =
        LazyLock::new(Default::default);

    /// Replacements requested while loading, created as soon as their data
    /// has been decoded.
    static PENDING: LazyLock<Mutex<FastSet<ReplacementKey>>> = LazyLock::new(Default::default);

    struct Packs {
        indexes: Vec<Arc<PackageIndex>>,
        by_hash: FastMap<u32, (usize, usize)>,
    }

    /// Loaded once at startup and read-only afterwards, so lookups need no
    /// lock.
    static PACKS: OnceLock<Packs> = OnceLock::new();
    static INIT: Once = Once::new();

    thread_local! {
        /// Set while this hook makes its own D3D calls, so the texture hooks
        /// forward them without treating them as the game's.
        static BUSY: Cell<bool> = const { Cell::new(false) };
    }

    fn busy() -> bool {
        BUSY.with(|b| b.get())
    }

    fn with_busy<T>(f: impl FnOnce() -> T) -> T {
        let previous = BUSY.with(|b| b.replace(true));
        let result = f();
        BUSY.with(|b| b.set(previous));
        result
    }

    /// Lock, ignoring poisoning: a panic elsewhere must not stop the game
    /// from rendering.
    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(|error| error.into_inner())
    }

    extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
        fn VirtualProtect(addr: *mut c_void, size: usize, new_protect: u32, old: *mut u32) -> i32;
        fn OutputDebugStringA(text: *const u8);
        fn GetModuleFileNameW(module: *mut c_void, buffer: *mut u16, size: u32) -> u32;
        fn CreateMutexW(attributes: *mut c_void, owner: i32, name: *const u16) -> *mut c_void;
        fn GetLastError() -> u32;
    }

    pub fn log(message: &str) {
        let line = format!("[retex-hook] {message}\n");
        let path = std::env::temp_dir().join("retex-hook.log");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = file.write_all(line.as_bytes());
        }
        if let Ok(text) = CString::new(line) {
            unsafe { OutputDebugStringA(text.as_ptr() as *const u8) };
        }
    }

    fn millis(since: Instant) -> f64 {
        since.elapsed().as_secs_f64() * 1e3
    }

    unsafe fn vtable(object: *mut c_void) -> *mut *mut c_void {
        *(object as *mut *mut *mut c_void)
    }

    unsafe fn patch_slot(vtable: *mut *mut c_void, index: usize, new: *mut c_void) -> *mut c_void {
        let slot = vtable.add(index);
        let mut old_protect = 0u32;
        VirtualProtect(
            slot as *mut c_void,
            mem::size_of::<*mut c_void>(),
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        );
        let previous = *slot;
        *slot = new;
        VirtualProtect(
            slot as *mut c_void,
            mem::size_of::<*mut c_void>(),
            old_protect,
            &mut old_protect,
        );
        previous
    }

    /// Point `vtable[index]` at `hook`, recording the original in `original`.
    ///
    /// Idempotent: a slot that already holds `hook` is left alone, so games
    /// that call `Direct3DCreate9`/`CreateDevice` repeatedly do not end up
    /// with the hook as its own "original". A second vtable with a different
    /// implementation is not patched, because each hook forwards to exactly
    /// one original. Returns whether the slot now holds `hook`.
    unsafe fn hook_slot(
        vtable: *mut *mut c_void,
        index: usize,
        hook: *mut c_void,
        original: &AtomicPtr<c_void>,
        name: &str,
    ) -> bool {
        let current = *vtable.add(index);
        if current == hook {
            return true;
        }
        match original.compare_exchange(
            std::ptr::null_mut(),
            current,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => log(&format!("hooked {name}")),
            Err(existing) if existing == current => {}
            Err(existing) => {
                log(&format!(
                    "not hooking {name} in vtable {vtable:p}: implementation {current:p} \
                     differs from {existing:p}"
                ));
                return false;
            }
        }
        patch_slot(vtable, index, hook);
        true
    }

    // -----------------------------------------------------------------------
    // Hashing
    // -----------------------------------------------------------------------

    unsafe fn hash_level_zero(texture: *mut c_void, desc: &D3DSurfaceDesc) -> Option<u32> {
        let vt = vtable(texture);
        let lock: LockRectFn = mem::transmute(*vt.add(IDIRECT3DTEXTURE9_LOCK_RECT));
        let unlock: UnlockRectFn = mem::transmute(*vt.add(IDIRECT3DTEXTURE9_UNLOCK_RECT));

        let mut locked = D3DLockedRect::default();
        let hr = lock(texture, 0, &mut locked, std::ptr::null(), D3DLOCK_READONLY);
        if hr < 0 || locked.bits.is_null() {
            return None;
        }
        let len =
            (desc.width as f64 * desc.height as f64 * bytes_per_pixel(desc.format) as f64) as usize;
        let slice = std::slice::from_raw_parts(locked.bits as *const u8, len);
        let hash = texmod_crc(slice);
        unlock(texture, 0);
        Some(hash)
    }

    /// True for D3D formats that carry meaningful alpha.
    fn format_has_alpha(format: u32) -> bool {
        matches!(
            format,
            0x15 | 0x19 | 0x1A | 0x1C | 0x1D | 0x1F | 0x20 | 0x24 | 0x33 | 0x34 | 0x3E | 0x41
        ) || matches!(format, 0x3254_5844..=0x3554_5844) // DXT2..DXT5
    }

    /// Hash a 2D texture's level 0; `Replace` only if a pack replaces it.
    unsafe fn hash_texture(texture: *mut c_void) -> Tex {
        let vt = vtable(texture);
        let get_desc: GetLevelDescFn = mem::transmute(*vt.add(IDIRECT3DTEXTURE9_GET_LEVEL_DESC));
        let mut desc = D3DSurfaceDesc::default();
        if get_desc(texture, 0, &mut desc) < 0 {
            return Tex::Original;
        }

        // Skip textures the game updates per frame (video, render targets):
        // hashing their transient contents can produce false matches.
        if desc.usage & (D3DUSAGE_DYNAMIC | D3DUSAGE_RENDERTARGET | D3DUSAGE_DEPTHSTENCIL) != 0 {
            return Tex::Original;
        }

        let started = Instant::now();
        let Some(hash) = hash_level_zero(texture, &desc) else {
            return Tex::Original;
        };
        if verbose() {
            log(&format!(
                "TEXTURE {}x{} fmt=0x{:08X} pool={} hash=0x{hash:08X} ({:.2} ms)",
                desc.width,
                desc.height,
                desc.format,
                desc.pool,
                millis(started)
            ));
        }
        if has_replacement(hash) {
            Tex::Replace {
                hash,
                format: desc.format,
            }
        } else {
            Tex::Original
        }
    }

    fn remember(texture: usize, state: Tex) {
        let mut textures = lock(&TEXTURES);
        if textures.len() < MAX_TRACKED_TEXTURES || textures.contains_key(&texture) {
            textures.insert(texture, state);
        }
    }

    // -----------------------------------------------------------------------
    // Texture method hooks
    // -----------------------------------------------------------------------

    fn originals(texture: *mut c_void) -> Option<VtableOriginals> {
        let key = unsafe { vtable(texture) } as usize;
        lock(&VTABLES).get(&key).copied()
    }

    /// Hook `Release` (any texture type) and, for 2D textures,
    /// `LockRect`/`UnlockRect`. Idempotent per vtable.
    unsafe fn ensure_texture_hooks(vt: *mut *mut c_void, is_2d: bool) {
        let key = vt as usize;
        let mut vtables = lock(&VTABLES);
        if vtables.contains_key(&key) {
            return;
        }
        let release_hook = hook_texture_release as *const () as *mut c_void;
        if *vt.add(IUNKNOWN_RELEASE) == release_hook {
            // Patched before but the entry is gone; nothing to forward to.
            return;
        }
        let mut entry = VtableOriginals {
            release: patch_slot(vt, IUNKNOWN_RELEASE, release_hook) as usize,
            lock: 0,
            unlock: 0,
        };
        if is_2d {
            entry.lock = patch_slot(
                vt,
                IDIRECT3DTEXTURE9_LOCK_RECT,
                hook_texture_lock as *const () as *mut c_void,
            ) as usize;
            entry.unlock = patch_slot(
                vt,
                IDIRECT3DTEXTURE9_UNLOCK_RECT,
                hook_texture_unlock as *const () as *mut c_void,
            ) as usize;
        }
        vtables.insert(key, entry);
        drop(vtables);
        log(&format!(
            "hooked texture methods (vtable {vt:p}, 2d={is_2d})"
        ));
    }

    unsafe extern "system" fn hook_texture_release(texture: *mut c_void) -> u32 {
        let Some(original) = originals(texture) else {
            // Only registered vtables are patched; leaking beats calling an
            // unknown address.
            return 0;
        };
        let release: ReleaseFn = mem::transmute(original.release);
        let remaining = release(texture);
        if remaining == 0 {
            let key = texture as usize;
            lock(&TEXTURES).remove(&key);
            lock(&WRITE_LOCKED).remove(&key);
            lock(&REWRITES).remove(&key);
        }
        remaining
    }

    unsafe extern "system" fn hook_texture_lock(
        texture: *mut c_void,
        level: u32,
        locked: *mut D3DLockedRect,
        rect: *const c_void,
        flags: u32,
    ) -> i32 {
        let Some(original) = originals(texture).filter(|o| o.lock != 0) else {
            return D3DERR_INVALIDCALL;
        };
        let lock_rect: LockRectFn = mem::transmute(original.lock);
        let hr = lock_rect(texture, level, locked, rect, flags);
        if hr >= 0 && level == 0 && flags & D3DLOCK_READONLY == 0 && !busy() {
            lock(&WRITE_LOCKED).insert(texture as usize);
        }
        hr
    }

    unsafe extern "system" fn hook_texture_unlock(texture: *mut c_void, level: u32) -> i32 {
        let Some(original) = originals(texture).filter(|o| o.unlock != 0) else {
            return D3DERR_INVALIDCALL;
        };
        let unlock_rect: UnlockRectFn = mem::transmute(original.unlock);
        let hr = unlock_rect(texture, level);
        if busy() {
            return hr;
        }
        let written = level == 0 && lock(&WRITE_LOCKED).remove(&(texture as usize));
        if written || READY_JOBS.load(Ordering::Acquire) > 0 {
            with_busy(|| texture_written(texture, written));
        }
        hr
    }

    /// The game finished writing a texture (usually while loading): hash it
    /// now, start decoding its replacement in the background, and create any
    /// replacements whose data is ready.
    unsafe fn texture_written(texture: *mut c_void, written: bool) {
        if written && !rewritten_too_often(texture as usize) {
            let mut state = hash_texture(texture);
            if let Tex::Replace { hash, format } = state {
                let existing = lock(&REPLACEMENTS).get(&(hash, format)).copied();
                match existing {
                    Some(0) => state = Tex::Original,
                    Some(pointer) => state = Tex::Ready(pointer),
                    None => {
                        lock(&PENDING).insert((hash, format));
                        LOADER.request(hash);
                    }
                }
            }
            remember(texture as usize, state);
        }
        if READY_JOBS.load(Ordering::Acquire) > 0 {
            if let Some(device) = texture_device(texture) {
                create_pending(device);
                release_object(device);
            }
        }
    }

    /// Count a rewrite; past the limit the texture is bound as is for good.
    fn rewritten_too_often(texture: usize) -> bool {
        let count = {
            let mut rewrites = lock(&REWRITES);
            let count = rewrites.entry(texture).or_insert(0);
            *count = count.saturating_add(1);
            *count
        };
        if count > MAX_REWRITES {
            if count == MAX_REWRITES + 1 {
                remember(texture, Tex::Original);
            }
            return true;
        }
        false
    }

    unsafe fn texture_device(texture: *mut c_void) -> Option<*mut c_void> {
        let get_device: GetDeviceFn =
            mem::transmute(*vtable(texture).add(IDIRECT3DRESOURCE9_GET_DEVICE));
        let mut device = std::ptr::null_mut();
        (get_device(texture, &mut device) >= 0 && !device.is_null()).then_some(device)
    }

    unsafe fn release_object(object: *mut c_void) {
        let release: ReleaseFn = mem::transmute(*vtable(object).add(IUNKNOWN_RELEASE));
        release(object);
    }

    /// Create every pending replacement whose data is already decoded.
    unsafe fn create_pending(device: *mut c_void) {
        let ready: Vec<ReplacementKey> = {
            let pending = lock(&PENDING);
            if pending.is_empty() {
                return;
            }
            pending
                .iter()
                .copied()
                .filter(|&(hash, _)| LOADER.is_ready(hash))
                .collect()
        };
        for (hash, format) in ready {
            lock(&PENDING).remove(&(hash, format));
            if let Some(job) = LOADER.try_take(hash) {
                create_and_record(device, hash, format, job, "load");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Background decoding
    // -----------------------------------------------------------------------

    enum Job {
        Queued,
        Running,
        Ready(Decoded),
        Failed,
    }

    struct Decoded {
        data: Vec<u8>,
        decode_ms: f64,
        background: bool,
    }

    struct LoaderState {
        jobs: FastMap<u32, Job>,
        queue: VecDeque<u32>,
        ready_bytes: usize,
        workers_started: bool,
    }

    /// Reads, decrypts and inflates pack entries on worker threads.
    struct Loader {
        state: Mutex<LoaderState>,
        changed: Condvar,
    }

    /// Number of decoded entries waiting; lets the hot paths skip the loader
    /// lock when there is nothing to collect.
    static READY_JOBS: AtomicUsize = AtomicUsize::new(0);

    static LOADER: LazyLock<Loader> = LazyLock::new(|| Loader {
        state: Mutex::new(LoaderState {
            jobs: FastMap::default(),
            queue: VecDeque::new(),
            ready_bytes: 0,
            workers_started: false,
        }),
        changed: Condvar::new(),
    });

    impl Loader {
        fn wait<'a>(&self, guard: MutexGuard<'a, LoaderState>) -> MutexGuard<'a, LoaderState> {
            self.changed
                .wait(guard)
                .unwrap_or_else(|error| error.into_inner())
        }

        /// Queue `hash` for background decoding (no-op if already known).
        fn request(&'static self, hash: u32) {
            let mut state = lock(&self.state);
            if state.jobs.contains_key(&hash) {
                return;
            }
            state.jobs.insert(hash, Job::Queued);
            state.queue.push_back(hash);
            if !state.workers_started {
                state.workers_started = true;
                let count = std::thread::available_parallelism()
                    .map_or(2, |n| n.get())
                    .saturating_sub(1)
                    .clamp(1, 3);
                for n in 0..count {
                    let _ = std::thread::Builder::new()
                        .name(format!("retex-decode-{n}"))
                        .spawn(move || self.work());
                }
                drop(state);
                log(&format!(
                    "decoding replacements on {count} background thread(s)"
                ));
            } else {
                drop(state);
            }
            self.changed.notify_all();
        }

        fn work(&self) {
            let mut state = lock(&self.state);
            loop {
                let next = if state.ready_bytes < MAX_READY_BYTES {
                    state.queue.pop_front()
                } else {
                    None
                };
                let Some(hash) = next else {
                    state = self.wait(state);
                    continue;
                };
                // A job taken over by a thread that needed it immediately is
                // no longer queued.
                if !matches!(state.jobs.get(&hash), Some(Job::Queued)) {
                    continue;
                }
                state.jobs.insert(hash, Job::Running);
                drop(state);

                let started = Instant::now();
                let result = decode(hash);
                let decoded = result.map(|data| Decoded {
                    data,
                    decode_ms: millis(started),
                    background: true,
                });

                state = lock(&self.state);
                match decoded {
                    Some(decoded) => {
                        state.ready_bytes += decoded.data.len();
                        state.jobs.insert(hash, Job::Ready(decoded));
                        READY_JOBS.fetch_add(1, Ordering::AcqRel);
                    }
                    None => {
                        state.jobs.insert(hash, Job::Failed);
                    }
                }
                self.changed.notify_all();
            }
        }

        fn is_ready(&self, hash: u32) -> bool {
            matches!(lock(&self.state).jobs.get(&hash), Some(Job::Ready(_)))
        }

        fn take_ready(state: &mut LoaderState, hash: u32) -> Option<Decoded> {
            match state.jobs.remove(&hash) {
                Some(Job::Ready(decoded)) => {
                    state.ready_bytes -= decoded.data.len();
                    READY_JOBS.fetch_sub(1, Ordering::AcqRel);
                    Some(decoded)
                }
                Some(other) => {
                    state.jobs.insert(hash, other);
                    None
                }
                None => None,
            }
        }

        /// Decoded data for `hash` if a worker has finished it.
        fn try_take(&self, hash: u32) -> Option<Decoded> {
            let decoded = Self::take_ready(&mut lock(&self.state), hash);
            if decoded.is_some() {
                self.changed.notify_all();
            }
            decoded
        }

        /// Decoded data for `hash`, now: takes it from a worker, waits for a
        /// worker already on it, or decodes it on this thread.
        fn take(&self, hash: u32) -> Option<Decoded> {
            let mut state = lock(&self.state);
            loop {
                match state.jobs.get(&hash) {
                    Some(Job::Ready(_)) => {
                        let decoded = Self::take_ready(&mut state, hash);
                        drop(state);
                        self.changed.notify_all();
                        return decoded;
                    }
                    Some(Job::Running) => state = self.wait(state),
                    Some(Job::Failed) => return None,
                    Some(Job::Queued) | None => {
                        state.jobs.insert(hash, Job::Running);
                        drop(state);
                        let started = Instant::now();
                        let data = decode(hash);
                        let mut state = lock(&self.state);
                        let result = match data {
                            Some(data) => {
                                state.jobs.remove(&hash);
                                Some(Decoded {
                                    data,
                                    decode_ms: millis(started),
                                    background: false,
                                })
                            }
                            None => {
                                state.jobs.insert(hash, Job::Failed);
                                None
                            }
                        };
                        drop(state);
                        self.changed.notify_all();
                        return result;
                    }
                }
            }
        }
    }

    /// Read, decrypt, inflate and verify the pack entry for `hash`.
    fn decode(hash: u32) -> Option<Vec<u8>> {
        let packs = PACKS.get()?;
        let &(pack_index, entry_index) = packs.by_hash.get(&hash)?;
        let index = &packs.indexes[pack_index];
        match index.read_entry(&index.entries[entry_index]) {
            Ok(data) => Some(data),
            Err(error) => {
                log(&format!("replacement 0x{hash:08X}: {error}"));
                None
            }
        }
    }

    // -----------------------------------------------------------------------
    // Replacement textures
    // -----------------------------------------------------------------------

    /// Minimal 32-bit BMP decoder; returns tightly packed top-down BGRA.
    fn decode_bmp(data: &[u8]) -> Option<Replacement> {
        if data.len() < 54 || &data[..2] != b"BM" {
            return None;
        }
        let offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
        let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
        let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
        let bpp = u16::from_le_bytes([data[28], data[29]]);
        if bpp != 32 || width <= 0 || height == 0 {
            return None;
        }
        let w = width as usize;
        let h = height.unsigned_abs() as usize;
        let stride = w * 4;
        let end = offset.checked_add(stride.checked_mul(h)?)?;
        if end > data.len() {
            return None;
        }
        let source = &data[offset..end];
        let mut pixels = vec![0u8; stride * h];
        if height > 0 {
            // BMP rows are bottom-up.
            for row in 0..h {
                let src = (h - 1 - row) * stride;
                pixels[row * stride..(row + 1) * stride]
                    .copy_from_slice(&source[src..src + stride]);
            }
        } else {
            pixels.copy_from_slice(source);
        }
        Some(Replacement {
            width: w as u32,
            height: h as u32,
            pixels,
        })
    }

    /// Create a texture from BMP bytes with D3DX, like TexMod's package DLL.
    unsafe fn d3dx_texture(device: *mut c_void, data: &[u8]) -> Option<*mut c_void> {
        type D3dxCreateFn =
            unsafe extern "system" fn(*mut c_void, *const c_void, u32, *mut *mut c_void) -> i32;

        // Resolved once; 0 when D3DX is not installed.
        static D3DX_CREATE: OnceLock<usize> = OnceLock::new();
        let function = *D3DX_CREATE.get_or_init(|| {
            let module = LoadLibraryA(b"d3dx9_43.dll\0".as_ptr());
            if module.is_null() {
                log("d3dx9_43.dll not available; using the built-in BMP decoder");
                return 0;
            }
            GetProcAddress(module, b"D3DXCreateTextureFromFileInMemory\0".as_ptr()) as usize
        });
        if function == 0 {
            return None;
        }
        let create: D3dxCreateFn = mem::transmute(function);
        let mut texture: *mut c_void = std::ptr::null_mut();
        let hr = create(
            device,
            data.as_ptr() as *const c_void,
            data.len() as u32,
            &mut texture,
        );
        if hr >= 0 && !texture.is_null() {
            Some(texture)
        } else {
            None
        }
    }

    unsafe fn upload_texture(
        device: *mut c_void,
        hash: u32,
        replacement: &Replacement,
    ) -> Option<*mut c_void> {
        let device_vt = vtable(device);
        let create: CreateTextureFn =
            mem::transmute(*device_vt.add(IDIRECT3DDEVICE9_CREATE_TEXTURE));
        let mut texture: *mut c_void = std::ptr::null_mut();
        let hr = create(
            device,
            replacement.width,
            replacement.height,
            1,
            0,
            D3DFMT_A8R8G8B8,
            D3DPOOL_MANAGED,
            &mut texture,
            std::ptr::null_mut(),
        );
        if hr < 0 || texture.is_null() {
            log(&format!(
                "replacement 0x{hash:08X}: CreateTexture failed 0x{hr:08X}"
            ));
            return None;
        }

        let texture_vt = vtable(texture);
        let lock: LockRectFn = mem::transmute(*texture_vt.add(IDIRECT3DTEXTURE9_LOCK_RECT));
        let unlock: UnlockRectFn = mem::transmute(*texture_vt.add(IDIRECT3DTEXTURE9_UNLOCK_RECT));
        let mut locked = D3DLockedRect::default();
        if lock(texture, 0, &mut locked, std::ptr::null(), 0) < 0 || locked.bits.is_null() {
            log(&format!("replacement 0x{hash:08X}: LockRect failed"));
            release_object(texture);
            return None;
        }

        let stride = replacement.width as usize * 4;
        for row in 0..replacement.height as usize {
            let destination = (locked.bits as *mut u8).add(row * locked.pitch as usize);
            let source = replacement.pixels[row * stride..].as_ptr();
            std::ptr::copy_nonoverlapping(source, destination, stride);
        }
        unlock(texture, 0);
        Some(texture)
    }

    /// Turn decoded pack data into a D3D texture: D3DX first (the same path
    /// TexMod uses: BMP -> texture with mips), the built-in decoder as a
    /// fallback.
    unsafe fn create_texture(
        device: *mut c_void,
        hash: u32,
        format: u32,
        data: &[u8],
    ) -> Option<(*mut c_void, &'static str)> {
        if let Some(texture) = d3dx_texture(device, data) {
            return Some((texture, "d3dx"));
        }
        let mut replacement = decode_bmp(data)?;
        if !format_has_alpha(format) {
            // The source format has no alpha; ignore whatever the BMP carries.
            for pixel in replacement.pixels.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
        }
        upload_texture(device, hash, &replacement).map(|texture| (texture, "bmp"))
    }

    /// Create the replacement from decoded data and record the result.
    unsafe fn create_and_record(
        device: *mut c_void,
        hash: u32,
        format: u32,
        decoded: Decoded,
        context: &str,
    ) -> Option<*mut c_void> {
        let started = Instant::now();
        let created = create_texture(device, hash, format, &decoded.data);
        let create_ms = millis(started);
        let texture = match created {
            Some((texture, method)) => {
                log(&format!(
                    "REPLACE 0x{hash:08X} ({method}): decoded in {:.0} ms {}, created in {create_ms:.0} ms at {context}",
                    decoded.decode_ms,
                    if decoded.background { "in the background" } else { "on the game thread" },
                ));
                Some(texture)
            }
            None => {
                log(&format!(
                    "replacement 0x{hash:08X} unavailable; keeping the original"
                ));
                None
            }
        };
        lock(&REPLACEMENTS).insert((hash, format), texture.map_or(0, |t| t as usize));
        texture
    }

    /// Replacement texture for `(hash, format)`, created now if needed.
    unsafe fn replacement(device: *mut c_void, hash: u32, format: u32) -> Option<*mut c_void> {
        if let Some(&pointer) = lock(&REPLACEMENTS).get(&(hash, format)) {
            return (pointer != 0).then_some(pointer as *mut c_void);
        }
        lock(&PENDING).remove(&(hash, format));
        match LOADER.take(hash) {
            Some(decoded) => create_and_record(device, hash, format, decoded, "first draw"),
            None => {
                lock(&REPLACEMENTS).insert((hash, format), 0);
                None
            }
        }
    }

    // -----------------------------------------------------------------------
    // Device hooks
    // -----------------------------------------------------------------------

    /// What to bind for a game texture: one lookup once it is known.
    unsafe fn bound_texture(device: *mut c_void, texture: *mut c_void) -> *mut c_void {
        let key = texture as usize;
        let known = lock(&TEXTURES).get(&key).copied();
        let state = match known {
            Some(state) => state,
            None => with_busy(|| {
                let vt = vtable(texture);
                let get_type: GetTypeFn = mem::transmute(*vt.add(IDIRECT3DBASETEXTURE9_GET_TYPE));
                let is_2d = get_type(texture) == D3DRTYPE_TEXTURE;
                ensure_texture_hooks(vt, is_2d);
                // Cube/volume textures are not replaceable (the legacy hash
                // only covers one 2D surface).
                let state = if is_2d {
                    hash_texture(texture)
                } else {
                    Tex::Original
                };
                remember(key, state);
                state
            }),
        };
        match state {
            Tex::Original => texture,
            Tex::Ready(pointer) => pointer as *mut c_void,
            Tex::Replace { hash, format } => {
                let created = with_busy(|| replacement(device, hash, format));
                remember(
                    key,
                    created.map_or(Tex::Original, |t| Tex::Ready(t as usize)),
                );
                created.unwrap_or(texture)
            }
        }
    }

    unsafe extern "system" fn hook_set_texture(
        device: *mut c_void,
        stage: u32,
        texture: *mut c_void,
    ) -> i32 {
        let target = if texture.is_null() {
            texture
        } else {
            bound_texture(device, texture)
        };
        let original: SetTextureFn = mem::transmute(ORIG_SET_TEXTURE.load(Ordering::Acquire));
        original(device, stage, target)
    }

    #[allow(clippy::too_many_arguments)]
    unsafe extern "system" fn hook_create_texture(
        device: *mut c_void,
        width: u32,
        height: u32,
        levels: u32,
        usage: u32,
        format: u32,
        pool: u32,
        out: *mut *mut c_void,
        shared: *mut c_void,
    ) -> i32 {
        let original: CreateTextureFn = mem::transmute(ORIG_CREATE_TEXTURE.load(Ordering::Acquire));
        let hr = original(
            device, width, height, levels, usage, format, pool, out, shared,
        );
        // Hook the texture methods before the game fills its first texture,
        // so it can be hashed while the game is still loading.
        if hr >= 0 && !out.is_null() && !(*out).is_null() {
            ensure_texture_hooks(vtable(*out), true);
        }
        hr
    }

    unsafe extern "system" fn hook_create_device(
        d3d9: *mut c_void,
        adapter: u32,
        device_type: u32,
        focus_window: *mut c_void,
        behavior: u32,
        present: *mut c_void,
        out: *mut *mut c_void,
    ) -> i32 {
        let original: CreateDeviceFn = mem::transmute(ORIG_CREATE_DEVICE.load(Ordering::Acquire));
        let hr = original(
            d3d9,
            adapter,
            device_type,
            focus_window,
            behavior,
            present,
            out,
        );

        if hr >= 0 && !out.is_null() && !(*out).is_null() {
            let device = *out;
            // Replacements belong to the device that created them; a new
            // device starts fresh. They are dropped, not released: their
            // device may already be gone.
            lock(&REPLACEMENTS).clear();
            lock(&PENDING).clear();
            lock(&TEXTURES).retain(|_, state| !matches!(state, Tex::Ready(_)));
            hook_slot(
                vtable(device),
                IDIRECT3DDEVICE9_SET_TEXTURE,
                hook_set_texture as *const () as *mut c_void,
                &ORIG_SET_TEXTURE,
                "IDirect3DDevice9::SetTexture",
            );
            hook_slot(
                vtable(device),
                IDIRECT3DDEVICE9_CREATE_TEXTURE,
                hook_create_texture as *const () as *mut c_void,
                &ORIG_CREATE_TEXTURE,
                "IDirect3DDevice9::CreateTexture",
            );
        }
        hr
    }

    unsafe extern "system" fn hook_direct3d_create9(sdk_version: u32) -> *mut c_void {
        let original: Direct3DCreate9Fn = mem::transmute(ORIG_D3D9.load(Ordering::Acquire));
        let d3d9 = original(sdk_version);

        if !d3d9.is_null() {
            hook_slot(
                vtable(d3d9),
                IDIRECT3D9_CREATE_DEVICE,
                hook_create_device as *const () as *mut c_void,
                &ORIG_CREATE_DEVICE,
                "IDirect3D9::CreateDevice",
            );
        }
        d3d9
    }

    // -----------------------------------------------------------------------
    // Startup
    // -----------------------------------------------------------------------

    /// Full path of the host executable (Unicode, any length).
    fn exe_path() -> Option<PathBuf> {
        let mut buffer = vec![0u16; 260];
        loop {
            let size = buffer.len() as u32;
            let len =
                unsafe { GetModuleFileNameW(std::ptr::null_mut(), buffer.as_mut_ptr(), size) };
            if len == 0 {
                return None;
            }
            // A full buffer means the path was truncated.
            if len < size {
                return Some(OsString::from_wide(&buffer[..len as usize]).into());
            }
            if buffer.len() >= 32_768 {
                return None;
            }
            buffer.resize(buffer.len() * 2, 0);
        }
    }

    fn config_paths() -> Vec<String> {
        let mut packs = Vec::new();
        let config: Option<PathBuf> = std::env::var("RETEX_CONFIG")
            .ok()
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .or_else(|| Some(exe_path()?.parent()?.join("retex.ini")));

        if let Some(path) = config {
            if let Ok(text) = std::fs::read_to_string(&path) {
                for line in text.lines() {
                    let line = line.trim().trim_start_matches('\u{feff}');
                    if let Some(value) = line.strip_prefix("pack=") {
                        let value = value.trim().trim_matches('"');
                        if !value.is_empty() {
                            packs.push(value.to_string());
                        }
                    }
                }
            }
        }

        if packs.is_empty() {
            if let Ok(value) = std::env::var("RETEX_PACK") {
                if !value.is_empty() {
                    packs.push(value);
                }
            }
        }
        packs
    }

    fn load_packs() {
        let mut packs = Packs {
            indexes: Vec::new(),
            by_hash: FastMap::default(),
        };
        for path in &config_paths() {
            match PackageIndex::open(path) {
                Ok(index) => {
                    let pack_index = packs.indexes.len();
                    let count = index.entries.len();
                    for (entry_index, entry) in index.entries.iter().enumerate() {
                        // Packs apply top to bottom: the first one wins.
                        if entry.hash != 0 {
                            packs
                                .by_hash
                                .entry(entry.hash)
                                .or_insert((pack_index, entry_index));
                        }
                    }
                    log(&format!("index loaded: {count} entries from {path}"));
                    packs.indexes.push(Arc::new(index));
                }
                Err(error) => log(&format!("index failed ({path}): {error}")),
            }
        }
        let _ = PACKS.set(packs);
    }

    fn has_replacement(hash: u32) -> bool {
        PACKS
            .get()
            .is_some_and(|packs| packs.by_hash.contains_key(&hash))
    }

    /// True if another copy of this hook (an ASI plugin plus an injected
    /// DLL, say) is already active in this process.
    unsafe fn another_copy_active() -> bool {
        let name: Vec<u16> = format!("Local\\retex-hook-{}", std::process::id())
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // Deliberately never closed: it marks the process for its lifetime.
        let handle = CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
        !handle.is_null() && GetLastError() == ERROR_ALREADY_EXISTS
    }

    unsafe fn install() {
        if another_copy_active() {
            log("another Retex hook is already active in this process; this copy stays idle");
            return;
        }
        let debug =
            std::env::var_os("RETEX_DEBUG").is_some_and(|value| !value.is_empty() && value != "0");
        VERBOSE.store(debug, Ordering::Relaxed);
        log(&format!(
            "initializing (pid={}, verbose={debug})",
            std::process::id()
        ));
        load_packs();
        let module = LoadLibraryA(b"d3d9.dll\0".as_ptr());
        if module.is_null() {
            log("failed to load d3d9.dll");
            return;
        }
        let target = GetProcAddress(module, b"Direct3DCreate9\0".as_ptr());
        if target.is_null() {
            log("Direct3DCreate9 not found");
            return;
        }

        match MinHook::create_hook(target, hook_direct3d_create9 as *const () as *mut c_void) {
            Ok(original) => {
                if MinHook::enable_all_hooks().is_ok() {
                    ORIG_D3D9.store(original, Ordering::Release);
                    log("detour installed on Direct3DCreate9");
                } else {
                    log("failed to enable detour");
                }
            }
            Err(status) => log(&format!("create_hook failed: {status:?}")),
        }
    }

    pub fn init() {
        INIT.call_once(|| unsafe { install() });
    }
}

#[cfg(all(windows, target_arch = "x86"))]
#[no_mangle]
pub extern "system" fn Initialize() -> i32 {
    imp::init();
    1
}

/// Ultimate ASI Loader looks for this export.
#[cfg(all(windows, target_arch = "x86"))]
#[no_mangle]
pub extern "system" fn InitializeASI() -> i32 {
    imp::init();
    1
}

#[cfg(all(windows, target_arch = "x86"))]
#[no_mangle]
pub extern "system" fn DllMain(
    _module: *mut std::ffi::c_void,
    _reason: u32,
    _reserved: *mut std::ffi::c_void,
) -> i32 {
    1
}

/// Stub for non-x86 builds so the workspace still compiles.
#[cfg(not(all(windows, target_arch = "x86")))]
#[no_mangle]
pub extern "system" fn Initialize() -> i32 {
    0
}
