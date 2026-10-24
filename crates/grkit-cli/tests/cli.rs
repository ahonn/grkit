use grkit_core::{effects, testing};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grkit"))
        .args(args)
        .output()
        .unwrap()
}

fn success(args: &[&str]) -> Value {
    let output = run(args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// A stand-in for a guide PDF, which the release builds with Typst.
const GUIDE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/guide.pdf");

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../effects")
}

/// The number of effect definitions in `effects/`.
fn source_count() -> usize {
    fs::read_dir(source())
        .unwrap()
        .filter(|entry| entry.as_ref().unwrap().path().join("effect.json").is_file())
        .count()
}

fn write_definition(root: &Path, id: &str) {
    let mut definition: Value =
        serde_json::from_slice(&fs::read(source().join("red-filter-mono-v1/effect.json")).unwrap())
            .unwrap();
    definition["effect_id"] = json!(id);
    let directory = root.join(id);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("effect.json"),
        serde_json::to_vec(&definition).unwrap(),
    )
    .unwrap();
}

fn compile(input: &Path, output: &Path) -> Output {
    run(&[
        "effects",
        "build",
        "--source",
        text(input),
        "--out",
        text(output),
    ])
}

fn fixture(root: &Path) -> (PathBuf, PathBuf) {
    let build = root.join("runtime");
    let effects = root.join("effects");
    success(&[
        "effects",
        "build",
        "--source",
        text(&source()),
        "--out",
        text(&effects),
    ]);
    let (manifest, files) = testing::build("gr3-2.10");
    for (name, bytes) in &files {
        let path = build.join("sd").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fs::write(build.join("build.json"), manifest).unwrap();
    (build, effects)
}

#[test]
fn exports_effects_without_overwriting() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("effects");
    let source = source();
    let args = [
        "effects",
        "build",
        "--source",
        text(&source),
        "--out",
        text(&output),
    ];
    let result = success(&args);
    assert_eq!(result["effects"], source_count());
    assert_eq!(fs::read_dir(&output).unwrap().count(), source_count());
    assert!(!run(&args).status.success());
}

#[test]
fn effect_compilation_is_independent_of_target_capacity() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input");
    fs::create_dir(&input).unwrap();
    let empty = tmp.path().join("empty");
    assert!(compile(&input, &empty).status.success());
    assert_eq!(fs::read_dir(empty).unwrap().count(), 0);
    for index in 0..21 {
        write_definition(&input, &format!("custom-{index:04}"));
    }
    let output = tmp.path().join("compiled");
    let result = compile(&input, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap()["effects"],
        21
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 21);

    let (build, _) = fixture(&tmp.path().join("package-input"));
    let archive = tmp.path().join("overflow.zip");
    let result = run(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--effects",
        text(&output),
        "--out",
        text(&archive),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("at most 20"));
    assert!(!archive.exists());
}

#[test]
fn actual_filename_collision_and_directory_identity_fail_before_output() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("compiled");
    // These distinct IDs share the first 32 SHA-256 bits used by FAT filenames.
    for id in ["collision-55045", "collision-70885"] {
        assert_eq!(effects::filename(id).unwrap(), "45560C60.GRF");
        write_definition(&input, id);
    }
    let result = compile(&input, &output);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("filename hash collision"));
    assert!(!output.exists());

    let input = tmp.path().join("mismatched");
    write_definition(&input, "custom-v1");
    fs::rename(input.join("custom-v1"), input.join("wrong-name")).unwrap();
    let result = compile(&input, &output);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("directory must match"));
    assert!(!output.exists());
}

#[test]
fn compiled_directory_extras_and_changed_runtime_bytes_are_rejected() {
    for mutation in ["extra", "runtime"] {
        let tmp = tempfile::tempdir().unwrap();
        let (build, effects) = fixture(tmp.path());
        if mutation == "extra" {
            fs::write(effects.join("extra.txt"), b"unexpected").unwrap();
        } else {
            fs::write(build.join("sd/SCRIPT/STARTUP.TTL"), b"changed").unwrap();
        }
        let output = tmp.path().join("package.zip");
        let result = run(&[
            "package",
            "--guide",
            GUIDE,
            "--build",
            text(&build),
            "--effects",
            text(&effects),
            "--out",
            text(&output),
        ]);
        assert!(!result.status.success());
        let message = String::from_utf8_lossy(&result.stderr);
        assert!(
            message.contains(if mutation == "extra" {
                "Unexpected compiled effect entry"
            } else {
                "Runtime hash or size mismatch"
            }),
            "{message}"
        );
        assert!(!output.exists());
    }
}

#[cfg(unix)]
#[test]
fn symlink_definitions_and_compiled_effects_are_rejected() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input/red-filter-mono-v1");
    fs::create_dir_all(&input).unwrap();
    symlink(
        source().join("red-filter-mono-v1/effect.json"),
        input.join("effect.json"),
    )
    .unwrap();
    let output = tmp.path().join("compiled");
    assert!(!compile(input.parent().unwrap(), &output).status.success());
    assert!(!output.exists());

    let (build, effects) = fixture(&tmp.path().join("package-input"));
    let path = fs::read_dir(&effects)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let target = tmp.path().join("external.grf");
    fs::rename(&path, &target).unwrap();
    symlink(&target, &path).unwrap();
    let result = run(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--out",
        text(&output),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Expected a regular file"));
    assert!(!output.exists());
}

#[test]
fn package_roundtrip_and_output_ownership() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, effects) = fixture(tmp.path());
    let zip = tmp.path().join("package.zip");
    let args = [
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--out",
        text(&zip),
    ];
    let packaged = success(&args);
    // Twelve runtime, selection, effect list, and document entries, then one
    // per effect.
    assert_eq!(packaged["entries"], 12 + source_count());
    assert_eq!(success(&["verify", text(&zip)])["effects"], source_count());
    let before = fs::read(&zip).unwrap();
    assert!(!run(&args).status.success());
    assert_eq!(fs::read(&zip).unwrap(), before);
    fs::write(build.join("sd/unlisted"), b"unexpected").unwrap();
    let rejected = tmp.path().join("rejected.zip");
    assert!(!run(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--out",
        text(&rejected)
    ])
    .status
    .success());
    assert!(!rejected.exists());
}

#[test]
fn package_targets_the_selected_camera() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, effects) = fixture(tmp.path());
    let zip = tmp.path().join("gr3x.zip");
    let packaged = success(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--camera",
        "gr3x",
        "--out",
        text(&zip),
    ]);
    assert_eq!(packaged["camera"], "gr3x");
    assert_eq!(packaged["language"], "en");
    let verified = success(&["verify", text(&zip)]);
    assert_eq!(verified["camera"], "gr3x");
    assert_eq!(verified["targets"], serde_json::json!(["gr3-2.10"]));
    // The language names the guide.
    let chinese = tmp.path().join("zh-Hans.zip");
    let packaged = success(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--language",
        "zh-Hans",
        "--out",
        text(&chinese),
    ]);
    assert_eq!(packaged["language"], "zh-Hans");
    assert_eq!(success(&["verify", text(&chinese)])["language"], "zh-Hans");
    assert!(!run(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--language",
        "fr",
        "--out",
        text(&tmp.path().join("fr.zip"))
    ])
    .status
    .success());
    let rejected = tmp.path().join("gr4.zip");
    let output = run(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--camera",
        "gr4",
        "--out",
        text(&rejected),
    ]);
    assert!(!output.status.success());
    assert!(!rejected.exists());
}

#[test]
fn package_selects_plugins_and_may_omit_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, effects) = fixture(tmp.path());
    let zip = tmp.path().join("selected.zip");
    let packaged = success(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--plugins",
        "image-control,framing,grain",
        "--out",
        text(&zip),
    ]);
    // Each package has the plugin file, so the count is the same.
    assert_eq!(packaged["entries"], 12 + source_count());
    assert_eq!(
        packaged["plugins"],
        json!(["image-control", "framing", "grain"])
    );
    let verified = success(&["verify", text(&zip)]);
    assert_eq!(
        verified["plugins"],
        json!(["image-control", "framing", "grain"])
    );

    // Without effects, Image Control has nothing to show, so another plugin must stay.
    let framing_only = tmp.path().join("framing.zip");
    let packaged = success(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--plugins",
        "framing",
        "--out",
        text(&framing_only),
    ]);
    // The entries include an empty effect list.
    assert_eq!(packaged["entries"], 12);
    assert_eq!(success(&["verify", text(&framing_only)])["effects"], 0);
    for plugins in ["image-control", "hdr", "guides,guides"] {
        let rejected = tmp.path().join("rejected.zip");
        let output = run(&[
            "package",
            "--guide",
            GUIDE,
            "--build",
            text(&build),
            "--plugins",
            plugins,
            "--out",
            text(&rejected),
        ]);
        assert!(!output.status.success(), "{plugins}");
        assert!(!rejected.exists(), "{plugins}");
    }
}

#[test]
fn invalid_archive_and_firmware_fail_without_panicking() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("invalid");
    fs::write(&bad, b"invalid input").unwrap();
    let verify = run(&["verify", text(&bad)]);
    assert_eq!(verify.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&verify.stderr).starts_with("grkit:"));
    let output = tmp.path().join("output");
    let build = run(&[
        "build",
        "--target",
        "gr3-2.10",
        "--firmware",
        text(&bad),
        "--out",
        text(&output),
        "--clang",
        "absent-compiler",
    ]);
    assert_eq!(build.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&build.stderr).contains("SHA-256"));
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn symlink_inputs_are_rejected() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let linked_source = tmp.path().join("source");
    symlink(source(), &linked_source).unwrap();
    let output = tmp.path().join("output");
    assert!(!run(&[
        "effects",
        "build",
        "--source",
        text(&linked_source),
        "--out",
        text(&output)
    ])
    .status
    .success());
    assert!(!output.exists());
}

#[test]
fn malformed_definition_is_rejected_before_output() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input/test");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("effect.json"), b"{\"schema\":1,\"schema\":2}").unwrap();
    let output = tmp.path().join("output");
    let result = run(&[
        "effects",
        "build",
        "--source",
        text(&tmp.path().join("input")),
        "--out",
        text(&output),
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn literal_backslash_cannot_alias_manifest_path() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, effects) = fixture(tmp.path());
    fs::write(build.join("sd/SCRIPT\\STARTUP.TTL"), b"unlisted").unwrap();
    let output = tmp.path().join("package.zip");
    assert!(!run(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&build),
        "--effects",
        text(&effects),
        "--out",
        text(&output)
    ])
    .status
    .success());
    assert!(!output.exists());
}

fn warm_cube(path: &Path) {
    let mut text = String::from("TITLE \"Warm\"\nLUT_3D_SIZE 5\n");
    for b in 0..5 {
        for g in 0..5 {
            for r in 0..5 {
                let [r, g, b] = [r, g, b].map(|v| v as f64 / 4.0);
                let row = [r * 0.94 + 0.05, g * 0.92 + 0.04, b * 0.86 + 0.03];
                text.push_str(&format!("{:.6} {:.6} {:.6}\n", row[0], row[1], row[2]));
            }
        }
    }
    fs::write(path, text).unwrap();
}

fn import(cube: &Path, firmware: &Path, id: &str, label: &str, effects: &Path) -> Output {
    run(&[
        "effects",
        "import",
        "--cube",
        text(cube),
        "--firmware",
        text(firmware),
        "--input",
        "srgb",
        "--exposure",
        "-0.5",
        "--id",
        id,
        "--name",
        "Warm",
        "--label",
        label,
        "--shape",
        "film",
        "--out",
        text(effects),
    ])
}

#[test]
fn cube_import_rejects_invalid_firmware_and_ids_before_output() {
    let tmp = tempfile::tempdir().unwrap();
    let cube = tmp.path().join("warm.cube");
    warm_cube(&cube);
    let firmware = tmp.path().join("firmware.bin");
    fs::write(&firmware, b"not firmware").unwrap();
    let effects = tmp.path().join("effects");
    let failed = import(&cube, &firmware, "warm-v1", "FV", &effects);
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("SHA-256"));
    let failed = import(&cube, &firmware, "../warm", "FV", &effects);
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Invalid effect ID"));
    assert!(!effects.exists());
}

#[test]
fn standard_base_gamma_is_read_and_checked() {
    use grkit_build::{firmware, target};
    let offset = (target::GR3_IMAGE_CONTROL.base_gamma - target::RTOS_BASE) as usize;
    let image = |table: &[u16; 256]| firmware::FirmwareImage {
        target: &target::GR3_210,
        body: [
            vec![0; offset],
            table.iter().flat_map(|v| v.to_le_bytes()).collect(),
        ]
        .concat(),
        sha256: "synthetic".into(),
        decoded_sha256: "synthetic".into(),
    };
    let mut table: [u16; 256] = std::array::from_fn(|i| match i {
        0 => 0,
        1..=8 => 944 * i as u16,
        _ => 7552 + ((i - 8) * 8832 / 247) as u16,
    });
    assert_eq!(
        firmware::standard_base_gamma(&image(&table)).unwrap(),
        table
    );
    for (index, value) in [(8, 7551), (0, 1), (255, 15999), (100, 0)] {
        let saved = table[index];
        table[index] = value;
        assert!(
            firmware::standard_base_gamma(&image(&table)).is_err(),
            "{index}"
        );
        table[index] = saved;
    }
}

/// End to end with the official firmware, which the repository cannot contain.
#[test]
fn imports_a_cube_into_a_buildable_effect_without_overwriting() {
    let Ok(firmware) = std::env::var("GRKIT_TEST_FIRMWARE") else {
        eprintln!(
            "SKIPPED cube import end to end: set GRKIT_TEST_FIRMWARE to GR III 2.10 firmware"
        );
        return;
    };
    let firmware = PathBuf::from(firmware);
    let tmp = tempfile::tempdir().unwrap();
    let cube = tmp.path().join("warm.cube");
    warm_cube(&cube);
    let effects = tmp.path().join("effects");
    let output = import(&cube, &firmware, "warm-v1", "FV", &effects);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    eprintln!("{summary}");
    assert_eq!(summary["effect_id"], "warm-v1");
    assert!(summary["fit"]["mean_de"].as_f64().unwrap() < 2.0);
    assert!(summary["fit"]["csp_max"].as_i64().unwrap() <= 240);
    let path = effects.join("warm-v1/effect.json");
    let written = fs::read(&path).unwrap();
    let definition = effects::parse_json(&written).unwrap();
    assert_eq!(definition["attribution"]["source"], "warm.cube");
    assert_eq!(definition["attribution"]["exposure"], -0.5);
    assert_eq!(
        definition.get("post_matrix").is_some(),
        summary["post_matrix"] == true
    );

    let failed = import(&cube, &firmware, "warm-v1", "FV", &effects);
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("already exists"));
    assert_eq!(fs::read(&path).unwrap(), written);
    assert!(!import(&cube, &firmware, "bad-v1", "ab", &effects)
        .status
        .success());
    assert!(!effects.join("bad-v1").exists());

    let compiled = tmp.path().join("compiled");
    let built = success(&[
        "effects",
        "build",
        "--source",
        text(&effects),
        "--out",
        text(&compiled),
    ]);
    assert_eq!(built["effects"], 1);
    let name = effects::filename("warm-v1").unwrap();
    let (effect, _) = effects::decode(&fs::read(compiled.join(name)).unwrap()).unwrap();
    assert_eq!(effect.to_json()["matrix"], definition["matrix"]);
}

#[test]
fn site_export_publishes_only_the_verified_runtime() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, effects) = fixture(tmp.path());
    fs::create_dir(build.join("objects")).unwrap();
    fs::write(build.join("objects/module_payload.h"), b"firmware bytes").unwrap();
    let site = tmp.path().join("site");
    let result = success(&["site", "--build", text(&build), "--out", text(&site)]);
    assert_eq!(result["target"], "gr3-2.10");
    assert_eq!(result["files"], 6);

    let exported = site.join("runtime/gr3-2.10");
    assert_eq!(
        fs::read(exported.join("build.json")).unwrap(),
        fs::read(build.join("build.json")).unwrap()
    );
    assert!(!exported.join("objects").exists());
    let mut entries: Vec<_> = fs::read_dir(&exported)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    entries.sort();
    assert_eq!(entries, ["build.json", "sd"]);

    // The exported tree is itself a valid build input.
    let package = tmp.path().join("package.zip");
    success(&[
        "package",
        "--guide",
        GUIDE,
        "--build",
        text(&exported),
        "--effects",
        text(&effects),
        "--out",
        text(&package),
    ]);
    // A second runtime for the same target is refused.
    assert!(
        !run(&["site", "--build", text(&build), "--out", text(&site)])
            .status
            .success()
    );
    assert_eq!(fs::read_dir(site.join("runtime")).unwrap().count(), 1);
}

#[test]
fn site_export_rejects_changed_runtime_files() {
    let tmp = tempfile::tempdir().unwrap();
    let (build, _) = fixture(tmp.path());
    fs::write(build.join("sd/SCRIPT/STARTUP.TTL"), b"changed").unwrap();
    let site = tmp.path().join("site");
    let result = run(&["site", "--build", text(&build), "--out", text(&site)]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Runtime hash or size mismatch"));
    assert!(!site.exists());
}

#[test]
fn effect_publish_rejects_unknown_firmware_before_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let firmware = tmp.path().join("fwdc239b.bin");
    fs::write(&firmware, b"not firmware").unwrap();
    let out = tmp.path().join("site/effects/gr3-2.10");
    let output = run(&[
        "effects",
        "publish",
        "--source",
        text(&source()),
        "--firmware",
        text(&firmware),
        "--out",
        text(&out),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not match GR III 2.10"));
    assert!(!out.exists());
}

/// End to end with the official firmware, which the repository cannot contain.
#[test]
fn effect_publish_writes_the_site_catalog() {
    let Ok(firmware) = std::env::var("GRKIT_TEST_FIRMWARE") else {
        eprintln!("SKIPPED effect publish: set GRKIT_TEST_FIRMWARE to GR III 2.10 firmware");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("site/effects/gr3-2.10");
    let effects_source = source();
    let args = [
        "effects",
        "publish",
        "--source",
        text(&effects_source),
        "--firmware",
        &firmware,
        "--out",
        text(&out),
    ];
    let result = success(&args);
    assert_eq!(result["target"], "gr3-2.10");
    assert_eq!(result["effects"], source_count());
    let catalog = effects::parse_json(&fs::read(out.join("catalog.json")).unwrap()).unwrap();
    // The table that `effects import` reads from the same firmware.
    let base: Vec<u16> = catalog["base_gamma"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| u16::try_from(v.as_u64().unwrap()).unwrap())
        .collect();
    assert_eq!(grkit_core::site::base_gamma(&base).unwrap()[8], 7552);
    let compiled = tmp.path().join("compiled");
    compile(&source(), &compiled);
    for entry in catalog["effects"].as_array().unwrap() {
        let file = entry["file"].as_str().unwrap();
        assert_eq!(
            fs::read(out.join(file)).unwrap(),
            fs::read(compiled.join(file)).unwrap()
        );
        let preview = fs::read(out.join(entry["preview"].as_str().unwrap())).unwrap();
        assert_eq!(preview.len(), 33 * 33 * 33 * 3);
    }
    assert!(!run(&args).status.success(), "an existing output is kept");
}

#[test]
fn guides_writes_one_typst_source_for_each_camera_and_language() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("guides");
    assert_eq!(success(&["guides", "--out", text(&out)])["guides"], 12);
    let source = fs::read_to_string(out.join("gr3x/zh-Hans/guide.typ")).unwrap();
    assert!(source.contains("仅适用于 RICOH GR IIIx 和 GR IIIx HDF 固件 1.60 或 1.70。"));
    assert!(source.contains("#link(<recovery>)") && source.contains("= 恢复 <recovery>"));
    assert!(out.join("gr3x/zh-Hans/images/grkit-root.jpg").is_file());
    assert!(out.join("gr3x/zh-Hans/images/modeset.png").is_file());
    // The GR IV guides show the Mode Set screen but no GR III menu screenshot.
    assert!(out.join("gr4/en/images/modeset.png").is_file());
    assert!(!out.join("gr4/en/images/grkit-root.jpg").exists());
    // The output must be new.
    assert!(!run(&["guides", "--out", text(&out)]).status.success());
}

#[test]
fn cameras_lists_the_runtime_build_of_each_release() {
    let cameras = success(&["cameras"])["cameras"].clone();
    let gr3x = cameras
        .as_array()
        .unwrap()
        .iter()
        .find(|camera| camera["id"] == "gr3x")
        .unwrap();
    assert_eq!(gr3x["modeset"], "00078490.609");
    assert_eq!(
        gr3x["releases"],
        serde_json::json!([
            {"firmware": "1.60", "target": "gr3-2.10", "experimental": false},
            {"firmware": "1.70", "target": "gr3-2.10", "experimental": true},
        ])
    );
}

#[test]
fn writes_an_identity_hald_without_overwriting() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("identity.png");
    let summary = success(&["effects", "hald", "--level", "4", "--out", text(&out)]);
    assert_eq!(
        (summary["level"].as_u64(), summary["side"].as_u64()),
        (Some(4), Some(64))
    );
    let cube = grkit_core::cube::Cube::read(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(cube.size, 16);
    let again = run(&["effects", "hald", "--level", "4", "--out", text(&out)]);
    assert!(!again.status.success());
    assert!(!run(&[
        "effects",
        "hald",
        "--level",
        "17",
        "--out",
        text(&tmp.path().join("b.png"))
    ])
    .status
    .success());
}

#[test]
fn release_plan_builds_each_target_and_packages_each_camera() {
    let plan = success(&["release-plan"]);
    let targets: Vec<_> = plan["runtimes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|runtime| runtime["target"].as_str().unwrap())
        .collect();
    assert_eq!(
        targets,
        ["gr3-2.10", "gr4-1.11", "gr4-1.20", "gr4-1.20-0528"]
    );
    assert_eq!(plan["runtimes"][2]["zip"], "gr4_v120.zip");
    assert_eq!(plan["runtimes"][2]["file"], "fwdc248b.bin");
    assert_eq!(plan["runtimes"][2]["official"], true);
    // A build that Ricoh does not publish comes only from the private backup.
    assert_eq!(plan["runtimes"][3]["zip"], "gr4_v120_0528.zip");
    assert_eq!(plan["runtimes"][3]["file"], "RTOS.BIN");
    assert_eq!(plan["runtimes"][3]["official"], false);
    // Each camera has one package, with each of its builds. The names are
    // those of the release assets.
    assert_eq!(
        plan["packages"],
        serde_json::json!([
            {"camera": "gr3", "targets": ["gr3-2.10"], "name": "gr3"},
            {"camera": "gr3x", "targets": ["gr3-2.10"], "name": "gr3x"},
            {"camera": "gr4", "targets": ["gr4-1.11", "gr4-1.20", "gr4-1.20-0528"], "name": "gr4"},
        ])
    );
}
