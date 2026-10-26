//! Parameter info (`textDocument/signatureHelp`): opens after `(` and `,`
//! (the server's trigger characters), after completing a call, or with
//! Ctrl+P (⌘P on macOS); follows the caret while open and closes when the
//! server has nothing to show or the caret leaves the call.

use crate::IdeState;
use archipel_ide_lsp::{
    RequestHandle,
    lsp_types::{
        ParameterLabel, SignatureHelp, SignatureHelpContext, SignatureHelpParams,
        SignatureHelpTriggerKind, TextDocumentIdentifier, TextDocumentPositionParams,
        request::SignatureHelpRequest,
    },
};
use std::path::PathBuf;

/// One signature prepared for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureLine {
    pub label: String,
    /// Char range of the active parameter in `label`, if any.
    pub active: Option<(usize, usize)>,
    /// Whether this is the server's active overload.
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInfo {
    pub path: PathBuf,
    /// Caret offset the info was computed for.
    pub offset: usize,
    pub signatures: Vec<SignatureLine>,
}

#[derive(Debug, Clone)]
struct PendingSignature {
    handle: RequestHandle,
    path: PathBuf,
    offset: usize,
    explicit: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SignatureState {
    pub popup: Option<ParameterInfo>,
    pending: Option<PendingSignature>,
    /// The raw response, sent back as `activeSignatureHelp` on re-triggers.
    last: Option<SignatureHelp>,
}

/// Converts a response into display lines (server order, active overload marked).
pub fn signature_lines(help: &SignatureHelp) -> Vec<SignatureLine> {
    let active_signature = help.active_signature.unwrap_or(0) as usize;
    help.signatures
        .iter()
        .enumerate()
        .map(|(index, signature)| {
            let parameter = signature
                .active_parameter
                .or(help.active_parameter)
                .map(|parameter| parameter as usize);
            let active = parameter
                .and_then(|parameter| signature.parameters.as_ref()?.get(parameter))
                .and_then(|parameter| match &parameter.label {
                    ParameterLabel::Simple(text) => {
                        let start = signature.label.find(text.as_str())?;
                        let start_char = signature.label[..start].chars().count();
                        Some((start_char, start_char + text.chars().count()))
                    }
                    ParameterLabel::LabelOffsets([start, end]) => {
                        utf16_range_to_chars(&signature.label, *start as usize, *end as usize)
                    }
                });
            SignatureLine {
                label: signature.label.clone(),
                active,
                current: index == active_signature,
            }
        })
        .collect()
}

fn utf16_range_to_chars(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let mut units = 0;
    let (mut start_char, mut end_char) = (None, None);
    for (index, c) in text.chars().enumerate() {
        if units == start {
            start_char = Some(index);
        }
        if units == end {
            end_char = Some(index);
        }
        units += c.len_utf16();
    }
    if units == end {
        end_char = Some(text.chars().count());
    }
    Some((start_char?, end_char?)).filter(|(start, end)| start <= end)
}

impl IdeState {
    /// Requests parameter info at the caret. `trigger` is the typed trigger
    /// character, `None` for Ctrl+P or a re-check while the popup is open.
    pub fn request_parameter_info(
        &mut self,
        trigger: Option<String>,
        explicit: bool,
    ) -> Result<(), String> {
        let document = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .ok_or("no active document")?;
        let offset = document.selections.primary().head;
        let path = document.path.clone();
        let (line, character) = document
            .text
            .lsp_position(offset)
            .map_err(|error| error.to_string())?;
        let absolute = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .join(&path);
        let uri =
            archipel_ide_lsp::client::file_uri(&absolute).map_err(|error| error.to_string())?;
        let retrigger = self.signature.popup.is_some();
        let context = SignatureHelpContext {
            trigger_kind: if trigger.is_some() {
                SignatureHelpTriggerKind::TRIGGER_CHARACTER
            } else if retrigger {
                SignatureHelpTriggerKind::CONTENT_CHANGE
            } else {
                SignatureHelpTriggerKind::INVOKED
            },
            trigger_character: trigger,
            is_retrigger: retrigger,
            active_signature_help: if retrigger {
                self.signature.last.clone()
            } else {
                None
            },
        };
        if let Some(pending) = self.signature.pending.take() {
            self.lsp.cancel_request(&pending.handle);
        }
        let handle = self
            .lsp
            .send_request::<SignatureHelpRequest>(
                &absolute,
                SignatureHelpParams {
                    context: Some(context),
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: archipel_ide_lsp::lsp_types::Position::new(
                            line as u32,
                            character as u32,
                        ),
                    },
                    work_done_progress_params: Default::default(),
                },
            )
            .map_err(|error| error.to_string())?;
        self.signature.pending = Some(PendingSignature {
            handle,
            path,
            offset,
            explicit,
        });
        Ok(())
    }

    pub fn close_parameter_info(&mut self) {
        self.signature.popup = None;
        self.signature.last = None;
        if let Some(pending) = self.signature.pending.take() {
            self.lsp.cancel_request(&pending.handle);
        }
    }

    /// Server signature-help trigger characters for the active document.
    fn signature_triggers(&self) -> (Vec<String>, Vec<String>) {
        let Some(provider) = self
            .active_absolute_path()
            .and_then(|path| self.lsp.capabilities_for(&path))
            .and_then(|capabilities| capabilities.signature_help_provider.as_ref())
        else {
            return (Vec::new(), Vec::new());
        };
        (
            provider.trigger_characters.clone().unwrap_or_default(),
            provider.retrigger_characters.clone().unwrap_or_default(),
        )
    }

    /// Opens or refreshes parameter info after typing.
    pub fn signature_after_typing(&mut self, typed: &str) {
        let (triggers, retriggers) = self.signature_triggers();
        let last = typed.chars().last().map(String::from);
        let triggered = last.as_ref().filter(|last| {
            triggers.contains(last) || (self.signature.popup.is_some() && retriggers.contains(last))
        });
        if let Some(trigger) = triggered.cloned() {
            let _ = self.request_parameter_info(Some(trigger), false);
        } else if self.signature.popup.is_some() || self.signature.pending.is_some() {
            let _ = self.request_parameter_info(None, false);
        }
    }

    /// Refreshes an open popup after the caret moved (closes it on another file).
    pub fn signature_after_caret_move(&mut self) {
        if self.signature.popup.is_none() {
            return;
        }
        let path = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .map(|document| document.path.clone());
        if path.as_ref() != self.signature.popup.as_ref().map(|popup| &popup.path) {
            self.close_parameter_info();
            return;
        }
        let _ = self.request_parameter_info(None, false);
    }

    pub(crate) fn poll_signature(&mut self) {
        let Some(pending) = self.signature.pending.clone() else {
            return;
        };
        let Some(result) = self
            .lsp
            .take_typed_response::<SignatureHelpRequest>(&pending.handle)
        else {
            return;
        };
        self.signature.pending = None;
        match result {
            Ok(Some(help)) if !help.signatures.is_empty() => {
                self.signature.popup = Some(ParameterInfo {
                    path: pending.path,
                    offset: pending.offset,
                    signatures: signature_lines(&help),
                });
                self.signature.last = Some(help);
            }
            Ok(_) | Err(_) => {
                if pending.explicit {
                    self.status = "No parameters".into();
                }
                self.signature.popup = None;
                self.signature.last = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_lsp::lsp_types::{ParameterInformation, SignatureInformation};

    fn signature(label: &str, parameters: Vec<ParameterLabel>) -> SignatureInformation {
        SignatureInformation {
            label: label.into(),
            documentation: None,
            parameters: Some(
                parameters
                    .into_iter()
                    .map(|label| ParameterInformation {
                        label,
                        documentation: None,
                    })
                    .collect(),
            ),
            active_parameter: None,
        }
    }

    #[test]
    fn active_parameters_are_located_by_text_or_utf16_offsets() {
        let help = SignatureHelp {
            signatures: vec![
                signature(
                    "greeting(String name, int times)",
                    vec![
                        ParameterLabel::Simple("String name".into()),
                        ParameterLabel::Simple("int times".into()),
                    ],
                ),
                signature(
                    "größe(ä: u8, b: u8)",
                    vec![
                        ParameterLabel::LabelOffsets([6, 11]),
                        ParameterLabel::LabelOffsets([13, 18]),
                    ],
                ),
            ],
            active_signature: Some(1),
            active_parameter: Some(1),
        };
        let lines = signature_lines(&help);
        assert_eq!(lines[0].active, Some((22, 31)));
        assert!(!lines[0].current);
        let (start, end) = lines[1].active.unwrap();
        let chars: String = lines[1]
            .label
            .chars()
            .skip(start)
            .take(end - start)
            .collect();
        assert_eq!(chars, "b: u8");
        assert!(lines[1].current);
        assert_eq!(utf16_range_to_chars("ab", 0, 5), None);
    }
}
