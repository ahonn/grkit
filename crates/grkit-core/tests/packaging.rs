use std::io::{Cursor, Read, Write};

use grkit_core::{camera, modules::Selection, packaging::*, runtime, testing, Files};
use serde_json::{json, Value};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

fn runtime_fixture() -> Files {
    testing::runtime_files(&testing::installer(), "gr3-2.10")
}

fn runtime() -> VerifiedRuntime {
    testing::runtime("gr3-2.10")
}

/// A stand-in for a guide PDF, which the release builds with Typst.
const PDF: &[u8] = b"%PDF-1.7\n% GRKit test guide\n";

fn english() -> Guide<'static> {
    Guide {
        language: Language::English,
        pdf: PDF,
    }
}

fn package() -> Vec<u8> {
    assemble(
        &[runtime()],
        &testing::effect_set(&["test-effect"]),
        Selection::ALL,
        &camera::GR3,
        &english(),
    )
    .unwrap()
    .bytes
}

fn with_sums(mut files: Files) -> Files {
    let sums = files
        .iter()
        .filter(|(name, _)| name.as_str() != "SHA256SUMS")
        .map(|(name, bytes)| format!("{}  {name}\n", sha256(bytes)))
        .collect::<String>();
    files.insert("SHA256SUMS".into(), sums.into_bytes());
    files
}

fn unpack(bytes: &[u8]) -> Files {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut files = Files::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        files.insert(entry.name().unwrap().into_owned(), bytes);
    }
    files
}

fn repack(files: &Files, compression: CompressionMethod) -> Vec<u8> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, blob) in files {
        archive
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(compression),
            )
            .unwrap();
        archive.write_all(blob).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

#[test]
fn package_round_trip_is_deterministic_and_preserves_metadata() {
    let first = package();
    assert_eq!(first, package());
    let verified = verify_package(&first).unwrap();
    assert_eq!(verified.effect_ids, ["test-effect"]);
    assert_eq!(verified.targets, ["gr3-2.10"]);
    let mut archive = ZipArchive::new(Cursor::new(first)).unwrap();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).unwrap();
        assert_eq!(entry.compression(), CompressionMethod::Stored);
        assert_eq!(entry.last_modified().unwrap().year(), 2026);
        let name = entry.name().unwrap();
        let expected = if name.ends_with(".BIN") || name.ends_with(".SH") {
            0o100755
        } else {
            0o100644
        };
        assert_eq!(entry.unix_mode(), Some(expected));
    }
}

#[test]
fn packages_ship_only_the_guide_the_licenses_and_checksums() {
    let files = unpack(&package());
    let documents: Vec<_> = files
        .keys()
        .filter(|name| !name.starts_with("SD_ROOT/"))
        .map(String::as_str)
        .collect();
    assert_eq!(
        documents,
        ["GRKit Installation Guide.pdf", "LICENSES.md", "SHA256SUMS"]
    );
    assert_eq!(files["GRKit Installation Guide.pdf"], PDF);
}

#[test]
fn manifests_notices_and_extra_documents_are_rejected() {
    let files = unpack(&package());
    assert!(verify_package(&repack(
        &with_sums(files.clone()),
        CompressionMethod::Stored
    ))
    .is_ok());
    for name in [
        "MANIFEST.json",
        "NOTICES/EFFECTS.md",
        "RECOVERY.md",
        "INSTALLATION.md",
        "images/grkit-root.jpg",
    ] {
        let mut extra = files.clone();
        extra.insert(name.into(), b"{}".to_vec());
        assert!(
            verify_package(&repack(&with_sums(extra), CompressionMethod::Stored)).is_err(),
            "{name}"
        );
    }
}

#[test]
fn legacy_deflated_packages_remain_supported() {
    let files = unpack(&package());
    assert!(verify_package(&repack(&files, CompressionMethod::Deflated)).is_ok());
}

#[test]
fn corrupted_profiles_documents_manifests_and_sums_are_rejected() {
    let original = unpack(&package());
    let profile = original.keys().find(|name| name.ends_with(".GRF")).unwrap();
    for name in [
        profile.as_str(),
        "GRKit Installation Guide.pdf",
        "LICENSES.md",
        "SHA256SUMS",
    ] {
        let mut files = original.clone();
        files.get_mut(name).unwrap().push(b' ');
        assert!(
            verify_package(&repack(&files, CompressionMethod::Stored)).is_err(),
            "{name}"
        );
    }
    let mut bytes = package();
    bytes.truncate(bytes.len() / 2);
    assert!(verify_package(&bytes).is_err());
}

#[test]
fn paths_and_unexpected_entries_are_rejected() {
    for path in [
        "",
        "/absolute",
        "../outside",
        "a/../b",
        "a/./b",
        "a//b",
        "a/",
        "a\\b",
        "C:a",
        "a\0b",
        "中文",
    ] {
        assert!(safe_path(path).is_err(), "{path:?}");
    }
    for name in [
        "../outside",
        "SD_ROOT/EXTRA",
        "installation.md",
        "NOTICES/nested/extra",
    ] {
        let mut files = unpack(&package());
        files.insert(name.into(), b"bad".to_vec());
        assert!(
            verify_package(&repack(&files, CompressionMethod::Stored)).is_err(),
            "{name}"
        );
    }
}

#[test]
fn entry_limits_and_required_files_are_enforced() {
    let mut files = unpack(&package());
    files.remove("SD_ROOT/DEVELOP.MOD");
    assert!(validate_contents(&files).is_err());
    let mut files = unpack(&package());
    files.insert("EXTRA/huge".into(), vec![0; MAX_FILE_SIZE + 1]);
    assert!(validate_contents(&files).is_err());
    let mut files = unpack(&package());
    files.insert("EXTRA/one".into(), vec![0; MAX_FILE_SIZE]);
    files.insert("EXTRA/two".into(), vec![0; MAX_FILE_SIZE]);
    assert!(validate_contents(&files).is_err());
    let mut files = unpack(&package());
    for index in 0..MAX_ENTRIES {
        files.insert(format!("EXTRA/{index}"), vec![]);
    }
    assert!(verify_package(&repack(&files, CompressionMethod::Stored)).is_err());
}

#[test]
fn production_boundaries_are_enforced() {
    for (key, value) in [
        ("profile", "diagnostic"),
        ("startup_activation", "test-effect"),
        ("target", "other"),
    ] {
        let files = runtime_fixture();
        let mut altered = manifest("gr3-2.10", &files);
        altered[key] = json!(value);
        assert!(load_runtime(&json_bytes(&Value::Object(altered)).unwrap(), &files).is_err());
    }
    for suffix in [b"--select test-effect".as_slice(), b"changed"] {
        let mut altered = runtime_fixture();
        altered
            .get_mut("SCRIPT/STARTUP.TTL")
            .unwrap()
            .extend_from_slice(suffix);
        assert!(validate_runtime(&altered, "gr3-2.10").is_err());
    }
}

#[test]
fn target_capacity_and_empty_effect_selection_are_supported() {
    let mut effects = EffectSet::default();
    for i in 0..runtime::MAX_COUNT {
        effects
            .insert(testing::effect(&format!("effect-{i}")))
            .unwrap();
    }
    let bytes = assemble(
        &[runtime()],
        &effects,
        Selection::ALL,
        &camera::GR3,
        &english(),
    )
    .unwrap()
    .bytes;
    assert_eq!(
        verify_package(&bytes).unwrap().effect_ids.len(),
        runtime::MAX_COUNT
    );
    let error = effects
        .insert(testing::effect("effect-overflow"))
        .unwrap_err();
    assert!(error.to_string().contains("at most 20 effects"), "{error}");
    let bytes = assemble(
        &[runtime()],
        &EffectSet::default(),
        Selection::ALL,
        &camera::GR3,
        &english(),
    )
    .unwrap()
    .bytes;
    assert_eq!(
        verify_package(&bytes).unwrap().effect_ids,
        Vec::<String>::new()
    );
}

#[test]
fn selected_plugins_are_packaged_and_verified() {
    let effects = testing::effect_set(&["test-effect"]);
    let plugins = Selection::from_names(["date-stamp", "image-control"]).unwrap();
    let bytes = assemble(&[runtime()], &effects, plugins, &camera::GR3, &english())
        .unwrap()
        .bytes;
    let files = unpack(&bytes);
    assert_eq!(
        files["SD_ROOT/SCRIPT/PLUGINS/PLUGINS.TXT"],
        b"image-control\ndate-stamp\n"
    );
    assert_eq!(
        verify_package(&bytes).unwrap().plugins.names(),
        ["image-control", "date-stamp"]
    );
    // A package with every plugin has the file too, so a copy over an older
    // package always replaces its selection.
    let plain = assemble(
        &[runtime()],
        &effects,
        Selection::ALL,
        &camera::GR3,
        &english(),
    )
    .unwrap()
    .bytes;
    assert_eq!(
        unpack(&plain)["SD_ROOT/SCRIPT/PLUGINS/PLUGINS.TXT"],
        Selection::ALL.file()
    );
    assert_eq!(
        verify_package(&plain).unwrap().plugins.names(),
        grkit_core::modules::NAMES
    );
    assert_eq!(plain, package());
}

#[test]
fn invalid_plugin_files_and_empty_menus_are_rejected() {
    for text in [
        b"hdr\n".as_slice(),
        b"guides guides",
        b"Framing",
        b"framing,guides",
        b"\xff",
        // Nothing to show.
        b"",
    ] {
        let mut files = unpack(&package());
        files.insert("SD_ROOT/SCRIPT/PLUGINS/PLUGINS.TXT".into(), text.to_vec());
        assert!(
            validate_contents(&with_sums(files)).is_err(),
            "{}",
            String::from_utf8_lossy(text)
        );
    }
    let mut files = unpack(&package());
    files.insert(
        "SD_ROOT/SCRIPT/PLUGINS/PLUGINS.TXT".into(),
        b" guides\r\n\tframing ".to_vec(),
    );
    assert_eq!(
        validate_contents(&with_sums(files))
            .unwrap()
            .plugins
            .names(),
        ["framing", "guides"]
    );
    // Image Control alone needs effects.
    let image_control = Selection::from_names(["image-control"]).unwrap();
    assert!(assemble(
        &[runtime()],
        &EffectSet::default(),
        image_control,
        &camera::GR3,
        &english()
    )
    .is_err());
    let bytes = assemble(
        &[runtime()],
        &testing::effect_set(&["test-effect"]),
        image_control,
        &camera::GR3,
        &english(),
    )
    .unwrap()
    .bytes;
    assert_eq!(
        verify_package(&bytes).unwrap().plugins.names(),
        ["image-control"]
    );
}

#[test]
fn malformed_manifest_records_are_rejected_without_panicking() {
    for value in [
        json!([]),
        json!(null),
        json!(42),
        json!({"files": null}),
        json!({"files": [null]}),
        json!({"files": [{"path": "x", "size": true, "sha256": "0".repeat(64)}]}),
        json!({"files": [{"path": "x", "size": -1, "sha256": "0".repeat(64)}]}),
        json!({"files": [{"path": "x", "size": 1.0, "sha256": "0".repeat(64)}]}),
    ] {
        assert!(
            read_manifest(&json_bytes(&value).unwrap()).is_err(),
            "{value}"
        );
    }
    assert!(read_manifest(br#"{"files":[],"files":[]}"#).is_err());
    assert!(read_manifest(
        &json_bytes(&json!({"files": [
            {"path": "X", "size": 0, "sha256": "0".repeat(64)},
            {"path": "x", "size": 0, "sha256": "0".repeat(64)}
        ]}))
        .unwrap()
    )
    .is_err());
}

#[test]
fn json_matches_existing_ascii_pretty_format() {
    assert_eq!(
        json_bytes(&json!({"z": [1, true], "a": "中😀\u{7f}"})).unwrap(),
        b"{\n  \"a\": \"\\u4e2d\\ud83d\\ude00\\u007f\",\n  \"z\": [\n    1,\n    true\n  ]\n}\n"
    );
}

#[test]
fn runtime_layout_matches_target_capacity() {
    assert_eq!(runtime::MAX_COUNT, 20);
    assert_eq!(
        runtime::layout(0).unwrap(),
        json!({"words": 43682, "icon_words": [41652]})
    );
    assert_eq!(
        runtime::layout(20).unwrap(),
        json!({"words": 65442, "icon_words": [64080, 9612]})
    );
    assert!(runtime::layout(21).is_err());
}

#[test]
fn allocation_includes_host_icons_at_chunk_boundaries() {
    for count in 0..=runtime::MAX_COUNT {
        let layout = runtime::layout(count).unwrap();
        let words = layout["words"].as_u64().unwrap() as usize;
        let icons: Vec<_> = layout["icon_words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        // Picture scratch (the date stamp masks, then the shot ring), the menu
        // text, then the live-view labels.
        assert_eq!(
            words * 4,
            512 + count * 0x1100 + 141376 + 72 + 16384 + 16384,
            "count {count}"
        );
        assert_eq!(
            icons.iter().sum::<usize>() * 4,
            (count + 26) * 6408,
            "count {count}"
        );
        assert!(std::iter::once(&words)
            .chain(&icons)
            .all(|v| (1..=65536).contains(v)));
        // The 26 host icons come first, so effects after the fourteenth
        // continue in a second 40-icon chunk.
        assert_eq!(
            icons.len(),
            if count <= 14 { 1 } else { 2 },
            "count {count}"
        );
    }
    assert!(runtime::layout(usize::MAX).is_err());
}

#[test]
fn duplicate_central_directory_entries_are_not_hidden_by_the_zip_index() {
    let original = package();
    let end = original.len() - 22;
    assert_eq!(&original[end..end + 4], b"PK\x05\x06");
    let start = u32::from_le_bytes(original[end + 16..end + 20].try_into().unwrap()) as usize;
    let variable: usize = [28, 30, 32]
        .iter()
        .map(|offset| {
            u16::from_le_bytes(
                original[start + offset..start + offset + 2]
                    .try_into()
                    .unwrap(),
            ) as usize
        })
        .sum();
    let record = &original[start..start + 46 + variable];
    let mut mutated = original[..end].to_vec();
    mutated.extend_from_slice(record);
    mutated.extend_from_slice(&original[end..]);
    let new_end = end + record.len();
    for offset in [8, 10] {
        let count = u16::from_le_bytes(
            mutated[new_end + offset..new_end + offset + 2]
                .try_into()
                .unwrap(),
        );
        mutated[new_end + offset..new_end + offset + 2].copy_from_slice(&(count + 1).to_le_bytes());
    }
    let size = u32::from_le_bytes(mutated[new_end + 12..new_end + 16].try_into().unwrap());
    mutated[new_end + 12..new_end + 16]
        .copy_from_slice(&(size + record.len() as u32).to_le_bytes());
    assert!(verify_package(&mutated).is_err());
}

#[test]
fn symlink_and_encrypted_entries_are_rejected() {
    for encrypted in [false, true] {
        let mut bytes = package();
        let end = bytes.len() - 22;
        let central = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
        if encrypted {
            bytes[central + 8] |= 1;
        } else {
            bytes[central + 5] = 3;
            bytes[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
        }
        assert!(verify_package(&bytes).is_err());
    }
}

#[test]
fn runtime_builds_must_match_their_manifest_exactly() {
    let files = runtime_fixture();
    let manifest = |files: &Files| testing::manifest("gr3-2.10", files);
    let runtime = load_runtime(&manifest(&files), &files).unwrap();
    assert_eq!(runtime.files(), &files);
    assert_eq!(runtime.target(), "gr3-2.10");

    let mut extra = files.clone();
    extra.insert("SCRIPT/EXTRA.SH".into(), b"x".to_vec());
    let mut missing = files.clone();
    missing.remove("SCRIPT/STARTUP.TTL");
    let mut changed = files.clone();
    changed.insert("SCRIPT/STARTUP.TTL".into(), b"changed".to_vec());
    for (actual, expected) in [
        (&extra, "differs from its manifest"),
        (&missing, "differs from its manifest"),
        (&changed, "hash or size mismatch"),
    ] {
        let message = format!("{:#}", load_runtime(&manifest(&files), actual).unwrap_err());
        assert!(message.contains(expected), "{message}");
    }
    let unknown = format!("{:#}", load_runtime(&manifest(&extra), &extra).unwrap_err());
    assert!(
        unknown.contains("Invalid runtime manifest entries"),
        "{unknown}"
    );
}

#[test]
fn each_camera_gets_its_modeset_file_and_guide() {
    let effects = testing::effect_set(&["test-effect"]);
    let gr3 = unpack(&package());
    let bytes = assemble(
        &[runtime()],
        &effects,
        Selection::ALL,
        &camera::GR3X,
        &english(),
    )
    .unwrap()
    .bytes;
    let verified = verify_package(&bytes).unwrap();
    assert_eq!(verified.camera.id, "gr3x");
    assert_eq!(verified.targets, ["gr3-2.10"]);
    let gr3x = unpack(&bytes);
    assert!(gr3x.contains_key("SD_ROOT/00078490.609"));
    assert!(!gr3x.contains_key("SD_ROOT/00078350.588"));
    // The guide source names GR IIIx, GR IIIx HDF, and their ModeSet file.
    let guide = String::from_utf8(installation_guide(&camera::GR3X, Language::English)).unwrap();
    assert!(guide.contains("For RICOH GR IIIx and GR IIIx HDF firmware 1.60 or 1.70 only."));
    assert!(guide.contains("`00078490.609`") && !guide.contains("00078350.588"));
    // The GR III guide is the resource with each release of its runtime build.
    assert_eq!(
        String::from_utf8(installation_guide(&camera::GR3, Language::English)).unwrap(),
        include_str!("../../../resources/INSTALLATION.md").replace(
            "GR III HDF firmware 2.10 only.",
            "GR III HDF firmware 2.10 or 2.20 only."
        )
    );
    // Every runtime and effect file is the same for both cameras.
    for (name, blob) in &gr3 {
        if !name.ends_with(".588") && name != "SHA256SUMS" {
            assert_eq!(&gr3x[name], blob, "{name}");
        }
    }
}

#[test]
fn the_guide_names_the_camera_once_and_the_modeset_file_twice() {
    // installation_guide replaces these strings, so each must occur exactly once.
    for (language, guide) in [
        (
            Language::English,
            include_str!("../../../resources/INSTALLATION.md"),
        ),
        (
            Language::SimplifiedChinese,
            include_str!("../../../resources/INSTALLATION.zh-Hans.md"),
        ),
        (
            Language::TraditionalChinese,
            include_str!("../../../resources/INSTALLATION.zh-Hant.md"),
        ),
        (
            Language::Japanese,
            include_str!("../../../resources/INSTALLATION.ja.md"),
        ),
    ] {
        // The resource names the first GR III release; the package guide
        // names each release of the runtime build.
        let phrase = language.camera_phrase(&camera::GR3, &["2.10"]);
        assert_eq!(guide.matches(&phrase).count(), 1, "{phrase}");
        let source = String::from_utf8(installation_guide(&camera::GR3, language)).unwrap();
        let releases = language.camera_phrase(&camera::GR3, &["2.10", "2.20"]);
        assert_eq!(source.matches(&releases).count(), 1, "{releases}");
        assert_eq!(guide.matches(camera::GR3.hdf).count(), 1, "{language:?}");
        // After the installation, and in the steps that turn SD scripts off.
        assert_eq!(
            guide.matches(camera::GR3.modeset).count(),
            2,
            "{language:?}"
        );
    }
}

#[test]
fn the_gr4_guide_names_each_release_once() {
    // installation_guide replaces the phrase, so it must occur exactly once.
    for (language, guide) in [
        (
            Language::English,
            include_str!("../../../resources/INSTALLATION-GR4.md"),
        ),
        (
            Language::SimplifiedChinese,
            include_str!("../../../resources/INSTALLATION-GR4.zh-Hans.md"),
        ),
        (
            Language::TraditionalChinese,
            include_str!("../../../resources/INSTALLATION-GR4.zh-Hant.md"),
        ),
        (
            Language::Japanese,
            include_str!("../../../resources/INSTALLATION-GR4.ja.md"),
        ),
    ] {
        let phrase = language.camera_phrase(&camera::GR4, &["1.11"]);
        assert_eq!(guide.matches(&phrase).count(), 1, "{phrase}");
        let source = String::from_utf8(installation_guide(&camera::GR4, language)).unwrap();
        let releases = language.camera_phrase(&camera::GR4, &["1.11", "1.20"]);
        assert_eq!(source.matches(&releases).count(), 1, "{releases}");
    }
}

#[test]
fn a_package_has_the_guide_of_its_language_only() {
    let effects = testing::effect_set(&["test-effect"]);
    for language in Language::ALL {
        let bytes = assemble(
            &[runtime()],
            &effects,
            Selection::ALL,
            &camera::GR3X,
            &Guide { language, pdf: PDF },
        )
        .unwrap()
        .bytes;
        assert_eq!(verify_package(&bytes).unwrap().language, language);
        let files = unpack(&bytes);
        assert_eq!(files[&language.guide_path()], PDF);
        let guides = files.keys().filter(|name| name.ends_with(".pdf")).count();
        assert_eq!(guides, 1, "{language:?}");
        // The guide source names GR IIIx and GR IIIx HDF in each language.
        let source = String::from_utf8(installation_guide(&camera::GR3X, language)).unwrap();
        assert!(source.contains(&language.camera_phrase(&camera::GR3X, &["1.60", "1.70"])));
        assert_eq!(source.matches(camera::GR3X.hdf).count(), 1, "{language:?}");
        assert!(!source.contains(camera::GR3.hdf));
        assert!(source.contains(camera::GR3X.modeset) && !source.contains(camera::GR3.modeset));
    }
    assert_eq!(
        Language::ALL.map(Language::guide_path),
        [
            "GRKit Installation Guide.pdf",
            "GRKit 安装指南.pdf",
            "GRKit 安裝指南.pdf",
            "GRKit インストールガイド.pdf",
        ]
    );
    // The GR IV guide names both releases of GR IV; each has its own build.
    assert_eq!(
        String::from_utf8(installation_guide(
            &camera::GR4,
            Language::TraditionalChinese
        ))
        .unwrap(),
        include_str!("../../../resources/INSTALLATION-GR4.zh-Hant.md")
            .replace("GR IV HDF 韌體 1.11。", "GR IV HDF 韌體 1.11 或 1.20。")
    );
}

#[test]
fn a_package_has_exactly_one_pdf_guide() {
    let files = unpack(&package());
    let mut two = files.clone();
    two.insert(Language::SimplifiedChinese.guide_path(), PDF.to_vec());
    assert!(verify_package(&repack(&with_sums(two), CompressionMethod::Stored)).is_err());
    let mut none = files.clone();
    none.remove("GRKit Installation Guide.pdf");
    assert!(verify_package(&repack(&with_sums(none), CompressionMethod::Stored)).is_err());
    let mut text = files;
    text.insert("GRKit Installation Guide.pdf".into(), b"# GRKit\n".to_vec());
    assert!(verify_package(&repack(&with_sums(text), CompressionMethod::Stored)).is_err());
    let text_guide = Guide {
        language: Language::English,
        pdf: b"# GRKit\n",
    };
    assert!(assemble(
        &[runtime()],
        &EffectSet::default(),
        Selection::ALL,
        &camera::GR3,
        &text_guide,
    )
    .is_err());
    assert!(Language::parse("fr").is_err());
    assert_eq!(Language::parse("ja").unwrap(), Language::Japanese);
}

#[test]
fn a_package_has_the_modeset_file_of_exactly_one_camera() {
    let mut files = unpack(&package());
    files.insert(
        "SD_ROOT/00078490.609".into(),
        files["SD_ROOT/00078350.588"].clone(),
    );
    assert!(validate_contents(&with_sums(files)).is_err());
    let mut files = unpack(&package());
    files.remove("SD_ROOT/00078350.588");
    assert!(validate_contents(&with_sums(files)).is_err());
    let mut files = unpack(&package());
    files.insert("SD_ROOT/00078350.588".into(), b"changed".to_vec());
    assert!(validate_contents(&with_sums(files)).is_err());
    assert!(camera::find("gr2").is_err());
    assert_eq!(camera::find("gr3x").unwrap(), &camera::GR3X);
    assert_eq!(camera::find("gr4").unwrap(), &camera::GR4);
}

fn gr4_runtimes() -> Vec<VerifiedRuntime> {
    testing::camera_runtimes(&camera::GR4)
}

#[test]
fn a_gr4_package_has_its_modeset_file_guide_and_modules() {
    let bytes = assemble(
        &gr4_runtimes(),
        &EffectSet::default(),
        Selection::ALL,
        &camera::GR4,
        &english(),
    )
    .unwrap()
    .bytes;
    let verified = verify_package(&bytes).unwrap();
    assert_eq!(verified.camera.id, "gr4");
    assert_eq!(verified.targets, ["gr4-1.11", "gr4-1.20", "gr4-1.20-0528"]);
    let files = unpack(&bytes);
    assert!(files.contains_key("SD_ROOT/00078560.636"));
    assert_eq!(files["GRKit Installation Guide.pdf"], PDF);
    assert_eq!(
        String::from_utf8(installation_guide(&camera::GR4, Language::English)).unwrap(),
        include_str!("../../../resources/INSTALLATION-GR4.md").replace(
            "GR IV HDF firmware 1.11 only.",
            "GR IV HDF firmware 1.11 or 1.20 only."
        )
    );
    // A GR III runtime does not run on GR IV.
    assert!(assemble(
        &[runtime()],
        &EffectSet::default(),
        Selection::ALL,
        &camera::GR4,
        &english()
    )
    .is_err());
    // GR IV has Image Control.
    let bytes = assemble(
        &gr4_runtimes(),
        &testing::effect_set(&["test-effect"]),
        Selection::ALL,
        &camera::GR4,
        &english(),
    )
    .unwrap()
    .bytes;
    assert_eq!(verify_package(&bytes).unwrap().effect_ids, ["test-effect"]);
    // GR IV has Framing, so Framing alone without effects shows a menu.
    let framing = Selection::from_names(["framing"]).unwrap();
    assert!(assemble(
        &gr4_runtimes(),
        &EffectSet::default(),
        framing,
        &camera::GR4,
        &english()
    )
    .is_ok());
    let image_control = Selection::from_names(["image-control"]).unwrap();
    assert!(assemble(
        &gr4_runtimes(),
        &EffectSet::default(),
        image_control,
        &camera::GR4,
        &english()
    )
    .is_err());
}

#[test]
fn scripts_off_packages_hold_only_the_disable_command_and_its_key() {
    for camera in camera::CAMERAS {
        let bytes = assemble_scripts_off(camera).unwrap();
        assert_eq!(bytes, assemble_scripts_off(camera).unwrap());
        let files = unpack(&bytes);
        let names: Vec<_> = files.keys().map(String::as_str).collect();
        let mut expected = [
            "SD_ROOT/DEVELOP.MOD".to_owned(),
            format!("SD_ROOT/{}", camera.modeset),
            "SHA256SUMS".to_owned(),
        ];
        expected.sort();
        assert_eq!(
            names,
            expected.iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert_eq!(
            files[&format!("SD_ROOT/{}", camera.modeset)],
            b"[SCRIPT_EN_MODE DIS]\r\n"
        );
        // The same key file as the activation, which only the command differs from.
        let activation = runtime::activation_files(camera);
        assert_eq!(files["SD_ROOT/DEVELOP.MOD"], activation["DEVELOP.MOD"]);
        assert_eq!(activation[camera.modeset], b"[SCRIPT_EN_MODE EN]\r\n");
        // It holds no GRKit code, so it is not a GRKit package.
        assert!(verify_package(&bytes).is_err());
    }
}

#[test]
fn dump_packages_hold_the_tool_files_and_their_checksums() {
    let contents: Files = DUMP_FILES
        .iter()
        .map(|name| ((*name).to_owned(), name.as_bytes().to_vec()))
        .collect();
    let bytes = assemble_dump(contents.clone()).unwrap();
    assert_eq!(bytes, assemble_dump(contents.clone()).unwrap());
    let files = unpack(&bytes);
    let mut expected: Vec<_> = DUMP_FILES.iter().copied().chain(["SHA256SUMS"]).collect();
    expected.sort();
    assert_eq!(
        files.keys().map(String::as_str).collect::<Vec<_>>(),
        expected
    );
    let sums = String::from_utf8(files["SHA256SUMS"].clone()).unwrap();
    for name in DUMP_FILES {
        assert!(sums.contains(&format!("{}  {name}", sha256(name.as_bytes()))));
    }
    // The program and the script are executable on the card.
    let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    for (name, mode) in [
        ("SD_ROOT/SCRIPT/GRDUMP/DUMP.BIN", 0o755),
        ("SD_ROOT/SCRIPT/GRDUMP/DUMP.SH", 0o755),
        ("README.txt", 0o644),
    ] {
        assert_eq!(
            archive.by_name(name).unwrap().unix_mode().unwrap() & 0o777,
            mode
        );
    }
    // A missing, empty, or extra file is refused.
    let mut missing = contents.clone();
    missing.remove("README.txt");
    assert!(assemble_dump(missing).is_err());
    let mut empty = contents.clone();
    empty.insert("SD_ROOT/SCRIPT/GRDUMP/DUMP.BIN".into(), Vec::new());
    assert!(assemble_dump(empty).is_err());
    let mut extra = contents;
    extra.insert("SD_ROOT/SCRIPT/STARTUP.BAK".into(), b"x".to_vec());
    assert!(assemble_dump(extra).is_err());
}

/// A GR IV package has the installer of each GR IV build, in release order,
/// and the script that selects one at each power-on. A GR III package keeps
/// the single installer and script of earlier packages.
#[test]
fn a_package_holds_the_installer_of_each_build_of_its_camera() {
    let gr4 = |runtimes: &[VerifiedRuntime]| {
        assemble(
            runtimes,
            &EffectSet::default(),
            Selection::ALL,
            &camera::GR4,
            &english(),
        )
        .map(|package| package.bytes)
    };
    let bytes = gr4(&gr4_runtimes()).unwrap();
    assert_eq!(
        verify_package(&bytes).unwrap().targets,
        ["gr4-1.11", "gr4-1.20", "gr4-1.20-0528"]
    );
    let files = unpack(&bytes);
    for (path, target) in [
        ("SD_ROOT/SCRIPT/PLUGINS/INSTALL.BIN", "gr4-1.11"),
        ("SD_ROOT/SCRIPT/PLUGINS/INSTALL2.BIN", "gr4-1.20"),
        ("SD_ROOT/SCRIPT/PLUGINS/INSTALL3.BIN", "gr4-1.20-0528"),
    ] {
        assert_eq!(files[path], testing::marked_installer(target), "{path}");
    }
    let script = String::from_utf8(files["SD_ROOT/SCRIPT/PLUGINS/INSTALL.SH"].clone()).unwrap();
    assert_eq!(
        script,
        "for installer in INSTALL.BIN INSTALL2.BIN INSTALL3.BIN; do\n\
         \x20 /media/storage1/SCRIPT/PLUGINS/$installer --known\n\
         \x20 status=$?\n\
         \x20 case $status in 48) ;; *) break ;; esac\n\
         done\n\
         case $status in 48)\n\
         \x20 for installer in INSTALL.BIN INSTALL2.BIN; do\n\
         \x20   /media/storage1/SCRIPT/PLUGINS/$installer --try\n\
         \x20   status=$?\n\
         \x20   case $status in 47) ;; *) break ;; esac\n\
         \x20 done\n\
         \x20 case $status in 47) /media/storage1/SCRIPT/PLUGINS/INSTALL3.BIN --install; status=$? ;; esac\n\
         \x20 ;;\n\
         esac\n\
         echo GR_MODULE_INSTALL_EXIT_$status\n\
         exit 0\n"
    );
    // The first installer scans: the plan is the same for each build. First
    // the script looks up the other installers.
    assert_eq!(
        String::from_utf8(files["SD_ROOT/SCRIPT/PLUGINS/SCAN.SH"].clone()).unwrap(),
        "for installer in INSTALL2.BIN INSTALL3.BIN; do [ -e /media/storage1/SCRIPT/PLUGINS/$installer ]; done\n\
         /media/storage1/SCRIPT/PLUGINS/INSTALL.BIN --scan\n\
         status=$?\n\
         echo GR_MODULE_SCAN_EXIT_$status\n\
         exit 0\n"
    );
    let gr3 = unpack(&package());
    assert_eq!(
        gr3["SD_ROOT/SCRIPT/PLUGINS/SCAN.SH"],
        b"/media/storage1/SCRIPT/PLUGINS/INSTALL.BIN --scan\nstatus=$?\necho GR_MODULE_SCAN_EXIT_$status\nexit 0\n"
    );
    assert_eq!(
        gr3["SD_ROOT/SCRIPT/PLUGINS/INSTALL.SH"],
        b"/media/storage1/SCRIPT/PLUGINS/INSTALL.BIN --install\nstatus=$?\necho GR_MODULE_INSTALL_EXIT_$status\nexit 0\n"
    );
    assert!(!gr3.contains_key("SD_ROOT/SCRIPT/PLUGINS/INSTALL2.BIN"));

    // Each build, in release order.
    let mut runtimes = gr4_runtimes();
    runtimes.swap(1, 2);
    assert!(format!("{:#}", gr4(&runtimes).unwrap_err()).contains("in release order"));
    assert!(gr4(&gr4_runtimes()[..1]).is_err());
    // A GR III runtime does not run on GR IV.
    assert!(gr4(&[runtime()]).is_err());
    // The installer and the build manifest must name the same build.
    let files = testing::runtime_files(&testing::marked_installer("gr4-1.20"), "gr4-1.20");
    let error = load_runtime(&testing::manifest("gr4-1.11", &files), &files).unwrap_err();
    assert!(error.to_string().contains("is the gr4-1.20 runtime"));

    // A package whose installers are swapped or missing is refused.
    let mut swapped = files_of(&bytes);
    let first = swapped["SD_ROOT/SCRIPT/PLUGINS/INSTALL.BIN"].clone();
    let second = swapped["SD_ROOT/SCRIPT/PLUGINS/INSTALL2.BIN"].clone();
    swapped.insert("SD_ROOT/SCRIPT/PLUGINS/INSTALL.BIN".into(), second);
    swapped.insert("SD_ROOT/SCRIPT/PLUGINS/INSTALL2.BIN".into(), first);
    assert!(validate_contents(&with_sums(swapped)).is_err());
    let mut missing = files_of(&bytes);
    missing.remove("SD_ROOT/SCRIPT/PLUGINS/INSTALL3.BIN");
    assert!(validate_contents(&with_sums(missing)).is_err());
}

/// The entries of a package without its checksums.
fn files_of(bytes: &[u8]) -> Files {
    let mut files = unpack(bytes);
    files.remove("SHA256SUMS");
    files
}

/// The effect list names the effect files of the package, so their effects
/// install before others that an earlier package left in the library.
#[test]
fn packages_list_their_effect_files() {
    let files = unpack(&package());
    let list = format!("SD_ROOT/{EFFECT_LIST}");
    let effects: Vec<_> = files
        .keys()
        .filter_map(|name| name.strip_prefix("SD_ROOT/SCRIPT/PLUGINS/EFFECTS/"))
        .map(|name| format!("{name}\n"))
        .collect();
    assert_eq!(effects.len(), 1);
    assert_eq!(files[&list], effects.concat().into_bytes());
    // A package without effects has an empty list.
    let none = assemble(
        &[runtime()],
        &EffectSet::default(),
        Selection::ALL.without("image-control").unwrap(),
        &camera::GR3,
        &english(),
    )
    .unwrap();
    assert_eq!(unpack(&none.bytes)[&list], b"");
    // A list that differs from the effects is rejected. A package from before
    // the effects library has no list.
    for wrong in [b"".as_slice(), b"00000000.GRF\n"] {
        let mut changed = files.clone();
        changed.insert(list.clone(), wrong.to_vec());
        assert!(validate_contents(&with_sums(changed)).is_err());
    }
    let mut legacy = files.clone();
    legacy.remove(&list);
    assert!(validate_contents(&with_sums(legacy)).is_ok());
}
