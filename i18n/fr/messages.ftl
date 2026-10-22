# Messages de l'interface en français.
# Les espaces avant « ? » et « : » sont insécables (U+00A0).
# Chaque message doit exister dans le fichier anglais de référence.

## Titre

app-title = Nettoyeur de dépôts

## Menu principal

menu-protect = Protéger des dépôts
menu-delete = Supprimer des dépôts
menu-language = Langue
menu-quit = Quitter

## Menu de portée

scope-all = Tous les dépôts
scope-since-last-deletion = Uniquement les dépôts depuis la dernière suppression

## Étiquettes

label-no-deletion-yet = pas encore de suppression
label-public = public
label-private = privé
label-fork = fork
label-archived = archivé
label-protected = protégé

## En-tête des listes, dont les parties sont séparées par " · "

repository-count = { $count ->
    [one] { $count } dépôt
   *[other] { $count } dépôts
}
protected-count = { $count ->
    [one] { $count } protégé
   *[other] { $count } protégés
}
marked-count = { $count } à supprimer
new-repository-count = { $count ->
    [one] { $count } nouveau dépôt depuis le { $since }
   *[other] { $count } nouveaux dépôts depuis le { $since }
}

## Aide des touches, affichée en bas de chaque écran

hint-main-menu = ↑/↓ naviguer · Entrée valider
hint-submenu = ↑/↓ naviguer · Entrée valider · Échap retour
hint-protection = ↑/↓ naviguer · Espace protéger · Entrée enregistrer · Échap annuler
hint-deletion = ↑/↓ naviguer · Espace marquer · Entrée supprimer · Échap annuler

## Suppression

loading-repositories = Chargement des dépôts…
no-repository-selected = Aucun dépôt sélectionné
# $count vaut au moins 1.
confirm-deletion = { $count ->
    [one] Voulez-vous vraiment supprimer ce dépôt ? [{ confirmation-key }/N]
   *[other] Voulez-vous vraiment supprimer ces { $count } dépôts ? [{ confirmation-key }/N]
}
# Touche qui confirme la suppression, en minuscule. Également affichée dans la question ci-dessus.
confirmation-key = o
deletion-progress = Suppression des dépôts… { $done }/{ $total }
deletion-summary = { $deleted ->
        [one] { $deleted } supprimé
       *[other] { $deleted } supprimés
    }, { $failed ->
        [one] { $failed } échec
       *[other] { $failed } échecs
    }. Voir le journal : { $path }
press-any-key = Appuyez sur une touche pour revenir au menu

## Erreurs

error-gh-not-installed = La GitHub CLI (gh) n'est pas installée. Installez-la depuis https://cli.github.com/
error-gh-not-logged-in = Vous n'êtes pas connecté à la GitHub CLI. Lancez : gh auth login
error-missing-delete-scope = Le token de la GitHub CLI n'a pas le scope delete_repo. Lancez : gh auth refresh -s delete_repo
error-network = Impossible de joindre GitHub : { $details }
