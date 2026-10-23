# User interface messages in English, the reference language.
# Every message must also exist in the other languages.

## Title

app-title = Repository Cleaner

## Main menu

menu-protect = Protect repositories
menu-delete = Delete repositories
menu-language = Language
menu-quit = Quit

## Scope menu

scope-all = All repositories
scope-since-last-deletion = Only repositories since last deletion

## Labels

label-no-deletion-yet = no deletion yet
label-public = public
label-private = private
label-fork = fork
label-archived = archived
label-protected = protected

## List header, whose parts are joined with " · "

repository-count = { $count ->
    [one] { $count } repository
   *[other] { $count } repositories
}
protected-count = { $count } protected
marked-count = { $count } marked for deletion
new-repository-count = { $count ->
    [one] { $count } new repository since { $since }
   *[other] { $count } new repositories since { $since }
}

## Sizes

size-kilobytes = { $size } KB
size-megabytes = { $size } MB
size-gigabytes = { $whole }.{ $tenth } GB

## Key hints, shown at the bottom of each screen

hint-main-menu = ↑/↓ move · Enter select
hint-submenu = ↑/↓ move · Enter select · Esc back
hint-protection = ↑/↓ move · Shift+↑/↓ jump · Ctrl+↑/↓ first/last · Space protect · Enter save · Esc cancel
hint-deletion = ↑/↓ move · Shift+↑/↓ jump · Ctrl+↑/↓ first/last · Space mark · Enter delete · Esc cancel

## Deletion

checking-access = Checking permissions…
loading-repositories = Loading repositories…
no-repository-selected = No repository selected
# $count is at least 1.
confirm-deletion = { $count ->
    [one] Do you really want to delete this repository? [{ confirmation-key }/N]
   *[other] Do you really want to delete these { $count } repositories? [{ confirmation-key }/N]
}
# Key that confirms the deletion, in lowercase. Also shown in the prompt above.
confirmation-key = y
more-repositories = … and { $count } more
deletion-progress = Deleting repositories…
# Label of the progress bar. $percent is a whole number from 0 to 100.
deletion-progress-count = { $done }/{ $total } · { $percent }%
deletion-summary = { $deleted } deleted, { $failed } failed. See log: { $path }
hint-deletion-progress = Esc stop after the current repository
press-any-key = Press any key to return to the menu

## Errors

error-gh-not-installed = GitHub CLI (gh) is not installed. Install it from https://cli.github.com/
error-gh-not-logged-in = You are not logged in to GitHub CLI. Run: gh auth login
error-missing-delete-scope = The GitHub CLI token is missing the delete_repo scope. Run: gh auth refresh -s delete_repo
error-request = GitHub request failed: { $details }
error-unexpected-response = Unexpected response from GitHub CLI: { $details }
error-config-save = Cannot save the configuration: { $details }
error-log = Cannot use the log file: { $details }
deletion-untouched = { $count ->
    [one] { $count } repository was left untouched.
   *[other] { $count } repositories were left untouched.
}
log-missing-result = { $repository } was processed but its result is missing from the log: check it on GitHub.
log-missing-end = Every repository was processed, but the end of the session could not be logged.
log-in-use = Another instance of the application is deleting repositories. Try again once it is done.
previous-session-interrupted = The previous deletion session was interrupted before the end. It is now marked as interrupted in { $path }.
previous-session-pending = { $repository } was being deleted and may have been deleted: check it on GitHub.
