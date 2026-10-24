app-title = Gestore dei caratteri
app-name = Caratteri

## Browser
tab-installed = Installati
tab-catalog = Catalogo di Google Fonts
search-placeholder = Cerca un carattere…
open-file = Apri file…
refresh = Aggiorna
scanning = Analisi dei caratteri…
installed-count = { $total ->
    [one] { $total } famiglia su questo computer
   *[other] { $total } famiglie su questo computer
}
installed-by-user = { $count ->
    [one] { $count } installata dall’utente
   *[other] { $count } installate dall’utente
}
catalog-count = { $total ->
    [one] { $total } famiglia su Google Fonts
   *[other] { $total } famiglie su Google Fonts
}
catalog-installed = { $count ->
    [one] { $count } già installata
   *[other] { $count } già installate
}
results-count = { $total ->
    [one] { $shown } di { $total } famiglia
   *[other] { $shown } di { $total } famiglie
}
face-count = { $count ->
    [one] { $count } stile
   *[other] { $count } stili
}
monospace = monospaziato
user-installed = installato dall’utente

## Family view
preview-placeholder = Testo di anteprima
pangram = Pranzo d’acqua fa volti sghembi — àèéìòù — 0123456789
faces = Stili
system = Sistema

## Actions
install = Installa
uninstall = Disinstalla
confirm = Conferma
cancel = Annulla
installed = Installato
installed-badge = installato
already-installed-system = Già installato (sistema)
installed-to = Installato in { $path }
already-installed = «{ $family }» è già installato
install-failed = Installazione non riuscita: { $error }
uninstalled = Carattere rimosso
uninstall-failed = Rimozione non riuscita: { $error }
uninstall-collection = { $count ->
    [one] Rimuove { $count } stile di questo file
   *[other] Rimuove tutti i { $count } stili di questo file
}
uninstall-family = Disinstalla la famiglia
uninstall-family-confirm = { $count ->
    [one] Rimuovi { $count } stile
   *[other] Rimuovi { $count } stili
}

## File view
open-font-title = Apri carattere
font-filter = Caratteri
faces-in-file = { $count ->
    [one] { $count } stile in questo file
   *[other] { $count } stili in questo file
}
preview-failed = Impossibile visualizzare l’anteprima di questo file
reading-font = Lettura del carattere…

## File chooser
chooser-failed = Impossibile aprire il selettore di file
chooser-failed-body =
    Il selettore di file è fornito da xdg-desktop-portal (xdg-desktop-portal-cosmic su COSMIC): verifica che sia in esecuzione.

    { $error }
close = Chiudi

## Catalog
catalog-search-placeholder = Cerca nel catalogo…
catalog-loading = Caricamento del catalogo…
catalog-unavailable = Catalogo non disponibile
retry = Riprova
variant-count = { $count ->
    [one] { $count } variante
   *[other] { $count } varianti
}
no-ttf = TTF non disponibile
variants-unavailable = Varianti non disponibili
variants-loading = Caricamento delle varianti…
preview-loading = Caricamento dell’anteprima…
preview-unavailable = Anteprima non disponibile
downloading = Scaricamento di { $family } { $variant }…
show-more = Mostra altro ({ $shown } di { $total })

## Menus
menu-view = Visualizza
about = Informazioni
theme = Tema
theme-system = Sistema
theme-light = Chiaro
theme-dark = Scuro

## Weights, styles and widths
weight-thin = Sottile
weight-extra-light = Extra leggero
weight-light = Leggero
weight-regular = Normale
weight-medium = Medio
weight-semi-bold = Semigrassetto
weight-bold = Grassetto
weight-extra-bold = Extra grassetto
weight-black = Nero
style-italic = Corsivo
style-oblique = Obliquo
stretch-ultra-condensed = Ultra condensato
stretch-extra-condensed = Extra condensato
stretch-condensed = Condensato
stretch-semi-condensed = Semi condensato
stretch-semi-expanded = Semi espanso
stretch-expanded = Espanso
stretch-extra-expanded = Extra espanso
stretch-ultra-expanded = Ultra espanso

## Catalog categories
category-sans-serif = Senza grazie
category-serif = Con grazie
category-display = Decorativo
category-handwriting = Calligrafico
category-monospace = Monospaziato

## Errors
error-io = Errore sul file: { $error }
error-invalid-font = Non è un file di caratteri valido
error-font-too-large = Il file di caratteri è troppo grande
error-verification = Il file installato non ha superato la verifica
error-not-user-font = Si possono rimuovere solo i caratteri della propria cartella dei caratteri
error-no-user-dir = Cartella dei caratteri non trovata (nessuna cartella personale)
error-network = Errore di rete: { $error }
error-catalog-response = Risposta inattesa dal catalogo: { $error }
error-download-too-large = Il file scaricato è troppo grande
error-catalog-id = Il catalogo ha restituito un identificatore di carattere non valido
error-untrusted-url = Download rifiutato da { $url }

## About
about-creator = Creatore
about-maintainer = Manutentore
about-link = Collegamento
about-repository = Repository
about-license = Licenza
about-license-name = GPL 3.0 o successiva
about-version = Versione
