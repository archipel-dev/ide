#[test]
#[ignore]
fn dump() {
    let src = std::fs::read_to_string(std::env::var("DUMP").unwrap()).unwrap();
    let mut p = tree_sitter::Parser::new();
    p.set_language(&tree_sitter_java::LANGUAGE.into()).unwrap();
    let t = p.parse(&src, None).unwrap();
    fn go(n: tree_sitter::Node, depth: usize, src: &str, f: Option<&str>) {
        let txt: String = if n.child_count() == 0 {
            format!(" {:?}", &src[n.byte_range()])
        } else {
            String::new()
        };
        println!(
            "{}{}{}{}",
            "  ".repeat(depth),
            f.map(|f| format!("{f}: ")).unwrap_or_default(),
            n.kind(),
            txt
        );
        let mut c = n.walk();
        for (i, ch) in n.children(&mut c).enumerate() {
            go(ch, depth + 1, src, n.field_name_for_child(i as u32));
        }
    }
    go(t.root_node(), 0, &src, None);
}
