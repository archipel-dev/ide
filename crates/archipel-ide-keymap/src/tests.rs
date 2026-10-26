use super::*;

fn keys(text: &str) -> Shortcut {
    text.parse().unwrap()
}

fn stroke(text: &str) -> KeyStroke {
    match keys(text) {
        Shortcut::Keys(strokes) => strokes[0].clone(),
        other => panic!("not a keystroke: {other:?}"),
    }
}

#[test]
fn parses_keystrokes_chords_mouse_and_gestures() {
    assert_eq!(
        keys("Ctrl+Shift+A"),
        Shortcut::Keys(vec![KeyStroke::new(
            Modifiers {
                ctrl: true,
                shift: true,
                ..Modifiers::NONE
            },
            "A"
        )])
    );
    // `+` as the key, NumPad keys and Java key names.
    assert_eq!(
        keys("Ctrl+NumPad +").first_stroke().unwrap().key,
        "NumPad +"
    );
    assert_eq!(keys("Ctrl+Shift+=").first_stroke().unwrap().key, "=");
    assert_eq!(keys("Alt+down"), keys("Alt+Down"));
    assert_eq!(keys("Cmd+close_bracket").first_stroke().unwrap().key, "]");
    assert_eq!(keys("Ctrl+Alt+Shift+d"), keys("Ctrl+Alt+Shift+D"));
    // Chords, including a chord whose second key is a comma.
    match keys("Cmd+k, Cmd+1") {
        Shortcut::Keys(strokes) => {
            assert_eq!(strokes.len(), 2);
            assert_eq!(strokes[1].key, "1");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(keys("Ctrl+,").first_stroke().unwrap().key, ",");
    assert_eq!(
        keys("Alt+Click (double)"),
        Shortcut::Mouse {
            modifiers: Modifiers {
                alt: true,
                ..Modifiers::NONE
            },
            button: MouseButton::Left,
            double: true
        }
    );
    assert!(matches!(
        keys("button142"),
        Shortcut::Mouse {
            button: MouseButton::Other(142),
            ..
        }
    ));
    assert_eq!(
        keys("double Shift"),
        Shortcut::DoubleModifier(Modifier::Shift)
    );
    assert_eq!(
        keys("double control"),
        Shortcut::DoubleModifier(Modifier::Ctrl)
    );
    assert!(matches!(keys("Force touch"), Shortcut::Gesture(_)));
    assert!("".parse::<Shortcut>().is_err());
    assert!("Ctrl+".parse::<Shortcut>().is_err());
    assert!("Ctrl+Shift".parse::<Shortcut>().is_err());
}

#[test]
fn labels_follow_platform_notation() {
    assert_eq!(keys("Ctrl+Shift+A").label(Notation::Text), "Ctrl+Shift+A");
    assert_eq!(keys("Cmd+Shift+A").label(Notation::Mac), "⇧⌘A");
    assert_eq!(keys("Cmd+Alt+L").label(Notation::Mac), "⌥⌘L");
    assert_eq!(keys("Alt+Enter").label(Notation::Mac), "⌥↩");
    assert_eq!(keys("double Shift").label(Notation::Text), "Double Shift");
    assert_eq!(keys("double Shift").label(Notation::Mac), "Double ⇧");
    assert_eq!(
        keys("Ctrl+K, Ctrl+1").label(Notation::Text),
        "Ctrl+K, Ctrl+1"
    );
    assert_eq!(keys("Ctrl+Click").label(Notation::Text), "Ctrl+Click");
}

#[test]
fn display_round_trips_through_parse() {
    for text in [
        "Ctrl+Shift+A",
        "Cmd+Alt+L",
        "Ctrl+NumPad +",
        "Ctrl+K, Ctrl+1",
        "double Shift",
        "Alt+Shift+Middle Click",
        "Cmd+Click (double)",
        "Force touch",
    ] {
        let shortcut = keys(text);
        assert_eq!(
            shortcut.to_string().parse::<Shortcut>().unwrap(),
            shortcut,
            "{text}"
        );
    }
}

#[test]
fn loads_every_bundled_keymap() {
    let set = KeymapSet::bundled().unwrap();
    assert_eq!(set.ids().count(), 26);
    for id in set.ids() {
        let keymap = set.keymap(id, &Overrides::default()).unwrap();
        assert!(keymap.bindings().len() > 500, "{id} has too few bindings");
    }
    assert!(matches!(
        set.keymap("Nope", &Overrides::default()),
        Err(KeymapError::UnknownKeymap(_))
    ));
}

#[test]
fn well_known_bindings_resolve_on_primary_keymaps() {
    let set = KeymapSet::shared().unwrap();
    let get = |id: &str| set.keymap(id, &Overrides::default()).unwrap();
    let windows = get("$default");
    let mac = get("Mac OS X 10.5+");
    let gnome = get("Default for GNOME");
    let kde = get("Default for KDE");
    let xwin = get("Default for XWin");
    let has = |keymap: &Keymap, action: &str, shortcut: &str| {
        assert!(
            keymap.shortcuts(action).contains(&keys(shortcut)),
            "{}: {action} should include {shortcut}, has {:?}",
            keymap.id,
            keymap.shortcuts(action)
        );
    };
    for keymap in [&windows, &gnome, &kde, &xwin] {
        has(keymap, "SearchEverywhere", "double Shift");
        has(keymap, "GotoAction", "Ctrl+Shift+A");
        has(keymap, "RecentFiles", "Ctrl+E");
        has(keymap, "ShowSettings", "Ctrl+Alt+S");
        has(keymap, "ActivateProjectToolWindow", "Alt+1");
    }
    has(&mac, "SearchEverywhere", "double Shift");
    has(&mac, "GotoAction", "Cmd+Shift+A");
    has(&mac, "RecentFiles", "Cmd+E");
    has(&mac, "ShowSettings", "Cmd+,");
    // Reformat Code: Ctrl+Alt+L / Cmd+Alt+L / KDE Alt+Shift+L.
    has(&windows, "ReformatCode", "Ctrl+Alt+L");
    has(&mac, "ReformatCode", "Cmd+Alt+L");
    has(&kde, "ReformatCode", "Alt+Shift+L");
    // Go to Class: Ctrl+N / Cmd+O; Add Selection for Next Occurrence: Alt+J / Ctrl+G.
    has(&windows, "GotoClass", "Ctrl+N");
    has(&mac, "GotoClass", "Cmd+O");
    has(&windows, "SelectNextOccurrence", "Alt+J");
    has(&mac, "SelectNextOccurrence", "Ctrl+G");
}

#[test]
fn desktop_picks_the_default_keymap() {
    assert_eq!(Desktop::MacOs.default_keymap(), "Mac OS X 10.5+");
    assert_eq!(Desktop::Windows.default_keymap(), "$default");
    assert_eq!(
        Desktop::from_desktop_name("ubuntu:GNOME").default_keymap(),
        "Default for GNOME"
    );
    assert_eq!(
        Desktop::from_desktop_name("KDE").default_keymap(),
        "Default for KDE"
    );
    assert_eq!(
        Desktop::from_desktop_name("XFCE").default_keymap(),
        "Default for XWin"
    );
    assert_eq!(display_name("$default"), "Windows");
    assert_eq!(display_name("Eclipse"), "Eclipse");
}

#[test]
fn lookup_handles_chords_and_conflicts() {
    let mut bindings = BTreeMap::new();
    bindings.insert("Find".to_owned(), vec![keys("Ctrl+F")]);
    bindings.insert("FindInEditor".to_owned(), vec![keys("Ctrl+F")]);
    bindings.insert("CollapseAll".to_owned(), vec![keys("Ctrl+K, Ctrl+0")]);
    bindings.insert("Comment".to_owned(), vec![keys("Ctrl+K, Ctrl+C")]);
    let keymap = Keymap::new("test", bindings);
    assert_eq!(
        keymap.lookup(&[stroke("Ctrl+F")]),
        Lookup::Actions(vec!["Find".into(), "FindInEditor".into()])
    );
    assert_eq!(keymap.lookup(&[stroke("Ctrl+K")]), Lookup::Prefix);
    assert_eq!(
        keymap.lookup(&[stroke("Ctrl+K"), stroke("Ctrl+C")]),
        Lookup::Actions(vec!["Comment".into()])
    );
    assert_eq!(
        keymap.lookup(&[stroke("Ctrl+K"), stroke("Ctrl+X")]),
        Lookup::None
    );
    assert_eq!(keymap.lookup(&[stroke("Ctrl+Q")]), Lookup::None);
    let conflicts = keymap.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[&keys("Ctrl+F")].len(), 2);
    assert_eq!(
        keymap.conflicts_with("Find", &keys("Ctrl+F")),
        vec!["FindInEditor".to_owned()]
    );
    // A single stroke that prefixes chords conflicts with them.
    assert_eq!(
        keymap.conflicts_with("New", &keys("Ctrl+K")),
        vec!["CollapseAll".to_owned(), "Comment".to_owned()]
    );
    assert_eq!(
        keymap.find_by_shortcut_text("ctrl+k", Notation::Text).len(),
        2
    );
}

#[test]
fn overrides_replace_and_remove_bindings_and_round_trip() {
    let set = KeymapSet::shared().unwrap();
    let mut overrides = Overrides::default();
    overrides
        .actions
        .insert("ReformatCode".into(), vec![keys("Ctrl+Shift+F")]);
    overrides.actions.insert("GotoClass".into(), vec![]);
    let keymap = set.keymap("$default", &overrides).unwrap();
    assert_eq!(keymap.shortcuts("ReformatCode"), &[keys("Ctrl+Shift+F")]);
    assert!(keymap.shortcuts("GotoClass").is_empty());
    match keymap.lookup(&[stroke("Ctrl+N")]) {
        Lookup::Actions(actions) => assert!(!actions.contains(&"GotoClass".to_owned())),
        Lookup::Prefix | Lookup::None => {}
    }
    let lines = overrides.to_lines();
    assert_eq!(lines, ["GotoClass", "ReformatCode\tCtrl+Shift+F"]);
    assert_eq!(Overrides::from_lines(&lines).unwrap(), overrides);
    assert!(Overrides::from_lines(&["X\tCtrl+".into()]).is_err());
}
