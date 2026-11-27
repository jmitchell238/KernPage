# metafile

Parses Windows Metafiles (WMF, and a practical subset of EMF) into a simple,
renderer-agnostic display list (`Metafile { bounds, inch, items, .. }`).

Written only from Microsoft's public open specifications, [MS-WMF] and
[MS-EMF] (published under the Open Specifications Promise). No code from other
projects was used. No dependencies.

```rust
let mf = metafile::parse(&bytes)?;
for item in &mf.items { /* draw */ }
```

Output coordinates are in the *base window space*: the logical space of the
window in effect at the first drawing record (for placeable WMF files this is
the space of the placeable bounding box). See the crate docs for details.

Licensed under GPL-3.0-only (versions up to 0.2.0: MIT OR Apache-2.0).

[MS-WMF]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/
[MS-EMF]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-emf/
