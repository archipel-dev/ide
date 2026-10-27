# Terminal

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (56)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `Terminal.AiAgents.LaunchJunieCli` | Launch Junie CLI |  |  | MISSING | `plugins/terminal` |
| `Terminal.ClearBuffer` | Clear Terminal | `Terminal.ReworkedTerminalContextMenu` |  | MISSING | `plugins/terminal` |
| `Terminal.ClearPrompt` | Clear Command Prompt |  | `control C` | MISSING | `plugins/terminal` |
| `Terminal.CloseSession` | Close Session |  | `control D` | MISSING | `plugins/terminal` |
| `Terminal.CloseTab` | Close Tab | `Terminal.ReworkedTerminalContextMenu` | as `CloseContent` | MISSING | `plugins/terminal` |
| `Terminal.CommandCompletion` | Command Completion |  | `TAB` | codeinsight/06-completion.md CI-CMP | `plugins/terminal` |
| `Terminal.CommandCompletion.InsertSuggestion` | Insert Command Completion Suggestion |  | as `EditorChooseLookupItem` | codeinsight/06-completion.md CI-CMP | `plugins/terminal` |
| `Terminal.CommandCompletion.Invoke` | Call Command Completion |  | as `CodeCompletion` | codeinsight/06-completion.md CI-CMP | `plugins/terminal` |
| `Terminal.CommandCompletion.SelectSuggestionAbove` | Select Suggestion Above |  | as `EditorUp` | codeinsight/06-completion.md CI-CMP | `plugins/terminal` |
| `Terminal.CommandCompletion.SelectSuggestionBelow` | Select Suggestion Below |  | as `EditorDown` | codeinsight/06-completion.md CI-CMP | `plugins/terminal` |
| `Terminal.CopyBlock` | Copy Block | `Terminal.OutputContextMenu` |  | MISSING | `plugins/terminal` |
| `Terminal.CopySelectedText` | Copy | `Terminal.OutputContextMenu`, `Terminal.PromptContextMenu`, `Terminal.ReworkedTerminalContextMenu` | `control C`, `control INSERT` | MISSING | `plugins/terminal` |
| `Terminal.CreateRectangularSelectionOnMouseDrag` | Create Rectangular Selection on Mouse Drag |  | `mouse shift alt button1` | MISSING | `plugins/terminal` |
| `Terminal.DeletePreviousWord` | Delete Previous Word |  | `control W` | MISSING | `plugins/terminal` |
| `Terminal.Escape` | Escape |  | as `EditorEscape` | MISSING | `plugins/terminal` |
| `Terminal.ExpandBlockSelectionAbove` | Expand Block Selection Above |  | as `EditorUpWithSelection` | MISSING | `plugins/terminal` |
| `Terminal.ExpandBlockSelectionBelow` | Expand Block Selection Below |  | as `EditorDownWithSelection` | MISSING | `plugins/terminal` |
| `Terminal.Find` | Find | `Terminal.ReworkedTerminalContextMenu` | as `Find` | MISSING | `plugins/terminal` |
| `Terminal.InsertInlineCompletion` | Insert Inline Proposal |  | `RIGHT` | codeinsight/06-completion.md CI-CMP | `plugins/terminal` |
| `Terminal.LineDown` | Line Down | `Terminal.ReworkedTerminalContextMenu` | `control DOWN` | MISSING | `plugins/terminal` |
| `Terminal.LineUp` | Line Up | `Terminal.ReworkedTerminalContextMenu` | `control UP` | MISSING | `plugins/terminal` |
| `Terminal.MoveToEditor` | Move to Editor | `ToolWindowTabContextMenu` |  | MISSING | `plugins/terminal` |
| `Terminal.MoveToolWindowTabLeft` | Move Tab Left |  |  | MISSING | `plugins/terminal` |
| `Terminal.MoveToolWindowTabRight` | Move Tab Right |  |  | MISSING | `plugins/terminal` |
| `Terminal.NewTab` | New Tab | `TerminalToolwindowActionGroup`, `Terminal.ReworkedTerminalContextMenu` | `control shift T` | MISSING | `plugins/terminal` |
| `Terminal.OpenInReworkedTerminal` | Open in Terminal | `RevealGroup`, `TouchBarDefault_alt` | as `Terminal.OpenInTerminal` | MISSING | `plugins/terminal` |
| `Terminal.OpenInTerminal` | Open in Terminal | `RevealGroup`, `TouchBarDefault_alt` |  | MISSING | `plugins/terminal` |
| `Terminal.PageDown` | Page Down | `Terminal.ReworkedTerminalContextMenu` | `shift PAGE_DOWN` | MISSING | `plugins/terminal` |
| `Terminal.PageUp` | Page Up | `Terminal.ReworkedTerminalContextMenu` | `shift PAGE_UP` | MISSING | `plugins/terminal` |
| `Terminal.Paste` | Paste | `Terminal.OutputContextMenu`, `Terminal.PromptContextMenu`, `Terminal.ReworkedTerminalContextMenu` | `control V`, `shift INSERT` | MISSING | `plugins/terminal` |
| `Terminal.PasteFromHistory` | Paste from History… | `Terminal.OutputContextMenu`, `Terminal.PromptContextMenu`, `Terminal.ReworkedTerminalContextMenu` | as `PasteMultiple` | MISSING | `plugins/terminal` |
| `Terminal.PasteSelection` | Paste From Selection Clipboard |  | as `EditorPasteFromX11` | MISSING | `plugins/terminal` |
| `Terminal.RenameSession` | Rename Session | `ToolWindowTabContextMenu` |  | MISSING | `plugins/terminal` |
| `Terminal.RenameSessionInEditor` | Rename Session | `EditorTabPopupMenu` |  | MISSING | `plugins/terminal` |
| `Terminal.SearchInCommandHistory` | Search in Command History |  | `control R` | MISSING | `plugins/terminal` |
| `Terminal.SelectAll` | Select All | `Terminal.ReworkedTerminalContextMenu` |  | MISSING | `plugins/terminal` |
| `Terminal.SelectBlockAbove` | Select Block Above |  | `UP`, `control UP` | MISSING | `plugins/terminal` |
| `Terminal.SelectBlockBelow` | Select Block Below |  | `DOWN` | MISSING | `plugins/terminal` |
| `Terminal.SelectLastBlock` | Select Last Block |  | `control UP` | MISSING | `plugins/terminal` |
| `Terminal.SelectPrompt` | Select Prompt |  | `control DOWN` | MISSING | `plugins/terminal` |
| `Terminal.Settings` | Settings | `Terminal.ToolWindowActions` |  | MISSING | `plugins/terminal` |
| `Terminal.ShowDocumentation` | Show Documentation |  | as `QuickJavaDoc` | MISSING | `plugins/terminal` |
| `Terminal.SmartCommandExecution.Debug` | Debug Highlighted Command Using IDE |  | `control shift ENTER` | MISSING | `plugins/terminal` |
| `Terminal.SmartCommandExecution.Run` | Run Highlighted Command Using IDE |  | `control ENTER` | MISSING | `plugins/terminal` |
| `Terminal.SwitchFocusToEditor` | Switch Focus To Editor |  |  | MISSING | `plugins/terminal` |
| `Terminal.ToggleEmulatorBadge` | Show Terminal Emulator Badge |  |  | MISSING | `plugins/terminal` |
| `Terminal.UseClassicEngine` | (set in code) | `Terminal.Engine` |  | MISSING | `plugins/terminal` |
| `Terminal.UseDoubleLinePrompt` | IDE Double-Line Prompt | `Terminal.PromptStyle` |  | MISSING | `plugins/terminal` |
| `Terminal.UseNewTerminalEngine` | (set in code) | `Terminal.Engine` |  | MISSING | `plugins/terminal` |
| `Terminal.UseReworkedEngine` | (set in code) | `Terminal.Engine` |  | MISSING | `plugins/terminal` |
| `Terminal.UseShellPrompt` | Shell Prompt (PS1) | `Terminal.PromptStyle` |  | MISSING | `plugins/terminal` |
| `Terminal.UseSingleLinePrompt` | IDE Single-Line Prompt | `Terminal.PromptStyle` |  | MISSING | `plugins/terminal` |
| `TerminalDecreaseFontSize` | Decrease Font Size |  | as `EditorDecreaseFontSize` | MISSING | `platform/terminal` |
| `TerminalIncreaseFontSize` | Increase Font Size |  | as `EditorIncreaseFontSize` | MISSING | `platform/terminal` |
| `TerminalNewPredefinedSession` | New Predefined Session | `TerminalToolwindowActionGroup` |  | MISSING | `plugins/terminal` |
| `TerminalResetFontSize` | Reset Font Size |  | as `EditorResetFontSize` | MISSING | `platform/terminal` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `terminal` | Terminal | tools |  | project | MISSING |

## Tool windows (1)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Terminal` | Terminal | bottom |  | TerminalToolWindowFactory | MISSING |
