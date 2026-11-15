# Desktop app

The desktop interface of Steam Manifest Downloader, written in Rust. The window is drawn with egui, the layout is computed by taffy, and the look comes from one stylesheet that the app reads itself.

## Pieces

| Path | Role |
|---|---|
| `assets/style.css` | The stylesheet. Colours, spacing and sizes are changed here. |
| `tools/index.html` | Describes every screen and dialog: tags, ids, classes and text keys. |
| `tools/gen_views.py` | Turns `index.html` into `src/app/view/`. Run it after every change to `index.html`. |
| `src/style` | Style engine. Parses the stylesheet and computes the style of one element: selectors, cascade, variables per theme, inheritance, shorthands, units. |
| `src/dom` | Builds a view out of elements. Every element gets its computed style, a taffy node for the layout and egui shapes for drawing. Handles hover, focus, active, transitions, pseudo elements, line boxes and scrolling. |
| `src/app/view` | Generated screens and dialogs. Do not edit by hand. |
| `src/app/logic` | What every button, field and dialog does, one module per area. |
| `src/app/ui_state.rs` | Element state by id: classes, text, value, checked, disabled. The logic works on this store, the views read it. |
| `src/app/tasks.rs` | Runs backend calls on the tokio runtime and posts the result back as a closure over `&mut App`. |
| `src/backend` | The commands on top of `smd-core`, callable by name with JSON arguments, plus the event channel. |
| `src/platform.rs` | File dialogs, links, notifications, sound, taskbar progress. |
| `src/store.rs` | Small key/value file (`gui-state.json`) for values like the theme. |
| `src/import.rs` | Reads those values from the web view storage of versions before the native interface, once. |
| `vendor/egui_taffy` | Copy of egui_taffy with exact measuring for text, layers for popups and bottom-sticky elements. |

## Changing something

- **Look**: edit `assets/style.css`.
- **Structure**: edit `tools/index.html`, then run `python3 src-gui/tools/gen_views.py`.
- **Behaviour**: edit the module in `src/app/logic`. The modules use `app.ui`: `app.ui.show("id")`, `app.ui.set_text("id", ..)`, `app.ui.set_class("id", "name", on)`.
- **Lists and other dynamic content** are markup strings handed to `dom.markup(..)`. Escape every value with `escape_html` first.
- **Texts** come from `locales/*.json` through `t!(app, "key")`, in English and German.
- **Backend calls** go through `app.tasks.call("command", args, |app, result| ..)` so the window never blocks.

## Rules the builder needs help with

- `:has(input:checked)` and `:focus-within` are state flags that the view sets on the element.
- `:empty` is a state flag too. The generator sets it for elements whose class has an `:empty` rule.
- `order` is supported for grid items with a negative value, they come first. Elsewhere emit the children in visual order.

## Tests

`cargo test -p smd-gui` runs the engine tests, among them one that feeds every declaration of the stylesheet through the engine and fails on anything it does not understand.
