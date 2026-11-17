// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Emphasis: one vocabulary for every state word the suite draws.
//!
//! A reply's `[[Done]]`, a task's `reported` and a message's `rejected` read
//! the same table, so the same state always has the same tone and symbol. The
//! seat prompt's reply rules are generated from it too, so what agents are told
//! to write and what the renderer draws cannot drift. Display only: stored text
//! keeps its Markdown.
//!
//! Adapted from pi-emphasis (MIT), https://github.com/johnlindquist/pi-emphasis
//! at f884b736c489944ee778ab951c7f78580f137b02, `index.ts`: the pill word tables
//! (`VOCABULARY`, `CONTEXT_WORDS`, `SYNONYMS`), the label and action lead-in
//! word lists, the detail rule of `splitLabel` and the shape of `replyGuidance`.
//! Symbols are Unicode text instead of Nerd Font glyphs, tones are this suite's
//! tokens, and task/message states join the table.
use crate::TOKENS;

/// What a word says about the thing beside it. Each tone has one colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Settled: done, passed, accepted.
    Success,
    /// Something is wrong.
    Error,
    /// Look here: an ask, a decision, an unchecked claim, a report to review.
    Warning,
    /// Running now.
    Active,
    /// Consequences: money, data loss, no undo. Drawn solid.
    Danger,
    /// The kind of work a reply is.
    Mode,
    /// Who owns what a change touches. Drawn solid.
    Owner,
    /// Context, and any other short label.
    Context,
}

impl Tone {
    /// The token colour, as `#RRGGBB`.
    pub fn hex(self) -> &'static str {
        let c = &TOKENS.color;
        match self {
            Tone::Success => c.ui.ok,
            Tone::Error | Tone::Danger => c.state.errored,
            Tone::Warning => c.state.waiting,
            Tone::Active | Tone::Owner => c.dark.accent,
            Tone::Mode => c.role.architect,
            Tone::Context => c.dark.accent_soft,
        }
    }
    /// A solid tone fills its pill; the others tint it.
    pub fn solid(self) -> bool {
        matches!(self, Tone::Danger | Tone::Owner)
    }
}

/// (canonical word, tone, symbol, accepted synonyms). The first words of each
/// tone are the ones the prompt teaches (`core`); synonyms display as the word.
/// (word, tone, symbol, synonyms).
type Word = (&'static str, Tone, &'static str, &'static [&'static str]);
const WORDS: &[Word] = &[
    ("Failed", Tone::Error, "✕", &["fail", "failing", "error"]),
    ("Blocked", Tone::Error, "⊘", &[]),
    ("Broken", Tone::Error, "✕", &[]),
    ("Unsafe", Tone::Error, "⚠", &[]),
    ("Rejected", Tone::Error, "✕", &[]),
    ("Down", Tone::Error, "↓", &[]),
    ("Needs you", Tone::Warning, "●", &[]),
    ("Decision", Tone::Warning, "◇", &[]),
    ("Unverified", Tone::Warning, "?", &[]),
    ("Partial", Tone::Warning, "◐", &[]),
    ("Skipped", Tone::Warning, "»", &[]),
    ("Pending", Tone::Warning, "◷", &["waiting"]),
    ("At risk", Tone::Warning, "⚠", &[]),
    ("Reported", Tone::Warning, "◉", &["asked"]),
    ("Review", Tone::Warning, "◉", &["needs review"]),
    ("Todo", Tone::Warning, "☐", &[]),
    ("Stale", Tone::Warning, "↺", &[]),
    ("Draft", Tone::Warning, "✎", &[]),
    ("In progress", Tone::Active, "◔", &["running"]),
    (
        "Done",
        Tone::Success,
        "✓",
        &["complete", "completed", "ok", "finished"],
    ),
    ("Passed", Tone::Success, "✓", &["pass"]),
    ("Observed", Tone::Success, "◉", &[]),
    ("Fixed", Tone::Success, "✓", &[]),
    ("Live", Tone::Success, "↑", &["shipped"]),
    ("Recommended", Tone::Success, "★", &[]),
    ("Keep", Tone::Success, "✓", &[]),
    ("Accepted", Tone::Success, "✓", &["approved"]),
    ("Ready", Tone::Success, "✓", &[]),
    ("Merged", Tone::Success, "✓", &[]),
    ("Verified", Tone::Success, "✓", &["safe", "healthy"]),
    ("Costs money", Tone::Danger, "$", &[]),
    ("Destructive", Tone::Danger, "✕", &[]),
    ("Irreversible", Tone::Danger, "⊗", &[]),
    ("Brainstorm", Tone::Mode, "✦", &[]),
    ("Investigate", Tone::Mode, "⌕", &[]),
    ("Preview", Tone::Mode, "◉", &[]),
    ("Implement", Tone::Mode, "⚒", &[]),
    ("Qualify", Tone::Mode, "∎", &[]),
    ("Owned", Tone::Owner, "●", &[]),
    ("Shared", Tone::Owner, "◎", &[]),
    ("Foreign", Tone::Owner, "○", &[]),
    ("Inferred", Tone::Context, "∴", &[]),
    ("New", Tone::Context, "+", &[]),
    ("Changed", Tone::Context, "Δ", &[]),
    ("Optional", Tone::Context, "○", &[]),
];

/// A pill read from its label: a known word, optionally followed by detail
/// ("Passed 12/12", "Failed: 3 tests").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pill {
    pub tone: Tone,
    pub symbol: &'static str,
    /// The canonical word followed by the label's detail.
    pub text: String,
}

/// The pill a label names, or `None` when its first word is not in the table
/// (then `[[label]]` stays an ordinary wiki link).
pub fn pill(label: &str) -> Option<Pill> {
    let text = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() || text.chars().count() > 32 {
        return None;
    }
    let lower = text.to_lowercase();
    // Longest match first, so "needs review" wins over "needs you"-like prefixes.
    let mut best: Option<(usize, &Word)> = None;
    for entry in WORDS {
        for word in std::iter::once(entry.0).chain(entry.3.iter().copied()) {
            let w = word.to_lowercase();
            let fits = lower == w
                || (lower.starts_with(&w)
                    && matches!(lower[w.len()..].chars().next(), Some(' ' | ':')));
            if fits && best.is_none_or(|(len, _)| w.len() > len) {
                best = Some((w.len(), entry));
            }
        }
    }
    let (len, &(word, tone, symbol, _)) = best?;
    Some(Pill {
        tone,
        symbol,
        text: format!("{word}{}", &text[len..]),
    })
}

/// The tone of a task, message or document state word (`reported`,
/// `accepted`, `running`, ...). The status colour every list draws.
pub fn state(word: &str) -> Option<Tone> {
    pill(word).map(|p| p.tone)
}

/// Headings and a line's leading bold label take a symbol from their word.
const LABELS: &[(&str, &str)] = &[
    ("Finding", "⌕"),
    ("Result", "⚑"),
    ("Status", "◔"),
    ("Evidence", "◉"),
    ("Cause", "↯"),
    ("Effect", "⇢"),
    ("Fix", "⚒"),
    ("Risk", "⚠"),
    ("Limits", "⚠"),
    ("Cost", "$"),
    ("Decision", "◇"),
    ("Options", "☰"),
    ("Plan", "☰"),
    ("Next", "→"),
    ("Why", "?"),
    ("How", "⚙"),
    ("Rule", "§"),
    ("Example", "❯"),
    ("Checked", "✓"),
    ("Verdict", "§"),
    ("Rollback", "↺"),
    ("Undo", "↺"),
    ("Review", "◉"),
    ("Scoreboard", "▤"),
    ("Removed", "−"),
    ("Changed", "Δ"),
    ("Files", "▤"),
    ("Commands", "❯"),
    ("Gaps", "◌"),
];
/// Bold lead-ins ending in a colon name the action a pill calls for; they take
/// a symbol anywhere in a line.
const ACTIONS: &[(&str, &str)] = &[
    ("Check", "☑"),
    ("Do", "▶"),
    ("Next", "→"),
    ("Unblock", "⊙"),
    ("Left", "…"),
    ("Run", "▶"),
    ("Guard", "◈"),
    ("Undo", "↺"),
    ("See", "↗"),
];

/// The symbol a heading or a leading bold label carries ("Risk", "Next:").
pub fn label(text: &str) -> Option<&'static str> {
    let word = text.trim().trim_end_matches(':').trim();
    LABELS
        .iter()
        .chain(ACTIONS)
        .find(|(w, _)| w.eq_ignore_ascii_case(word))
        .map(|(_, s)| *s)
}

/// The symbol of a bold action lead-in anywhere in a line ("Check:", "Do:").
/// The colon is required.
pub fn action(text: &str) -> Option<&'static str> {
    let word = text.trim().strip_suffix(':')?.trim();
    ACTIONS
        .iter()
        .find(|(w, _)| w.eq_ignore_ascii_case(word))
        .map(|(_, s)| *s)
}

/// The tone of an inline mark: ✓ settled, ✗ wrong, → a step, tree lines dim.
pub fn mark(c: char) -> Option<Tone> {
    match c {
        '✓' | '✔' => Some(Tone::Success),
        '✗' | '✘' => Some(Tone::Error),
        '→' => Some(Tone::Active),
        _ => None,
    }
}

/// The reply-style rules a seat receives, with every word list read from the
/// table above. Short on purpose: it rides in every seat's prompt.
pub fn guidance() -> String {
    let core = |tone: Tone| {
        WORDS
            .iter()
            .filter(|w| w.1 == tone)
            .map(|w| format!("[[{}]]", w.0))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let words = |list: &[(&str, &str)]| list.iter().map(|(w, _)| *w).collect::<Vec<_>>().join(", ");
    format!(
        "## Reply style\n\n\
The bboxy renderer draws pills and symbols in your replies, task reports and messages. Stored text stays plain Markdown.\n\n\
- **Pills.** Write `[[Word]]` beside what it describes; the word picks colour and symbol. Use exactly these words, optionally with short detail (`[[Passed 212/212]]`, `[[Failed: 3 tests]]`):\n\
  - red, wrong: {}\n\
  - yellow, look here: {}\n\
  - blue, running: {}\n\
  - green, settled: {}\n\
  - solid red, consequences, required before such an action: {}\n\
  - cyan, kind of work; open a substantive reply with one: {}\n\
  - solid blue, ownership of what a change touches: {}\n\
  - soft blue, context: {}\n\
- **Evidence.** Mark claims [[Observed]], [[Inferred]] or [[Unverified]].\n\
- **Every red, yellow and solid red pill carries its action**, introduced by a bold lead-in: {}. Example: `[[Unverified]] the remote seat. **Check:** send a redirect mid-turn.`\n\
- **Labels.** Start sections with `##` headings or bold lead labels; these words get a symbol: {}. Never type symbols yourself.\n\
- **Scoreboard.** A long report may open with one line of counts: `[[Done 3]] [[Failed 1]] [[Needs you 1]]`.\n\
- **Shapes.** `→` for a sequence, `✓`/`✗` beside checked items, `├──`/`└──` for what belongs to what.\n\
- Other `[[slug]]` links stay knowledge links. Keep pills out of code, paths, commands and exact formats.\n",
        core(Tone::Error),
        core(Tone::Warning),
        core(Tone::Active),
        core(Tone::Success),
        core(Tone::Danger),
        core(Tone::Mode),
        core(Tone::Owner),
        core(Tone::Context),
        words(ACTIONS),
        words(LABELS),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn words_synonyms_and_detail_read_as_one_pill() {
        assert_eq!(pill("Done").unwrap().tone, Tone::Success);
        assert_eq!(pill("completed").unwrap().text, "Done");
        assert_eq!(pill("Passed 212/212").unwrap().text, "Passed 212/212");
        assert_eq!(pill("Failed: 3 tests").unwrap().text, "Failed: 3 tests");
        assert_eq!(pill("needs review").unwrap().text, "Review");
        assert_eq!(pill("Needs you").unwrap().symbol, "●");
        assert!(pill("Costs money").unwrap().tone.solid());
        // Unknown words and knowledge slugs stay links.
        assert_eq!(pill("tasks/2026-11-13-acp-steering-parity"), None);
        assert_eq!(pill("Donegal"), None);
        assert_eq!(pill(""), None);
    }
    #[test]
    fn task_and_message_states_share_the_pill_tones() {
        assert_eq!(state("running"), Some(Tone::Active));
        assert_eq!(state("reported"), Some(Tone::Warning));
        assert_eq!(state("accepted"), Some(Tone::Success));
        assert_eq!(state("done"), Some(Tone::Success));
        assert_eq!(state("rejected"), Some(Tone::Error));
        assert_eq!(state("blocked"), Some(Tone::Error));
        assert_eq!(state("asked"), Some(Tone::Warning));
        assert_eq!(state("superseded"), None);
        // Not started reads muted, as before.
        assert_eq!(state("open"), None);
        assert_eq!(state("assigned"), None);
    }
    #[test]
    fn labels_and_actions_take_symbols() {
        assert_eq!(label("Risk:"), Some("⚠"));
        assert_eq!(label("Next"), Some("→"));
        assert_eq!(action("Check:"), Some("☑"));
        assert_eq!(action("Check"), None);
        assert_eq!(label("Banana"), None);
    }
    #[test]
    fn the_prompt_teaches_exactly_the_drawn_words() {
        let g = guidance();
        for (word, ..) in WORDS {
            assert!(g.contains(&format!("[[{word}]]")), "{word} missing");
        }
    }
}
