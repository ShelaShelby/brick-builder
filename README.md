# Brick Builder

## ⬇️ Download (Windows)

### [**Download Brick-Builder-windows.zip**](https://github.com/ShelaShelby/brick-builder/releases/latest/download/Brick-Builder-windows.zip)

1. **Download** the zip with the link above.
2. **Unzip** it (right-click the file and choose *Extract All...*).
3. **Double-click `brick-builder.exe`** in the unzipped `Brick-Builder` folder.

Windows may show a blue SmartScreen box saying *"Windows protected your PC"*,
because the app is not signed. Click **More info**, then **Run anyway**. You
only need to do this once. Nothing else needs to be installed.

All versions are listed on the [releases page](https://github.com/ShelaShelby/brick-builder/releases/latest).

---

A native desktop app for building with toy bricks in 3D. Build freely on a big
baseplate, open one of five ready-made builds, and follow auto-generated
step-by-step instructions (also exportable as PDF).

Written in Rust with [Bevy](https://bevyengine.org) and
[egui](https://github.com/emilk/egui) (via `bevy_egui`). All parts are drawn in
code, so there are no asset files. The designs are original, generic brick
shapes.

## Run from source (developers)

Requirements: Rust (stable) and, on Windows, the Visual Studio C++ Build Tools.

```
cargo run --release
```

The first build downloads and compiles the dependencies, which takes a few minutes.

Run the tests (connection check, the five builds, instructions, file format, PDF):

```
cargo test --release
```

## Controls

| Action | How |
| --- | --- |
| Place a part | Pick a part and colour on the left, then left-click (Build tool) |
| Rotate part | `R` |
| Remove a brick | Right-click it, or hover and press `Delete` |
| Orbit / pan / zoom | Right-drag / middle-drag (or Shift+right-drag) / mouse wheel |
| Turn the view | `Q` / `E` |
| Select bricks | Select tool (`S`): click, Shift+click, or drag a box |
| Move selection | Drag a selected brick, or arrow keys and PgUp/PgDn |
| Rotate / delete / repaint selection | `R` / `Delete` / click a colour |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y` |
| New / open / save | `Ctrl+N` / `Ctrl+O` / `Ctrl+S` (files use the `.bricks` extension) |
| Instructions | Left/right arrow keys, the slider, or Back/Next |
| Help | `F1` |

## Connection check

Two parts are connected only where studs on one actually sit in the underside of
the other, on the stud grid and stacked directly on top of each other. Bricks
that only touch side by side hold nothing.

The app builds a connection graph rooted at the baseplates and shows the result
live:

- **Red**: the brick is floating, with no path to a baseplate.
- **Orange**: the brick (or a group of bricks) is weak, held on by one single stud.

The status bar shows the total bricks, stud joints and weak/floating counts.
You can still place floating bricks; they are only flagged.

## Ready-made builds

| Build | Size | Bricks |
| --- | --- | --- |
| Cozy Cottage | Small | 73 |
| Harbour Lighthouse | Medium | 373 |
| Castle Gate | Large | 939 |
| Railway Station | XL | 1915 |
| Town Square | XXL | 4677 |

The builds are generated in code (see `src/builds`). The tests check that each
one has zero flagged bricks, no overlapping parts and valid instructions.

## Instructions

Instructions are generated from any build. The **Make instructions** button
does this for your own build. Bricks are placed bottom to top, and a brick is
only placed once it can attach to something already built, so every step can
really be built. Small builds get a few bricks per step and big builds get more.
**Export PDF** writes one page per step with the step picture and the parts list.

## Project layout

- `src/parts.rs`, `src/model.rs`: parts, colours, placed bricks, save format
- `src/connect.rs`: connection check (stud joints, floating and weak detection)
- `src/instructions.rs`: step generation and validation
- `src/builds/`: the five ready-made builds
- `src/meshgen.rs`: brick meshes (rounded edges, studs)
- `src/raster.rs`, `src/pdf.rs`: software renderer for thumbnails, icons and the PDF
- `src/app/`: the Bevy app (scene, camera, editor, UI)
