# TexMod 0.9b â€” format and implementation notes

Reverse-engineered from the shipped binary (`Texmod.exe`, SHA-256
`f662be61eee7c3d2849e1c734b3b83e9e25fa4e873c7352852130f0c0ceb98af`,
2006-04-21) and a real third-party package (`redux1.0.8.tpf`, 345 MB).
Everything below was observed or verified against those artifacts. No TexMod
source was available; this is a clean-room reconstruction.

---

## 1. Binary layers

```
Texmod.exe (597,287 B)
â””â”€ NsPack ("packed by nspack") stub
   â””â”€ MSVC C++ app, PDB path: h:\- coding -\TexMod\TMLoad\Bin\TMLoad.pdb
      â”œâ”€ payload A  XOR 0x91  â†’ package-mode DLL   (240,887 B, NsPack-packed)
      â””â”€ payload B  XOR 0xC8  â†’ logging-mode DLL   (238,144 B, NsPack-packed)
```

| Payload | Image VA | Size | XOR key | PDB |
|---|---|---|---|---|
| A | `0x430050` | `0x3ACF7` | `0x91` | â€” |
| B | `0x46AD48` | `0x3A240` | `0xC8` | `TexMod_Log.pdb` |

Both DLLs are PE32, preferred base `0x10000000`, and are themselves NsPack
packed (kernel32-only import stubs). They statically link ZipArchive
(Tadeusz Dracz), zlib 1.2.3, libjpeg 6b, libpng, and D3DX; both call
`Direct3DCreate9` and `VirtualProtect`.

Microsoft Defender classifies the unpacked DLLs as
`Trojan:Win32/Sabsik.EN.A!ml` (ML heuristic) â€” expected for a process-injecting
D3D hook.

---

## 2. `.tpf` container

A package is a ZIP archive with two layers of protection:

1. **XOR mask** â€” every 4 bytes XORed with the dword `0x3FA43FA4`
   (byte pattern `A4 3F A4 3F`). Discovered in `TexMod.exe:0x404ED0`.
2. **ZipCrypto** â€” traditional PKWARE encryption, deflate compression,
   with a fixed 42-byte binary password:

```
73 2A 63 7D 5F 0A A6 BD 7D 65 7E 67 61 2A 7F 7F
74 61 67 5B 60 70 45 74 5C 22 74 5D 6E 6A 73 41
77 6E 46 47 77 49 0C 4B 46 6F
```

Entry naming:

```
<TARGET.EXE>_0x<8 hex checksum>.<ext>     e.g. TRUEEXE.EXE_0xA3891C0F.bmp
```

- The archive comment is the package comment (shown in Package Mode).
- `texmod.def` is an optional entry containing one `0xHASH|filename` line per
  texture, in the same format the Build mode accepts.
- Observed flags: `0x0009` (encrypted + data descriptor). The local header
  still carries the real CRC and sizes.

Sample pack (`redux1.0.8.tpf`): 333 BMPs + `texmod.def`, 880 MB
uncompressed, comment `AMTxDRxRL â€¦ NFS Underground Redux v1.08 â€¦ (c) 2017`.

---

## 3. Texture checksum

TexMod uses **CRC-32 with polynomial `0xEDB88320`, init `0xFFFFFFFF`, and no
final XOR** (i.e. the complement of standard CRC-32). Confirmed in the logging
DLL:

- table builder: `0x10008C00`
- CRC routine: `0x10008CE0`
- call site: `0x10008DF9`

**Verified by execution:** loading the logging DLL and calling its CRC routine on
a 1 KiB test vector (`bytes(i % 256)`) returns `0x48F4B3D9`, identical to
`texmod_crc()` in `research/reference/tpf.py` (and in `crates/retex-core`). The bpp function at `0x10008A50` was likewise
called for every format value and its results are the table below.

**Verified on live D3D9 textures:** `research/reference/native/hash_probe.cpp` creates real
textures and calls TexMod's `GetTextureHash` at `+0x8D50`:

| Case | TexMod | `texture_hash()` | Hashed |
|---|---|---|---|
| A8R8G8B8 8Ã—8 | `0x06A47B50` | `0x06A47B50` | 256 / 256 B |
| DXT1 8Ã—8 | `0x3568890B` | `0x3568890B` | 32 / 128 B |
| R8G8B8 5Ã—5 (pitch 16) | `0x7BA00B15` | `0x7BA00B15` | 75 / 80 B |

The last row proves the pitch bug: 75 = 5Â·5Â·3 bytes are hashed out of an 80-byte
locked region, so row padding is included and the final row is truncated.

Note: `GetTextureHash` returns -1 for `D3DPOOL_DEFAULT` textures. TexMod copies
those to a system surface first (the same `GetRenderTargetData` path uMod uses)
and hashes the copy.

Input length is computed as a **float**:

```
len = width * height * bytes_per_pixel(D3DFMT)
```

and TexMod hashes exactly that many bytes from the start of the locked surface,
**ignoring the surface pitch**. This is a bug in TexMod and is a compatibility
requirement: on surfaces whose pitch exceeds `width * bpp`, the hash covers
row padding and truncates the final rows.

`bytes_per_pixel` comes from a float table at `0x10008A50`. Values below were
verified by **executing** that function, not just reading the table:

| D3DFMT | bpp | | D3DFMT | bpp |
|---|---|---|---|---|
| R8G8B8 (`0x14`) | 3 | | DXT1 (`'DXT1'`) | **0.5** |
| A8R8G8B8 (`0x15`) | 4 | | DXT2 (`'DXT2'`) | 1 |
| X8R8G8B8 (`0x16`) | 4 | | DXT3 (`'DXT3'`) | 1 |
| R5G6B5 (`0x17`) | 2 | | DXT4 (`'DXT4'`) | 1 |
| A1R5G5B5 (`0x19`) | 2 | | DXT5 (`'DXT5'`) | 1 |
| A4R4G4B4 (`0x1A`) | 2 | | R16F (`0x3C`) | 2 |
| R3G3B2 (`0x1B`) | 1 | | G16R16F (`0x3D`) | 2 |
| A8 (`0x1C`) | 1 | | A16B16G16R16F (`0x3E`) | 4 |
| unknown | **0.5** | | R32F/G32R32F (`0x3F/0x40`) | 4 |

Compressed textures use the FourCC constants (`D3DFMT_DXT1 =
0x31545844`, â€¦), not the small numeric values. The small values `0x44`â€“`0x48`
fall into the 0.5 fallback and are not valid `D3DFMT`s.

Because the hash is taken from the raw surface, a pack's BMP dump cannot
reproduce its own hash when the source texture was compressed (DXT) â€” the BMP
is a decoded copy. Hash parity must be checked against the live surface.

---

## 4. D3D9 hooking

The logging DLL installs an inline hook on `Direct3DCreate9`:

1. `SetDllDirectory` / `GetModuleFileName` to build the path to the real
   `d3d9.dll` (avoids re-entering itself).
2. `LoadLibraryA("d3d9.dll")` + `GetProcAddress("Direct3DCreate9")`.
3. Patch the function prologue with a jump to the replacement at
   `0x10001020`; the original bytes are relocated into a trampoline at
   `0x10001000`.
4. The replacement calls the trampoline, then wraps the returned
   `IDirect3D9` in a proxy object (vtable patching), so `CreateDevice` can wrap
   `IDirect3DDevice9`, and texture/draw calls can be intercepted.

The exe side injects the DLL with `CreateProcessA` (suspended),
`VirtualAllocEx` + `WriteProcessMemory` + `CreateRemoteThread`, after enabling
`SeDebugPrivilege`; a file mapping (`CreateFileMappingA`/`MapViewOfFile`) is
used to share the package data with the injected DLL. The DLL has **no
exports** â€” it is driven entirely by that shared state and its DllMain thread.

---

## 5. Package build / logging quirks

- Logged textures are written as 32-bit BMPs: 14-byte file header + 40-byte
  DIB header (`bfOffBits = 54`), `biSizeImage = width*height*4 + 2`, and the
  file is 2 bytes longer than `54 + width*height*4` (observed consistently).
  The extra 2 bytes are ignored by readers.
- `texmod.log`/definition lines use the format `0x%.8X|%s`.
- Registry state lives under `HKCU\Software\TexMod`
  (`Packages`, `recentpkg0`, `Format`, `DefinitionFile`, `Outpath`, â€¦).
- Main window class `tmlwndcls`; target picker is a standard `#32770` file
  dialog; the package dropdown is a `#32768` popup menu.

---

## 6. Compatibility checklist for a reimplementation

- [x] XOR mask `0x3FA43FA4` (32-bit, little-endian)
- [x] ZipCrypto with the 42-byte password, deflate
- [x] `<EXE>_0x<HASH>.<bmp|dds>` naming, `texmod.def`, ZIP comment
- [x] CRC-32 variant (no final XOR) over `bpp*w*h` of the locked surface
- [x] CRC routine and float bpp function verified by executing TexMod's code
- [x] hash parity verified on live D3D9 textures (A8R8G8B8, DXT1, pitch-padded)
- [ ] decide whether to keep the pitch bug (required for legacy matching)
- [ ] reproduce the 2-byte BMP padding when writing legacy packages

## 7. Known unknowns

- Exact purpose of the 2 trailing BMP bytes.
- Shared-memory layout between `TMLoad` and the injected DLL (only relevant if
  reusing the original DLLs).
- Which `IDirect3DDevice9` vtable slots are replaced (proxy table analysis is
  incomplete).
- Whether packages are matched by exe-name prefix or by hash alone (the sample
  pack targets `TRUEEXE.EXE` while the game is `SPEED.EXE`).

