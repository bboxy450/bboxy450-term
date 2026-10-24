---
id: 01M5NK2SRDBT1KBFY3MHTZAS8S
type: decision
number: 52
title: "everything is branded bboxy450-, the fox is the face everywhere, records carry images by wikilink"
status: decided
decided: 2026-10-23
operator:
  - "invariance # something - everything is branded as bboxy450- prefix, everything!"
  - "and our profile identity picture as part of invariance and document should include images, we can have image folder and wiki links to images, that would work!"
links:
  - { to: "[[bboxy450-prefix]]", rel: constrains }
  - { to: "[[images-resolve]]", rel: constrains }
  - { to: "[[document-viewer]]", rel: decides }
  - { to: "[[licensing]]", rel: decides }
created: 2026-10-23T23:56:02Z
updated: "2026-10-24T15:36:34Z"
---
# Decision 52: brand and face

Decided 2026-10-23.

**Brand.** Every name the outside world meets starts with `bboxy450-`: crates, binaries, the GitHub organisation, channels, plugins, cross-OS kits. Internal `bbt-*` names last only while a crate is internal; the licensing map names what each publishes as. Invariant [[bboxy450-prefix]], check `sh scripts/brand.sh`, in the default set, held today. Rejected: renaming the internal crates now (see [[d51-frontmatter-is-the-gui-kit]]); a brand rule without a check.

**Face.** The electric-blue fox in the black hoodie is the face of everything bboxy450: every profile, package, agent and record that shows a picture. It lives in the repository at `.bboxy/images/bboxy.png`, byte for byte the identity file, hash pinned; nothing generates another or substitutes a stock image. It is identity, not licensed code ([[d50-licence-when-mit-when-apache]]).

**Images in records.** Records carry images by wikilink: `![[bboxy.png]]` renders the image inline, `[[bboxy.png]]` links to it; images live in `.bboxy/images/` as files, not records (they carry no frontmatter and no number; an image a seat produces is an artifact of kind image, [[work-items]]). The viewer draws them at true size, never scaled ([[renderer]] §4.4b applies). Invariant [[images-resolve]], check `sh scripts/images.sh`: the face is the identity file and every image wikilink resolves. Rejected: images inline as data URIs (unreadable diffs); images anywhere in the tree (links that rot).

Owner: Corvin Duskmantle <ARCHITECT>.

**Amended 2026-10-24 (operator, verbatim: "we always use my bboxy icon and identity pic on everything").** Every bundle, app and probe uses an icon generated from the face: a tracked `.icns`, `.iconset` or `AppIcon*` carries a `.source-sha256` sidecar equal to the face's hash, and `scripts/images.sh` fails any that does not ([[images-resolve]]).
