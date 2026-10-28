//! The model's shortcut text equals the keymap's, for every binding of every shipped keymap
//! and both notations, so frontends that format `model::Shortcut` show what the GUI showed.

use archipel_ide_keymap::{KeymapSet, Notation, Overrides, Shortcut};
use archipel_ide_view::keys::model_shortcut;
use archipel_kit_model::Notation as ModelNotation;

#[test]
fn model_shortcut_labels_equal_the_keymap_labels() {
    let set = KeymapSet::shared().expect("keymaps");
    let ids: Vec<String> = set.ids().map(str::to_owned).collect();
    let mut checked = 0;
    for id in ids {
        let keymap = set.keymap(&id, &Overrides::default()).expect("keymap");
        for shortcuts in keymap.bindings().values() {
            for shortcut in shortcuts {
                if matches!(shortcut, Shortcut::Gesture(_)) {
                    continue;
                }
                for (a, b) in [(Notation::Text, ModelNotation::Text), (Notation::Mac, ModelNotation::Mac)] {
                    assert_eq!(
                        model_shortcut(shortcut, b == ModelNotation::Mac).label(b),
                        shortcut.label(a),
                        "{id}"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 10_000, "{checked}");
}
