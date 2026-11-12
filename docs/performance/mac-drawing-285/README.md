# Mac drawing navigation in BIMCraft Rust

Issue [#285](https://github.com/IPedrax/BIMCraft/issues/285) reports slow pan and zoom in editable floor plans, sections and elevations. The [maintainer's update](https://github.com/IPedrax/BIMCraft/issues/285#issuecomment-6813576000) identifies a curve triangulation change fixed after release.

**Builds.** Optimized arm64 builds (rustc 1.99.1) of the v0.11.0 tag (`323afba89d2ab6842d7f6b6cbba48242867c9aba`) and the November 12 main revision (`0c3ca7723e2333d7eba73a0282f2c2ac4b7aa701`). Apple M5 Pro, 48 GB, macOS 27.0.1 (26A434).

**Fixture.** The release created one native project using the stock 20-storey highrise, 48 circular detail lines on its Level 1, a section through its middle and a south elevation. Both builds opened identical bytes. The project contains 9,571 elements. To open the exact file in the desktop app, run `gzip -dc tower-views.bimc.gz > tower-views.bimc`, then open `tower-views.bimc` with File → Open. Use Floor Plans → Level 1, Sections → Section 1 and Elevations → South. Its uncompressed SHA-256 is `6f58a655bdd4b16aed672c0642f4b82f082663e387364e5cd1e07e28e0692f8e`. Regenerating the sample gives it new GUIDs.

**Method.** In an isolated checkout for each revision, copy `plan_navigation.rs` into `crates/ui-egui/examples/` and enable the `wgpu` feature on the existing `egui_kittest` dev dependency in `crates/ui-egui/Cargo.toml`. Build with `cargo build --release -p bimcraft-ui-egui --example plan_navigation`. Copy each revision's binary to a separate path. From a folder with those two binaries:

```sh
./v011-bench create tower-views.bimc
for view in plan section elevation; do
    ./v011-bench metal tower-views.bimc "$view"
    ./main-bench metal tower-views.bimc "$view"
done
```

For a direct visual check, add an output PNG path after the view name. Each process opens a fresh session, injects middle-button drags and wheel events into the egui view, and checks the projected model coordinates for each gesture. The renderer uses the native Metal adapter with a 1280 × 800 offscreen target. The timer includes UI processing, tessellation, GPU execution and synchronous pixel readback. The 10 ms pause between inputs is excluded. These figures describe warmed drawing frames. They do not time opening a project, waiting for a background worker to replace its preview with final geometry after an edit, physical input delivery, window composition or a Retina display.

Each gesture has 20 warm-up frames followed by 100 measured frames. The separately monitored comparison below includes one full pair per view. The three preliminary paired trials per view alternated which revision ran first across trials. The low-background run sampled process activity roughly once per second. Each retained trial had three quiet checks before launch and stayed below 350% external CPU (3.5 of 18 cores), without an observed active Rust compiler or BIMCraft test executable. The separate initial-load-check.jsonl captures the preliminary run which only checked for an active compiler at launch. A concurrent core test suite was observed during that run, so its numbers are excluded from the table.

## Results

Each cell gives the median / 95th percentile frame time in milliseconds from the first complete low-background comparison. Each gesture was measured over 100 frames. The raw per-trial summaries are in [results.jsonl](results.jsonl).

| View | Gesture | v0.11.0 median / p95 | main median / p95 |
|---|---|---:|---:|
| Plan | Pan | 7.8 / 8.2 | 6.2 / 6.9 |
| Plan | Zoom | 7.7 / 8.2 | 6.4 / 7.0 |
| Section | Pan | 6.8 / 7.2 | 6.7 / 7.3 |
| Section | Zoom | 6.7 / 7.2 | 6.9 / 7.4 |
| Elevation | Pan | 6.1 / 6.5 | 6.2 / 6.8 |
| Elevation | Zoom | 6.2 / 6.6 | 6.1 / 6.7 |

The slowest single frame in the quiet comparison was 8.5 ms on v0.11.0 and 7.6 ms on main. The largest p95 among the median-of-three preliminary paired trials was 11.3 ms on the release and 10.8 ms on main. The preliminary run contained an isolated 103.2 ms release pan frame and a 32.3 ms main pan frame during variable machine load.

Neither build reproduced a sustained stall on this fixture. A small drawing which reproduces the reported issue on the reporter’s hardware would let a developer distinguish input or presentation delay, slow background regeneration, and a particular geometry path.
