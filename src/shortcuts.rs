use egui::{Key, KeyboardShortcut, Modifiers};

/// Focus the search field
pub const FOCUS_SEARCH: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::L);

/// Navigate note list down
pub const NAV_DOWN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::J);

/// Navigate note list up
pub const NAV_UP: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);

/// Create a new note
pub const NEW_NOTE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);

/// Rename the selected note
pub const RENAME_NOTE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::R);

/// Delete the selected note
pub const DELETE_NOTE: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Delete);

/// Paste clipboard as new note
pub const PASTE_AS_NOTE: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::V);

/// Open settings/preferences window
pub const OPEN_SETTINGS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma);

/// Toggle bookmark on the selected note
pub const BOOKMARK_TOGGLE: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::D);

/// Deselect the current note and focus search
pub const DESELECT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);

/// Focus the tag input field
pub const FOCUS_TAGS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::T);

// ─── Find Next/Prev shortcuts (Stream 2) ─────────────────────────────────

/// Find next search match in editor
pub const FIND_NEXT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::G);

/// Find previous search match in editor
pub const FIND_PREV: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::G);

// ─── Formatting shortcuts (Stream 5) ───────────────────────────────────────

/// Toggle bold (**text**)
pub const BOLD: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::B);

/// Toggle italic (*text*)
pub const ITALIC: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::I);

/// Toggle strikethrough (~~text~~)
pub const STRIKETHROUGH: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);

/// Toggle inline code (`text`)
pub const CODE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::E);

/// Indent selected lines (add 4 spaces)
pub const INDENT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::CloseBracket);

/// Outdent selected lines (remove up to 4 spaces)
pub const OUTDENT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::OpenBracket);

// ─── Import/Export/Print shortcuts (Stream 6) ─────────────────────────────

/// Import a file as a new note
pub const IMPORT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::I);

/// Export the current note
pub const EXPORT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::E);

/// Print the current note (open as HTML in browser)
pub const PRINT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::P);
