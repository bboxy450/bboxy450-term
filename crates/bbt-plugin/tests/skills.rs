// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

use bbt_plugin::skills::{
    MAX_CONTENT_BYTES, MAX_DESCRIPTION_BYTES, MAX_NAME_BYTES, MAX_SKILLS, MAX_SKILLS_BYTES, Skill,
    validate,
};

fn skill() -> Skill {
    Skill {
        name: "knowledge-base".into(),
        description: "Read and write records with the current knowledge tools.".into(),
        content: "---\nname: knowledge-base\n---\n\nResolve before fetching.\n".into(),
    }
}

fn code(skills: &[Skill]) -> String {
    validate(skills).unwrap_err().code
}

#[test]
fn empty_and_sixteen_unique_skills_are_allowed_but_duplicates_are_not() {
    validate(&[]).unwrap();
    let mut skills: Vec<_> = (0..MAX_SKILLS)
        .map(|index| {
            let mut entry = skill();
            entry.name = format!("skill-{index}");
            entry
        })
        .collect();
    validate(&skills).unwrap();
    skills.push(skill());
    assert_eq!(code(&skills), "skills_too_large");
    assert_eq!(code(&[skill(), skill()]), "duplicate_skill");
}

#[test]
fn names_are_bounded_ascii_words_with_single_internal_hyphens() {
    for name in ["a", "0", "a-0-b", &"a".repeat(MAX_NAME_BYTES)] {
        let mut entry = skill();
        entry.name = name.into();
        validate(&[entry]).unwrap();
    }
    for name in [
        "",
        "-a",
        "a-",
        "a--b",
        "Upper",
        "a_b",
        "a/b",
        "é",
        "a\0b",
        &"a".repeat(MAX_NAME_BYTES + 1),
    ] {
        let mut entry = skill();
        entry.name = name.into();
        assert_eq!(code(&[entry]), "invalid_skill", "name={name:?}");
    }
}

#[test]
fn descriptions_and_content_are_nonblank_and_content_rejects_nul() {
    for blank in ["", " \r\n\t", "\u{2003}\u{a0}"] {
        let mut entry = skill();
        entry.description = blank.into();
        assert_eq!(code(&[entry]), "invalid_skill");
        let mut entry = skill();
        entry.content = blank.into();
        assert_eq!(code(&[entry]), "invalid_skill");
    }
    let mut entry = skill();
    entry.content = "valid text\0hidden tail".into();
    assert_eq!(code(&[entry]), "invalid_skill");
}

#[test]
fn utf8_limits_count_bytes_and_keep_exact_document_content() {
    let mut entry = skill();
    entry.description = "é".repeat(MAX_DESCRIPTION_BYTES / 2);
    entry.content = "é".repeat(MAX_CONTENT_BYTES / 2);
    validate(std::slice::from_ref(&entry)).unwrap();
    let restored: Skill = serde_json::from_slice(&serde_json::to_vec(&entry).unwrap()).unwrap();
    assert_eq!(restored, entry);
    let mut oversized = entry.clone();
    oversized.description.push('x');
    assert_eq!(code(&[oversized]), "skill_too_large");
    entry.content.push('x');
    assert_eq!(code(&[entry]), "skill_too_large");
}

#[test]
fn whole_array_budget_includes_metadata_and_json_punctuation() {
    let mut skills: Vec<_> = (0..4)
        .map(|index| {
            let mut entry = skill();
            entry.name = format!("skill-{index}");
            entry.content.clear();
            entry
        })
        .collect();
    let overhead = serde_json::to_vec(&skills).unwrap().len();
    for entry in &mut skills[..3] {
        entry.content = "x".repeat(MAX_CONTENT_BYTES);
    }
    skills[3].content = "x".repeat(MAX_SKILLS_BYTES - overhead - 3 * MAX_CONTENT_BYTES);
    assert_eq!(serde_json::to_vec(&skills).unwrap().len(), MAX_SKILLS_BYTES);
    validate(&skills).unwrap();
    skills[3].content.push('x');
    assert!(skills[3].content.len() <= MAX_CONTENT_BYTES);
    assert_eq!(code(&skills), "skills_too_large");
}

#[test]
fn json_escaping_can_exceed_envelope_budget_without_exceeding_text_limits() {
    let skills: Vec<_> = (0..4)
        .map(|index| {
            let mut entry = skill();
            entry.name = format!("skill-{index}");
            entry.content = format!("x{}", "\u{1}".repeat(20_000));
            entry
        })
        .collect();
    assert!(
        skills
            .iter()
            .map(|entry| entry.content.len())
            .sum::<usize>()
            < MAX_SKILLS_BYTES
    );
    assert!(
        skills
            .iter()
            .all(|entry| entry.content.len() <= MAX_CONTENT_BYTES)
    );
    assert!(serde_json::to_vec(&skills).unwrap().len() > MAX_SKILLS_BYTES);
    assert_eq!(code(&skills), "skills_too_large");
}
