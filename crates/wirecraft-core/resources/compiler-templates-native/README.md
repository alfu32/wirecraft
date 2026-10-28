# Wirecraft-authored compiler template sets

Threadworks 2.6.7 has no template set for these technologies, so they are not part of the
byte-identical baseline (`../compiler-templates`, see PROVENANCE.json) or of its generated
overlay (`../compiler-templates-wirecraft`, see scripts/template_overlay.py). They are edited
by hand, use the same roles, context and Pebble/MiniJinja syntax as the baseline sets, and
are embedded by `build.rs` under the `native/` prefix.

- `rust/`: a single-file Rust program with no dependencies (`rustc main.rs` or the
  generated `Cargo.toml`). Link buffers are file-scope statics with interior mutability;
  `push`/`pop` move typed values through them.
