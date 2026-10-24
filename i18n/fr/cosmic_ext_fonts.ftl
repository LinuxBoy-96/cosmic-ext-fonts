app-title = Gestionnaire de polices
app-name = Polices

## Browser
tab-installed = Installées
tab-catalog = Catalogue Google Fonts
search-placeholder = Rechercher une police…
open-file = Ouvrir un fichier…
refresh = Actualiser
scanning = Analyse des polices…
installed-count = { $total ->
    [one] { $total } famille sur cet ordinateur
   *[other] { $total } familles sur cet ordinateur
}
installed-by-user = { $count ->
    [one] { $count } installée par l’utilisateur
   *[other] { $count } installées par l’utilisateur
}
catalog-count = { $total ->
    [one] { $total } famille sur Google Fonts
   *[other] { $total } familles sur Google Fonts
}
catalog-installed = { $count ->
    [one] { $count } déjà installée
   *[other] { $count } déjà installées
}
results-count = { $total ->
    [one] { $shown } sur { $total } famille
   *[other] { $shown } sur { $total } familles
}
face-count = { $count ->
    [one] { $count } face
   *[other] { $count } faces
}
monospace = monospace
user-installed = installée par l’utilisateur

## Family view
preview-placeholder = Texte d’aperçu
pangram = Portez ce vieux whisky au juge blond qui fume — àâçéèêëîïôœùûüÿ — 0123456789
faces = Faces
system = Système

## Actions
install = Installer
uninstall = Désinstaller
confirm = Confirmer
cancel = Annuler
installed = Installée
installed-badge = installée
already-installed-system = Déjà installée (système)
installed-to = Installée dans { $path }
already-installed = « { $family } » est déjà installée
install-failed = Échec de l’installation : { $error }
uninstalled = Police supprimée
uninstall-failed = Échec de la suppression : { $error }
uninstall-collection = { $count ->
    [one] Supprime { $count } face de ce fichier
   *[other] Supprime les { $count } faces de ce fichier
}
uninstall-family = Désinstaller la famille
uninstall-family-confirm = { $count ->
    [one] Supprimer { $count } face
   *[other] Supprimer { $count } faces
}

## File view
open-font-title = Ouvrir une police
font-filter = Polices
faces-in-file = { $count ->
    [one] { $count } face dans ce fichier
   *[other] { $count } faces dans ce fichier
}
preview-failed = Impossible de prévisualiser ce fichier
reading-font = Lecture de la police…

## File chooser
chooser-failed = Impossible d’ouvrir le sélecteur de fichiers
chooser-failed-body =
    Le sélecteur de fichiers vient de xdg-desktop-portal (xdg-desktop-portal-cosmic sous COSMIC) : vérifiez qu’il est lancé.

    { $error }
close = Fermer

## Catalog
catalog-search-placeholder = Rechercher dans le catalogue…
catalog-loading = Chargement du catalogue…
catalog-unavailable = Catalogue indisponible
retry = Réessayer
variant-count = { $count ->
    [one] { $count } variante
   *[other] { $count } variantes
}
no-ttf = TTF indisponible
variants-unavailable = Variantes indisponibles
variants-loading = Chargement des variantes…
preview-loading = Chargement de l’aperçu…
preview-unavailable = Aperçu indisponible
downloading = Téléchargement de { $family } { $variant }…
show-more = Afficher plus ({ $shown } sur { $total })

## Menus
menu-view = Affichage
about = À propos
theme = Thème
theme-system = Système
theme-light = Clair
theme-dark = Sombre

## Weights, styles and widths
weight-thin = Fin
weight-extra-light = Extra-léger
weight-light = Léger
weight-regular = Normal
weight-medium = Moyen
weight-semi-bold = Demi-gras
weight-bold = Gras
weight-extra-bold = Extra-gras
weight-black = Noir
style-italic = Italique
style-oblique = Oblique
stretch-ultra-condensed = Ultra-condensé
stretch-extra-condensed = Extra-condensé
stretch-condensed = Condensé
stretch-semi-condensed = Semi-condensé
stretch-semi-expanded = Semi-étendu
stretch-expanded = Étendu
stretch-extra-expanded = Extra-étendu
stretch-ultra-expanded = Ultra-étendu

## Catalog categories
category-sans-serif = Sans empattement
category-serif = Avec empattement
category-display = Titrage
category-handwriting = Manuscrite
category-monospace = Monospace

## Errors
error-io = Erreur de fichier : { $error }
error-invalid-font = Ce n’est pas un fichier de police valide
error-font-too-large = Le fichier de police est trop volumineux
error-verification = Le fichier installé n’a pas passé la vérification
error-not-user-font = Seules les polices de votre propre dossier de polices peuvent être supprimées
error-no-user-dir = Dossier de polices introuvable (aucun dossier personnel)
error-network = Erreur réseau : { $error }
error-catalog-response = Réponse inattendue du catalogue : { $error }
error-download-too-large = Le fichier téléchargé est trop volumineux
error-catalog-id = Le catalogue a renvoyé un identifiant de police invalide
error-untrusted-url = Téléchargement refusé depuis { $url }

## About
about-creator = Créateur
about-maintainer = Mainteneur
about-link = Lien
about-repository = Dépôt
about-license = Licence
about-license-name = GPL 3.0 ou ultérieure
about-version = Version
