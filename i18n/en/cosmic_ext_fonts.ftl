app-title = Font Manager
app-name = Fonts

## Browser
tab-installed = Installed
tab-catalog = Google Fonts catalog
search-placeholder = Search fonts…
open-file = Open file…
refresh = Refresh
scanning = Scanning fonts…
installed-count = { $total ->
    [one] { $total } family on this computer
   *[other] { $total } families on this computer
}
installed-by-user = { $count } user-installed
catalog-count = { $total ->
    [one] { $total } family on Google Fonts
   *[other] { $total } families on Google Fonts
}
catalog-installed = { $count } already installed
results-count = { $total ->
    [one] { $shown } of { $total } family
   *[other] { $shown } of { $total } families
}
face-count = { $count ->
    [one] { $count } face
   *[other] { $count } faces
}
monospace = monospace
user-installed = user-installed

## Family view
preview-placeholder = Preview text
pangram = The quick brown fox jumps over the lazy dog — 0123456789
faces = Faces
system = System

## Actions
install = Install
uninstall = Uninstall
confirm = Confirm
cancel = Cancel
installed = Installed
installed-badge = installed
already-installed-system = Already installed (system)
installed-to = Installed to { $path }
already-installed = “{ $family }” is already installed
install-failed = Installation failed: { $error }
uninstalled = Font removed
uninstall-failed = Removal failed: { $error }
uninstall-collection = { $count ->
    [one] Removes the { $count } face of this file
   *[other] Removes all { $count } faces of this file
}
uninstall-family = Uninstall family
uninstall-family-confirm = { $count ->
    [one] Remove { $count } face
   *[other] Remove { $count } faces
}

## File view
open-font-title = Open font
font-filter = Fonts
faces-in-file = { $count ->
    [one] { $count } face in this file
   *[other] { $count } faces in this file
}
preview-failed = This file cannot be previewed
reading-font = Reading font…

## File chooser
chooser-failed = Could not open the file chooser
chooser-failed-body =
    The file chooser comes from xdg-desktop-portal (xdg-desktop-portal-cosmic on COSMIC): check that it is running.

    { $error }
close = Close

## Catalog
catalog-search-placeholder = Search the catalog…
catalog-loading = Loading catalog…
catalog-unavailable = Catalog unavailable
retry = Retry
variant-count = { $count ->
    [one] { $count } variant
   *[other] { $count } variants
}
no-ttf = No TTF available
variants-unavailable = Variants unavailable
variants-loading = Loading variants…
preview-loading = Loading preview…
preview-unavailable = Preview unavailable
downloading = Downloading { $family } { $variant }…
show-more = Show more ({ $shown } of { $total })

## Menus
menu-view = View
about = About
theme = Theme
theme-system = System
theme-light = Light
theme-dark = Dark

## Weights, styles and widths
weight-thin = Thin
weight-extra-light = Extra Light
weight-light = Light
weight-regular = Regular
weight-medium = Medium
weight-semi-bold = Semi Bold
weight-bold = Bold
weight-extra-bold = Extra Bold
weight-black = Black
style-italic = Italic
style-oblique = Oblique
stretch-ultra-condensed = Ultra Condensed
stretch-extra-condensed = Extra Condensed
stretch-condensed = Condensed
stretch-semi-condensed = Semi Condensed
stretch-semi-expanded = Semi Expanded
stretch-expanded = Expanded
stretch-extra-expanded = Extra Expanded
stretch-ultra-expanded = Ultra Expanded

## Catalog categories
category-sans-serif = Sans serif
category-serif = Serif
category-display = Display
category-handwriting = Handwriting
category-monospace = Monospace

## Errors
error-io = File error: { $error }
error-invalid-font = Not a valid font file
error-font-too-large = The font file is too large
error-verification = The installed file failed verification
error-not-user-font = Only fonts in your own font folder can be removed
error-no-user-dir = Cannot find your font folder (no home directory)
error-network = Network error: { $error }
error-catalog-response = Unexpected response from the catalog: { $error }
error-download-too-large = The downloaded file is too large
error-catalog-id = The catalog returned an invalid font id
error-untrusted-url = Refusing to download from { $url }

## About
about-creator = Creator
about-maintainer = Maintainer
about-link = Link
about-repository = Repository
about-license = License
about-license-name = GPL 3.0 or later
about-version = Version
