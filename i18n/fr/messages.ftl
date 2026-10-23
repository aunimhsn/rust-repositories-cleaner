# Messages de l'interface en français.
# Les espaces avant « ? », « : » et « % » sont insécables (U+00A0).
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

## Tailles

size-kilobytes = { $size } Ko
size-megabytes = { $size } Mo
size-gigabytes = { $whole },{ $tenth } Go

## Aide des touches, affichée en bas de chaque écran

hint-main-menu = ↑/↓ naviguer · Entrée valider
hint-submenu = ↑/↓ naviguer · Entrée valider · Échap retour
hint-protection = ↑/↓ naviguer · Maj+↑/↓ sauter · Ctrl+↑/↓ début/fin · Espace protéger · Entrée enregistrer · Échap annuler
hint-deletion = ↑/↓ naviguer · Maj+↑/↓ sauter · Ctrl+↑/↓ début/fin · Espace marquer · Entrée supprimer · Échap annuler

## Suppression

checking-access = Vérification des permissions…
loading-repositories = Chargement des dépôts…
no-repository-selected = Aucun dépôt sélectionné
# $count vaut au moins 1.
confirm-deletion = { $count ->
    [one] Voulez-vous vraiment supprimer ce dépôt ? [{ confirmation-key }/N]
   *[other] Voulez-vous vraiment supprimer ces { $count } dépôts ? [{ confirmation-key }/N]
}
# Touche qui confirme la suppression, en minuscule. Également affichée dans la question ci-dessus.
confirmation-key = o
more-repositories = … et { $count } de plus
deletion-progress = Suppression des dépôts…
# Libellé de la barre de progression. $percent est un nombre entier de 0 à 100.
deletion-progress-count = { $done }/{ $total } · { $percent } %
deletion-summary = { $deleted ->
        [one] { $deleted } supprimé
       *[other] { $deleted } supprimés
    }, { $failed ->
        [one] { $failed } échec
       *[other] { $failed } échecs
    }. Voir le journal : { $path }
hint-deletion-progress = Échap arrêter après le dépôt en cours
press-any-key = Appuyez sur une touche pour revenir au menu

## Erreurs

error-gh-not-installed = La GitHub CLI (gh) n'est pas installée. Installez-la depuis https://cli.github.com/
error-gh-not-logged-in = Vous n'êtes pas connecté à la GitHub CLI. Lancez : gh auth login
error-missing-delete-scope = Le token de la GitHub CLI n'a pas le scope delete_repo. Lancez : gh auth refresh -s delete_repo
error-request = La requête à GitHub a échoué : { $details }
error-unexpected-response = Réponse inattendue de la GitHub CLI : { $details }
error-config-save = Impossible d'enregistrer la configuration : { $details }
error-log = Impossible d'utiliser le journal : { $details }
deletion-untouched = { $count ->
    [one] { $count } dépôt n'a pas été touché.
   *[other] { $count } dépôts n'ont pas été touchés.
}
log-missing-result = { $repository } a été traité mais son résultat manque dans le journal : vérifiez-le sur GitHub.
log-missing-end = Tous les dépôts ont été traités, mais la fin de la session n'a pas pu être inscrite dans le journal.
log-in-use = Une autre instance de l'application est en train de supprimer des dépôts. Réessayez une fois qu'elle aura terminé.
previous-session-interrupted = La session de suppression précédente a été interrompue avant la fin. Elle est désormais marquée comme interrompue dans { $path }.
previous-session-pending = { $repository } était en cours de suppression et a peut-être été supprimé : vérifiez-le sur GitHub.
