app-title = Typsnittshanterare
app-name = Typsnitt

## Bläddrare
tab-installed = Installerad
tab-catalog = Google typsnitt katalog
search-placeholder = Sök efter typsnitt…
open-file = Öppna fil…
refresh = Uppdatera
scanning = Skannar typsnitt…
installed-count = { $total ->
    [one] { $total } familj på den här datorn
   *[other] { $total } familjer på den här datorn

installed-by-user = { $count } användarinstallerad
catalog-count = { $total ->
    [one] { $total } familj på Google typsnitt
   *[other] { $total } familjer på Google typsnitt
}
catalog-installed = { $count } redan installerad
results-count = { $total ->
    [one] { $shown } av { $total } familj
   *[other] { $shown } av { $total } familjer
}
face-count = { $count ->
    [one] { $count } typsnitt
   *[other] { $count } flera typsnitt
}
monospace = monospace
user-installed = användarinstallerad

## Familjevy
preview-placeholder = Förhandsgranskningstext
pangram = Den snabba bruna räven hoppar över den lata hunden — 0123456789
faces = Typsnitt
system = System

## Åtgärder
install = Installera
uninstall = Avinstallera
confirm = Bekräfta
cancel = Avbryt
installed = Installerad
installed-badge = installerad
already-installed-system = Redan installerad (system)
installed-to = Installerad till { $path }
already-installed = “{ $family }” är redan installerad
install-failed = Installation misslyckades: { $error }
uninstalled = Teckensnitt borttaget
uninstall-failed = Borttagning misslyckades: { $error }
uninstall-collection = { $count ->
    [one] Tar bort the { $count } ansiktet på den här filen
   *[other] Tar bort alla { $count } ansikten på det här filerna
}
uninstall-family = Avinstallera familj
uninstall-family-confirm = { $count ->
    [one] Ta bort { $count } teckensnitt
   *[other] Ta bort { $count } flera teckensnitt
}

## Filvy
open-font-title = Öppna teckensnitt
font-filter = Teckensnitt
faces-in-file = { $count ->
    [one] { $count } typsnitt i denna fil
   *[other] { $count } flera typsnitt i denna fil
}
preview-failed = Den här filen kan inte förhandsgranskas
reading-font = Läser teckensnitt…

## Filväljaren
chooser-failed = Kunde inte öppna filväljaren
chooser-failed-body =
    Filväljaren kommer från xdg-desktop-portal (xdg-desktop-portal-cosmic på COSMIC): Kontrollera att det är igång.

    { $error }
close = Stäng

## Katalog
catalog-search-placeholder = Sök i katalogen…
catalog-loading = Laddar katalog…
catalog-unavailable= Katalog otillgänglig
retry = Försök igen
variant-count = { $count ->
    [one] { $count } variant
   *[other] { $count } varianter
}
no-ttf = Ingen TTF tillgänglig
variants-unavailable = Varianter ej tillgängliga
variants-loading = Laddar varianter…
preview-loading = Laddar förhandsgranskning…
preview-unavailable = Förhandsgranskning ej tillgänglig
downloading = Laddar ner { $family } { $variant }…
show-more = Visa mer ({ $shown } of { $total })

## Menyer
menu-view = Visa
about = Om
theme = Tema
theme-system = System
theme-light = Ljust
theme-dark = Mörkt

## Vikter, stilar och bredder
weight-thin = Tunn
weight-extra-light = Extra lätt
weight-light = Lätt
weight-regular = Vanlig
weight-medium = Medel
weight-semi-bold = Halvfet
weight-bold = Fet
weight-extra-bold = Extra fet
weight-black = Svart
style-italic = Kursiv
style-oblique = Sned
stretch-ultra-condensed = Ultra smal
stretch-extra-condensed = Extra smal
stretch-condensed = Smal
stretch-semi-condensed = Halv smal
stretch-semi-expanded = Halvutökad
stretch-expanded = Utökad
stretch-extra-expanded = Extra utökad
stretch-ultra-expanded = Ultra utökad

## Katalogkategorier
category-sans-serif = Sans serif
category-serif = Serif
category-display = Visa
category-handwriting = Handstil
category-monospace = Monospace

## Fel
error-io = Fil fel: { $error }
error-invalid-font = Inte en giltig typsnittsfil
error-font-too-large = Typsnittsfilen är för stor
error-verification = Den installerade filen klarade inte verifieringen
error-not-user-font = Endast teckensnitt i din egna teckensnittsmapp kan tas bort
error-no-user-dir = Kan inte hitta din teckensnittsmapp (ingen hemkatalog)
error-network = Nätverksfel: { $error }
error-catalog-response = Oväntat svar från katalog: { $error }
error-download-too-large = Den nedladdade filen är för stor
error-catalog-id = Katalogen returnerade ett ogiltigt teckensnitts-ID
error-untrusted-url = Vägrar att ladda ner från { $url }

## Om
about-creator = Skapare
about-maintainer = Underhållare
about-link = Länk
about-repository = Källkod
about-license = Licens
about-license-name = GPL 3.0 eller senare
about-version = Version
