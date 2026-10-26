//! Every extension point is reachable through the registry and its mock
//! implementation honors the contract (the fixtures other packages start
//! from).

use archipel_ide_analysis::assist::*;
use archipel_ide_analysis::daemon::*;
use archipel_ide_analysis::editing::*;
use archipel_ide_analysis::index::*;
use archipel_ide_analysis::inspect::*;
use archipel_ide_analysis::intention::*;
use archipel_ide_analysis::modcommand::{ModCommandExecutor, apply_edits};
use archipel_ide_analysis::provider::{AnalysisProvider, Merger, ProviderMode};
use archipel_ide_analysis::refactor::*;
use archipel_ide_analysis::resolve::*;
use archipel_ide_analysis::testing::*;
use archipel_ide_analysis::{
    CancelToken, Change, FileMeta, FileRev, IndexStamp, InterfaceSummarizer, LanguageId,
    ModCommand, Severity, Snapshot, SyntaxSnapshot,
};
use std::sync::Arc;

fn setup() -> (Fixture, Snapshot) {
    let mut fixture = Fixture::new();
    let (java, rust) = (fixture.java, fixture.rust);
    fixture.open(java);
    fixture.open(rust);
    let snap = fixture.host.snapshot();
    (fixture, snap)
}

const JAVA: LanguageId = LanguageId("java");

#[test]
fn registry_serves_every_slot_for_every_language() {
    let (_fixture, snap) = setup();
    let reg = snap.registry();
    for lang in archipel_ide_language::LANGUAGES {
        let id = lang.language_id();
        assert!(reg.get::<dyn FoldingBuilder>(id).is_some(), "{id}");
        assert!(reg.get::<dyn StructureProvider>(id).is_some());
        assert!(reg.get::<dyn ExtendSelectionHandler>(id).is_some());
        assert!(reg.get::<dyn BraceMatcher>(id).is_some());
        assert!(reg.get::<dyn InterfaceSummarizer>(id).is_some());
    }
    assert_eq!(reg.all::<dyn Pass>(JAVA).len(), 1);
    assert_eq!(reg.all::<dyn Annotator>(JAVA).len(), 1);
    assert_eq!(reg.all::<dyn DynExternalAnnotator>(JAVA).len(), 1);
    assert_eq!(reg.all::<dyn LocalInspection>(JAVA).len(), 1);
    assert_eq!(reg.all::<dyn GlobalInspection>(JAVA).len(), 1);
    assert_eq!(reg.all::<dyn IntentionAction>(JAVA).len(), 1);
    assert!(reg.get::<dyn Resolver>(JAVA).is_some());
    assert!(reg.get::<dyn ReferenceProvider>(JAVA).is_some());
    assert!(reg.get::<dyn UsageSearcher>(JAVA).is_some());
    assert!(reg.get::<dyn ReadWriteAccessDetector>(JAVA).is_some());
    assert!(reg.get::<dyn CompletionWeigher>(JAVA).is_some());
    assert!(reg.get::<dyn DocumentationProvider>(JAVA).is_some());
    assert!(reg.get::<dyn ParameterInfoProvider>(JAVA).is_some());
    assert!(reg.get::<dyn FormattingService>(JAVA).is_some());
    assert!(reg.get::<dyn TemplateContextType>(JAVA).is_some());
    assert!(reg.get::<dyn RefactoringSupport>(JAVA).is_some());
    assert!(reg.get::<dyn RefactoringListener>(JAVA).is_some());
    assert!(reg.get::<dyn Commenter>(JAVA).is_some());
    assert!(reg.get::<dyn QuoteHandler>(JAVA).is_some());
    assert!(reg.get::<dyn Indenter>(JAVA).is_some());
    assert!(reg.get::<dyn TypedHandler>(JAVA).is_some());
    assert!(reg.get::<dyn SmartEnterProcessor>(JAVA).is_some());
    assert!(reg.get::<dyn MoveStatementHandler>(JAVA).is_some());
    assert!(reg.get::<dyn StickyScopesProvider>(JAVA).is_some());
    assert!(reg.get::<dyn BreadcrumbsProvider>(JAVA).is_some());
    assert!(reg.get::<dyn EntryPoint>(JAVA).is_some());
    assert!(reg.get::<dyn InspectionSuppressor>(JAVA).is_some());
    // The LSP stand-in registered last: native contributors come first.
    let completion = reg.entries::<dyn CompletionContributor>(JAVA);
    let ids: Vec<_> = completion.iter().map(|(r, _)| r.id).collect();
    assert_eq!(ids, ["mock-words", "mock-lsp-completion"]);
    assert_eq!(MockLspProvider.id(), "mock-lsp");
}

#[test]
fn daemon_contracts_produce_highlights() {
    let (fixture, snap) = setup();
    let rust = fixture.rust;
    let syntax = snap.syntax(rust).expect("syntax");
    let cancel = CancelToken::new();
    let len = syntax.text().len() as u32;
    let pass = SyntaxErrorPass;
    let out = pass
        .collect(&PassCx {
            snap: &snap,
            file: rust,
            syntax: &syntax,
            range: 0..len,
            visible: true,
            cancel: &cancel,
        })
        .expect("pass");
    assert!(out.infos.is_empty());
    let mut markup = VecMarkup::default();
    ReplaceSlice.apply(out, &mut markup);
    // Annotator over the tree.
    let annotator = WordAnnotator::new("FIXME");
    let mut holder = AnnotationHolder::new(archipel_ide_analysis::ToolId::native("mock-word"));
    let cx = AnnotateCx {
        snap: &snap,
        syntax: &syntax,
        cancel: &cancel,
        tier: archipel_ide_analysis::CompletenessTier::G0,
        dumb: false,
    };
    for node in syntax.root().preorder() {
        if annotator.interesting().contains(node.element_type()) {
            annotator
                .annotate(node, &mut holder, &cx)
                .expect("annotate");
        }
    }
    let infos = holder.into_infos();
    assert_eq!(infos.len(), 1);
    assert_eq!(
        &syntax.text()[infos[0].range.start as usize..infos[0].range.end as usize],
        "FIXME"
    );
    assert_eq!(infos[0].severity, Severity::WEAK_WARNING);
    // External annotator through the object-safe form.
    let external: Arc<dyn DynExternalAnnotator> = Arc::new(FixmeExternalAnnotator);
    let info = external.collect_dyn(&snap, rust).expect("collect");
    let result = external.run_dyn(info, &cancel).expect("run");
    let mut holder = AnnotationHolder::new(archipel_ide_analysis::ToolId::native("external"));
    external.apply_dyn(rust, FileRev(1), result, &mut holder);
    assert_eq!(holder.infos().len(), 1);
    markup.replace(&holder.infos()[0].tool.clone(), 0..len, holder.into_infos());
    assert_eq!(markup.query(0..len).len(), 1);
}

#[test]
fn inspections_fixes_and_intentions_return_commands() {
    let (fixture, snap) = setup();
    let java = fixture.java;
    let syntax = snap.syntax(java).expect("syntax");
    let cancel = CancelToken::new();
    let options = InspectionOptions::default();
    let mut holder = ProblemsHolder::new(true);
    EmptyBlockInspection
        .check_file(
            &FileCx {
                snap: &snap,
                file: java,
                syntax: &syntax,
                cancel: &cancel,
                options: &options,
                on_the_fly: true,
            },
            &mut holder,
        )
        .expect("inspect");
    let problems = holder.into_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(
        &syntax.text()[problems[0].range.start as usize..problems[0].range.end as usize],
        "{}"
    );
    let fix = problems[0].fixes[0]
        .fix(
            &problems[0],
            &FixCx {
                snap: &snap,
                file: java,
                syntax: &syntax,
                cancel: &cancel,
            },
        )
        .expect("fix");
    let fixed = fix
        .preview(java, syntax.text())
        .expect("edits")
        .expect("applies");
    assert!(fixed.contains("void empty() \n"));
    // Intention at an identifier.
    let caret = syntax.text().find("greet").expect("greet") as u32 + 1;
    let cx = ActionCx {
        snap: &snap,
        file: java,
        syntax: &syntax,
        caret,
        selection: None,
        cancel: &cancel,
    };
    assert_eq!(
        UppercaseIntention.presentation(&cx).map(|p| p.text),
        Some("Upper-case 'greet'".into())
    );
    let command = UppercaseIntention.perform(&cx).expect("perform");
    let mut executor = InMemoryExecutor::default();
    executor
        .files
        .insert(java, (FileRev(1), syntax.text().to_string()));
    let outcome = executor.execute(&command).expect("execute");
    assert_eq!(outcome.files_changed, vec![java]);
    assert!(executor.files[&java].1.contains("GREET("));
    // Stale revisions are refused (optimistic concurrency).
    assert!(executor.execute(&command).is_err());
    // Global inspection and suppression.
    let mut sink: Vec<_> = Vec::new();
    TooManyFilesInspection { limit: 1 }
        .run(
            &AnalysisScope::Project,
            &GlobalCx {
                snap: &snap,
                cancel: &cancel,
            },
            &mut sink,
        )
        .expect("global");
    assert_eq!(sink.len(), 1);
    let suppressed = SyntaxSnapshot::parse(
        java,
        FileRev(1),
        syntax.lang(),
        "class A {\n// noinspection MockEmptyBlock\nvoid f() {}\n}".into(),
    )
    .expect("parse");
    let method = suppressed
        .root()
        .preorder()
        .find(|n| n.kind() == "method_declaration")
        .expect("method");
    assert!(CommentSuppressor.is_suppressed(method, "MockEmptyBlock"));
    assert!(!CommentSuppressor.is_suppressed(method, "Other"));
}

#[test]
fn resolve_references_and_usages() {
    let (fixture, snap) = setup();
    let java = fixture.java;
    let syntax = snap.syntax(java).expect("syntax");
    let cancel = CancelToken::new();
    let scope = SearchScope::Project;
    let cx = ResolveCx {
        snap: &snap,
        cancel: &cancel,
        scope: &scope,
        incomplete_code: false,
    };
    let at = syntax.text().find("greet").expect("greet") as u32;
    let leaf = syntax.node_at(at).expect("leaf");
    let results = FileLocalResolver
        .resolve_at(&syntax, leaf, &cx)
        .expect("resolve");
    assert_eq!(results.0.len(), 1);
    let target = results.0[0].target.clone();
    assert_eq!(&*target.name, "greet");
    let provider = NameReferenceProvider::for_language(syntax.lang());
    let mut refs = Vec::new();
    provider.references(leaf, &mut refs);
    assert_eq!(refs.len(), 1);
    assert!(refs[0].is_reference_to(&target, &cx).expect("is ref"));
    assert_eq!(
        refs[0].rename_edit("hi").map(|e| e.range),
        Some(leaf.range())
    );
    let mut usages = Vec::new();
    TextUsageSearcher
        .search(&target, &cx, &mut |usage| {
            usages.push(usage);
            true
        })
        .expect("search");
    assert_eq!(usages.len(), 1);
    assert_eq!(usages[0].kind, UsageKind::Declaration);
    let count = syntax
        .root()
        .preorder()
        .find(|n| {
            n.text() == "count"
                && n.parent()
                    .is_some_and(|p| p.kind() == "assignment_expression")
        })
        .expect("count");
    assert_eq!(AssignmentAccess.access(count), UsageKind::Write);
    assert!(
        refs[0]
            .variants(&cx)
            .expect("variants")
            .iter()
            .any(|item| &*item.label == "App")
    );
}

#[test]
fn file_indexes_store_and_query_through_snapshots() {
    let (mut fixture, snap) = setup();
    let store = Arc::new(MemIndexStore::new());
    for (file, path) in snap.files().iter() {
        let text = snap.file_text(file).expect("text");
        let meta = FileMeta {
            id: file,
            path: path.to_path_buf(),
            len: text.len() as u64,
            mtime: None,
            content_hash: None,
            language: snap.language_id(file),
            is_open: snap.is_open(file),
        };
        let syntax = snap.syntax(file);
        let input = IndexInput {
            file: &meta,
            text: &text,
            syntax: syntax.as_ref(),
        };
        index_file(&*store, &FileNameIndex, &input).expect("index");
        index_file(&*store, &WordCountIndex, &input).expect("index");
    }
    let stamp = store.commit(FileNameIndex::ID);
    assert_eq!(stamp, IndexStamp(1));
    fixture.host.set_index_store(store.clone());
    assert!(fixture.host.snapshot().require::<FileNameIndex>().is_err());
    fixture.host.apply(Change::IndexCommit {
        index: FileNameIndex::ID,
        stamp,
        state: IndexState::Ready,
    });
    fixture.host.apply(Change::IndexCommit {
        index: WordCountIndex::ID,
        stamp,
        state: IndexState::Ready,
    });
    let snap = fixture.host.snapshot();
    let names = snap.require::<FileNameIndex>().expect("ready");
    let files = names.files_for_key(&"app.java".to_string()).expect("files");
    assert_eq!(files.iter().collect::<Vec<_>>(), vec![fixture.java]);
    let keys = names
        .all_keys(&|key: &String| key.ends_with(".rs"))
        .expect("keys");
    assert_eq!(keys, vec!["lib.rs".to_string()]);
    let words = snap.require::<WordCountIndex>().expect("ready");
    let counts = words
        .values(&"count".to_string(), &SearchScope::Project)
        .expect("values");
    assert_eq!(counts, vec![(fixture.java, 3)]);
    store.remove_file(WordCountIndex::ID, fixture.java);
    assert!(
        words
            .values(&"count".to_string(), &SearchScope::Project)
            .expect("values")
            .is_empty()
    );
    assert!(store.forward(FileNameIndex::ID, fixture.rust).len() == 1);
}

#[test]
fn assistance_contracts() {
    let (fixture, snap) = setup();
    let java = fixture.java;
    let syntax = snap.syntax(java).expect("syntax");
    let cancel = CancelToken::new();
    let params = CompletionParams {
        snap: &snap,
        file: java,
        syntax: &syntax,
        offset: 0,
        prefix: "gr",
        invocation_count: 1,
        cancel: &cancel,
    };
    let mut result = CompletionResultSet::default();
    for contributor in snap.registry().all::<dyn CompletionContributor>(JAVA) {
        contributor.fill(&params, &mut result).expect("fill");
    }
    let labels: Vec<_> = result
        .items()
        .iter()
        .map(|item| item.label.to_string())
        .collect();
    assert_eq!(labels, ["greet", "grFromServer"]);
    let merged = LabelMerger.merge(
        result.items().to_vec(),
        vec![archipel_ide_analysis::resolve::LookupItem::new("greet")],
        ProviderMode::Both,
    );
    assert_eq!(merged.len(), 2);
    assert_eq!(PrefixWeigher.weigh(&result.items()[0], &params), 1);
    let scope = SearchScope::Project;
    let at = syntax.text().find("greet").expect("greet") as u32;
    let target = FileLocalResolver
        .resolve_at(
            &syntax,
            syntax.node_at(at).expect("leaf"),
            &ResolveCx {
                snap: &snap,
                cancel: &cancel,
                scope: &scope,
                incomplete_code: false,
            },
        )
        .expect("resolve")
        .0[0]
        .target
        .clone();
    let docs = StructureDocumentation
        .documentation(&snap, &target, &cancel)
        .expect("docs")
        .expect("some");
    assert_eq!(docs.definition.as_deref(), Some("greet(String): String"));
    assert_eq!(
        NoParameterInfo.parameter_info(&snap, &syntax, 0, &cancel),
        Ok(None)
    );
    let messy = SyntaxSnapshot::parse(java, FileRev(1), syntax.lang(), "class A {  \n}\t\n".into())
        .expect("parse");
    let edits = TrailingWhitespaceFormatter
        .format(&messy, None, &CodeStyle::default(), &cancel)
        .expect("format");
    assert_eq!(
        apply_edits(messy.text(), &edits).as_deref(),
        Ok("class A {\n}\n")
    );
    assert!(EverywhereContext.is_in_context(&syntax, 0));
}

#[test]
fn refactoring_and_editing_contracts() {
    let (fixture, snap) = setup();
    let java = fixture.java;
    let syntax = snap.syntax(java).expect("syntax");
    let cancel = CancelToken::new();
    let caret = syntax.text().find("count;").expect("count") as u32;
    let cx = RefactorCx {
        snap: &snap,
        file: java,
        caret,
        selection: None,
        target: None,
        new_name: Some("total"),
        cancel: &cancel,
    };
    assert!(TextRename.is_available(RefactoringKind::Rename, &cx));
    let plan = TextRename
        .prepare(RefactoringKind::Rename, &cx)
        .expect("plan");
    assert_eq!(plan.usages.len(), 3);
    assert!(plan.conflicts.is_empty());
    let renamed = plan
        .command
        .preview(java, syntax.text())
        .expect("edits")
        .expect("applies");
    assert_eq!(renamed.matches("total").count(), 3);
    assert!(
        TextRename
            .validate_name(RefactoringKind::Rename, "")
            .is_err()
    );
    let clash = RefactorCx {
        new_name: Some("name"),
        ..cx
    };
    assert_eq!(
        TextRename
            .prepare(RefactoringKind::Rename, &clash)
            .expect("plan")
            .conflicts
            .len(),
        2
    );
    // Editing services.
    assert_eq!(LineCommenter("//").line_comment_prefix(), Some("//"));
    let in_string = syntax.text().find("Hello").expect("hello") as u32;
    assert!(!PairQuotes.should_pair(&syntax, in_string, '"'));
    assert!(PairQuotes.should_pair(&syntax, 0, '"'));
    let line = syntax.text()[..syntax.text().find("count = count").expect("stmt")]
        .matches('\n')
        .count() as u32;
    assert_eq!(
        BraceDepthIndenter.indent_for_line(&syntax, line, &IndentOptions::default()),
        Some(8)
    );
    assert_eq!(
        PairingTypedHandler.before_typed(&syntax, 0, '('),
        TypedDecision::InsertPair(')')
    );
    let statement = syntax.text().find("count = count + 1").expect("stmt") as u32;
    assert!(
        SemicolonSmartEnter
            .smart_enter(&syntax, statement)
            .is_none()
    );
    assert!(
        NoMoveStatement
            .move_statement(&syntax, 0..1, true)
            .is_none()
    );
    let crumbs = StructureBreadcrumbs.breadcrumbs(&syntax, statement);
    assert_eq!(
        crumbs.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
        ["App", "greet"]
    );
    let sticky = StructureSticky.sticky_scopes(&syntax, line, 5);
    assert_eq!(sticky.len(), 2);
    let main = SyntaxSnapshot::parse(
        java,
        FileRev(1),
        syntax.lang(),
        "class A { static void main(String[] a) {} }".into(),
    )
    .expect("parse");
    let method = main
        .root()
        .preorder()
        .find(|n| n.kind() == "method_declaration")
        .expect("method");
    assert!(MainEntryPoint.is_entry_point(method));
    // Generic services answer through the registry.
    let folds = snap
        .registry()
        .get::<dyn FoldingBuilder>(JAVA)
        .expect("folding")
        .folds(&syntax, 0..syntax.text().len() as u32, &cancel)
        .expect("folds");
    assert!(!folds.is_empty());
    let structure = snap
        .registry()
        .get::<dyn StructureProvider>(JAVA)
        .expect("structure")
        .structure(&syntax, &cancel)
        .expect("items");
    assert_eq!(structure[0].name, "App");
    let open = syntax.text().find('{').expect("brace") as u32;
    assert!(
        snap.registry()
            .get::<dyn BraceMatcher>(JAVA)
            .expect("braces")
            .matching(&syntax, open)
            .is_some()
    );
    let grown = snap
        .registry()
        .get::<dyn ExtendSelectionHandler>(JAVA)
        .expect("extend")
        .extend(&syntax, caret..caret);
    assert!(grown.is_some_and(|r| r.start <= caret && r.end > caret));
    let listener = RecordingListener::default();
    listener.renamed(&plan_target(&snap, java), "total");
    assert_eq!(listener.events.lock().len(), 1);
    let _ = ModCommand::Nothing;
}

fn plan_target(
    snap: &Snapshot,
    file: archipel_ide_analysis::FileId,
) -> archipel_ide_analysis::Target {
    let syntax = snap.syntax(file).expect("syntax");
    let scope = SearchScope::Project;
    let cancel = CancelToken::new();
    let at = syntax.text().find("App").expect("App") as u32;
    FileLocalResolver
        .resolve_at(
            &syntax,
            syntax.node_at(at).expect("leaf"),
            &ResolveCx {
                snap,
                cancel: &cancel,
                scope: &scope,
                incomplete_code: false,
            },
        )
        .expect("resolve")
        .0[0]
        .target
        .clone()
}
