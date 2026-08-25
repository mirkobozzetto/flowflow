# Verification bundle

Safe, already run by ship:

```
cargo check --features mobile
cargo clippy --features mobile
cargo test --test space_error_test
grep -rn 'e.to_string()' src/ui/sidebar src/ui/settings/spaces.rs   # no output
```

Device (T03, Mirko):

1. Sidebar, space menu > Renommer, with the server not deployed:
   red line in French under the header.
2. Menu > Membres: a member without a public name shows a circle icon
   and 6 grey characters, "Retirer" still offered to the owner.
3. + > Copier le lien: panel closes, grey line « Lien copié » under the
   header. Tap the menu or open any panel: the line is gone.
