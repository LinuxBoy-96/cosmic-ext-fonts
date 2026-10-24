app-title = Schriftenverwaltung
app-name = Schriften

## Browser
tab-installed = Installiert
tab-catalog = Google-Fonts-Katalog
search-placeholder = Schriften durchsuchen…
open-file = Datei öffnen…
refresh = Aktualisieren
scanning = Schriften werden eingelesen…
installed-count = { $total ->
    [one] { $total } Familie auf diesem Computer
   *[other] { $total } Familien auf diesem Computer
}
installed-by-user = { $count } vom Benutzer installiert
catalog-count = { $total ->
    [one] { $total } Familie bei Google Fonts
   *[other] { $total } Familien bei Google Fonts
}
catalog-installed = { $count } bereits installiert
results-count = { $total ->
    [one] { $shown } von { $total } Familie
   *[other] { $shown } von { $total } Familien
}
face-count = { $count ->
    [one] { $count } Schnitt
   *[other] { $count } Schnitte
}
monospace = dicktengleich
user-installed = vom Benutzer installiert

## Family view
preview-placeholder = Vorschautext
pangram = Franz jagt im komplett verwahrlosten Taxi quer durch Bayern — äöüß — 0123456789
faces = Schnitte
system = System

## Actions
install = Installieren
uninstall = Entfernen
confirm = Bestätigen
cancel = Abbrechen
installed = Installiert
installed-badge = installiert
already-installed-system = Bereits installiert (System)
installed-to = Installiert nach { $path }
already-installed = „{ $family }“ ist bereits installiert
install-failed = Installation fehlgeschlagen: { $error }
uninstalled = Schrift entfernt
uninstall-failed = Entfernen fehlgeschlagen: { $error }
uninstall-collection = { $count ->
    [one] Entfernt { $count } Schnitt dieser Datei
   *[other] Entfernt alle { $count } Schnitte dieser Datei
}
uninstall-family = Familie entfernen
uninstall-family-confirm = { $count ->
    [one] { $count } Schnitt entfernen
   *[other] { $count } Schnitte entfernen
}

## File view
open-font-title = Schrift öffnen
font-filter = Schriften
faces-in-file = { $count ->
    [one] { $count } Schnitt in dieser Datei
   *[other] { $count } Schnitte in dieser Datei
}
preview-failed = Diese Datei kann nicht angezeigt werden
reading-font = Schrift wird gelesen…

## File chooser
chooser-failed = Die Dateiauswahl konnte nicht geöffnet werden
chooser-failed-body =
    Die Dateiauswahl stammt von xdg-desktop-portal (unter COSMIC xdg-desktop-portal-cosmic): Prüfen Sie, ob es läuft.

    { $error }
close = Schließen

## Catalog
catalog-search-placeholder = Katalog durchsuchen…
catalog-loading = Katalog wird geladen…
catalog-unavailable = Katalog nicht verfügbar
retry = Erneut versuchen
variant-count = { $count ->
    [one] { $count } Variante
   *[other] { $count } Varianten
}
no-ttf = Kein TTF verfügbar
variants-unavailable = Varianten nicht verfügbar
variants-loading = Varianten werden geladen…
preview-loading = Vorschau wird geladen…
preview-unavailable = Vorschau nicht verfügbar
downloading = { $family } { $variant } wird heruntergeladen…
show-more = Mehr anzeigen ({ $shown } von { $total })

## Menus
menu-view = Ansicht
about = Info
theme = Design
theme-system = System
theme-light = Hell
theme-dark = Dunkel

## Weights, styles and widths
weight-thin = Dünn
weight-extra-light = Extraleicht
weight-light = Leicht
weight-regular = Normal
weight-medium = Mittel
weight-semi-bold = Halbfett
weight-bold = Fett
weight-extra-bold = Extrafett
weight-black = Schwarz
style-italic = Kursiv
style-oblique = Schräg
stretch-ultra-condensed = Ultraschmal
stretch-extra-condensed = Extraschmal
stretch-condensed = Schmal
stretch-semi-condensed = Halbschmal
stretch-semi-expanded = Halbbreit
stretch-expanded = Breit
stretch-extra-expanded = Extrabreit
stretch-ultra-expanded = Ultrabreit

## Catalog categories
category-sans-serif = Serifenlos
category-serif = Serifen
category-display = Display
category-handwriting = Handschrift
category-monospace = Dicktengleich

## Errors
error-io = Dateifehler: { $error }
error-invalid-font = Keine gültige Schriftdatei
error-font-too-large = Die Schriftdatei ist zu groß
error-verification = Die installierte Datei hat die Prüfung nicht bestanden
error-not-user-font = Nur Schriften im eigenen Schriftordner können entfernt werden
error-no-user-dir = Schriftordner nicht gefunden (kein persönlicher Ordner)
error-network = Netzwerkfehler: { $error }
error-catalog-response = Unerwartete Antwort des Katalogs: { $error }
error-download-too-large = Die heruntergeladene Datei ist zu groß
error-catalog-id = Der Katalog hat eine ungültige Schrift-ID geliefert
error-untrusted-url = Download von { $url } verweigert

## About
about-creator = Urheber
about-maintainer = Betreuer
about-link = Link
about-repository = Quellcode
about-license = Lizenz
about-license-name = GPL 3.0 oder später
about-version = Version
