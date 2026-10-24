app-title = Gestor de fuentes
app-name = Fuentes

## Browser
tab-installed = Instaladas
tab-catalog = Catálogo de Google Fonts
search-placeholder = Buscar fuentes…
open-file = Abrir archivo…
refresh = Actualizar
scanning = Analizando fuentes…
installed-count = { $total ->
    [one] { $total } familia en este equipo
   *[other] { $total } familias en este equipo
}
installed-by-user = { $count ->
    [one] { $count } instalada por el usuario
   *[other] { $count } instaladas por el usuario
}
catalog-count = { $total ->
    [one] { $total } familia en Google Fonts
   *[other] { $total } familias en Google Fonts
}
catalog-installed = { $count ->
    [one] { $count } ya instalada
   *[other] { $count } ya instaladas
}
results-count = { $total ->
    [one] { $shown } de { $total } familia
   *[other] { $shown } de { $total } familias
}
face-count = { $count ->
    [one] { $count } variante
   *[other] { $count } variantes
}
monospace = monoespaciada
user-installed = instalada por el usuario

## Family view
preview-placeholder = Texto de muestra
pangram = Quiere la boca exhausta vid, kiwi, piña y fugaz jamón — áéíóúüñ ¡¿ — 0123456789
faces = Variantes
system = Sistema

## Actions
install = Instalar
uninstall = Desinstalar
confirm = Confirmar
cancel = Cancelar
installed = Instalada
installed-badge = instalada
already-installed-system = Ya instalada (sistema)
installed-to = Instalada en { $path }
already-installed = «{ $family }» ya está instalada
install-failed = Error al instalar: { $error }
uninstalled = Fuente eliminada
uninstall-failed = Error al eliminar: { $error }
uninstall-collection = { $count ->
    [one] Elimina { $count } variante de este archivo
   *[other] Elimina las { $count } variantes de este archivo
}
uninstall-family = Desinstalar la familia
uninstall-family-confirm = { $count ->
    [one] Eliminar { $count } variante
   *[other] Eliminar { $count } variantes
}

## File view
open-font-title = Abrir fuente
font-filter = Fuentes
faces-in-file = { $count ->
    [one] { $count } variante en este archivo
   *[other] { $count } variantes en este archivo
}
preview-failed = No se puede previsualizar este archivo
reading-font = Leyendo la fuente…

## File chooser
chooser-failed = No se pudo abrir el selector de archivos
chooser-failed-body =
    El selector de archivos lo proporciona xdg-desktop-portal (xdg-desktop-portal-cosmic en COSMIC): compruebe que esté en ejecución.

    { $error }
close = Cerrar

## Catalog
catalog-search-placeholder = Buscar en el catálogo…
catalog-loading = Cargando el catálogo…
catalog-unavailable = Catálogo no disponible
retry = Reintentar
variant-count = { $count ->
    [one] { $count } variante
   *[other] { $count } variantes
}
no-ttf = TTF no disponible
variants-unavailable = Variantes no disponibles
variants-loading = Cargando variantes…
preview-loading = Cargando la vista previa…
preview-unavailable = Vista previa no disponible
downloading = Descargando { $family } { $variant }…
show-more = Mostrar más ({ $shown } de { $total })

## Menus
menu-view = Ver
about = Acerca de
theme = Tema
theme-system = Sistema
theme-light = Claro
theme-dark = Oscuro

## Weights, styles and widths
weight-thin = Fina
weight-extra-light = Extraligera
weight-light = Ligera
weight-regular = Normal
weight-medium = Media
weight-semi-bold = Seminegrita
weight-bold = Negrita
weight-extra-bold = Extranegrita
weight-black = Negra
style-italic = Cursiva
style-oblique = Oblicua
stretch-ultra-condensed = Ultracondensada
stretch-extra-condensed = Extracondensada
stretch-condensed = Condensada
stretch-semi-condensed = Semicondensada
stretch-semi-expanded = Semiexpandida
stretch-expanded = Expandida
stretch-extra-expanded = Extraexpandida
stretch-ultra-expanded = Ultraexpandida

## Catalog categories
category-sans-serif = Sin serifa
category-serif = Con serifa
category-display = Decorativa
category-handwriting = Manuscrita
category-monospace = Monoespaciada

## Errors
error-io = Error de archivo: { $error }
error-invalid-font = No es un archivo de fuente válido
error-font-too-large = El archivo de fuente es demasiado grande
error-verification = El archivo instalado no superó la verificación
error-not-user-font = Solo se pueden eliminar las fuentes de la propia carpeta de fuentes
error-no-user-dir = No se encuentra la carpeta de fuentes (sin carpeta personal)
error-network = Error de red: { $error }
error-catalog-response = Respuesta inesperada del catálogo: { $error }
error-download-too-large = El archivo descargado es demasiado grande
error-catalog-id = El catálogo devolvió un identificador de fuente no válido
error-untrusted-url = Descarga rechazada desde { $url }

## About
about-creator = Creador
about-maintainer = Mantenedor
about-link = Enlace
about-repository = Repositorio
about-license = Licencia
about-license-name = GPL 3.0 o posterior
about-version = Versión
