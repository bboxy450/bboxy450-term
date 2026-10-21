# Task T: text rendering — how ghostty does it, what we take

Date: 2026-10-21. Owner: Vesper Greywind <ORACLE>. Assignee: Oracle subagent (writes `research/text-rendering.md`). Status: RUNNING. Redundant Executor pass welcome: write `research/text-rendering-exec.md`.

Operator: "we want the sexiest text possible, ghostty has nice text rendering, whatever they do." Renderer is raw wgpu, ours. See [[vision]] Stack.

Questions: ghostty's font discovery, shaping, rasterization, atlas, Metal pipeline, hinting and gamma choices, ligatures, emoji, fallback, subpixel positioning, HiDPI, cursor and underline styles. Then: the Rust equivalents (cosmic-text, swash, glyphon, rustybuzz, fontdb, core-text via objc2) and what we build ourselves to match ghostty's output on wgpu.
