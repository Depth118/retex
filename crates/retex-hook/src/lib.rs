//! Retex D3D9 hook (32-bit).
//!
//! M2: inline detour on `Direct3DCreate9`, vtable hooks on `CreateDevice` and
//! `SetTexture`, legacy texture hash computed on first bind.
//!
//! M3: loads a `.tpf` pack (`RETEX_PACK` environment variable) and replaces
//! bound textures whose hash has a matching entry.
//!
//! Load as an ASI plugin (export `Initialize`) or inject with
//! `retex-inject`.

#![cfg_attr(not(all(windows, target_arch = "x86")), allow(unused))]

#[cfg(all(windows, target_arch = "x86"))]
mod imp {
    #![allow(clippy::manual_c_str_literals)]
    use minhook::MinHook;
    use retex_core::{bytes_per_pixel, texmod_crc, PackageIndex};
    use std::collections::HashMap;
    use std::ffi::{c_void, CString, OsString};
    use std::io::Write;
    use std::mem;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::sync::{LazyLock, Mutex, Once, OnceLock};

    const PAGE_EXECUTE_READWRITE: u32 = 0x40;

    const IUNKNOWN_RELEASE: usize = 2;
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

    static ORIG_D3D9: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static ORIG_CREATE_DEVICE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static ORIG_SET_TEXTURE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static ORIG_TEXTURE_RELEASE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    /// Texture pointer -> `(hash, format)` when the pack replaces it, `None`
    /// when it never will. Entries are evicted when the texture is released,
    /// so a recycled pointer never inherits a stale hash.
    type TextureInfo = Option<(u32, u32)>;
    static HASH_CACHE: LazyLock<Mutex<HashMap<usize, TextureInfo>>> =
        LazyLock::new(Default::default);

    /// `(device, hash, format)` -> replacement texture, or 0 if creating it
    /// failed (so a broken entry is not retried on every bind).
    type ReplacementKey = (usize, u32, u32);
    static REPLACEMENT_CACHE: LazyLock<Mutex<HashMap<ReplacementKey, usize>>> =
        LazyLock::new(Default::default);
    struct Packs {
        indexes: Vec<PackageIndex>,
        by_hash: HashMap<u32, (usize, usize)>,
    }

    static PACKS: Mutex<Option<Packs>> = Mutex::new(None);
    static INIT: Once = Once::new();

    extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
        fn VirtualProtect(addr: *mut c_void, size: usize, new_protect: u32, old: *mut u32) -> i32;
        fn OutputDebugStringA(text: *const u8);
        fn GetModuleFileNameW(module: *mut c_void, buffer: *mut u16, size: u32) -> u32;
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

    /// Hash a texture; `Some` only if the pack replaces it.
    unsafe fn texture_info(texture: *mut c_void) -> Option<(u32, u32)> {
        let vt = vtable(texture);
        let get_desc: GetLevelDescFn = mem::transmute(*vt.add(IDIRECT3DTEXTURE9_GET_LEVEL_DESC));
        let mut desc = D3DSurfaceDesc::default();
        if get_desc(texture, 0, &mut desc) < 0 {
            return None;
        }

        // Skip textures the game updates per frame (video, render targets):
        // hashing their transient contents can produce false matches.
        if desc.usage & (D3DUSAGE_DYNAMIC | D3DUSAGE_RENDERTARGET | D3DUSAGE_DEPTHSTENCIL) != 0 {
            return None;
        }

        let hash = hash_level_zero(texture, &desc)?;
        log(&format!(
            "TEXTURE {}x{} fmt=0x{:08X} pool={} hash=0x{hash:08X}",
            desc.width, desc.height, desc.format, desc.pool
        ));
        has_replacement(hash).then_some((hash, desc.format))
    }

    /// `(hash, format)` of a texture the pack replaces, hashing it only on
    /// its first bind.
    unsafe fn texture_info_cached(texture: *mut c_void) -> Option<(u32, u32)> {
        let key = texture as usize;
        if let Some(cached) = HASH_CACHE.lock().ok()?.get(&key) {
            return *cached;
        }

        let vt = vtable(texture);
        let get_type: GetTypeFn = mem::transmute(*vt.add(IDIRECT3DBASETEXTURE9_GET_TYPE));
        if get_type(texture) != D3DRTYPE_TEXTURE {
            return None;
        }
        let info = texture_info(texture);

        // Only cache what we will hear about being released; otherwise a new
        // texture allocated at the same address would inherit this result.
        let tracked = hook_slot(
            vt,
            IUNKNOWN_RELEASE,
            hook_texture_release as *const () as *mut c_void,
            &ORIG_TEXTURE_RELEASE,
            "IDirect3DTexture9::Release",
        );
        if tracked {
            if let Ok(mut cache) = HASH_CACHE.lock() {
                if cache.len() < MAX_TRACKED_TEXTURES {
                    cache.insert(key, info);
                }
            }
        }
        info
    }

    unsafe extern "system" fn hook_texture_release(texture: *mut c_void) -> u32 {
        let original: ReleaseFn = mem::transmute(ORIG_TEXTURE_RELEASE.load(Ordering::Acquire));
        let remaining = original(texture);
        if remaining == 0 {
            if let Ok(mut cache) = HASH_CACHE.lock() {
                cache.remove(&(texture as usize));
            }
        }
        remaining
    }

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
        let paths = config_paths();
        if paths.is_empty() {
            return;
        }
        let mut packs = Packs {
            indexes: Vec::new(),
            by_hash: HashMap::new(),
        };
        for path in &paths {
            match PackageIndex::open(path) {
                Ok(index) => {
                    let pack_index = packs.indexes.len();
                    let count = index.entries.len();
                    for (entry_index, entry) in index.entries.iter().enumerate() {
                        if entry.hash != 0 {
                            packs
                                .by_hash
                                .entry(entry.hash)
                                .or_insert((pack_index, entry_index));
                        }
                    }
                    log(&format!("index loaded: {count} entries from {path}"));
                    packs.indexes.push(index);
                }
                Err(error) => log(&format!("index failed ({path}): {error}")),
            }
        }
        if let Ok(mut slot) = PACKS.lock() {
            *slot = Some(packs);
        }
    }

    fn has_replacement(hash: u32) -> bool {
        PACKS
            .lock()
            .ok()
            .and_then(|guard| Some(guard.as_ref()?.by_hash.contains_key(&hash)))
            .unwrap_or(false)
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

    unsafe fn create_replacement(
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
                "replacement {:#010X}: CreateTexture failed 0x{hr:08X}",
                hash
            ));
            return None;
        }

        let texture_vt = vtable(texture);
        let lock: LockRectFn = mem::transmute(*texture_vt.add(IDIRECT3DTEXTURE9_LOCK_RECT));
        let unlock: UnlockRectFn = mem::transmute(*texture_vt.add(IDIRECT3DTEXTURE9_UNLOCK_RECT));
        let mut locked = D3DLockedRect::default();
        if lock(texture, 0, &mut locked, std::ptr::null(), 0) < 0 || locked.bits.is_null() {
            log(&format!("replacement {hash:#010X}: LockRect failed"));
            return None;
        }

        let stride = replacement.width as usize * 4;
        for row in 0..replacement.height as usize {
            let destination = (locked.bits as *mut u8).add(row * locked.pitch as usize);
            let source = replacement.pixels[row * stride..].as_ptr();
            std::ptr::copy_nonoverlapping(source, destination, stride);
        }
        unlock(texture, 0);

        log(&format!(
            "REPLACE hash=0x{hash:08X} -> {}x{}",
            replacement.width, replacement.height
        ));
        Some(texture)
    }

    /// Replacement texture for `hash` on `device`, created on first use.
    unsafe fn replacement_for(device: *mut c_void, hash: u32, format: u32) -> Option<*mut c_void> {
        let key = (device as usize, hash, format);
        if let Some(&pointer) = REPLACEMENT_CACHE.lock().ok()?.get(&key) {
            return (pointer != 0).then_some(pointer as *mut c_void);
        }

        let texture = load_replacement(device, hash, format);
        if texture.is_none() {
            log(&format!(
                "replacement 0x{hash:08X} unavailable; keeping the original"
            ));
        }
        if let Ok(mut cache) = REPLACEMENT_CACHE.lock() {
            cache.insert(key, texture.map_or(0, |t| t as usize));
        }
        texture
    }

    unsafe fn load_replacement(device: *mut c_void, hash: u32, format: u32) -> Option<*mut c_void> {
        let data = {
            let guard = PACKS.lock().ok()?;
            let packs = guard.as_ref()?;
            let (pack_index, entry_index) = *packs.by_hash.get(&hash)?;
            let index = &packs.indexes[pack_index];
            match index.read_entry(&index.entries[entry_index]) {
                Ok(data) => data,
                Err(error) => {
                    log(&format!("replacement 0x{hash:08X}: {error}"));
                    return None;
                }
            }
        };

        // Prefer D3DX (the same path TexMod uses: BMP -> texture with mips).
        if let Some(texture) = d3dx_texture(device, &data) {
            log(&format!("REPLACE hash=0x{hash:08X} (d3dx)"));
            return Some(texture);
        }

        let mut replacement = decode_bmp(&data)?;
        if !format_has_alpha(format) {
            // The source format has no alpha; ignore whatever the BMP carries.
            for pixel in replacement.pixels.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
        }
        create_replacement(device, hash, &replacement)
    }

    unsafe extern "system" fn hook_set_texture(
        device: *mut c_void,
        stage: u32,
        texture: *mut c_void,
    ) -> i32 {
        let mut target = texture;
        if !texture.is_null() {
            if let Some((hash, format)) = texture_info_cached(texture) {
                if let Some(replacement) = replacement_for(device, hash, format) {
                    target = replacement;
                }
            }
        }
        let original: SetTextureFn = mem::transmute(ORIG_SET_TEXTURE.load(Ordering::Acquire));
        original(device, stage, target)
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
            // A new device may reuse a destroyed one's address; its cached
            // replacements belonged to the old device. They are dropped, not
            // released: their device is gone.
            if let Ok(mut cache) = REPLACEMENT_CACHE.lock() {
                cache.retain(|&(owner, _, _), _| owner != device as usize);
            }
            hook_slot(
                vtable(device),
                IDIRECT3DDEVICE9_SET_TEXTURE,
                hook_set_texture as *const () as *mut c_void,
                &ORIG_SET_TEXTURE,
                "IDirect3DDevice9::SetTexture",
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

    unsafe fn install() {
        log(&format!("initializing (pid={})", std::process::id()));
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
