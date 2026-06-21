use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Res = Result<(), Box<dyn std::error::Error>>;

const TAURI_CONF: &str = "app/src-tauri/tauri.conf.json";
const ROOT_MANIFEST: &str = "Cargo.toml";
const APP_MANIFEST: &str = "app/src-tauri/Cargo.toml";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or_default();
    let rest = if args.is_empty() { &[][..] } else { &args[1..] };
    let root = repo_root();

    let result = match task {
        "codegen" => codegen(&root),
        "build" => build(&root, has_flag(rest, "--exe-only")),
        "bundle" => bundle(&root),
        "portable" => portable(&root),
        "sign" => sign(&root, &positional(rest)),
        "version" => match flag_value(rest, "--set") {
            Some(v) => set_version(&root, &v),
            None => Err("usage: cargo xtask version --set X.Y.Z".into()),
        },
        "release" => release(
            &root,
            flag_value(rest, "--version"),
            has_flag(rest, "--upload"),
            has_flag(rest, "--allow-dirty"),
        ),
        "" => {
            eprintln!("usage: cargo xtask <codegen|build|bundle|portable|sign|version|release>");
            std::process::exit(2);
        }
        other => Err(format!("unknown task '{other}'").into()),
    };

    if let Err(e) = result {
        eprintln!("\x1b[31mxtask {task} failed:\x1b[0m {e}");
        std::process::exit(1);
    }
}

fn codegen(root: &Path) -> Res {
    run(
        "cargo",
        ["test", "--manifest-path", APP_MANIFEST, "export_bindings"],
        root,
    )
}

fn build(root: &Path, exe_only: bool) -> Res {
    codegen(root)?;
    if exe_only {
        tauri_build(root, true)?;
        let exe = root.join("app/src-tauri/target/release/vbl-pro-2.exe");
        collect(root, &[exe])
    } else {
        bundle(root)
    }
}

/// Build the UI then the production app via the Tauri CLI. `no_bundle` skips the installer.
/// This must go through `cargo tauri build` (not plain `cargo build`), otherwise the exe runs in
/// dev mode and points at the dev-server URL instead of the embedded UI.
fn tauri_build(root: &Path, no_bundle: bool) -> Res {
    if !has_tool("cargo", ["tauri", "--version"], root) {
        return Err("`cargo tauri` not found — install with `cargo install tauri-cli`".into());
    }
    run("bun", ["run", "--cwd", "app/ui", "build"], root)?;
    let args: Vec<&str> = if no_bundle {
        vec!["tauri", "build", "--no-bundle"]
    } else {
        vec!["tauri", "build"]
    };
    run("cargo", args, &root.join("app/src-tauri"))
}

fn bundle(root: &Path) -> Res {
    // Clear old installers so a stale version can't be collected into the release.
    let dir = root.join("app/src-tauri/target/release/bundle/nsis");
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    tauri_build(root, false)?;
    let installers = find_with_ext(&dir, "exe");
    if installers.is_empty() {
        return Err(format!("no NSIS installer found in {}", dir.display()).into());
    }
    // Collect the installer(s) plus their updater `.sig` signatures.
    let mut to_collect = installers;
    to_collect.extend(find_with_ext(&dir, "sig"));
    collect(root, &to_collect)
}

/// Build the production exe and zip it into `release/` as a portable distribution.
fn portable(root: &Path) -> Res {
    tauri_build(root, true)?;
    zip_portable(root)
}

/// Zip the already-built production exe into `release/` (no rebuild).
fn zip_portable(root: &Path) -> Res {
    let exe = root.join("app/src-tauri/target/release/vbl-pro-2.exe");
    if !exe.exists() {
        return Err(format!("release exe not found at {}", exe.display()).into());
    }
    let version = read_version(root)?;
    let dir = root.join("release");
    fs::create_dir_all(&dir)?;
    let out = dir.join(format!("vbl-pro-2-{version}-portable-x64.zip"));
    zip_files(&out, &[(&exe, "vbl-pro-2.exe")])?;
    println!("✓ wrote {}", out.display());
    Ok(())
}

/// Empty the `release/` directory so stale artifacts from prior builds can't leak into a release.
fn clean_release_dir(root: &Path) -> Res {
    let dir = root.join("release");
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(())
}

/// Write a deflate zip containing `(source path, name in archive)` entries.
fn zip_files(out: &Path, entries: &[(&Path, &str)]) -> Res {
    use std::io::Write;
    let file = fs::File::create(out)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (src, name) in entries {
        zip.start_file(*name, options)?;
        zip.write_all(&fs::read(src)?)?;
    }
    zip.finish()?;
    Ok(())
}

fn sign(root: &Path, files: &[String]) -> Res {
    let Ok(cert) = env::var("VBL_SIGN_CERT") else {
        eprintln!("signing skipped: set VBL_SIGN_CERT (PFX path) and VBL_SIGN_PASSWORD to enable.");
        return Ok(());
    };
    let password = env::var("VBL_SIGN_PASSWORD").unwrap_or_default();
    let ts_url = env::var("VBL_SIGN_TIMESTAMP_URL")
        .unwrap_or_else(|_| "http://timestamp.digicert.com".into());
    if files.is_empty() {
        return Err("usage: cargo xtask sign <file> [file…]".into());
    }
    for file in files {
        run(
            "signtool",
            [
                "sign", "/fd", "SHA256", "/f", &cert, "/p", &password, "/tr", &ts_url, "/td",
                "SHA256", file,
            ],
            root,
        )?;
    }
    Ok(())
}

fn set_version(root: &Path, version: &str) -> Res {
    validate_version(version)?;

    let conf_path = root.join(TAURI_CONF);
    let conf = fs::read_to_string(&conf_path)?;
    let conf = set_json_string(&conf, "\"version\":", version)
        .ok_or("could not find \"version\" in tauri.conf.json")?;
    fs::write(&conf_path, conf)?;

    set_toml_version(
        &root.join(ROOT_MANIFEST),
        &["workspace", "package", "version"],
        version,
    )?;
    set_toml_version(&root.join(APP_MANIFEST), &["package", "version"], version)?;

    println!("✓ version set to {version}");
    Ok(())
}

fn release(root: &Path, version: Option<String>, upload: bool, allow_dirty: bool) -> Res {
    if !allow_dirty {
        let status = capture("git", ["status", "--porcelain"], root)?;
        if !status.trim().is_empty() {
            return Err("working tree is dirty; commit/stash or pass --allow-dirty".into());
        }
    }

    if let Some(v) = &version {
        set_version(root, v)?;
    }
    let version = read_version(root)?;
    println!("▶ releasing v{version}");

    clean_release_dir(root)?;
    bundle(root)?;
    // Reuse the production exe the installer build already produced — no second compile.
    zip_portable(root)?;

    let release_dir = root.join("release");
    let installers = find_with_ext(&release_dir, "exe");
    let strings: Vec<String> = installers.iter().map(|p| p.display().to_string()).collect();
    sign(root, &strings)?;

    write_update_manifest(root, &version, &installers)?;

    // Everything in release/ (installers + portable zip) goes to the draft.
    let mut artifacts = installers;
    artifacts.extend(find_with_ext(&release_dir, "zip"));

    if upload {
        upload_draft(root, &version, &artifacts)?;
    } else {
        println!(
            "✓ artifacts in {} (pass --upload to publish a GitHub draft)",
            release_dir.display()
        );
    }
    Ok(())
}

fn write_update_manifest(root: &Path, version: &str, artifacts: &[PathBuf]) -> Res {
    // Pick the installer for *this* version (defensive against any stale artifacts).
    let installer = artifacts
        .iter()
        .find(|p| {
            has_ext(p, "exe")
                && p.file_name()
                    .and_then(OsStr::to_str)
                    .map(|n| n.contains(version))
                    .unwrap_or(false)
        })
        .or_else(|| artifacts.iter().find(|p| has_ext(p, "exe")));
    let Some(installer) = installer else {
        eprintln!("⚠ no installer artifact; skipping update manifest");
        return Ok(());
    };
    // GitHub replaces spaces in release-asset names with dots; match that in the download URL.
    let name = installer
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .replace(' ', ".");
    let repo = env::var("GITHUB_REPOSITORY").unwrap_or_else(|_| "OWNER/REPO".into());
    let url = format!("https://github.com/{repo}/releases/download/v{version}/{name}");

    let sig_path = installer.with_extension("exe.sig");
    let signature = fs::read_to_string(&sig_path).unwrap_or_else(|_| {
        eprintln!(
            "no update signature ({}); set up `tauri signer` keys to enable in-app updates",
            sig_path.display()
        );
        String::new()
    });

    let manifest = serde_json::json!({
        "version": version,
        "notes": format!("VBL Pro 2 v{version}"),
        "pub_date": rfc3339_now(),
        "platforms": {
            "windows-x86_64": { "signature": signature, "url": url }
        }
    });
    let out = root.join("release/latest.json");
    fs::write(&out, serde_json::to_string_pretty(&manifest)?)?;
    println!("✓ wrote {}", out.display());
    Ok(())
}

fn upload_draft(root: &Path, version: &str, artifacts: &[PathBuf]) -> Res {
    if !has_tool("gh", ["--version"], root) {
        return Err("`gh` CLI not found — install it or upload manually".into());
    }
    let tag = format!("v{version}");
    let mut args: Vec<String> = vec![
        "release".into(),
        "create".into(),
        tag.clone(),
        "--draft".into(),
        "--title".into(),
        format!("VBL Pro 2 {tag}"),
        "--notes".into(),
        format!("Release {tag}."),
        root.join("release/latest.json").display().to_string(),
    ];
    args.extend(artifacts.iter().map(|p| p.display().to_string()));
    run("gh", args.iter().map(String::as_str), root)
}

fn read_version(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let conf = fs::read_to_string(root.join(TAURI_CONF))?;
    json_string_value(&conf, "\"version\":").ok_or_else(|| "version not found".into())
}

fn set_toml_version(path: &Path, keys: &[&str], version: &str) -> Res {
    let mut doc = fs::read_to_string(path)?.parse::<toml_edit::DocumentMut>()?;
    let mut item = doc.as_item_mut();
    for key in keys {
        item = item
            .get_mut(key)
            .ok_or_else(|| format!("{}: missing key '{key}'", path.display()))?;
    }
    *item = toml_edit::value(version);
    fs::write(path, doc.to_string())?;
    Ok(())
}

fn set_json_string(content: &str, key: &str, value: &str) -> Option<String> {
    let after = content.find(key)? + key.len();
    let open = after + content[after..].find('"')?;
    let close = open + 1 + content[open + 1..].find('"')?;
    Some(format!("{}{value}{}", &content[..=open], &content[close..]))
}

fn json_string_value(content: &str, key: &str) -> Option<String> {
    let after = content.find(key)? + key.len();
    let open = after + content[after..].find('"')? + 1;
    let close = open + content[open..].find('"')?;
    Some(content[open..close].to_string())
}

fn validate_version(v: &str) -> Res {
    let ok = v.split('.').count() == 3
        && v.split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    if ok {
        Ok(())
    } else {
        Err(format!("invalid version '{v}' (expected X.Y.Z)").into())
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is under the workspace root")
        .to_path_buf()
}

fn run<I, S>(program: &str, args: I, cwd: &Path) -> Res
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<S> = args.into_iter().collect();
    let shown: Vec<String> = args
        .iter()
        .map(|a| a.as_ref().to_string_lossy().into_owned())
        .collect();
    println!("\x1b[36m» {program} {}\x1b[0m", shown.join(" "));
    let status = Command::new(program)
        .args(&args)
        .current_dir(cwd)
        .status()
        .map_err(|e| format!("failed to run {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}").into())
    }
}

fn capture<I, S>(program: &str, args: I, cwd: &Path) -> Result<String, Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let out = Command::new(program).args(args).current_dir(cwd).output()?;
    if !out.status.success() {
        return Err(format!("{program} exited with {}", out.status).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn has_tool<I, S>(program: &str, args: I, cwd: &Path) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn collect(root: &Path, artifacts: &[PathBuf]) -> Res {
    let dir = root.join("release");
    fs::create_dir_all(&dir)?;
    for src in artifacts {
        let name = src.file_name().ok_or("artifact has no file name")?;
        let dst = dir.join(name);
        fs::copy(src, &dst)
            .map_err(|e| format!("copy {} -> {}: {e}", src.display(), dst.display()))?;
        println!("✓ collected {}", dst.display());
    }
    Ok(())
}

fn find_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| has_ext(p, ext))
        .collect()
}

fn has_ext(path: &Path, ext: &str) -> bool {
    path.extension().and_then(OsStr::to_str) == Some(ext)
}

fn rfc3339_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let i = args.iter().position(|a| a == flag)?;
    args.get(i + 1).cloned()
}

fn positional(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect()
}
