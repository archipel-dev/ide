//! Dialog contents: a text field with optional check boxes (Find in Files options).

use archipel_kit::openapi::ui::ValidationInfo;
use archipel_kit_tui::controls::{
    CheckBoxControl, CheckState, FieldEvent, TextFieldControl, TextFieldState,
};
use archipel_kit_tui::dialog::DialogContent;
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::Event;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::runtime::Runtime;
use std::cell::RefCell;
use std::rc::Rc;

const FIELD: Id = Id::new("tui.input.field");
const CHECK: Id = Id::new("tui.input.check");

/// What the dialog hands back when it closes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Input {
    pub text: String,
    pub flags: Vec<bool>,
}

type Validator = Box<dyn Fn(&str) -> Option<String>>;

/// One text field and a row of check boxes.
pub struct InputContent {
    label: String,
    placeholder: String,
    field: TextFieldState,
    options: Vec<(String, CheckState)>,
    required: bool,
    validator: Option<Validator>,
    shared: Rc<RefCell<Input>>,
}

impl InputContent {
    /// A field labeled `label`, starting with `initial`; the result is kept in `shared`.
    pub fn new(label: &str, initial: &str, shared: Rc<RefCell<Input>>) -> Self {
        let field = TextFieldState::with_text(initial);
        let mut content = Self {
            label: label.to_owned(),
            placeholder: String::new(),
            field,
            options: Vec::new(),
            required: false,
            validator: None,
            shared,
        };
        content.publish();
        content
    }

    #[must_use]
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = text.to_owned();
        self
    }

    /// Adds a check box.
    #[must_use]
    pub fn option(mut self, label: &str, on: bool) -> Self {
        self.options.push((
            label.to_owned(),
            if on {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            },
        ));
        self.publish();
        self
    }

    /// An empty text blocks OK. Off by default: the dialog validates 300 ms after the last
    /// change, so a fast typist's Enter would be refused; the caller reports an empty value.
    #[must_use]
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// A check that returns an error message for a bad text.
    #[must_use]
    pub fn validate_with(mut self, check: impl Fn(&str) -> Option<String> + 'static) -> Self {
        self.validator = Some(Box::new(check));
        self
    }

    fn publish(&mut self) {
        *self.shared.borrow_mut() = Input {
            text: self.field.text().to_owned(),
            flags: self.options.iter().map(|(_, s)| s.is_checked()).collect(),
        };
    }
}

impl DialogContent for InputContent {
    fn height(&self, _width: u16) -> u16 {
        1 + self.options.len().min(1) as u16 * 2
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let p = rt.look.palette;
        TextFieldControl::new(FIELD)
            .label(&self.label)
            .placeholder(&self.placeholder)
            .on(p.darker_black)
            .show(
                rt,
                &self.field,
                Rect::new(area.x, area.y, area.width, 1),
                buf,
            );
        let mut x = area.x;
        let y = area.y + 2;
        for (index, (label, state)) in self.options.iter().enumerate() {
            if y >= area.bottom() {
                break;
            }
            let rect = CheckBoxControl::new(CHECK.with(index as u64), label)
                .on(p.darker_black)
                .show(rt, *state, x, y, buf);
            x = rect.right() + 2;
        }
    }

    fn event(&mut self, rt: &mut Runtime, event: &Event) -> bool {
        let mut used = matches!(
            TextFieldControl::new(FIELD).handle(rt, &mut self.field, event),
            FieldEvent::Changed | FieldEvent::Consumed | FieldEvent::Cleared
        );
        for (index, (label, state)) in self.options.iter_mut().enumerate() {
            used |= CheckBoxControl::new(CHECK.with(index as u64), label).handle(rt, state, event);
        }
        self.publish();
        used
    }

    fn revision(&self) -> u64 {
        self.field.revision
    }

    fn validate(&self) -> Vec<ValidationInfo> {
        let text = self.field.text();
        if self.required && text.trim().is_empty() {
            return vec![ValidationInfo::error("Enter a value")];
        }
        match self.validator.as_ref().and_then(|check| check(text)) {
            Some(message) => vec![ValidationInfo::error(message)],
            None => Vec::new(),
        }
    }

    fn preferred_focus(&self) -> Option<Id> {
        Some(FIELD)
    }
}
