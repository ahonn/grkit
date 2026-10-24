# Architecture

GRKit adds features to Ricoh GR cameras without changing the firmware on the camera.
A package on the SD card installs private code into RAM at each power-on, and hooks connect that code to the running RTOS.
This document describes how the parts fit together. The source code and its comments hold the addresses, formats, and limits.

## Overview

```text
 developer computer / browser            camera
 ────────────────────────────            ──────────────────────────────────────────
 grkit build    ──► runtime build        Linux:  loader script ──► INSTALL
 grkit effects  ──► GRF effects                     │  checks firmware, writes code,
 grkit package  ──► GRKit.zip  ── SD card ──►       │  records, icons, and hooks
 site (wasm)    ──► GRKit.zip                       ▼
                                         RTOS:   hooks ──► menu, Image Control,
                                                 Framing, picture encode
```

There are three execution domains:

- Rust tools run on the developer computer or, through WebAssembly, in the browser.
- The installer runs in the camera's Linux, started by an SD script.
- The hooks and plugins run in the camera's RTOS.

| Path | Contents |
| --- | --- |
| `crates/grkit-core` | Formats, effect compilation, artwork, runtime templates, package assembly. No file or process access. |
| `crates/grkit-build` | Firmware checks, patch generation, Clang and LLD invocation. |
| `crates/grkit-cli` | The `grkit` command and its filesystem rules. |
| `crates/grkit-wasm` | Package assembly in the browser, with the same checks as the CLI. |
| `runtime/` | Native C and assembly: installer, hooks, menu host, plugins. |
| `resources/` | Installation guides (Markdown, compiled to PDF by the release). |
| `effects/` | Built-in effect definitions. See [effects/README.md](../effects/README.md). |
| `web/` | The static site: package builder and effect preview. See [web/README.md](../web/README.md). |

The CLI embeds `runtime/` and `resources/`, so it works outside the checkout.

## Targets

A target is one firmware build. Its module in `crates/grkit-build/src/target/` holds its data, the private tail, and the site of each hook in its build.

The targets of one camera line form a family, such as the GR IV builds. They share the runtime code and the hooks. The family module in `crates/grkit-build/src/target/` lists each hook once: its symbol, the code component that holds it, its extension point, and whether its site gets a B or a BL. A build gives only the address and the stock instruction of each hook, so the builds of a family have the same hooks.
Each family has the header `runtime/targets/<family>.h`, which each build header of the family includes. It holds the object layouts, events, and constants of the family. It also holds the values that the shared code takes from each family, such as the icon slot and the Framing ratios. The shared code has no default for them, so a new family that does not define one does not build.
The build header `runtime/targets/<target>.h` holds the firmware addresses and the values that differ in that build.

Tests require each build of a family to have a site for each hook of the family and the same header definitions. A build declares the definitions that only it has in `BUILD_ONLY_DEFINITIONS`. A change for one build therefore fails the tests until it reaches each build of the family.

| Target | Cameras |
| --- | --- |
| `gr3-2.10` | GR III 2.10 and 2.20, GR IIIx 1.60 and 1.70, and their HDF models |
| `gr4-1.11` | GR IV 1.11, GR IV HDF, GR IV Monochrome |
| `gr4-1.20` | GR IV 1.20 and GR IV HDF 1.20, GR IV Monochrome 1.10 |
| `gr4-1.20-0528` | GR IV 1.20, the factory build of GR IV 30th Anniversary Edition cameras |

GR IIIx 1.60 runs the RTOS code of GR III 2.10, so both cameras use one runtime. They differ only in the ModeSet file that enables SD scripts.
GR III 2.20 and GR IIIx 1.70 change only the end screen. Their code moves by less than the relocation radius, so INSTALL [relocates](#relocation) the `gr3-2.10` runtime to them. `camera.rs` lists the releases of each camera and the runtime build of each release.
GR IV 1.20 changes the code around several hooks and adds the monochrome Image Control types 31 and 32. It therefore has its own target (see [Porting](#porting)). Its hooks are those of 1.11.
GR IV 30th Anniversary Edition cameras have an earlier 1.20 build from the factory, with code of both 1.11 and 1.20, so it has its own target. Ricoh does not publish it, so the target builds from a dump of a camera (`Source::Dump`).
An HDF camera runs the firmware of its base camera and uses the package of that camera.
GR IV has its own layout and all plugins.

## Installation

At power-on, the loader script waits until Linux mounts the card. Then it runs two steps:

1. SCAN moves the effects of the package into the effects library, selects the effects, and writes an allocation plan that holds them. The startup script allocates the arrays and stays resident, so the arrays stay allocated.
2. INSTALL writes the records of the planned effects and the icons, publishes the code, and installs the hooks last. It writes `INSTALL.OK`, or `INSTALL.ERR` with a status.

A camera has one package. When the camera has several targets, the package holds the installer of each, and `INSTALL.SH` selects one at each power-on: first by the identity word of the running build, then by [relocation](#relocation) for a build that no target names (`runtime::package_runtime_files`).

INSTALL changes nothing on a firmware that it does not expect. Each firmware address must resolve in the running image (see [Relocation](#relocation)), and each hook site must hold its stock instruction.

Each hook belongs to one extension point. INSTALL installs only the points of the selected plugins, so a plugin that is not selected changes no firmware.
`SCRIPT/PLUGINS/PLUGINS.TXT` selects the plugins.
Each package has `PLUGINS.TXT` and `PACKAGE.TXT`, also when it selects every plugin or has no effects, and the other package files have the same names in each release. A copy of a new package over an old one, by replace or by merge, therefore leaves the files of the new package in `SCRIPT`, and the data folder keeps the settings and the effects. An update needs no delete step.

The effects library is `GRKIT/EFFECTS`, in the data folder. The package brings its effects in `SCRIPT/PLUGINS/EFFECTS`, the inbox, and lists their files in `SCRIPT/PLUGINS/PACKAGE.TXT`. When a package is new, SCAN copies its listed effects into the library, where each replaces the file with the same name. `GRKIT/IMPORTED.TXT` records the package that the library has, with a hash of each file. So a copy of the same package changes nothing, and an effect that a person deleted from the library stays deleted until a package with a different file brings it. The first power-on with the library copies every valid inbox file, because releases before the library kept the effects there. The installer deletes and renames no file in these folders: on the camera's Linux a rename does not replace a file, and on GR IV exFAT a delete made later lookups fail. Without Image Control in the selection, no effect installs.
GRKit installs the effects of the library, at most 20. The effects of the last package come first, and each package has the list. The other effects fill the remaining places in ID order. A listed effect that SCAN could not copy installs from the inbox. INSTALL takes the effects from the plan that SCAN wrote, so both steps install the same effects. SCAN writes `GRKIT/EFFECTS.TXT`, the list of the installed effects and their files, when the list changes.

`grkit_core::abi` defines the shared header, the effect records, and the picture scratch. It generates `runtime/abi.h`.

The resident loader saves the plugin selections and the Fn adjustments in `GRKIT/STATE.TXT`. It writes only after a change, and only through the camera's own file system.
`GRKIT` is the data folder of GRKit, outside `SCRIPT`. No package holds it, so a copy of a new package never replaces the settings. The loader creates it.
Each plugin names its state file (`plugins::StateFile`). The loader restores the files in a fixed order and keeps a key with the value 0 out of the file.
Releases before the data folder kept the state in `SCRIPT/PLUGINS`. The loader reads those files when the data folder has none, saves the values in the data folder at once, and then deletes the old files.
A script error frees the arrays that the hooks use, so the resident script checks each array index and divides only by constants.

## Plugins

The core is in `runtime/`: hooks, installer, menu host, and the hosts of the extension points. Each feature is a plugin in `runtime/plugins/<id>/`, linked statically into the core.
A plugin includes only `runtime/plugin.h`, and a test checks that no plugin names a firmware symbol or address.

| Plugin | Extension points |
| --- | --- |
| Image Control | `menu.page`, `tuning`, `picture.encode` |
| Framing | `menu.page`, `viewfinder.overlay`, `capture.aspect` |
| Guides | `menu.page`, `viewfinder.overlay` |
| Date Stamp | `menu.page`, `picture.encode` |
| Grain | `menu.page`, `pipeline.grain` |

A point with several plugins calls their handlers in registry order.

`grkit_core::plugins::PLUGINS` is the only list of plugins. Its order is the menu order.
The generated files (`runtime/generated/*.h`, `web/src/generated/plugins.ts`) come from the registry, and a test fails if one differs.

### Add a plugin

1. Add the C code in `runtime/plugins/<id>/`: `page.h`, and a handler file if the plugin has layers or encoders. Include only `runtime/plugin.h`.
2. In `page.h`, define `<ID>_PAGE`, with `<ID>_TILES` as its tile count.
3. Add the entry to `PLUGINS` in `crates/grkit-core/src/plugins.rs`. Declare its header words in `words`. A saved key refers to one of them with `Slot::Own`.
4. Add its tiles to `grkit_core::artwork`, its menu text to `grkit_core::menu_text`, and its site text to `web/src/i18n.ts`.
5. Run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`.
6. Run the tests. They fail if a tile count, a handler order, or an extension point does not agree with the code.

## Menu

GRKit adds a private entry to the ADJ toolbar, drawn like the native entries. The root lists the installed plugins, then About. Each plugin page owns its rows.
The private menu reuses the native ADJ value list, so it behaves like a native entry.

The menu text follows the camera language. It is data, not code, because the menu region is small.

## Image Control

Image Control selects an effect for still pictures and the live view by changing a private active-record pointer. Off keeps the native processing.

An effect (GRFX0001) replaces the Standard color matrix and tone curves. An optional extension adds a post matrix and selective color.

An effect can also drive the camera's hue-sector stage, which changes colors by hue and by luminance in still pictures and the live view. The installer writes a parameter set for each effect with sectors, and hooks point the stage at it. Effects without sectors keep the native set.
Highlight correction halves the input of the stage. The installer therefore writes a second set for that case, and the hooks pick it by the camera's flag for the capture. `grkit_core::hue` describes the stage.

After the color stages, the camera reduces the chroma of very dark and very light areas. The fit model and the site previews include this reduction, so the previews show no more tint than the photos.

Each effect record has its own Fn adjustments. Fn opens the native parameter page, and hooks redirect its model to the record, so native settings and effect settings stay separate.
Photos record the effect and its adjustments in the maker note, so playback shows them.

## Framing

Framing draws a mask and a ratio badge into the viewfinder overlay. Guides draws its lines in the same path, inside the photo area of the selected ratio.

After INSTALL, the overlay core also shows a short "GRKit" label in the first still live view, so the user sees that GRKit has loaded. The label uses the overlay hooks, so it shows only when Framing or Guides is installed.

The live-view labels, the load cue and the Framing ratios, are rendered in Roboto Condensed Light at build time, in the size and style of the native live-view text (`grkit_core::artwork::overlay_labels`). The installer copies them after the menu text, because the code region has no room for them.

Framing lists the same ratios on every camera. A ratio that the camera has natively sets the native aspect, so the native viewfinder, photo, and DNG apply it: 1:1 on GR III, and 16:9, 4:3, and 1:1 on GR IV. The page shows the ratio that the camera applies, so a change of the native aspect in the camera menu shows there too.
Framing crops the other ratios. A crop sets the native aspect to 3:2 and captures through a hidden capture aspect. The live view stays 3:2, and DNG files keep the full sensor data and describe the crop.
XPan and 2.35:1 share the 16:9 capture aspect with their own heights. 5:4 shares the 4:3 capture aspect with its own widths, which the pipeline crops from the center. Playback finds the geometry of a photo from its size.
GR IV has a live-view mode for each capture aspect, so its live-view hooks keep the 3:2 mode for a crop. In native 16:9, XPan and 2.35:1 also crop the native photos.
The camera saves a crop as a value, not as a menu row, so a release can add ratios without changing the meaning of a saved value.

## Recorded pictures

A hook on the JPEG encode processes each picture plane before encoding: the main picture, the screennail, and the thumbnail. It applies selective color, then the date stamp.
Each still build stores its effect record with its shot settings, so continuous shooting keeps the effect of each shot.
Grain sets the native grain stage of recorded pictures. The live view and DNG files do not have grain.

## Relocation

Some cameras show a supported version but run another build of it. Each runtime is built from one published image, the pinned build. INSTALL finds each firmware address in the running image and moves each reference to it.

- Each firmware address in the source is a symbol (`FIRMWARE(address)`). `grkit_build::relocation` turns the link relocations into tables.
- For each address, the build stores a window of the pinned image around it that is unique.
- INSTALL (`runtime/relocation.h`) searches each window in the running image. Each window must match once.

A failure stops INSTALL with status 47 before it changes the firmware. `INSTALL.ERR` then lists the addresses that failed. `runtime/relocation.h` gives the reason codes.

## Porting

A new firmware version needs one of two changes:

- If its code moves by less than the relocation radius, the existing target runs on it. Add the release to `camera.rs` with the existing target.
- If its code moves farther, or code at a hook changed, it needs a new target in the family of the existing one. The package of the camera then holds the installer of each target (see [Installation](#installation)).

`grkit port` finds each address of an existing target in another build. It uses the windows of [Relocation](#relocation), within a larger radius. It takes the hook and patch sites of the target module, the `FIRMWARE()` addresses of the build header, and the data addresses of `Target`. It writes drafts of the build header and of the target module for the new build.
The port cannot find an address whose code or data changed. It also does not check values that are not addresses, such as object offsets, text IDs, and enum values.

A data address that code loads gets that code as a second source, and the sources must agree. A port whose sources are all in windows of mostly addresses needs a review, because a table that grew can match at another start.
`"relocatable": true` means that the plan of the installer resolves each address of the target in the other build. A build has more symbols than the target names, so a test of the target on the new firmware must still confirm it.
The family header has no firmware addresses, because the port reads only the build header.

To add a target for a new firmware build:

1. Run `grkit port --target <target> --firmware <its firmware> --to <new firmware> --out <directory>`. Use `--to-rtos` for an RTOS image, such as a GRDUMP capture.
2. If the output gives `"relocatable": true`, add the release to the existing target instead. Test the existing target on the new firmware.
3. Make `runtime/targets/<new target>.h` from `header.h` of the output. Resolve each `PORT` comment: port the address by hand, or check the offset that an expression adds. Check each value that is not an address.
4. Save `target.rs` of the output as `crates/grkit-build/src/target/<module>.rs`. The module name is the target name with `_` for `-` and without `.`. Resolve each `PORT` comment. Give the name, the firmware, its SHA-256 values, its download, its identity word, and the header definition of the new target. Port each address that has no port, and check each value that is not an address. Add the module to `crates/grkit-build/src/target.rs`, to `TARGETS`, and to `SOURCES`.
5. If the code at a site changed, change the hook of the family so that it fits the code of each build. The runtime tests a definition that only the new build has with `#ifdef`. Add that definition to `BUILD_ONLY_DEFINITIONS`.
6. Select the header in `runtime/firmware.h`. Add the release to `camera.rs` as experimental.
7. Run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`. The site packages each target of a camera, so `web/src/cameras.ts` changes only for a new camera.
8. Add the firmware zip, or the zip of the dump, to the private firmware backup (`docs/releasing.md`). The release workflow builds each target of `grkit release-plan`, so it needs no change.
9. Run `grkit port` again with `--check <new target>`. No address may differ without a review.
10. Run the tests, build the target from the official firmware, and test it on the new firmware. Set the release to not experimental only after a test on a camera.

If a step is missing, a test names it, except steps 8 and 10.

A firmware update that cannot be undone moves each camera to the new build, so the old build gets fewer users over time.
A maintainer can freeze such a build when keeping it current costs more than it gives. A frozen build still builds and gets fixes, but a new feature does not have to reach it.

To freeze a build, let `components::hooks` leave out the hooks that the build does not get. Exempt the build in the family test for the definitions that it does not get. The runtime must then leave the feature out on that build, as the settings page does without `SETTINGS_SUBMENU`.
The C code gates a feature on a header definition, and the assembly and INSTALL follow the hooks of the build. If a build has one of them without the other, the assembly stops the build.

## Code areas

All private code is in the reservation after the RTOS image. GRKit changes no byte of the image except the hook sites and a few in-place patches.
`build.json` gives the address, size, and limit of each code component. The builder rejects overflow, code outside the reservation, changed hook instructions, and overlapping patches.

## Testing

- Rust tests pin the GRF bytes, the artwork, the generated headers, and the loader scripts.
- Host C tests check the GRF parser, the effect catalog, the menu, Framing, and the picture code. The menu, Framing, and picture tests run for each target, with the expectations of its family where the behavior differs.
- `--ignored` tests build the runtime with an ARM toolchain and a local firmware. See [CONTRIBUTING.md](../CONTRIBUTING.md).
- The synthetic `--ignored` tests need no firmware. They build a target of each family on a pseudo-random image, as the release builds them, and resolve the relocation tables of INSTALL against that image.
