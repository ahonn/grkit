# GRKit

English | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md)

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit adds film looks and shooting tools to RICOH GR cameras. It loads from the SD card and does not write the camera firmware.

![The GRKit page in the ADJ menu](https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.jpg)

Photo: [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/) by nagi usano, [CC BY 2.0](https://creativecommons.org/licenses/by/2.0/).

## Features

GRKit adds a sixth entry, GRK, to the ADJ menu. It has five modules:

- **Image Control**: film looks such as Portra 400, Velvia 100, and Tri-X 400. The viewfinder previews the look.
- **Framing**: XPan (65:24), 2.35:1, 16:9, 4:3, 5:4, or 1:1 photos, on every camera. A ratio that the camera has uses its own aspect ratio. For the other ratios, the viewfinder dims the area outside the photo.
- **Guides**: thirds, center, golden ratio, or diagonal lines.
- **Date Imprint**: an orange date on the photo, like a film camera's data back.
- **Grain**: film grain on the photo, in three strengths.

The last row shows the GRKit version.

You select the modules when you make a package. The menu shows only those modules.

## Supported cameras

| Camera | Firmware | Status |
| --- | --- | --- |
| RICOH GR III | 2.10 | Tested on a camera |
| RICOH GR III | 2.20 | Experimental |
| RICOH GR III HDF | 2.10 | Supported |
| RICOH GR III HDF | 2.20 | Experimental |
| RICOH GR IIIx | 1.60 | Supported |
| RICOH GR IIIx | 1.70 | Experimental |
| RICOH GR IIIx HDF | 1.60 | Supported |
| RICOH GR IIIx HDF | 1.70 | Experimental |
| RICOH GR IV | 1.11 | Tested on a camera |
| RICOH GR IV | 1.20 | Experimental |
| RICOH GR IV HDF | 1.11 | Supported |
| RICOH GR IV HDF | 1.20 | Experimental |
| RICOH GR IV Monochrome | 1.00 | Supported, without Image Control |
| RICOH GR IV Monochrome | 1.10 | Experimental, without Image Control |

A supported camera runs the same runtime as a tested camera, but is not tested itself. An experimental firmware version has not been tested on a camera.

GR III 2.20 and GR IIIx 1.70 use the same package as 2.10 and 1.60. GR IV 1.11 and 1.20 use one package: at each power-on, GRKit finds the firmware of the camera. Each HDF model runs the firmware of its base camera and uses the same package. GR IV Monochrome runs the firmware of GR IV and shows its own version: 1.00 is GR IV 1.11, and 1.10 is GR IV 1.20. Other models and firmware versions are not supported. To see the firmware version, open the camera menu.

A camera can run another build of a supported firmware version. If GRKit does not load on a supported camera, the [firmware dump guide](resources/DUMP.md) tells you how to send its firmware image.

> [!WARNING]
> GRKit is experimental. Use it at your own risk.
> GRKit changes the camera memory only until the camera turns off. The installation turns on the camera's SD script setting, and this setting stays on until you turn it off.
> If GRKit fails, the camera can stop during startup. Read the [recovery guide](resources/RECOVERY.md) before you install GRKit.

## Install

To make a package, go to [grkit.app](https://grkit.app). Select the camera, the modules, and the effects, then download `GRKit.zip`.

Each package contains the installation guide for its camera as a PDF, in the language that you select. The recovery guide is at its end. The sources are:

- [GR III, GR IIIx, and their HDF models](resources/INSTALLATION.md)
- [GR IV, GR IV HDF, and GR IV Monochrome](resources/INSTALLATION-GR4.md)

To update GRKit, copy the contents of `SD_ROOT` of the new package over the old files. Your settings and effects stay in `GRKIT`.
To remove GRKit, turn the camera off and delete the `SCRIPT` and `GRKIT` folders from the card. `GRKIT` holds your GRKit settings.
To turn the SD script setting off, use the files that [grkit.app](https://grkit.app) gives below the installation steps. The installation guide gives the steps.

## Build from source

You need Rust 1.92 or later, Clang and LLD with ARM support, and the official firmware file from Ricoh. The repository does not contain firmware.

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
sh .github/scripts/install-guide-tools.sh build/guide-tools
sh .github/scripts/build-guides.sh grkit build/guide-tools build/guides
grkit package --build build/runtime --effects build/effects --guide build/guides/gr3/en.pdf --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

`install-guide-tools.sh` downloads Typst and the fonts, on Linux x86_64 or macOS arm64. For other cameras, fewer plugins, and other guide languages, see [Build packages](CONTRIBUTING.md#build-packages).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the setup, the tests, and the pull request rules.
If you test GRKit on a camera, open a device report issue with the result, as a pass or a failure. Reports for GR IIIx and the HDF models are the most useful.

## Documentation

- [Architecture](docs/architecture.md)
- [Release process](docs/releasing.md)
- [Effects](effects/README.md)
- [Web site](web/README.md)

## License

The code is licensed under the [MIT License](LICENSE). Each package contains `LICENSES.md`, which gives the licenses of its files.

- The effects are licensed under CC BY-SA 4.0 or MIT. [effects/NOTICE.md](effects/NOTICE.md) gives the license of each effect.
- The icon text uses the [Inter](https://github.com/rsms/inter) typeface under the [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt).
- The effect fit is ported from [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods) under the [MIT License](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt).

RICOH and GR are trademarks of Ricoh Company, Ltd. Film names are trademarks of their owners. GRKit is an independent project. It is not affiliated with, endorsed by, or supported by Ricoh or by a film manufacturer.
