use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use retex_core as tm;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "retex",
    version,
    about = "Retex: .tpf packages, texture hashes, verification"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List package contents
    List {
        tpf: PathBuf,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Validate a package (structure, CRCs, bitmaps, definition file)
    Verify { tpf: PathBuf },
    /// Extract entries to a directory
    Extract {
        tpf: PathBuf,
        out: PathBuf,
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Build a package from a definition file
    Build {
        tpf: PathBuf,
        def: PathBuf,
        #[arg(long, default_value = "TEXMOD.EXE")]
        exe: String,
        #[arg(long)]
        comment: Option<String>,
    },
    /// Print the TexMod texture hash of a BMP
    Hash { bmp: PathBuf },
    /// Install the ASI hook into a game directory and write retex.ini
    Install {
        game_dir: PathBuf,
        #[arg(long)]
        hook: Option<PathBuf>,
        #[arg(long = "pack", required = true)]
        packs: Vec<PathBuf>,
    },
    /// Launch a game and inject the hook (no ASI loader needed)
    Launch {
        target: PathBuf,
        /// Replacement pack(s) to load, in order (writes retex.ini)
        #[arg(long = "pack")]
        packs: Vec<PathBuf>,
    },
    /// Create a shortcut that launches a game with Retex
    Shortcut {
        target: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, default_value = "")]
        args: String,
        /// Launch through `retex launch` (injector) instead of the game directly
        #[arg(long)]
        inject: bool,
        /// Pack(s) to write to retex.ini when --inject is used
        #[arg(long = "pack")]
        packs: Vec<PathBuf>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::List { tpf, limit } => list(&tpf, limit),
        Command::Verify { tpf } => verify(&tpf),
        Command::Extract { tpf, out, limit } => extract(&tpf, &out, limit),
        Command::Build {
            tpf,
            def,
            exe,
            comment,
        } => build(&tpf, &def, &exe, comment),
        Command::Hash { bmp } => hash(&bmp),
        Command::Install {
            game_dir,
            hook,
            packs,
        } => install(&game_dir, hook.as_deref(), &packs),
        Command::Launch { target, packs } => launch(&target, &packs),
        Command::Shortcut {
            target,
            name,
            output,
            args,
            inject,
            packs,
        } => shortcut(
            &target,
            name.as_deref(),
            output.as_deref(),
            &args,
            inject,
            &packs,
        ),
    }
}

fn list(tpf: &Path, limit: usize) -> Result<()> {
    let package = tm::tpf::read(tpf)?;
    println!("entries: {}", package.entries.len());
    println!("comment: {:?}", package.comment_text());
    for entry in package.entries.iter().take(limit) {
        println!(
            "  0x{:08X}  {:>10}  {}",
            entry.hash,
            entry.data.len(),
            entry.name
        );
    }
    Ok(())
}

fn verify(tpf: &Path) -> Result<()> {
    let package = tm::tpf::read(tpf)?;

    let mut def_map: HashMap<String, u32> = HashMap::new();
    if let Some(def) = package
        .entries
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case("texmod.def"))
    {
        let text = String::from_utf8_lossy(&def.data);
        for (hash, file) in tm::tpf::parse_definition(&text)? {
            def_map.insert(file, hash);
        }
    }

    let mut valid = 0usize;
    let mut invalid = 0usize;
    let mut mismatched = 0usize;
    let mut bytes = 0usize;

    for entry in &package.entries {
        if entry.name.to_ascii_lowercase().ends_with(".def") {
            continue;
        }
        bytes += entry.data.len();
        if is_valid_bmp(&entry.data) {
            valid += 1;
        } else {
            invalid += 1;
        }
        if let Some(&expected) = def_map.get(&entry.name) {
            if expected != entry.hash {
                mismatched += 1;
            }
        }
    }

    let missing = def_map
        .keys()
        .filter(|name| !package.entries.iter().any(|e| &e.name == *name))
        .count();

    println!("entries:        {}", valid + invalid);
    println!("valid bitmaps:  {valid}");
    println!("invalid:        {invalid}");
    println!("payload bytes:  {bytes}");
    println!(
        "texmod.def:     {} ({} entries)",
        if def_map.is_empty() { "no" } else { "yes" },
        def_map.len()
    );
    println!("def mismatches: {mismatched}");
    println!("def references missing files: {missing}");
    println!("comment:        {:?}", package.comment_text());

    if invalid > 0 || mismatched > 0 || missing > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn is_valid_bmp(data: &[u8]) -> bool {
    if data.len() < 54 || &data[..2] != b"BM" {
        return false;
    }
    let off = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
    let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
    let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let bpp = u16::from_le_bytes([data[28], data[29]]) as usize;
    if bpp == 0 {
        return false;
    }
    let need = off + (w.unsigned_abs() as usize) * (h.unsigned_abs() as usize) * (bpp / 8);
    need <= data.len()
}

fn extract(tpf: &Path, out: &Path, limit: Option<usize>) -> Result<()> {
    let package = tm::tpf::read(tpf)?;
    fs::create_dir_all(out)?;
    let count = limit
        .unwrap_or(package.entries.len())
        .min(package.entries.len());
    for entry in package.entries.iter().take(count) {
        let file = Path::new(&entry.name)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("entry.bin"));
        fs::write(out.join(file), &entry.data)
            .with_context(|| format!("writing {}", entry.name))?;
    }
    println!("extracted {count} file(s) to {}", out.display());
    Ok(())
}

fn build(tpf: &Path, def: &Path, exe: &str, comment: Option<String>) -> Result<()> {
    let text =
        fs::read_to_string(def).with_context(|| format!("reading definition {}", def.display()))?;
    let items = tm::tpf::parse_definition(&text)?;
    let base = def.parent().unwrap_or_else(|| Path::new("."));

    let mut entries = Vec::with_capacity(items.len() + 1);
    let mut def_lines = String::new();

    for (hash, relative) in items {
        let source = base.join(&relative);
        let data =
            fs::read(&source).with_context(|| format!("reading texture {}", source.display()))?;
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bmp")
            .to_ascii_lowercase();
        let name = tm::tpf::entry_name(exe, hash, &ext);
        def_lines.push_str(&format!("0x{hash:08X}|{name}\r\n"));
        entries.push(tm::PackageEntry { name, hash, data });
    }

    entries.push(tm::PackageEntry {
        name: "texmod.def".into(),
        hash: 0,
        data: def_lines.into_bytes(),
    });

    let package = tm::Package {
        comment: comment.unwrap_or_default().into_bytes(),
        entries,
    };
    tm::tpf::write(tpf, &package)?;
    println!(
        "wrote {} ({} entries)",
        tpf.display(),
        package.entries.len()
    );
    Ok(())
}

fn hash(bmp: &Path) -> Result<()> {
    let data = fs::read(bmp).with_context(|| format!("reading {}", bmp.display()))?;
    if data.len() < 54 || &data[..2] != b"BM" {
        bail!("not a BMP file");
    }
    let off = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
    let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
    let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let bpp = u16::from_le_bytes([data[28], data[29]]) as usize;
    if bpp == 0 {
        bail!("unsupported BMP bit depth");
    }
    let len = (w.unsigned_abs() as usize) * (h.unsigned_abs() as usize) * (bpp / 8);
    let end = off + len;
    if end > data.len() {
        bail!("truncated BMP pixel data");
    }
    println!("0x{:08X}", tm::texmod_crc(&data[off..end]));
    Ok(())
}

fn install(game_dir: &Path, hook: Option<&Path>, packs: &[PathBuf]) -> Result<()> {
    if !game_dir.is_dir() {
        bail!("game directory not found: {}", game_dir.display());
    }
    let hook_path = match hook {
        Some(path) => path.to_path_buf(),
        None => locate_hook()?,
    };
    let destination = game_dir.join("retex.asi");
    fs::copy(&hook_path, &destination).with_context(|| {
        format!(
            "copying {} to {}",
            hook_path.display(),
            destination.display()
        )
    })?;

    let config_path = write_config(game_dir, packs)?;

    println!("installed {}", destination.display());
    println!("wrote     {}", config_path.display());
    println!("note: the game needs Ultimate ASI Loader (dinput8.dll) to load retex.asi");
    println!("      or use `retex launch <game.exe>` / `retex shortcut --inject`");
    Ok(())
}

fn write_config(game_dir: &Path, packs: &[PathBuf]) -> Result<PathBuf> {
    let mut config = String::from("# Retex\r\n");
    for pack in packs {
        let absolute = std::path::absolute(pack).unwrap_or_else(|_| pack.clone());
        config.push_str(&format!("pack={}\r\n", absolute.display()));
    }
    let config_path = game_dir.join("retex.ini");
    fs::write(&config_path, config)
        .with_context(|| format!("writing {}", config_path.display()))?;
    Ok(config_path)
}

fn launch(target: &Path, packs: &[PathBuf]) -> Result<()> {
    let target = std::path::absolute(target)?;
    if !target.is_file() {
        bail!("target not found: {}", target.display());
    }
    let game_dir = target.parent().unwrap_or_else(|| Path::new("."));
    if !packs.is_empty() {
        let config = write_config(game_dir, packs)?;
        println!("wrote {}", config.display());
    }

    let hook = locate_hook()?;
    let injector = locate_injector()?;
    let status = std::process::Command::new(&injector)
        .arg(&hook)
        .arg(&target)
        .status()
        .with_context(|| format!("running {}", injector.display()))?;
    if !status.success() {
        bail!("injector failed ({status})");
    }
    Ok(())
}

fn locate_injector() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let candidates = [
        dir.join("retex-inject.exe"),
        dir.join("../i686-pc-windows-msvc/release/retex-inject.exe"),
        dir.join("../../i686-pc-windows-msvc/release/retex-inject.exe"),
        PathBuf::from("target/i686-pc-windows-msvc/release/retex-inject.exe"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!("could not find retex-inject.exe; build it with `cargo build -p retex-inject --target i686-pc-windows-msvc --release`")
}

fn locate_hook() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let candidates = [
        dir.join("retex_hook.dll"),
        dir.join("retex.asi"),
        dir.join("../i686-pc-windows-msvc/release/retex_hook.dll"),
        PathBuf::from("target/i686-pc-windows-msvc/release/retex_hook.dll"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!("could not find retex_hook.dll; pass --hook <path>")
}

fn shortcut(
    target: &Path,
    name: Option<&str>,
    output: Option<&Path>,
    extra_args: &str,
    inject: bool,
    packs: &[PathBuf],
) -> Result<()> {
    let target = std::path::absolute(target)?;
    if !target.is_file() {
        bail!("target not found: {}", target.display());
    }
    let output = match output {
        Some(path) => path.to_path_buf(),
        None => desktop_dir()?,
    };
    fs::create_dir_all(&output)?;

    let title = name.map(str::to_string).unwrap_or_else(|| {
        format!(
            "{} (Retex)",
            target
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Game")
        )
    });
    let link = output.join(format!("{title}.lnk"));
    let working_dir = target.parent().unwrap_or_else(|| Path::new("."));

    let (link_target, link_args) = if inject {
        if !packs.is_empty() {
            let config = write_config(working_dir, packs)?;
            println!("wrote {}", config.display());
        }
        let cli = std::env::current_exe()?;
        let mut args = format!("launch \"{}\"", target.display());
        if !extra_args.is_empty() {
            args.push(' ');
            args.push_str(extra_args);
        }
        (cli, args)
    } else {
        (target.clone(), extra_args.to_string())
    };

    let icon = shortcut_icon(&target);
    create_shortcut(&link, &link_target, &link_args, working_dir, &icon)?;
    println!("created {}", link.display());
    Ok(())
}

/// Pick the icon for a game shortcut. Patched or cracked executables often
/// carry no icon resource, so fall back to an `.ico` in the game folder, then
/// to another executable there that has one, and finally to Retex's own icon.
fn shortcut_icon(target: &Path) -> PathBuf {
    if icon_count(target) > 0 {
        return target.to_path_buf();
    }
    let Some(dir) = target.parent() else {
        return target.to_path_buf();
    };
    let Ok(read_dir) = fs::read_dir(dir) else {
        return target.to_path_buf();
    };
    let mut files: Vec<PathBuf> = read_dir
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect();
    files.sort();

    let extension_is = |path: &Path, wanted: &str| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case(wanted))
    };
    if let Some(ico) = files.iter().find(|path| extension_is(path, "ico")) {
        return ico.clone();
    }

    const SKIP: [&str; 5] = ["unins", "setup", "install", "autorun", "config"];
    files
        .iter()
        .filter(|path| extension_is(path, "exe"))
        .filter(|path| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            !SKIP.iter().any(|word| name.contains(word))
        })
        .find(|path| icon_count(path) > 0)
        .cloned()
        .or_else(|| std::env::current_exe().ok())
        .unwrap_or_else(|| target.to_path_buf())
}

#[cfg(windows)]
fn icon_count(path: &Path) -> u32 {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::ExtractIconExW;

    let path_h = HSTRING::from(path.as_os_str());
    // An index of -1 with no output buffers returns the number of icons.
    unsafe { ExtractIconExW(&path_h, -1, None, None, 0) }
}

#[cfg(not(windows))]
fn icon_count(_path: &Path) -> u32 {
    0
}

fn desktop_dir() -> Result<PathBuf> {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return Ok(Path::new(&profile).join("Desktop"));
    }
    bail!("cannot determine the desktop path; pass --output")
}

#[cfg(windows)]
fn create_shortcut(
    link: &Path,
    target: &Path,
    args: &str,
    working_dir: &Path,
    icon: &Path,
) -> Result<()> {
    use windows::core::{Interface, HSTRING, PCWSTR};
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;

        let target_h = HSTRING::from(target.as_os_str());
        let args_h = HSTRING::from(args);
        let dir_h = HSTRING::from(working_dir.as_os_str());
        let link_h = HSTRING::from(link.as_os_str());
        let icon_h = HSTRING::from(icon.as_os_str());

        shell_link.SetPath(PCWSTR(target_h.as_ptr()))?;
        shell_link.SetArguments(PCWSTR(args_h.as_ptr()))?;
        shell_link.SetWorkingDirectory(PCWSTR(dir_h.as_ptr()))?;
        shell_link.SetIconLocation(PCWSTR(icon_h.as_ptr()), 0)?;

        let persist: IPersistFile = shell_link.cast()?;
        persist.Save(PCWSTR(link_h.as_ptr()), BOOL(1))?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn create_shortcut(
    _link: &Path,
    _target: &Path,
    _args: &str,
    _working_dir: &Path,
    _icon: &Path,
) -> Result<()> {
    bail!("shortcut creation is only supported on Windows")
}
