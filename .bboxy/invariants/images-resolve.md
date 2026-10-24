---
id: 01M5NK2SRCVNWRPHJCTMF5E0XQ
type: invariant
number: 55
title: the face is the identity file and every image wikilink resolves
status: active
check: sh scripts/images.sh
cost: a wrong face is a wrong identity on every profile the records feed; a broken image link is a document that lies about what it shows
links:
  - { to: "[[document-viewer]]", rel: constrains }
  - { to: "[[d52-brand-and-face]]", rel: decision }
created: 2026-10-23T23:56:02Z
updated: "2026-10-24T15:36:34Z"
---
# The face is the identity file and every image wikilink resolves

**Property.** `.bboxy/images/bboxy.png` is byte for byte the operator's identity
file (`~/.agents/identity/bboxy.png`, the electric-blue fox in the black hoodie,
800 × 800; sha256 `bc5cc774…6370` pinned in the script), every image
wikilink `![[name.ext]]` in a record under `.bboxy/` names a file that exists in
`.bboxy/images/`, and **every tracked app icon (`.icns`, `.iconset`, `AppIcon*`)
carries a `.source-sha256` sidecar equal to the face's hash** (operator,
2026-10-24: "we always use my bboxy icon and identity pic on everything") ([[document-viewer]] images; operator, 2026-10-23: "our profile
identity picture as part of invariance", "documents should include images, we
can have image folder and wiki links to images").

**Check.** `sh scripts/images.sh` — hash compare plus a resolve loop over every
`![[…]]` with an image extension; exit 0 held, non-zero names the link or the
hash. Fast, in the default set.

**Cost of a violation.** A wrong face is a wrong identity on every profile the
records feed; a broken image link is a document that lies about what it shows.

**Evidence.** `sh scripts/images.sh` — `images: held` at the commit that adds it
(one image, zero links yet).

Owner: Corvin Duskmantle <ARCHITECT>.
