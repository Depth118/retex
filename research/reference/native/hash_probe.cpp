// Hash probe: creates real D3D9 textures, fills them with a deterministic
// pattern, then calls TexMod's own GetTextureHash (logging DLL +0x8D50) and
// prints the hash plus the raw locked surface so it can be recomputed.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <d3d9.h>
#include <stdio.h>

typedef int(__stdcall* HASHFN)(IDirect3DTexture9*, int*, int*);

static void dump(const char* tag, const BYTE* p, int n) {
    printf("%s ", tag);
    for (int i = 0; i < n; i++) printf("%02X", p[i]);
    printf("\n");
}

static IDirect3DTexture9* make_tex(IDirect3DDevice9* dev, UINT w, UINT h, D3DFORMAT fmt,
                                   BYTE seed, BYTE* snapshot, int* snap_len) {
    IDirect3DTexture9* tex = NULL;
    // SCRATCH keeps the locked bytes exactly as written (pool != 0 so TexMod hashes it)
    HRESULT hr = dev->CreateTexture(w, h, 1, 0, fmt, D3DPOOL_SCRATCH, &tex, NULL);
    if (FAILED(hr)) { printf("CreateTexture failed 0x%08X\n", hr); return NULL; }
    D3DLOCKED_RECT lr;
    if (FAILED(tex->LockRect(0, &lr, NULL, 0))) { tex->Release(); return NULL; }
    int total = lr.Pitch * h;
    for (int i = 0; i < total; i++) ((BYTE*)lr.pBits)[i] = (BYTE)(seed + i * 13);
    memcpy(snapshot, lr.pBits, total);
    *snap_len = total;
    tex->UnlockRect(0);
    printf("surface pitch=%d total=%d for %ux%u fmt=%u\n", lr.Pitch, total, w, h, (unsigned)fmt);
    return tex;
}

int main(int argc, char** argv) {
    if (argc < 2) { printf("usage: d3d9probe <tm_payload_b.dll>\n"); return 1; }

    WNDCLASSA wc = {0};
    wc.lpfnWndProc = DefWindowProcA;
    wc.hInstance = GetModuleHandleA(NULL);
    wc.lpszClassName = "tmprobe";
    RegisterClassA(&wc);
    HWND hwnd = CreateWindowA("tmprobe", "", WS_OVERLAPPEDWINDOW, 0, 0, 64, 64,
                              NULL, NULL, wc.hInstance, NULL);

    HMODULE d3d = LoadLibraryA("d3d9.dll");
    typedef IDirect3D9*(WINAPI* CREATE9)(UINT);
    CREATE9 create9 = (CREATE9)GetProcAddress(d3d, "Direct3DCreate9");
    IDirect3D9* d3d9 = create9(D3D_SDK_VERSION);
    if (!d3d9) { printf("Direct3DCreate9 failed\n"); return 2; }

    D3DPRESENT_PARAMETERS pp = {0};
    pp.Windowed = TRUE;
    pp.SwapEffect = D3DSWAPEFFECT_DISCARD;
    pp.BackBufferFormat = D3DFMT_UNKNOWN;
    pp.BackBufferWidth = 64;
    pp.BackBufferHeight = 64;
    pp.hDeviceWindow = hwnd;

    IDirect3DDevice9* dev = NULL;
    HRESULT hr = d3d9->CreateDevice(D3DADAPTER_DEFAULT, D3DDEVTYPE_HAL, hwnd,
                                    D3DCREATE_SOFTWARE_VERTEXPROCESSING, &pp, &dev);
    if (FAILED(hr)) {
        printf("HAL failed 0x%08X, trying REF\n", hr);
        hr = d3d9->CreateDevice(D3DADAPTER_DEFAULT, D3DDEVTYPE_REF, hwnd,
                                D3DCREATE_SOFTWARE_VERTEXPROCESSING, &pp, &dev);
    }
    if (FAILED(hr)) { printf("CreateDevice failed 0x%08X\n", hr); return 3; }

    static BYTE s1[8 * 8 * 4], s2[8 * 8 * 4], s3[5 * 16 * 2];
    int n1 = 0, n2 = 0, n3 = 0;
    IDirect3DTexture9* t1 = make_tex(dev, 8, 8, D3DFMT_A8R8G8B8, 0x11, s1, &n1);
    IDirect3DTexture9* t2 = make_tex(dev, 8, 8, D3DFMT_DXT1, 0x22, s2, &n2);
    IDirect3DTexture9* t3 = make_tex(dev, 5, 5, D3DFMT_R8G8B8, 0x33, s3, &n3);

    HMODULE tm = LoadLibraryA(argv[1]);
    if (!tm) { printf("LoadLibrary TexMod DLL failed %lu\n", GetLastError()); return 4; }
    const char* modname = strrchr(argv[1], '\\');
    modname = modname ? modname + 1 : argv[1];
    BYTE* base = (BYTE*)GetModuleHandleA(modname);
    printf("texmod dll base=%p\n", base);
    HASHFN gh = (HASHFN)(base + 0x8D50);

    int w = 0, h = 0;
    int h1 = gh(t1, &w, &h);
    printf("A8R8G8B8_8x8 hash=0x%08X w=%d h=%d\n", (unsigned)h1, w, h);
    dump("A8R8G8B8_8x8_DATA", s1, n1);

    w = h = 0;
    int h2 = gh(t2, &w, &h);
    printf("DXT1_8x8 hash=0x%08X w=%d h=%d\n", (unsigned)h2, w, h);
    dump("DXT1_8x8_DATA", s2, n2);

    w = h = 0;
    int h3 = gh(t3, &w, &h);
    printf("R8G8B8_5x5 hash=0x%08X w=%d h=%d\n", (unsigned)h3, w, h);
    dump("R8G8B8_5x5_DATA", s3, n3);

    return 0;
}
