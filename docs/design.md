# Design notes

## Goals

- Drop-in replacement for TexMod 0.9b on Windows 10/11.
- Read and write legacy `.tpf` exactly (see `docs/texmod-format.md`).
- No packing, no obfuscation, signed binaries, no admin by default.
- Scriptable (CLI), seamless to launch, and safe to run alongside ReShade.

## Architecture

```
retex-core      Rust lib: .tpf v1, ZipCrypto, hash, bpp table (done)
retex-cli       Rust bin: list/verify/extract/build/hash/install/launch (done)
retex-hook      cdylib: D3D9 interception + package application (done)
retex-inject    32-bit bin: suspended start + DLL injection (done)
retex-app       egui desktop app over the core/CLI (done)
```

The core is platform-neutral; only `retex-hook`, `retex-inject` and the
shortcut code are Windows-specific.

## Delivery modes (in order of preference)

1. **ASI plugin** — `retex.asi` loaded by Ultimate ASI Loader. Works in the
   user's existing NFSU1/NFSU2 installs (they already ship `dinput8.dll`),
   avoids `d3d9.dll` conflicts with ReShade, no injection, no admin.
2. **Injector** — `retex-inject` creates the game suspended, injects
   `retex_hook.dll`, calls its `Initialize` export, resumes. For games without
   an ASI loader. The hook still reads `retex.ini` next to the game exe.
3. **d3d9 proxy** — last resort; conflicts with ReShade and other proxies.

`retex shortcut --inject` points the shortcut at `retex launch <game>`, so
double-clicking it is a one-click modded launch without an ASI loader.

## Hook plan (M2/M3)

- MinHook detour on `Direct3DCreate9`; then patch device vtable slots:
  - `IDirect3D9::CreateDevice` index **16** → wrap the device.
  - `IDirect3DDevice9::SetTexture` index **65** → hash and swap in a
    replacement.
- Hash with the legacy algorithm (pitch bug included) so existing packs match.
- Replacement textures are created with `D3DXCreateTextureFromFileInMemory`
  (same loader TexMod uses: mips, format and alpha); manual upload fallback.
- Skip `DYNAMIC`, `RENDERTARGET` and `DEPTHSTENCIL` textures so transient
  video/frame contents cannot produce false hash matches.
- Handle `D3DPOOL_DEFAULT` via a system-memory copy before hashing where the
  driver refuses to lock.

## Profiles and shortcuts (M4)

Planned profile (`%APPDATA%\Retex\games\<id>.json`):

```json
{
  "id": "nfsu1",
  "name": "Need for Speed: Underground",
  "exe": "D:\\Games\\...\\Speed.exe",
  "packs": ["redux1.0.8.tpf"],
  "launch": { "args": [], "working_dir": "D:\\Games\\..." },
  "hook": { "mode": "asi", "hotkeys": { "toggle_overlay": "F8" } }
}
```

Implemented today:

- `retex install <game-dir> --pack <tpf>` — copies the hook as `retex.asi`
  and writes `retex.ini` (ASI mode).
- `retex launch <game.exe> [--pack <tpf>]` — writes `retex.ini` when packs
  are given, then runs the 32-bit injector (injector mode).
- `retex shortcut <game.exe> [--inject] [--pack <tpf>]` — writes a `.lnk`
  via `IShellLinkW` + `IPersistFile`; with `--inject` the target is
  `retex launch <game>`, using the game's icon. No admin, no UAC.

## Roadmap

| Milestone | Content |
|---|---|
| M1 done | core + CLI + legacy compatibility + verification |
| M2 done | hook skeleton, logging mode (hash + dump) validated on live D3D9 textures |
| M3 done | pack loading and `SetTexture` replacement; D3DX texture creation |
| M4 done | ASI install, injector (`retex-inject` + `retex launch`), `.lnk` shortcuts |
| M5 done | egui desktop app: game setup, pack ordering, inspector/verify, launch/install/shortcut |

## Validation

M2/M3/M4 were validated in the retail game:

- Game: Need for Speed: Underground (SPEED.EXE, D3D9), Ultimate ASI Loader
  (`dinput8.dll`), ReShade 6.8 as `d3d9.dll`.
- Pack: `redux1.0.8.tpf` (334 entries, 345 MB), loaded lazily by index.
- ASI mode: device hooked, 15 textures hashed, 7 replaced in 25 seconds,
  including `0xA3891C0F` (1024x512 DXT1 -> 2048x1024 BMP).
- Injector mode: `retex-inject` started `Speed.exe` suspended, injected the
  hook, called `Initialize`, resumed; the log shows `initializing (pid=...)`
  and the same replacements.
- The attract/legal screen displays the **Need for Speed Underground REDUX**
  logo from the pack (verified by screenshot) in both modes.
- No crash; the game continued running.
- Desktop app: loaded the 334-entry Redux pack lazily, showed 839.7 MB of
  texture data, verified it in a background thread, and launched the game via
  the same CLI engine.

DEFAULT-pool textures were lockable on this driver; the `GetRenderTargetData`
copy path is still needed for drivers that refuse.

## Risks

- **ReShade/other hooks**: ASI mode avoids the `d3d9.dll` proxy fight; inline
  hooks must chain cleanly.
- **Anti-cheat**: never inject into online games by default; warn and allow a
  blocklist.
- **Packaged apps**: Windows 11 packaged processes can reject remote threads
  (`ERROR_ACCESS_DENIED`); use ASI mode for those, injector mode for classic
  Win32 games.
- **32-bit games**: `retex-hook` and `retex-inject` must be built for
  `i686-pc-windows-msvc`; the CLI/GUI stay 64-bit.
- **Hash compatibility**: never "fix" the pitch bug in legacy mode.
- **Memory**: stream/lazy-load packs instead of mirroring TexMod's full load.
