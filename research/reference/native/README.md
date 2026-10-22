# Native verification probes

`hash_probe.cpp` creates real D3D9 textures (A8R8G8B8, DXT1, and a pitch-padded
R8G8B8 case), then calls TexMod's own `GetTextureHash` in the unpacked logging
DLL (`tm_payload_b.dll`, offset `+0x8D50`) and prints the hash plus the raw
locked surface.

It is a research tool: it needs a 32-bit build, a D3D9 device, and a copy of the
extracted TexMod logging DLL.

Build (Developer Command Prompt for VS 2022):

    cl /nologo /EHsc /O2 hash_probe.cpp /link d3d9.lib user32.lib gdi32.lib

Run:

    hash_probe.exe <path\to\tm_payload_b.dll>

Expected results for the current probe data:

    A8R8G8B8_8x8  hash=0x06A47B50
    DXT1_8x8      hash=0x3568890B
    R8G8B8_5x5    hash=0x7BA00B15

See `docs/texmod-format.md` for how these are reproduced in `tools/tpf.py`.
