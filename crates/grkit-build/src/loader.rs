use crate::{
    firmware::FirmwareImage,
    relocation::{self, Planner, Region, Source},
    target, Anchor, PatchPlan,
};
use anyhow::{ensure, Context, Result};
use grkit_core::{
    artwork::{off_pixels, plugins_pixels, version_pixels},
    runtime::{
        icon_stride, HOST_ICON_COUNT, ICONS_PER_CHUNK, LINUX_DATA_DIRECTORY, LINUX_DIRECTORY,
        MAX_CHUNKS, MAX_COUNT,
    },
};
use std::fmt::Write;

fn array(output: &mut String, name: &str, bytes: &[u8], is_const: bool) {
    let qualifier = if is_const { "const " } else { "" };
    write!(
        output,
        "static {qualifier}u8 {name}[] __attribute__((aligned(4))) = {{"
    )
    .unwrap();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write!(output, "{byte}").unwrap();
    }
    output.push_str("};\n");
}
fn words(output: &mut String, name: &str, values: impl IntoIterator<Item = u32>) {
    write!(output, "static const u32 {name}[] = {{").unwrap();
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write!(output, "{value:#x}").unwrap();
    }
    output.push_str("};\n");
}

/// The generated installer header. `version` is the GRKit version that the
/// About page of the menu shows.
pub fn header(plan: &PatchPlan, body: &FirmwareImage, version: &str) -> Result<String> {
    let target = body.target;
    let modules = target.layout.is_some();
    // A family with processing hooks has Image Control and its native data.
    let image_control = match target.layout {
        Some(_) if target.family.processing_source.is_some() => Some(
            target
                .image_control
                .with_context(|| format!("{} lacks native Image Control data", target.firmware))?,
        ),
        _ => None,
    };
    let mut constants = vec![
        ("MAX_COUNT", MAX_COUNT as u32),
        ("MAX_CHUNKS", MAX_CHUNKS as u32),
        ("ICONS_PER_CHUNK", ICONS_PER_CHUNK as u32),
        ("ICON_STRIDE", icon_stride(target.name)? as u32),
        ("HOST_ICON_COUNT", HOST_ICON_COUNT as u32),
        ("MAP_BASE", 0x51000000),
        ("MAP_SIZE", 0xf000000),
        ("ARRAY_COUNT", target.array_count),
        ("ARRAY_TABLE", target.array_table),
        ("HEAP_MIN", target.heap_min),
        ("HEAP_END", target.heap_end),
    ];
    let mut stock_record = None;
    if modules {
        constants.push(("ENGLISH_SLOT", target.english_slot));
        if let Some(native) = image_control {
            let (record_address, record) = body.stock_record()?;
            stock_record = Some(record);
            constants.extend([
                ("STOCK_RECORD", record_address),
                ("TABLE_BYTES", native.table_bytes),
                ("SETTINGS_DOMAIN", native.settings_domain),
                ("SETTINGS_IC_TYPE", native.settings_type),
                ("IC_TYPE_COUNT", native.type_count),
            ]);
        }
        let name = if target.pixel_base_indirect {
            "ICON_DRAWER_SLOT"
        } else {
            "ICON_PIXEL_BASE_SLOT"
        };
        constants.push((name, target.pixel_base_slot));
    }
    // The installer relocates a runtime with modules: it finds these
    // addresses in the running image.
    let relocatable = modules;
    let mut addresses = Vec::new();
    if relocatable {
        constants.retain(|&(name, value)| {
            let moves = matches!(
                name,
                "ARRAY_COUNT"
                    | "ARRAY_TABLE"
                    | "HEAP_MIN"
                    | "ENGLISH_SLOT"
                    | "STOCK_RECORD"
                    | "SETTINGS_DOMAIN"
                    | "ICON_DRAWER_SLOT"
                    | "ICON_PIXEL_BASE_SLOT"
            );
            if moves {
                addresses.push((name, value));
            }
            !moves
        });
    }
    constants.extend([
        ("PATCH_COUNT", plan.patches.len() as u32),
        ("CODE_RELOCATION_COUNT", plan.relocations.len() as u32),
    ]);
    let mut result = String::new();
    if !modules {
        // The installer then prepares only the private header.
        result.push_str("#define GRKIT_NO_MODULES 1\n");
    } else if image_control.is_none() {
        // The installer prepares the menu artwork but no effect records.
        result.push_str("#define GRKIT_NO_IMAGE_CONTROL 1\n");
    }
    // The package names its runtime build with this string (packaging).
    writeln!(
        result,
        "static const char target_marker[] __attribute__((used)) = \"{}{}\";",
        grkit_core::packaging::TARGET_MARKER,
        target.name
    )?;
    if target.icon_descriptor_bytes()? == 12 {
        // GR IV icon descriptors: width and a valid flag, height, pixel offset.
        result.push_str("#define GRKIT_ICON_DESCRIPTOR_12 1\n");
    }
    if let Some(page) = target.native_tone_page {
        // GR IV also records the native ImageTone values 0x8100 and up.
        writeln!(result, "#define GRKIT_NATIVE_TONE_PAGE {page:#x}")?;
    }
    for (name, value) in constants {
        writeln!(result, "#define {name} {value:#x}U")?;
    }
    if relocatable {
        relocation_tables(&mut result, plan, body, &addresses)?;
    }
    // `--known` installs only on a build that the runtime names.
    writeln!(
        result,
        "#define RTOS_IDENTITY {:#x}U",
        target::RTOS_IDENTITY
    )?;
    writeln!(
        result,
        "#define RTOS_IDENTITY_COUNT {}U",
        target.identities.len()
    )?;
    if !target.identities.is_empty() {
        words(
            &mut result,
            "rtos_identities",
            target.identities.iter().copied(),
        );
    }
    // The effects folder of the package is the inbox of the effects library.
    let paths = [
        ("INBOX_PATH", LINUX_DIRECTORY, "/EFFECTS"),
        ("PLAN_PATH", LINUX_DIRECTORY, "/MODULE.BIN"),
        ("PLAN_TMP_PATH", LINUX_DIRECTORY, "/MODULE.TMP"),
        ("LOCK_PATH", LINUX_DIRECTORY, "/INSTALL.LCK"),
        ("DONE_PATH", LINUX_DIRECTORY, "/INSTALL.OK"),
        ("FAILED_PATH", LINUX_DIRECTORY, "/INSTALL.ERR"),
        ("PLUGINS_PATH", LINUX_DIRECTORY, "/PLUGINS.TXT"),
        ("PACKAGE_PATH", LINUX_DIRECTORY, "/PACKAGE.TXT"),
        ("DATA_PATH", LINUX_DATA_DIRECTORY, ""),
        ("LIBRARY_PATH", LINUX_DATA_DIRECTORY, "/EFFECTS"),
        ("INDEX_PATH", LINUX_DATA_DIRECTORY, "/EFFECTS.TXT"),
        ("IMPORTED_PATH", LINUX_DATA_DIRECTORY, "/IMPORTED.TXT"),
    ];
    for (name, directory, suffix) in paths {
        writeln!(
            result,
            "#define {name} {}",
            serde_json::to_string(&format!("{directory}{suffix}"))?
        )?;
    }
    if modules {
        if let Some(record) = stock_record {
            array(&mut result, "stock_record", record, true);
        }
        array(&mut result, "off_pixels", &off_pixels(), true);
        array(&mut result, "plugins_pixels", &plugins_pixels(), true);
        array(&mut result, "version_pixels", &version_pixels(), true);
        array(
            &mut result,
            "module_pixels",
            &grkit_core::plugins::tiles().concat(),
            true,
        );
        // The installer copies the menu text table after the picture scratch.
        let text = grkit_core::menu_text::table(version)?;
        writeln!(result, "#define MENU_TEXT_BYTES {}U", text.len())?;
        array(&mut result, "menu_text", &text, true);
        // The live-view labels follow the menu text.
        let labels = grkit_core::artwork::overlay_labels();
        writeln!(result, "#define LABELS_BYTES {}U", labels.len())?;
        array(&mut result, "overlay_labels", &labels, true);
    }
    if let Some(native) = image_control {
        let mut tables = Vec::new();
        for address in native.tables {
            tables.extend_from_slice(body.read(address, native.table_bytes as usize)?);
        }
        array(&mut result, "stock_tables", &tables, true);
    }
    words(
        &mut result,
        "code_relocations",
        plan.relocations
            .iter()
            .flat_map(|(index, offset)| [*index as u32, *offset as u32]),
    );
    for (i, patch) in plan.patches.iter().enumerate() {
        // The installer replaces the stock bytes of a relocated patch with
        // those of the running image.
        let fixed = patch.site.is_none();
        array(&mut result, &format!("before_{i}"), &patch.before, fixed);
        array(&mut result, &format!("after_{i}"), &patch.after, false);
    }
    // Kind 1 marks a patch after the decoded RTOS image. The installer does
    // not require its stock bytes, because a restart keeps earlier GRKit code.
    let image_end = target::RTOS_BASE + body.body.len() as u32;
    // The points of each plugin, in plugin order.
    words(
        &mut result,
        "plugin_points",
        grkit_core::plugins::PLUGINS
            .iter()
            .map(grkit_core::plugins::Plugin::point_mask),
    );
    result.push_str("static struct patch patches[] = {\n");
    for (i, patch) in plan.patches.iter().enumerate() {
        writeln!(
            result,
            "{{{:#x},{},{},before_{i},after_{i}}},",
            patch.address,
            patch.after.len(),
            u32::from(patch.address >= image_end)
        )?;
    }
    result.push_str("};\n");
    // The extension points of each patch, in a separate table so the patch
    // table keeps its format.
    words(
        &mut result,
        "patch_points",
        plan.patches.iter().map(|patch| patch.points),
    );
    Ok(result)
}

/// The symbol of a firmware address that the private code or the installer
/// uses, by the region that holds it.
pub(crate) fn firmware_symbol(planner: &mut Planner, address: u32) -> Result<usize> {
    let target = planner.target();
    if address == target.heap_min {
        planner.startup(4)
    } else if address >= relocation::DATA_START + target.data_size {
        planner.referenced(address)
    } else if address >= relocation::DATA_START {
        // A table of pointers has no unique content, but code loads its start.
        planner
            .content(address)
            .or_else(|_| planner.referenced(address))
    } else {
        planner.code(address)
    }
}

/// Whether the word at `address` of the pinned image changes when code or
/// data moves: a branch, or a MOVW or MOVT of a firmware address.
fn moves(body: &FirmwareImage, address: u32) -> Result<bool> {
    let word = |address: u32| -> Result<u32> {
        Ok(u32::from_le_bytes(body.read(address, 4)?.try_into()?))
    };
    let stock = word(address)?;
    if (stock >> 25) & 7 == 5 {
        return Ok(true);
    }
    let movw = stock & 0x0ff0_0000 == 0x0300_0000;
    if !movw && stock & 0x0ff0_0000 != 0x0340_0000 {
        return Ok(false);
    }
    // The other half of the pair: the MOVT after a MOVW, or the MOVW before a
    // MOVT, of the same register within five words.
    let register = stock & 0xf000;
    for distance in 1..=5u32 {
        let other = if movw {
            address + distance * 4
        } else {
            address - distance * 4
        };
        let other_word = word(other)?;
        let kind = if movw { 0x0340_0000 } else { 0x0300_0000 };
        if other_word & 0x0ff0_0000 == kind && other_word & 0xf000 == register {
            let (low, high) = if movw {
                (stock, other_word)
            } else {
                (other_word, stock)
            };
            let value = relocation::move_immediate(high) << 16 | relocation::move_immediate(low);
            return Ok((0x5300_0000..0x5600_0000).contains(&value));
        }
    }
    Ok(false)
}

/// The relocation tables of the installer (`runtime/relocation.h`). Each
/// address in `addresses` becomes a macro of its resolved symbol.
fn relocation_tables(
    result: &mut String,
    plan: &PatchPlan,
    body: &FirmwareImage,
    addresses: &[(&str, u32)],
) -> Result<()> {
    const NONE: u32 = 0xffff;
    let target = body.target;
    let layout = target.layout.context("A relocated target has a layout")?;
    let mut planner = Planner::new(body)?;
    let tail = planner.tail(layout.tail.0);
    // The hooks that replace a branch: the private code calls its target.
    let mut targets = std::collections::BTreeMap::new();
    for patch in &plan.patches {
        if let (Some(Anchor::Firmware(site)), Ok(word)) =
            (patch.site, <[u8; 4]>::try_from(patch.before.as_slice()))
        {
            if let Some(address) = relocation::branch_target(site, u32::from_le_bytes(word)) {
                targets.insert(address, site);
            }
        }
    }
    let anchor = |planner: &mut Planner, anchor: Anchor| -> Result<usize> {
        match anchor {
            Anchor::Tail => Ok(tail),
            Anchor::Firmware(address) => match targets.get(&address) {
                Some(&site) => planner.branch_target(site),
                None => firmware_symbol(planner, address),
            },
        }
    };
    let mut sites = Vec::new();
    // How the installer compares the stock word of a firmware site: 1 under
    // the mask, for a branch or a MOVW or MOVT of a firmware address, which
    // can move, or 0 exactly, for a constant that the private code relies on.
    let mut compare = Vec::new();
    for patch in &plan.patches {
        sites.push(match patch.site {
            Some(site) => anchor(&mut planner, site)? as u32,
            None => NONE,
        });
        compare.push(match patch.site {
            Some(Anchor::Firmware(address)) => {
                ensure!(
                    patch.before.len() == 4,
                    "A relocated patch at {address:#x} is not one word"
                );
                u32::from(moves(body, address)?)
            }
            _ => 0,
        });
    }
    let mut references = Vec::new();
    for reference in &plan.references {
        let symbol = anchor(&mut planner, reference.anchor)?;
        ensure!(
            sites[reference.patch] != NONE,
            "A reference lies in a fixed patch"
        );
        references.push([
            reference.patch as u32,
            reference.offset as u32,
            symbol as u32,
            u32::from(reference.branch),
        ]);
    }
    // The installer addresses.
    let mut defines = Vec::new();
    for &(name, address) in addresses {
        let (base, offset) = target
            .relocation_bases
            .iter()
            .find(|(field, _)| *field == address)
            .map_or((address, 0), |&(field, base)| (base, field - base));
        let symbol = firmware_symbol(&mut planner, base)?;
        defines.push(format!(
            "#define {name} (relocated[{symbol}] + {offset:#x}U)"
        ));
    }
    let mut tables = Vec::new();
    let mut initialized = Vec::new();
    if let Some(native) = target.image_control {
        for address in native.tables {
            tables.push(planner.content(address)? as u32);
        }
        for address in native.record_tail {
            initialized.push(planner.content(address)? as u32);
        }
    }
    // The relocation checks the startup literals. Each other guard is a MOVW
    // and MOVT pair whose address must not lie after the image, where the
    // private code goes.
    let mut guards = Vec::new();
    for &(address, expected) in layout.guards {
        if address == relocation::STARTUP_LITERALS {
            continue;
        }
        let bytes = hex::decode(expected)?;
        ensure!(
            bytes.len() == 8 && body.read(address, 8)? == bytes,
            "Firmware guard differs at {address:#x}"
        );
        let movw = u32::from_le_bytes(bytes[..4].try_into()?);
        let movt = u32::from_le_bytes(bytes[4..].try_into()?);
        ensure!(
            movw & 0x0ff0_0000 == 0x0300_0000 && movt & 0x0ff0_0000 == 0x0340_0000,
            "A guard is a MOVW and MOVT pair"
        );
        guards.push(planner.code(address)? as u32);
    }
    let relocation = planner.finish();
    let startup =
        relocation::startup_window(&relocation).context("The relocation has no startup window")?;

    // The tables, in the order of runtime/relocation.h.
    let mut pool = Vec::new();
    let mut windows = String::new();
    for window in &relocation.windows {
        writeln!(
            windows,
            "{{{:#x},{},{},{},{},{}}},",
            window.start,
            pool.len(),
            window.words.len(),
            window.probe,
            u32::from(window.region == Region::Data),
            u32::from(window.search)
        )?;
        pool.extend_from_slice(&window.words);
    }
    words(result, "relocation_words", pool);
    writeln!(
        result,
        "static const struct relocation_window relocation_windows[] = {{\n{windows}}};"
    )?;
    words(
        result,
        "relocation_symbols",
        relocation.symbols.iter().map(|symbol| symbol.address),
    );
    result.push_str("static const struct relocation_source relocation_sources[] = {\n");
    let mut count = 0;
    for (index, symbol) in relocation.symbols.iter().enumerate() {
        ensure!(
            !symbol.sources.is_empty(),
            "Symbol {:#x} has no source",
            symbol.address
        );
        for source in &symbol.sources {
            // kind, window, b, a
            let (kind, window, b, a) = match *source {
                Source::Window { window, offset } => (0, window as u32, NONE, offset),
                Source::Branch {
                    window,
                    index,
                    verify,
                } => (1, window as u32, verify.map_or(NONE, |v| v as u32), index),
                Source::Literal { window, index } => (2, window as u32, NONE, index),
                Source::Pair { window, movw, movt } => (3, window as u32, movt, movw),
                Source::Startup { index } => (4, 0, NONE, index),
                Source::Tail => (5, 0, NONE, 0),
            };
            writeln!(result, "{{{index},{kind},0,{window},{b:#x},{a}}},")?;
            count += 1;
        }
    }
    result.push_str("};\n");
    writeln!(result, "#define GRKIT_RELOCATE 1")?;
    writeln!(
        result,
        "#define RELOCATION_SYMBOL_COUNT {}U",
        relocation.symbols.len()
    )?;
    writeln!(
        result,
        "#define RELOCATION_WINDOW_COUNT {}U",
        relocation.windows.len()
    )?;
    writeln!(
        result,
        "static const struct relocation_tables relocation_tables = {{relocation_windows,{},relocation_words,relocation_symbols,{},relocation_sources,{count},{startup},{:#x},{:#x}}};",
        relocation.windows.len(),
        relocation.symbols.len(),
        relocation::RADIUS,
        layout.tail.1 - layout.tail.0,
    )?;
    result.push_str("static u32 relocated[RELOCATION_SYMBOL_COUNT];\n");
    words(result, "patch_symbols", sites);
    words(result, "patch_compare", compare);
    writeln!(
        result,
        "#define RELOCATION_REFERENCE_COUNT {}U",
        references.len()
    )?;
    words(
        result,
        "relocation_references",
        references.into_iter().flatten(),
    );
    for define in defines {
        writeln!(result, "{define}")?;
    }
    if !tables.is_empty() {
        words(result, "table_symbols", tables);
        result.push_str("#define TABLE_ADDRESS(i) relocated[table_symbols[i]]\n");
        words(result, "initialized_symbols", initialized);
    }
    writeln!(result, "#define GUARD_PAIR_COUNT {}U", guards.len())?;
    words(result, "guard_symbols", guards);
    Ok(())
}
