app-title = Gestor de fontes
app-name = Fontes

## Browser
tab-installed = Instaladas
tab-catalog = Catálogo do Google Fonts
search-placeholder = Pesquisar fontes…
open-file = Abrir ficheiro…
refresh = Atualizar
scanning = A analisar as fontes…
installed-count = { $total ->
    [one] { $total } família neste computador
   *[other] { $total } famílias neste computador
}
installed-by-user = { $count ->
    [one] { $count } instalada pelo utilizador
   *[other] { $count } instaladas pelo utilizador
}
catalog-count = { $total ->
    [one] { $total } família no Google Fonts
   *[other] { $total } famílias no Google Fonts
}
catalog-installed = { $count ->
    [one] { $count } já instalada
   *[other] { $count } já instaladas
}
results-count = { $total ->
    [one] { $shown } de { $total } família
   *[other] { $shown } de { $total } famílias
}
face-count = { $count ->
    [one] { $count } variante
   *[other] { $count } variantes
}
monospace = monoespaçada
user-installed = instalada pelo utilizador

## Family view
preview-placeholder = Texto de pré-visualização
pangram = Um pequeno jabuti xereta viu dez cegonhas felizes — àáâãçéêíóôõú — 0123456789
faces = Variantes
system = Sistema

## Actions
install = Instalar
uninstall = Desinstalar
confirm = Confirmar
cancel = Cancelar
installed = Instalada
installed-badge = instalada
already-installed-system = Já instalada (sistema)
installed-to = Instalada em { $path }
already-installed = «{ $family }» já está instalada
install-failed = Falha na instalação: { $error }
uninstalled = Fonte removida
uninstall-failed = Falha na remoção: { $error }
uninstall-collection = { $count ->
    [one] Remove { $count } variante deste ficheiro
   *[other] Remove as { $count } variantes deste ficheiro
}
uninstall-family = Desinstalar a família
uninstall-family-confirm = { $count ->
    [one] Remover { $count } variante
   *[other] Remover { $count } variantes
}

## File view
open-font-title = Abrir fonte
font-filter = Fontes
faces-in-file = { $count ->
    [one] { $count } variante neste ficheiro
   *[other] { $count } variantes neste ficheiro
}
preview-failed = Não é possível pré-visualizar este ficheiro
reading-font = A ler a fonte…

## File chooser
chooser-failed = Não foi possível abrir o seletor de ficheiros
chooser-failed-body =
    O seletor de ficheiros vem do xdg-desktop-portal (xdg-desktop-portal-cosmic no COSMIC): verifique se está em execução.

    { $error }
close = Fechar

## Catalog
catalog-search-placeholder = Pesquisar no catálogo…
catalog-loading = A carregar o catálogo…
catalog-unavailable = Catálogo indisponível
retry = Tentar novamente
variant-count = { $count ->
    [one] { $count } variante
   *[other] { $count } variantes
}
no-ttf = TTF indisponível
variants-unavailable = Variantes indisponíveis
variants-loading = A carregar as variantes…
preview-loading = A carregar a pré-visualização…
preview-unavailable = Pré-visualização indisponível
downloading = A transferir { $family } { $variant }…
show-more = Mostrar mais ({ $shown } de { $total })

## Menus
menu-view = Ver
about = Acerca
theme = Tema
theme-system = Sistema
theme-light = Claro
theme-dark = Escuro

## Weights, styles and widths
weight-thin = Fino
weight-extra-light = Extra leve
weight-light = Leve
weight-regular = Normal
weight-medium = Médio
weight-semi-bold = Seminegrito
weight-bold = Negrito
weight-extra-bold = Extranegrito
weight-black = Preto
style-italic = Itálico
style-oblique = Oblíquo
stretch-ultra-condensed = Ultracondensado
stretch-extra-condensed = Extracondensado
stretch-condensed = Condensado
stretch-semi-condensed = Semicondensado
stretch-semi-expanded = Semiexpandido
stretch-expanded = Expandido
stretch-extra-expanded = Extraexpandido
stretch-ultra-expanded = Ultraexpandido

## Catalog categories
category-sans-serif = Sem serifa
category-serif = Com serifa
category-display = Decorativa
category-handwriting = Manuscrita
category-monospace = Monoespaçada

## Errors
error-io = Erro de ficheiro: { $error }
error-invalid-font = Não é um ficheiro de fonte válido
error-font-too-large = O ficheiro de fonte é demasiado grande
error-verification = O ficheiro instalado não passou na verificação
error-not-user-font = Só é possível remover as fontes da própria pasta de fontes
error-no-user-dir = Pasta de fontes não encontrada (sem pasta pessoal)
error-network = Erro de rede: { $error }
error-catalog-response = Resposta inesperada do catálogo: { $error }
error-download-too-large = O ficheiro transferido é demasiado grande
error-catalog-id = O catálogo devolveu um identificador de fonte inválido
error-untrusted-url = Transferência recusada de { $url }

## About
about-creator = Criador
about-maintainer = Mantenedor
about-link = Ligação
about-repository = Repositório
about-license = Licença
about-license-name = GPL 3.0 ou posterior
about-version = Versão
