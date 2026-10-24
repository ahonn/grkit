# GRKit

[English](README.md) | 简体中文 | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md)

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit 为 RICOH GR 相机增加胶片风格和拍摄辅助功能。它从存储卡加载，不改写相机固件。

![ADJ 模式中的 GRKit 页面](https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.jpg)

照片：nagi usano 拍摄的 [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)，[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)。

## 功能

GRKit 在 ADJ 模式中增加第 6 个功能 GRK，包含五个模块：

- **影像控制（Image Control）**：胶片风格，例如 Portra 400、Velvia 100 和 Tri-X 400。可以在实时显示中预览效果。
- **画幅（Framing）**：在所有机型上拍摄 XPan（65:24）、2.35:1、16:9、4:3、5:4 或 1:1 照片。相机自带的画幅使用相机的 [长宽比]。其他画幅在显示屏上把照片以外的区域变暗。
- **辅助线（Guides）**：三分法、中心、黄金分割或对角线。
- **加印日期摄影（Date Imprint）**：在照片上加印橙色日期，效果类似胶片相机的数据后背。
- **颗粒感（Grain）**：为照片添加胶片颗粒感，有弱、中、强三档。

最后一行显示 GRKit 的版本号。

制作安装包时，你可以选择包含哪些模块。菜单只显示这些模块。

## 支持的机型

| 机型 | 固件 | 状态 |
| --- | --- | --- |
| RICOH GR III | 2.10 | 已经过真机测试 |
| RICOH GR III | 2.20 | 实验性 |
| RICOH GR III HDF | 2.10 | 已支持 |
| RICOH GR III HDF | 2.20 | 实验性 |
| RICOH GR IIIx | 1.60 | 已支持 |
| RICOH GR IIIx | 1.70 | 实验性 |
| RICOH GR IIIx HDF | 1.60 | 已支持 |
| RICOH GR IIIx HDF | 1.70 | 实验性 |
| RICOH GR IV | 1.11 | 已经过真机测试 |
| RICOH GR IV | 1.20 | 实验性 |
| RICOH GR IV HDF | 1.11 | 已支持 |
| RICOH GR IV HDF | 1.20 | 实验性 |
| RICOH GR IV Monochrome | 1.00 | 已支持，不含影像控制 |
| RICOH GR IV Monochrome | 1.10 | 实验性，不含影像控制 |

“已支持”表示该机型与经过真机测试的机型使用同一个运行时，但本身未经真机测试。“实验性”表示该固件版本未经真机测试。

GR III 2.20 和 GR IIIx 1.70 使用与 2.10 和 1.60 相同的包。GR IV 1.11 和 1.20 使用同一个包，GRKit 在每次开机时识别相机的固件。HDF 机型运行基础机型的固件，使用同一个包。GR IV Monochrome 运行 GR IV 的固件，但显示自己的版本号：1.00 对应 GR IV 1.11，1.10 对应 GR IV 1.20。不支持其他机型和固件版本。你可以在相机菜单中查看固件版本。

同一个固件版本可能有不同的构建。如果 GRKit 在受支持的相机上不加载，[固件导出指南](resources/DUMP.zh-Hans.md)说明了如何发送相机的固件映像。

> [!WARNING]
> GRKit 是实验性项目，使用风险由你自行承担。
> GRKit 只在相机开机期间修改内存，关机后修改即失效。安装过程会打开相机的 SD 脚本设置，这个设置会保持打开，直到你把它关闭。
> 如果 GRKit 出错，相机可能在启动时停止响应。安装前请先阅读[恢复指南](resources/RECOVERY.zh-Hans.md)。

## 安装

在 [grkit.app](https://grkit.app) 选择相机、模块和效果，然后下载 `GRKit.zip`。

每个安装包都包含对应机型的安装指南，格式为 PDF，语言是下载时选择的语言。恢复指南在其末尾。源文件：

- [GR III、GR IIIx 及其 HDF 机型](resources/INSTALLATION.zh-Hans.md)
- [GR IV、GR IV HDF 和 GR IV Monochrome](resources/INSTALLATION-GR4.zh-Hans.md)

要更新 GRKit，把新安装包中 `SD_ROOT` 的内容复制到存储卡，覆盖旧文件。设置和效果保存在 `GRKIT` 中，不会丢失。
要移除 GRKit，请先关闭相机，再删除存储卡中的 `SCRIPT` 和 `GRKIT` 文件夹。`GRKIT` 保存 GRKit 的设置。
要关闭 SD 脚本设置，请使用 [grkit.app](https://grkit.app) 安装步骤下方提供的文件。具体步骤见安装指南。

## 从源码构建

你需要 Rust 1.92 或更高版本、支持 ARM 的 Clang 和 LLD，以及 Ricoh 官方固件文件。仓库中不包含固件。

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
sh .github/scripts/install-guide-tools.sh build/guide-tools
sh .github/scripts/build-guides.sh grkit build/guide-tools build/guides
grkit package --build build/runtime --effects build/effects --guide build/guides/gr3/en.pdf --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

`install-guide-tools.sh` 会下载 Typst 和字体，支持 Linux x86_64 和 macOS arm64。其他机型、减少插件和其他指南语言的打包方法见 [CONTRIBUTING.md](CONTRIBUTING.md#build-packages)（英文）。

## 参与贡献

环境配置、测试和提交规则见 [CONTRIBUTING.md](CONTRIBUTING.md)。
如果你在真机上测试了 GRKit，请提交一个 Device report issue，说明结果是通过还是失败。GR IIIx 和 HDF 机型的报告最有帮助。

## 文档（英文）

- [架构](docs/architecture.md)
- [发布流程](docs/releasing.md)
- [效果](effects/README.md)
- [网站](web/README.md)

## 许可

代码采用 [MIT 许可证](LICENSE)。每个安装包都包含 `LICENSES.md`，列出包内文件的许可。

- 效果采用 CC BY-SA 4.0 或 MIT 许可。[effects/NOTICE.md](effects/NOTICE.md) 列出每个效果的许可。
- 图标文字使用 [Inter](https://github.com/rsms/inter) 字体，采用 [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt)。
- 效果拟合方法移植自 [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods)，采用 [MIT 许可证](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)。

RICOH 和 GR 是 Ricoh Company, Ltd. 的商标。胶片名称是其所有者的商标。GRKit 是独立项目，与 Ricoh 及任何胶片厂商无关，也未获得它们的认可或支持。
