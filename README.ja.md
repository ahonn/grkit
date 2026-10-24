# GRKit

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | 日本語

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit は、RICOH GR カメラにフィルムルックと撮影ツールを追加します。メモリーカードから読み込まれ、カメラのファームウェアには書き込みません。

![ADJ モードの GRKit のページ](https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.jpg)

写真：nagi usano による [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/)、[CC BY 2.0](https://creativecommons.org/licenses/by/2.0/)。

## 機能

GRKit は ADJ モードに 6 番目の項目 GRK を追加します。GRK には 5 つのモジュールがあります。

- **イメージコントロール（Image Control）**：Portra 400、Velvia 100、Tri-X 400 などのフィルムルックです。撮影画面でルックをプレビューできます。
- **フレーミング（Framing）**：すべての機種で XPan（65:24）、2.35:1、16:9、4:3、5:4、1:1 の写真を撮影できます。カメラにある比率はカメラの［アスペクト比］を使います。ほかの比率では、撮影画面で写真の範囲外が暗く表示されます。
- **グリッド（Guides）**：9 分割、画面中央、黄金比、対角線の線を表示します。
- **日付入れ撮影（Date Imprint）**：フィルムカメラのデータバックのように、写真にオレンジ色の日付を入れます。
- **粒状感（Grain）**：写真にフィルムの粒状感を加えます。強さは 3 段階です。

最後の行には GRKit のバージョンが表示されます。

モジュールはパッケージの作成時に選びます。メニューには、選んだモジュールだけが表示されます。

## 対応カメラ

| カメラ | ファームウェア | 状態 |
| --- | --- | --- |
| RICOH GR III | 2.10 | 実機でテスト済み |
| RICOH GR III | 2.20 | 試験的 |
| RICOH GR III HDF | 2.10 | 対応済み |
| RICOH GR III HDF | 2.20 | 試験的 |
| RICOH GR IIIx | 1.60 | 対応済み |
| RICOH GR IIIx | 1.70 | 試験的 |
| RICOH GR IIIx HDF | 1.60 | 対応済み |
| RICOH GR IIIx HDF | 1.70 | 試験的 |
| RICOH GR IV | 1.11 | 実機でテスト済み |
| RICOH GR IV | 1.20 | 試験的 |
| RICOH GR IV HDF | 1.11 | 対応済み |
| RICOH GR IV HDF | 1.20 | 試験的 |
| RICOH GR IV Monochrome | 1.00 | 対応済み。イメージコントロールなし |
| RICOH GR IV Monochrome | 1.10 | 試験的。イメージコントロールなし |

「対応済み」のカメラは、実機でテスト済みのカメラと同じランタイムで動作しますが、そのカメラ自体はテストしていません。「試験的」のファームウェアバージョンは、実機でテストしていません。

GR III 2.20 と GR IIIx 1.70 は、2.10 と 1.60 と同じパッケージを使います。GR IV 1.11 と 1.20 は同じパッケージを使い、GRKit は起動のたびにカメラのファームウェアを識別します。HDF モデルはベースのカメラと同じファームウェアで動作し、同じパッケージを使います。GR IV Monochrome は GR IV のファームウェアで動作し、独自のバージョンを表示します。1.00 は GR IV 1.11、1.10 は GR IV 1.20 です。ほかの機種やファームウェアのバージョンには対応していません。ファームウェアのバージョンは、カメラのメニューで確認できます。

対応するファームウェアのバージョンにも、別のビルドがあることがあります。対応するカメラで GRKit が読み込まれない場合は、[ファームウェア書き出しガイド](resources/DUMP.ja.md)の手順でファームウェアイメージを送ってください。

> [!WARNING]
> GRKit は試験的なプロジェクトです。自己責任で使用してください。
> GRKit がカメラのメモリーを変更するのは、電源を切るまでの間だけです。インストールするとカメラの SD スクリプトの設定がオンになり、オフにするまでオンのままです。
> GRKit に問題が起きると、起動の途中でカメラが止まることがあります。インストールの前に[復旧ガイド](resources/RECOVERY.ja.md)を読んでください。

## インストール

パッケージを作成するには、[grkit.app](https://grkit.app) を開きます。カメラ、モジュール、エフェクトを選び、`GRKit.zip` をダウンロードします。

各パッケージには、そのカメラのインストールガイドが、選んだ言語の PDF で入っています。復旧ガイドはその末尾にあります。ソースファイルは次のとおりです。

- [GR III、GR IIIx とそれぞれの HDF モデル](resources/INSTALLATION.ja.md)
- [GR IV、GR IV HDF、GR IV Monochrome](resources/INSTALLATION-GR4.ja.md)

GRKit をアップデートするには、新しいパッケージの `SD_ROOT` の中身をカードにコピーし、古いファイルを上書きします。設定とエフェクトは `GRKIT` に残ります。
GRKit を削除するには、カメラの電源を切ってから、カードの `SCRIPT` フォルダーと `GRKIT` フォルダーを削除します。`GRKIT` には GRKit の設定が入っています。
SD スクリプトの設定をオフにするには、[grkit.app](https://grkit.app) のインストール手順の下にあるファイルを使います。手順はインストールガイドにあります。

## ソースからのビルド

Rust 1.92 以降、ARM に対応した Clang と LLD、Ricoh の公式ファームウェアファイルが必要です。リポジトリにはファームウェアは含まれていません。

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
sh .github/scripts/install-guide-tools.sh build/guide-tools
sh .github/scripts/build-guides.sh grkit build/guide-tools build/guides
grkit package --build build/runtime --effects build/effects --guide build/guides/gr3/en.pdf --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

`install-guide-tools.sh` は Typst とフォントをダウンロードします。Linux x86_64 と macOS arm64 に対応しています。ほかのカメラ、プラグインを減らす場合、ほかの言語のガイドについては [CONTRIBUTING.md](CONTRIBUTING.md#build-packages)（英語）を参照してください。

## コントリビューション

セットアップ、テスト、プルリクエストのルールは [CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。
GRKit を実機でテストした場合は、Device report の Issue を作成し、成功か失敗かの結果を報告してください。GR IIIx と HDF モデルの報告が特に役立ちます。

## ドキュメント（英語）

- [アーキテクチャ](docs/architecture.md)
- [リリース手順](docs/releasing.md)
- [エフェクト](effects/README.md)
- [ウェブサイト](web/README.md)

## ライセンス

コードは [MIT ライセンス](LICENSE)で提供されています。各パッケージには、含まれるファイルのライセンスを記載した `LICENSES.md` が入っています。

- エフェクトのライセンスは CC BY-SA 4.0 または MIT です。各エフェクトのライセンスは [effects/NOTICE.md](effects/NOTICE.md) に記載されています。
- アイコンの文字には、[Inter](https://github.com/rsms/inter) 書体を [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt) のもとで使用しています。
- エフェクトのフィッティング手法は、[gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods) から [MIT ライセンス](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)のもとで移植したものです。

RICOH および GR は、Ricoh Company, Ltd. の商標です。フィルム名は、それぞれの所有者の商標です。GRKit は独立したプロジェクトであり、Ricoh やフィルムメーカーとは提携しておらず、承認や支援も受けていません。
