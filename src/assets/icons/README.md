These SVG icons are local copies downloaded from the Lucide icon set through the Iconify public API.

Source pattern: https://api.iconify.design/lucide/{icon}.svg?height=24

The app/favicon source icon is `public/zplayer.svg`, downloaded from:
https://api.iconify.design/fluent-emoji-flat/play-button.svg

The generated bundle icons in `src-tauri/icons/` are produced from `public/zplayer.svg`.

The `png/` files are generated from the SVG copies with `rsvg-convert` and used as CSS mask sources,
because some WebView mask implementations render external SVG masks as solid squares.

They are kept in the repository so the app does not hotlink online icon assets at runtime.
