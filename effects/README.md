# Image Control effects

Each directory with an `effect.json` defines one effect. The directory name is the stable effect ID. The camera keeps the selected effect and its Fn adjustments by this ID. A refit keeps the ID. Give an effect a new ID only when its look changes so much that it is no longer the same effect. Its saved settings then reset. The site and the release packages publish every effect in this directory. [NOTICE.md](NOTICE.md) gives the source and the license of each effect.

Effects run in the camera's processing, not on the JPEG: the matrix acts before the Standard base gamma, and the curves and `post_matrix` act after it. Effects add no grain or glow.

The compiler writes self-contained GRFX0001 files with a 40 x 40 icon. `golden.json` pins the compiled files and icons. Change it only for an intentional change of a definition or of the artwork.

## Definition fields

| Field | Required | Value |
|---|---|---|
| `schema` | Yes | `grkit-effect-v1` |
| `effect_id` | Yes | The stable ID, the same as the directory name |
| `name` | Yes | 1 to 16 printable ASCII characters |
| `flags` | Yes | 0, or 1 for monochrome |
| `matrix` | Yes | Nine signed Q13 coefficients, row-major. Each row sums to 8192. |
| `curves` or `curve_knots` | Yes | Three 256-sample curves of 0 to 16383, or knots for each channel |
| `icon` | Yes | A label tile, a film box, or a motion picture film label. See [Icons](#icons). |
| `attribution` | Yes | The origin, source, and license |
| `post_matrix` | No | Nine signed Q13 coefficients, row-major, after the tone curves |
| `selective_color` | No | Keeps the chroma of one hue range. `crates/grkit-core/src/selective.rs` defines the fields. |
| `hue_sectors` | No | The camera's hue-sector stage. `crates/grkit-core/src/hue.rs` defines the fields. |

A monochrome effect cannot have `post_matrix`, `selective_color`, or `hue_sectors`.

## Icons

| Form | Fields |
|---|---|
| Label tile | `label` (1 to 4 capitals or digits) and `shape` (`film`, `filter`, or `plain`) |
| Film box | `template: "box"`, `brand`, `name`, an optional `number`, and `band` |
| Motion picture film label | `template: "cine"`, `brand`, `name`, and an optional `number` |

- The icon shows the words of the effect name and the brand, in capitals. A cover without `number` shows the name large.
- A motion picture film label shows the film number in place of the series name: Vision3 500T shows `5219` above `500T`. The target description must give that number.
- Stock codes, papers, and process details go in the target description.
- If a text does not fit, the compiler stops with an error.

## Sources

Each published effect is a fit of one film simulation LUT. `sources/` holds the scripts that bake the LUTs, the upstream notices, and `manifest.json`, which pins each LUT by SHA-256. The LUTs are not in the repository.

To refit an effect:

1. Bake its LUT with the script and the generator revision in `sources/manifest.json`. For a HaldCLUT, download the file at its `upstream` path.
2. Compare the SHA-256 with the manifest. Floating-point results can differ on another platform.
3. Run `grkit effects import` with the `input` and `exposure` of the old `attribution`.
4. Restore `icon` and the `license`, `target`, and `generator` of `attribution`. Then update `golden.json`.

## Import a LUT

`grkit effects import` fits a `.cube` LUT or a HaldCLUT image (PNG or TIFF) to the effect model and writes `OUT/ID/effect.json`. The fit needs the Standard base gamma, which the command reads from your copy of the official GR III 2.10 firmware. The repository contains no firmware data.

```sh
grkit effects import --lut look.cube --firmware fwdc239b.bin --input standard \
  --id my-look-v1 --name "My Look" --label FV --shape film --out effects
```

`--input` selects the encoding of the LUT input: `standard` (a GR JPEG in Standard), `srgb`, `linear`, `vlog`, `flog`, `flog2`, or `flog2c`. The default, `auto`, guesses Standard or a log from the data. `--exposure` shifts the scene light before the LUT. `grkit effects import --help` lists every option.

Check the grade and the warnings before you use an imported effect, then add the license of the LUT to `attribution`. The site runs the same fit in the browser. See [LUT import](../web/README.md#lut-import).

The fit follows [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods) ([MIT](../crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)). `crates/grkit-core/src/fit/` describes the model.
