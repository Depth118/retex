# TexMod reverse-engineering archive

Everything produced while reverse-engineering the original **TexMod 0.9b**
(2006, RS). This folder is a historical record and a verification suite for
Retex; the implementation lives in [`../crates`](../crates).

## Layout

```
evidence/     probe output and extracted strings used as evidence
scripts/      analysis scripts (Python + PowerShell) and UI-automation attempts
reference/    clean-room Python reference implementation + native hash probe
```

## Original binaries (not included)

The original TexMod 0.9b files are not redistributed here. To reproduce the
analysis, obtain your own copy and check it against these hashes:

| Artifact | SHA-256 |
|---|---|
| `Texmod.exe` | `f662be61eee7c3d2849e1c734b3b83e9e25fa4e873c7352852130f0c0ceb98af` |
| `Readme.txt` | `2b3eb04307eb4878e1df4078a418298886ca70bd3bfb1c5be4da4f0b42a9d31a` |

`Texmod.exe` is packed with NsPack, and its unpacked hook DLLs are flagged by
Microsoft Defender as `Trojan:Win32/Sabsik.EN.A!ml` (an ML heuristic). Do not
run it in online games.

The analysis scripts expect their inputs (the in-memory image `texmod_mem.bin`
and the DLL payloads XORed with `0xAA`, `tm_a_unpacked.xor` /
`tm_b_unpacked.xor`) in a `work` folder next to them, `scripts/analysis/work/`.
Decode the XORed payloads with:

```python
open(out, "wb").write(bytes(b ^ 0xAA for b in open(path, "rb").read()))
```

The 345 MB reference package and the extracted BMPs are intentionally **not**
stored in the repository.

## Reproducing the verification

- Format and hash spec: [`../docs/texmod-format.md`](../docs/texmod-format.md)
- Python reference: `python reference/test_tpf.py`
- Live D3D9 hash probe: `reference/native/README.md`
  (result: `A8R8G8B8 0x06A47B50`, `DXT1 0x3568890B`, `R8G8B8 5x5 0x7BA00B15`)

## Findings in one paragraph

TexMod is a 32-bit MSVC application that injects one of two NsPack-packed DLLs
(package mode key `0x91`, logging mode key `0xC8`) into a D3D9 game. The DLLs
inline-hook `Direct3DCreate9` and wrap the returned interfaces. Packages
(`.tpf`) are ZIP archives XOR-obfuscated with `0x3FA43FA4` and ZipCrypto-encrypted
with a fixed 42-byte binary password. Texture identity is a CRC-32 variant
(poly `0xEDB88320`, no final XOR) over `bpp * width * height` bytes of the locked
surface, ignoring pitch — a bug that any compatible tool must reproduce.
