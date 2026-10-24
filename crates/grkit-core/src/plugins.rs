//! The plugin registry: the one list of GRKit plugins and of the facts that the
//! core, the build, the packages, and the site need about each plugin.
//!
//! A plugin is C code in `runtime/plugins/<id>/`: a menu page (`page.h`) and,
//! optionally, one file of extension-point handlers. The core includes that
//! code at build time. `PLUGINS.TXT` selects the plugins that the installer
//! installs, and the installer changes no firmware for the others.
//!
//! `generated()` writes the registry for C (`runtime/generated/`) and for the
//! site (`web/src/generated/plugins.ts`). A test keeps the checked-in files
//! current: run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test
//! generated` after a change.

use std::fmt::Write;

use crate::{
    abi::{HOST_FRAME_RATIO, HOST_GUIDE, HOST_PLUGIN_AREA, HOST_STAMP},
    artwork, camera,
};

/// An extension point: the firmware changes that a group of plugins needs.
/// The installer applies the patches of a point only when an installed plugin
/// needs it. The hooks of each family in `grkit_build::target` give their points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    /// The private ADJ entry, its list, text, and icons.
    MenuPage,
    /// Image Control: private processing records, the effect tag, and the
    /// native Image Control pages.
    Tuning,
    /// Drawing over the still live view.
    ViewfinderOverlay,
    /// The capture aspect and the recorded picture geometry.
    CaptureAspect,
    /// Changes to the JPEG planes of a recorded picture.
    PictureEncode,
    /// The native grain stage of recorded pictures.
    PipelineGrain,
}

impl Point {
    pub const ALL: [Point; 6] = [
        Point::MenuPage,
        Point::Tuning,
        Point::ViewfinderOverlay,
        Point::CaptureAspect,
        Point::PictureEncode,
        Point::PipelineGrain,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Point::MenuPage => "menu.page",
            Point::Tuning => "tuning",
            Point::ViewfinderOverlay => "viewfinder.overlay",
            Point::CaptureAspect => "capture.aspect",
            Point::PictureEncode => "picture.encode",
            Point::PipelineGrain => "pipeline.grain",
        }
    }

    pub fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// A word of the private header that belongs to one plugin. The header holds
/// the words of all plugins after its core fields, in plugin order
/// (`GRKIT_HOST_PLUGIN_AREA`). C code names it `GRKIT_PLUGIN_<PLUGIN>_<NAME>`.
pub struct Word {
    pub name: &'static str,
    pub doc: &'static str,
}

/// Where a state value lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// The ImageTone code of the active Image Control record.
    Effect,
    /// A core field of the header, at this byte offset.
    Core(usize),
    /// A word of the plugin (`Plugin::words`), by its index.
    Own(usize),
}

/// A state file of the resident loader in `runtime::DATA_FOLDER`. The loader
/// restores the files in the order of `ALL`, so the settings of the main file
/// apply first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateFile {
    /// The selections of the plugins and the Fn adjustments of the effects.
    Main,
}

impl StateFile {
    pub const ALL: [StateFile; 1] = [StateFile::Main];

    /// The file name without its extension. The file is `<stem>.TXT`, and the
    /// loader writes `<stem>.TMP` before it replaces the file.
    pub fn stem(self) -> &'static str {
        match self {
            Self::Main => "STATE",
        }
    }
}

/// A value that a state file keeps across power-off.
pub struct StateKey {
    /// The key name. The file names it `<plugin id>.<name>`.
    pub name: &'static str,
    pub slot: Slot,
    /// The largest value. The loader restores a larger value as 0.
    pub max: u32,
}

/// A `viewfinder.overlay` layer: C functions of the plugin's handler file.
pub struct Layer {
    /// `u32 value(volatile u32 *h)`: the committed value, 0 for Off.
    pub value: &'static str,
    /// `void paint(const Surface *, u32 value)`.
    pub paint: &'static str,
    /// Paint in the live view only, not on the ADJ canvas.
    pub live_only: bool,
    /// The core paints the layers of all plugins in ascending order.
    pub order: u32,
}

/// A `picture.encode` handler: C functions of the plugin's handler file.
pub struct Encoder {
    /// `u32 wanted(volatile u32 *h, const PictureSet *)`.
    pub wanted: &'static str,
    /// `void encode(volatile u32 *h, PictureSet *)`.
    pub encode: &'static str,
    /// The core calls the encoders of all plugins in ascending order.
    pub order: u32,
}

pub struct Plugin {
    /// The name in `PLUGINS.TXT` and the directory in `runtime/plugins/`.
    pub id: &'static str,
    /// The extension points whose patches the plugin needs. A plugin with a
    /// menu page needs `MenuPage`.
    pub points: &'static [Point],
    /// The plugin shows only with effects. Without effects, the installer and
    /// the menu treat it as not installed.
    pub needs_effects: bool,
    /// The plugin's own header words.
    pub words: &'static [Word],
    pub state: &'static [StateKey],
    /// The state file that keeps the keys of `state`.
    pub state_file: StateFile,
    /// The number of artwork tiles. Tile 0 is the root tile of the page.
    pub tiles: usize,
    /// The artwork tiles, 40 x 40 RGBA each.
    pub artwork: fn() -> Vec<Vec<u8>>,
    /// The handler file in `runtime/plugins/<id>/`, if the plugin has layers or
    /// encoders. The core includes it in the component of those handlers.
    pub handlers: Option<&'static str>,
    pub layers: &'static [Layer],
    pub encoders: &'static [Encoder],
}

impl Plugin {
    /// The C name stem: `PLUGIN_<stem>`, `<stem>_PAGE`, `<stem>_TILES`.
    pub fn c_name(&self) -> String {
        self.id.replace('-', "_").to_ascii_uppercase()
    }

    /// The site key, in camel case.
    pub fn key(&self) -> String {
        let mut parts = self.id.split('-');
        let mut key = parts.next().unwrap_or_default().to_owned();
        for part in parts {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                key.push(first.to_ascii_uppercase());
                key.extend(chars);
            }
        }
        key
    }

    /// The bits (`Point::bit`) of the points that the plugin needs.
    pub fn point_mask(&self) -> u32 {
        self.points.iter().fold(0, |mask, point| mask | point.bit())
    }
}

fn no_artwork() -> Vec<Vec<u8>> {
    Vec::new()
}

/// The plugins in menu order. Plugin n is bit n of a selection
/// (`GRKIT_HOST_PLUGINS`) and module n of the menu.
pub const PLUGINS: [Plugin; 5] = [
    Plugin {
        id: "image-control",
        points: &[Point::MenuPage, Point::Tuning, Point::PictureEncode],
        needs_effects: true,
        words: &[],
        state_file: StateFile::Main,
        state: &[StateKey {
            name: "effect",
            slot: Slot::Effect,
            max: 0xffff,
        }],
        tiles: 0,
        artwork: no_artwork,
        handlers: Some("selective.c"),
        layers: &[],
        encoders: &[Encoder {
            wanted: "selective_wanted",
            encode: "selective_encode",
            order: 10,
        }],
    },
    Plugin {
        id: "framing",
        points: &[
            Point::MenuPage,
            Point::ViewfinderOverlay,
            Point::CaptureAspect,
        ],
        needs_effects: false,
        words: &[],
        state_file: StateFile::Main,
        state: &[StateKey {
            name: "ratio",
            slot: Slot::Core(HOST_FRAME_RATIO),
            max: 6,
        }],
        tiles: 7,
        artwork: artwork::framing_tiles,
        handlers: Some("framing.c"),
        layers: &[
            Layer {
                value: "framing_value",
                paint: "framing_mask",
                live_only: false,
                order: 10,
            },
            // The badge covers the guides that cross it.
            Layer {
                value: "framing_badge_value",
                paint: "framing_badge",
                live_only: true,
                order: 30,
            },
        ],
        encoders: &[],
    },
    Plugin {
        id: "guides",
        points: &[Point::MenuPage, Point::ViewfinderOverlay],
        needs_effects: false,
        words: &[],
        state_file: StateFile::Main,
        state: &[StateKey {
            name: "style",
            slot: Slot::Core(HOST_GUIDE),
            max: 5,
        }],
        tiles: 6,
        artwork: artwork::guides_tiles,
        handlers: Some("guides.c"),
        layers: &[Layer {
            value: "guides_value",
            paint: "guides_paint",
            live_only: false,
            order: 20,
        }],
        encoders: &[],
    },
    Plugin {
        id: "date-stamp",
        points: &[Point::MenuPage, Point::PictureEncode],
        needs_effects: false,
        words: &[Word {
            name: "look",
            doc: "Date Imprint look: 0 classic seven-segment digits, 1 GR1s dot-matrix \
                  digits, 2 GR II digits.",
        }],
        state_file: StateFile::Main,
        state: &[
            StateKey {
                name: "style",
                slot: Slot::Core(HOST_STAMP),
                max: 5,
            },
            StateKey {
                name: "look",
                slot: Slot::Own(0),
                max: 2,
            },
        ],
        tiles: 6,
        artwork: artwork::stamp_tiles,
        handlers: Some("stamp.c"),
        layers: &[],
        // After selective color, so the stamp keeps its own color.
        encoders: &[Encoder {
            wanted: "stamp_wanted",
            encode: "stamp_encode",
            order: 20,
        }],
    },
    Plugin {
        id: "grain",
        points: &[Point::MenuPage, Point::PipelineGrain],
        needs_effects: false,
        words: &[Word {
            name: "level",
            doc: "Grain level: 0 Off, 1 weak, 2 medium, 3 strong. The native grain \
                  stage of recorded pictures uses it instead of the native grain settings.",
        }],
        state_file: StateFile::Main,
        state: &[StateKey {
            name: "level",
            slot: Slot::Own(0),
            max: 3,
        }],
        tiles: 4,
        artwork: artwork::grain_tiles,
        handlers: None,
        layers: &[],
        encoders: &[],
    },
];

pub const COUNT: usize = PLUGINS.len();

/// The plugin names in menu order.
pub const NAMES: [&str; COUNT] = {
    let mut names = [""; COUNT];
    let mut i = 0;
    while i < COUNT {
        names[i] = PLUGINS[i].id;
        i += 1;
    }
    names
};

/// The artwork tiles of all plugins. They follow each other in menu order.
pub const TILES: usize = {
    let mut tiles = 0;
    let mut i = 0;
    while i < COUNT {
        tiles += PLUGINS[i].tiles;
        i += 1;
    }
    tiles
};

/// The bits of the plugins that show only with effects.
pub const NEEDS_EFFECTS: u32 = {
    let mut mask = 0;
    let mut i = 0;
    while i < COUNT {
        if PLUGINS[i].needs_effects {
            mask |= 1 << i;
        }
        i += 1;
    }
    mask
};

/// A plugin word with its header offset.
pub struct WordEntry {
    /// `PLUGIN_<PLUGIN>_<NAME>`, without the `GRKIT_` prefix.
    pub name: String,
    pub offset: usize,
    pub doc: &'static str,
}

/// The words of all plugins, in plugin order, from `GRKIT_HOST_PLUGIN_AREA`.
pub fn words() -> Vec<WordEntry> {
    PLUGINS
        .iter()
        .flat_map(|plugin| plugin.words.iter().map(move |word| (plugin, word)))
        .enumerate()
        .map(|(index, (plugin, word))| WordEntry {
            name: format!(
                "PLUGIN_{}_{}",
                plugin.c_name(),
                word.name.replace('-', "_").to_ascii_uppercase()
            ),
            offset: HOST_PLUGIN_AREA + 4 * index,
            doc: word.doc,
        })
        .collect()
}

/// The header offset of word `index` of plugin `plugin`.
fn own_word(plugin: usize, index: usize) -> usize {
    assert!(
        index < PLUGINS[plugin].words.len(),
        "{} has no word {index}",
        PLUGINS[plugin].id
    );
    let before: usize = PLUGINS[..plugin].iter().map(|p| p.words.len()).sum();
    HOST_PLUGIN_AREA + 4 * (before + index)
}

/// A state key with its plugin.
pub struct StateEntry {
    /// `<plugin id>.<key name>`.
    pub name: String,
    /// The header word that holds the value, or `None` for the Image Control
    /// effect, which the file keeps as the ImageTone code of the record.
    pub word: Option<usize>,
    pub max: u32,
    /// The bit of the plugin in `GRKIT_HOST_PLUGINS`.
    pub plugin: u32,
    pub file: StateFile,
}

/// The state keys of all plugins, in menu order.
pub fn state_keys() -> Vec<StateEntry> {
    PLUGINS
        .iter()
        .enumerate()
        .flat_map(|(index, plugin)| {
            plugin.state.iter().map(move |key| StateEntry {
                name: format!("{}.{}", plugin.id, key.name),
                word: match key.slot {
                    Slot::Effect => None,
                    Slot::Core(offset) => Some(offset),
                    Slot::Own(word) => Some(own_word(index, word)),
                },
                max: key.max,
                plugin: index as u32,
                file: plugin.state_file,
            })
        })
        .collect()
}

/// The artwork tiles of all plugins, in menu order. The menu addresses each
/// tile by its offset from the first tile.
pub fn tiles() -> Vec<Vec<u8>> {
    PLUGINS
        .iter()
        .flat_map(|plugin| (plugin.artwork)())
        .collect()
}

const GENERATED_C: &str = "Generated from crates/grkit-core/src/plugins.rs. Do not edit: \
    change the registry there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core \
    --test generated.";

fn c_file(guard: &str, doc: &str, body: &str) -> String {
    let mut out = String::new();
    crate::abi::comment(&mut out, GENERATED_C);
    crate::abi::comment(&mut out, doc);
    writeln!(out, "#ifndef {guard}\n#define {guard}\n").unwrap();
    out.push_str(body);
    writeln!(out, "\n#endif").unwrap();
    out
}

/// `runtime/generated/plugins.h`: constants for any C file.
fn plugins_header() -> String {
    let mut body = String::new();
    crate::abi::comment(
        &mut body,
        "Plugin n is bit n of a selection (GRKIT_HOST_PLUGINS) and module n of \
         the menu.",
    );
    for (index, plugin) in PLUGINS.iter().enumerate() {
        writeln!(body, "#define PLUGIN_{} {index}U", plugin.c_name()).unwrap();
    }
    writeln!(body, "#define PLUGIN_COUNT {COUNT}U").unwrap();
    crate::abi::comment(&mut body, "The PLUGINS.TXT names in plugin order.");
    let names: Vec<String> = NAMES.iter().map(|name| format!("\"{name}\"")).collect();
    writeln!(body, "#define PLUGIN_NAMES {}", names.join(", ")).unwrap();
    crate::abi::comment(
        &mut body,
        "The plugins that show only with effects. Without effects, the installer \
         and the menu treat them as not installed.",
    );
    writeln!(body, "#define PLUGIN_NEEDS_EFFECTS {NEEDS_EFFECTS:#x}U").unwrap();
    crate::abi::comment(
        &mut body,
        "The artwork tiles of each plugin. They follow each other in plugin order \
         and end below the GRKit tile.",
    );
    for plugin in &PLUGINS {
        writeln!(body, "#define {}_TILES {}U", plugin.c_name(), plugin.tiles).unwrap();
    }
    writeln!(body, "#define PLUGIN_TILES {TILES}U").unwrap();
    crate::abi::comment(
        &mut body,
        "The largest value of each state key. The loader restores a larger value as 0.",
    );
    for plugin in &PLUGINS {
        for key in plugin.state {
            writeln!(
                body,
                "#define {}_{}_MAX {}U",
                plugin.c_name(),
                key.name.replace('-', "_").to_ascii_uppercase(),
                key.max
            )
            .unwrap();
        }
    }
    c_file(
        "GRKIT_GENERATED_PLUGINS_H",
        "The plugins in menu order.",
        &body,
    )
}

/// `runtime/generated/pages.h`: the menu pages, for `runtime/modules.h`.
fn pages_header() -> String {
    let mut body = String::new();
    for plugin in &PLUGINS {
        writeln!(body, "#include \"../plugins/{}/page.h\"", plugin.id).unwrap();
    }
    let pages: Vec<String> = PLUGINS
        .iter()
        .map(|plugin| format!("{}_PAGE", plugin.c_name()))
        .collect();
    writeln!(body, "\n#define PLUGIN_PAGES {}", pages.join(", ")).unwrap();
    c_file(
        "GRKIT_GENERATED_PAGES_H",
        "The menu page of each plugin, in plugin order.",
        &body,
    )
}

/// The handler files of the plugins with a handler in `has`, then their
/// entries in ascending order.
fn handlers<T>(
    has: fn(&'static Plugin) -> &'static [T],
    order: fn(&T) -> u32,
) -> (String, Vec<(&'static Plugin, &'static T)>) {
    let all: &'static [Plugin] = &PLUGINS;
    let mut includes = String::new();
    let mut entries = Vec::new();
    for plugin in all {
        let list = has(plugin);
        if list.is_empty() {
            continue;
        }
        let file = plugin
            .handlers
            .expect("a plugin with handlers names its file");
        writeln!(includes, "#include \"../plugins/{}/{file}\"", plugin.id).unwrap();
        entries.extend(list.iter().map(|entry| (plugin, entry)));
    }
    entries.sort_by_key(|(_, entry)| order(entry));
    (includes, entries)
}

/// `runtime/generated/layers.h`: the `viewfinder.overlay` layers, for
/// `runtime/framing.c`.
fn layers_header() -> String {
    let (mut body, layers) = handlers(|plugin| plugin.layers, |layer| layer.order);
    body.push_str("\n#define PLUGIN_LAYERS \\\n");
    for (index, (plugin, layer)) in layers.iter().enumerate() {
        let end = if index + 1 == layers.len() { "" } else { " \\" };
        writeln!(
            body,
            "    {{PLUGIN_{},{},{},{}}},{end}",
            plugin.c_name(),
            layer.value,
            layer.paint,
            u32::from(layer.live_only)
        )
        .unwrap();
    }
    c_file(
        "GRKIT_GENERATED_LAYERS_H",
        "The viewfinder.overlay layers of all plugins, in painting order.",
        &body,
    )
}

/// `runtime/generated/encoders.h`: the `picture.encode` handlers, for
/// `runtime/picture.c`.
fn encoders_header() -> String {
    let (mut body, encoders) = handlers(|plugin| plugin.encoders, |encoder| encoder.order);
    body.push_str("\n#define PLUGIN_ENCODERS \\\n");
    for (index, (plugin, encoder)) in encoders.iter().enumerate() {
        let end = if index + 1 == encoders.len() {
            ""
        } else {
            " \\"
        };
        writeln!(
            body,
            "    {{PLUGIN_{},{},{}}},{end}",
            plugin.c_name(),
            encoder.wanted,
            encoder.encode
        )
        .unwrap();
    }
    c_file(
        "GRKIT_GENERATED_ENCODERS_H",
        "The picture.encode handlers of all plugins, in call order.",
        &body,
    )
}

/// `web/src/generated/plugins.ts`: the plugins and the plugins of each camera.
fn site_module() -> String {
    let mut out = String::new();
    out.push_str(
        "// Generated from crates/grkit-core/src/plugins.rs. Do not edit: change the\n\
         // registry there, then run\n\
         // GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.\n\n",
    );
    out.push_str(
        "/** The plugins in menu order. `needsEffects`: the plugin shows only with effects. */\n",
    );
    out.push_str("export const PLUGINS = [\n");
    for plugin in &PLUGINS {
        writeln!(
            out,
            "  {{ key: '{}', plugin: '{}', needsEffects: {} }},",
            plugin.key(),
            plugin.id,
            plugin.needs_effects
        )
        .unwrap();
    }
    out.push_str("] as const;\n\n");
    out.push_str("export type PluginKey = (typeof PLUGINS)[number]['key'];\n\n");
    out.push_str("/** The plugins that the runtime of each camera has, in menu order. */\n");
    out.push_str("export const CAMERA_PLUGINS = {\n");
    for camera in camera::CAMERAS {
        let keys: Vec<String> = camera
            .plugins
            .iter()
            .map(|id| {
                let plugin = PLUGINS
                    .iter()
                    .find(|plugin| plugin.id == *id)
                    .expect("a camera plugin is in the registry");
                format!("'{}'", plugin.key())
            })
            .collect();
        writeln!(out, "  {}: [{}],", camera.id, keys.join(", ")).unwrap();
    }
    out.push_str("} as const satisfies Readonly<Record<string, readonly PluginKey[]>>;\n");
    out
}

/// The generated files, as paths from the repository root and contents.
pub fn generated() -> Vec<(&'static str, String)> {
    vec![
        ("runtime/generated/plugins.h", plugins_header()),
        ("runtime/generated/pages.h", pages_header()),
        ("runtime/generated/layers.h", layers_header()),
        ("runtime/generated/encoders.h", encoders_header()),
        ("web/src/generated/plugins.ts", site_module()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_plugin_has_a_page_and_consistent_handlers() {
        for plugin in &PLUGINS {
            assert!(
                plugin.points.contains(&Point::MenuPage),
                "{} has no menu page",
                plugin.id
            );
            assert_eq!(
                (plugin.artwork)().len(),
                plugin.tiles,
                "{} draws a different number of tiles",
                plugin.id
            );
            let handles = !plugin.layers.is_empty() || !plugin.encoders.is_empty();
            assert_eq!(
                plugin.handlers.is_some(),
                handles,
                "{} names a handler file only with handlers",
                plugin.id
            );
            // One handler file belongs to one component.
            assert!(plugin.layers.is_empty() || plugin.encoders.is_empty());
            assert_eq!(
                !plugin.layers.is_empty(),
                plugin.points.contains(&Point::ViewfinderOverlay),
                "{} layers and points disagree",
                plugin.id
            );
            assert_eq!(
                !plugin.encoders.is_empty(),
                plugin.points.contains(&Point::PictureEncode),
                "{} encoders and points disagree",
                plugin.id
            );
        }
    }

    #[test]
    fn handler_orders_are_distinct() {
        let mut layers: Vec<_> = PLUGINS
            .iter()
            .flat_map(|plugin| plugin.layers.iter().map(|layer| layer.order))
            .collect();
        let mut encoders: Vec<_> = PLUGINS
            .iter()
            .flat_map(|plugin| plugin.encoders.iter().map(|encoder| encoder.order))
            .collect();
        for list in [&mut layers, &mut encoders] {
            let count = list.len();
            list.sort_unstable();
            list.dedup();
            assert_eq!(list.len(), count);
        }
    }

    #[test]
    fn plugin_names_are_c_and_site_names() {
        let keys: Vec<_> = PLUGINS.iter().map(Plugin::key).collect();
        assert_eq!(
            keys,
            ["imageControl", "framing", "guides", "dateStamp", "grain"]
        );
        assert_eq!(PLUGINS[3].c_name(), "DATE_STAMP");
        assert!(NAMES.iter().all(|name| name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')));
    }
}
