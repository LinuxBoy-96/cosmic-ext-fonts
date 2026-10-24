app-title = Lettertypebeheer
app-name = Lettertypen

## Browser
tab-installed = Geïnstalleerd
tab-catalog = Google Fonts-catalogus
search-placeholder = Lettertypen zoeken…
open-file = Bestand openen…
refresh = Vernieuwen
scanning = Lettertypen worden ingelezen…
installed-count = { $total ->
    [one] { $total } familie op deze computer
   *[other] { $total } families op deze computer
}
installed-by-user = { $count } door gebruiker geïnstalleerd
catalog-count = { $total ->
    [one] { $total } familie op Google Fonts
   *[other] { $total } families op Google Fonts
}
catalog-installed = { $count } al geïnstalleerd
results-count = { $total ->
    [one] { $shown } van { $total } familie
   *[other] { $shown } van { $total } families
}
face-count = { $count ->
    [one] { $count } variant
   *[other] { $count } varianten
}
monospace = monospace
user-installed = door gebruiker geïnstalleerd

## Family view
preview-placeholder = Voorbeeldtekst
pangram = Pa’s wijze lynx bezag vroom het fikse aquaduct — éèëïóöü — 0123456789
faces = Varianten
system = Systeem

## Actions
install = Installeren
uninstall = Verwijderen
confirm = Bevestigen
cancel = Annuleren
installed = Geïnstalleerd
installed-badge = geïnstalleerd
already-installed-system = Al geïnstalleerd (systeem)
installed-to = Geïnstalleerd in { $path }
already-installed = “{ $family }” is al geïnstalleerd
install-failed = Installatie mislukt: { $error }
uninstalled = Lettertype verwijderd
uninstall-failed = Verwijderen mislukt: { $error }
uninstall-collection = { $count ->
    [one] Verwijdert { $count } variant uit dit bestand
   *[other] Verwijdert alle { $count } varianten uit dit bestand
}
uninstall-family = Familie verwijderen
uninstall-family-confirm = { $count ->
    [one] { $count } variant verwijderen
   *[other] { $count } varianten verwijderen
}

## File view
open-font-title = Lettertype openen
font-filter = Lettertypen
faces-in-file = { $count ->
    [one] { $count } variant in dit bestand
   *[other] { $count } varianten in dit bestand
}
preview-failed = Dit bestand kan niet worden weergegeven
reading-font = Lettertype wordt gelezen…

## File chooser
chooser-failed = Kan de bestandskiezer niet openen
chooser-failed-body =
    De bestandskiezer komt van xdg-desktop-portal (xdg-desktop-portal-cosmic onder COSMIC): controleer of die draait.

    { $error }
close = Sluiten

## Catalog
catalog-search-placeholder = Zoeken in de catalogus…
catalog-loading = Catalogus wordt geladen…
catalog-unavailable = Catalogus niet beschikbaar
retry = Opnieuw proberen
variant-count = { $count ->
    [one] { $count } variant
   *[other] { $count } varianten
}
no-ttf = Geen TTF beschikbaar
variants-unavailable = Varianten niet beschikbaar
variants-loading = Varianten worden geladen…
preview-loading = Voorbeeld wordt geladen…
preview-unavailable = Voorbeeld niet beschikbaar
downloading = { $family } { $variant } wordt gedownload…
show-more = Meer tonen ({ $shown } van { $total })

## Menus
menu-view = Beeld
about = Over
theme = Thema
theme-system = Systeem
theme-light = Licht
theme-dark = Donker

## Weights, styles and widths
weight-thin = Dun
weight-extra-light = Extra licht
weight-light = Licht
weight-regular = Normaal
weight-medium = Medium
weight-semi-bold = Halfvet
weight-bold = Vet
weight-extra-bold = Extra vet
weight-black = Zwart
style-italic = Cursief
style-oblique = Schuin
stretch-ultra-condensed = Ultrasmal
stretch-extra-condensed = Extra smal
stretch-condensed = Smal
stretch-semi-condensed = Semismal
stretch-semi-expanded = Semibreed
stretch-expanded = Breed
stretch-extra-expanded = Extra breed
stretch-ultra-expanded = Ultrabreed

## Catalog categories
category-sans-serif = Schreefloos
category-serif = Met schreef
category-display = Display
category-handwriting = Handschrift
category-monospace = Monospace

## Errors
error-io = Bestandsfout: { $error }
error-invalid-font = Geen geldig lettertypebestand
error-font-too-large = Het lettertypebestand is te groot
error-verification = Het geïnstalleerde bestand haalde de controle niet
error-not-user-font = Alleen lettertypen in de eigen lettertypemap kunnen worden verwijderd
error-no-user-dir = Lettertypemap niet gevonden (geen persoonlijke map)
error-network = Netwerkfout: { $error }
error-catalog-response = Onverwacht antwoord van de catalogus: { $error }
error-download-too-large = Het gedownloade bestand is te groot
error-catalog-id = De catalogus gaf een ongeldige lettertype-id terug
error-untrusted-url = Download van { $url } geweigerd

## About
about-creator = Maker
about-maintainer = Onderhouder
about-link = Link
about-repository = Broncode
about-license = Licentie
about-license-name = GPL 3.0 of later
about-version = Versie
