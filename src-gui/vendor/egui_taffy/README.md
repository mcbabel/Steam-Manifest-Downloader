# egui_taffy (vendored)

Copy of [egui_taffy](https://github.com/PPakalns/egui_taffy/) 0.16.0 by Pēteris Pakalns, MIT licensed (see `LICENSE`).

Changed for Steam Manifest Downloader:

- Leaf nodes can carry a measure function (`LeafMeasure`), so wrapped text is sized by taffy for the width it is given instead of from the size it had in the previous frame.
