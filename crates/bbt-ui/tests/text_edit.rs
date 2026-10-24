use bbt_ui::text_edit::{Command, Editor, Motion};

#[test]
fn ime_replaces_the_original_selection_only_on_commit() {
    let mut value = "a🙂z".to_string();
    let mut editor = Editor::new(&value);
    editor.handle(
        &mut value,
        Command::SetCaret {
            at: 1,
            extend: false,
        },
    );
    editor.handle(
        &mut value,
        Command::SetCaret {
            at: 3,
            extend: true,
        },
    );
    editor.handle(
        &mut value,
        Command::Preedit {
            text: "你好".into(),
            caret: Some(2),
        },
    );
    assert_eq!(value, "a🙂z");
    assert_eq!(editor.edit().replacement, Some([1, 3]));
    assert!(!editor.handle(&mut value, Command::Insert("x".into())));
    assert!(!editor.handle(
        &mut value,
        Command::Move {
            motion: Motion::End,
            extend: false
        }
    ));
    editor.handle(
        &mut value,
        Command::Preedit {
            text: "你".into(),
            caret: Some(1),
        },
    );
    assert_eq!(value, "a🙂z");
    editor.handle(&mut value, Command::Commit("你".into()));
    assert_eq!(value, "a你z");
    assert_eq!(editor.edit().caret, Some(2));
    assert!(!editor.composing());
}

#[test]
fn unicode_boundaries_cancellation_and_directional_selection_survive() {
    let mut value = "a🙂z".to_string();
    let mut editor = Editor::new(&value);
    assert!(!editor.handle(
        &mut value,
        Command::SetCaret {
            at: 2,
            extend: false
        }
    ));
    editor.handle(
        &mut value,
        Command::Move {
            motion: Motion::Left,
            extend: false,
        },
    );
    editor.handle(
        &mut value,
        Command::Move {
            motion: Motion::Left,
            extend: true,
        },
    );
    assert_eq!(editor.edit().select, Some([1, 3]));
    editor.handle(
        &mut value,
        Command::Preedit {
            text: "🙂".into(),
            caret: None,
        },
    );
    assert!(editor.edit().hide_caret);
    assert!(!editor.handle(
        &mut value,
        Command::Preedit {
            text: "🙂".into(),
            caret: Some(1)
        }
    ));
    editor.handle(&mut value, Command::CancelPreedit);
    assert_eq!(value, "a🙂z");
    assert_eq!(editor.edit().select, Some([1, 3]));
    editor.handle(&mut value, Command::Delete { backwards: false });
    assert_eq!(value, "az");
    editor.handle(&mut value, Command::Delete { backwards: true });
    assert_eq!(value, "z");
}
