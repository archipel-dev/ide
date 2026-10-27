# AI, MCP, tips, feedback and statistics

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (45)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `$SearchWeb` | Search with Google | `EditorPopupMenu`, `ConsoleEditorPopupMenu` |  | MISSING | `platform/platform-impl` |
| `About` | About | `HelpMenu`, `WelcomeScreen.Documentation`, `WelcomeScreen.Options` |  | MISSING | `platform/platform-impl` |
| `BrowseWeb` | Browse Web… |  |  | MISSING | `platform/platform-impl` |
| `ContextHelp` | Context Help | `Git.WorkingTrees.ToolwindowGroup.Toolbar` | `F1` | MISSING | `platform/platform-api` |
| `CsatFeedbackAction` | Show CSAT Feedback Dialog |  |  | MISSING | `platform/feedback` |
| `CsatFeedbackNextDayAction` | Show Next Date for CSAT Feedback Dialog |  |  | MISSING | `platform/feedback` |
| `DumpFeaturesAndTips` | Dump Features and Tips Information | `TipsAndFeatures` |  | MISSING | `plugins/tips-of-the-day` |
| `DumpInvalidTipsOfTheDay` | Dump Invalid Tips of the Day | `TipsAndFeatures` |  | MISSING | `plugins/tips-of-the-day` |
| `DumpMLCompletionFeatures` | Copy ML Completion Features To Clipboard | `MaintenanceGroup` | `ctrl shift alt 9` | codeinsight/06-completion.md CI-CMP | `plugins/completion-ml-ranking` |
| `Help.JetBrainsTV` | Product on YouTube | `HelpMenu`, `WelcomeScreen.Documentation`, `WelcomeScreen.LearnIdeHelp` |  | MISSING | `platform/platform-impl` |
| `Help.KeymapReference` | Keyboard Shortcuts PDF | `HelpMenu`, `WelcomeScreen.Documentation`, `WelcomeScreen.LearnIdeHelp` |  | MISSING | `platform/platform-impl` |
| `HelpTopics` | Help | `HelpMenu`, `WelcomeScreen.Documentation`, `WelcomeScreen.LearnIdeHelp` |  | MISSING | `platform/platform-impl` |
| `OnboardingTestDialogAction` | [INTERNAL] 4 Transfer: Onboarding Test Dialog Action |  |  | MISSING | `plugins/ide-startup` |
| `OnlineDocAction` | Getting Started | `HelpMenu`, `WelcomeScreen.LearnIdeHelp` |  | MISSING | `platform/platform-impl` |
| `OpenHtmlInEmbeddedBrowser` | (set in code) |  |  | MISSING | `platform/platform-impl` |
| `OpenLog` | Open Log in Editor | `HelpMenu` |  | MISSING | `platform/platform-impl` |
| `PredictNextFileCandidates` | Show Next File Candidates |  |  | MISSING | `plugins/filePrediction` |
| `ProductivityGuide` | My Productivity | `ProductivityFeatures` |  | MISSING | `plugins/tips-of-the-day` |
| `PromoBeans` | Beans |  |  | MISSING | `java/java-promo` |
| `PromoDatabase` | Database |  |  | MISSING | `java/java-promo` |
| `PromoEndpoints` | Endpoints |  |  | MISSING | `java/java-promo` |
| `PromoKubernetes` | Kubernetes |  |  | MISSING | `java/java-promo` |
| `PromoOpenAPI` | Configure OpenAPI Sources… |  |  | MISSING | `java/java-promo` |
| `PromoPersistence` | Persistence |  |  | MISSING | `java/java-promo` |
| `PromoProfiler` | IntelliJ Profiler |  |  | MISSING | `java/java-promo` |
| `PromoSpring` | Change Active Spring Profiles… |  |  | MISSING | `java/java-promo` |
| `ReportProblem` | Submit a Bug Report… | `HelpMenu` |  | MISSING | `platform/platform-impl` |
| `SelectAndDumpInvalidTipsOfTheDay` | Select and Dump Invalid Tips of the Day | `TipsAndFeatures` |  | MISSING | `plugins/tips-of-the-day` |
| `SelectAndShowTip` | Select and Show Tip | `TipsAndFeatures` |  | MISSING | `plugins/tips-of-the-day` |
| `SendFeedback` | Submit Feedback… | `HelpMenu` |  | MISSING | `platform/platform-impl` |
| `ShowDemoFeedbackDialog` | Show Demo Feedback Dialog | `FeedbackDialogGroup` |  | MISSING | `platform/feedback` |
| `ShowDemoFeedbackDialogWithEmail` | Show Demo Feedback Dialog with Email | `FeedbackDialogGroup` |  | MISSING | `platform/feedback` |
| `ShowExternalDemoFeedbackWithStatsAction` | Show External Demo Feedback Survey | `FeedbackDialogGroup` |  | MISSING | `platform/feedback` |
| `ShowInIdeDemoFeedbackWithStats` | Show in-IDE Demo Feedback Survey | `FeedbackDialogGroup` |  | MISSING | `platform/feedback` |
| `ShowLocalizationFeedbackDialog` | Show Localization Feedback Survey |  |  | MISSING | `platform/feedback` |
| `ShowLog` | Show Log in File Manager | `HelpMenu` |  | MISSING | `platform/platform-impl` |
| `ShowMcpTools` | Show MCP Tools | `ToolsMenu` |  | MISSING | `plugins/mcp-server` |
| `ShowTips` | Tip of the Day | `WelcomeScreen.Documentation`, `WelcomeScreen.LearnIdeHelp`, `ProductivityFeatures` |  | MISSING | `plugins/tips-of-the-day` |
| `ShowUpgradeFeedbackNotification` | Show Update Feedback Survey |  |  | MISSING | `platform/feedback` |
| `ShowWslSatisfactionDialog` | Show WSL Satisfaction Notification |  |  | MISSING | `platform/feedback` |
| `StartNewUiOnboarding` | Start New UI Onboarding |  |  | MISSING | `platform/new-ui-onboarding` |
| `StartNewUsersOnboarding` | Start Onboarding Tour | `LearnGroup` |  | MISSING | `platform/new-users-onboarding` |
| `TechnicalSupport` | Contact Support… | `HelpMenu` |  | MISSING | `platform/platform-impl` |
| `TrainLocalModelsAction` | Train Local ML Models for the Current Project |  |  | MISSING | `plugins/ml-local-models` |
| `WhatsNewAction` | What's New | `WelcomeScreen.LearnIdeHelp`, `HelpMenu` |  | MISSING | `platform/platform-impl` |

## Settings pages (2)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `com.intellij.mcpserver.settings` | IDE as MCP Server |  | ai | application | MISSING |
| `com.intellij.mcpserver.settings.filter` | Exposed Tools |  | com.intellij.mcpserver.settings | application | MISSING |

## Status bar widgets (1)

| Id | Factory | Order | Spec |
|---|---|---|---|
| `McpServerStatusBarWidget` | `McpServerStatusBarWidgetFactory` | after PowerSaveMode | MISSING |
