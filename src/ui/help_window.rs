pub fn get_help_text() -> &'static str {
    r#"
╔════════════════════════════════════════════════════════════════╗
║                      PMGR - Package Manager                    ║
║                         Keyboard Shortcuts                     ║
╚════════════════════════════════════════════════════════════════╝

NAVIGATION
  ↑ / k              Move up in the list
  ↓ / j              Move down in the list

SELECTION & ACTIONS
  TAB                Toggle selection (multi-select mode)
  ENTER              Confirm selection and proceed
  ESC                Cancel and exit

SEARCH
  Type characters    Filter packages by name (fuzzy search)
  Backspace          Delete last character from search

LAYOUT
  Alt+O              Switch to horizontal layout
  Alt+V              Switch to vertical layout

SYSTEM
  Ctrl+U             Update entire system (sudo pacman -Syu)

HELP
  ?                  Show/hide this help screen

───────────────────────────────────────────────────────────────────

TIPS
  • Use fuzzy search to quickly find packages
  • TAB to select multiple packages before confirming
  • System updates run in a floating window
  • Updates close automatically on success, Alt+X if error

"#
}
