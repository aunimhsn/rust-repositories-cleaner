# Specifications

## Purpose

`rust-repositories-cleaner` is a console application written in Rust that lets the user bulk delete repositories from their GitHub account through a simple interactive menu, while protecting the repositories they want to keep.

The expected volume is a few hundred repositories (around 330 today).

## Design principles

- Minimalist, clean and sober interface that goes straight to the point.
- A short plain-text title only: no ASCII art banner, no emoji.
- Colors are used sparingly, only to convey meaning (red for deletion, green for protection, gray for disabled).
- Each screen shows a key hint at the bottom, wrapped on two lines in narrow terminals (for example `↑/↓ move · Space mark · Enter delete · Esc cancel`).
- A short message is shown while waiting for GitHub ("Checking permissions…", "Loading repositories…"). Keys pressed while waiting are ignored.

## GitHub access

- The application reuses the token of the GitHub CLI (`gh`).
- At startup, it only checks that `gh` is installed and logged in: protecting repositories does not need any extra permission.
- Deleting repositories requires the `delete_repo` scope on that token. It is checked when opening **Delete repositories**, so the permission to delete is only granted when it is needed.

## Navigation overview

```
Main menu
├── Protect repositories
│   ├── All repositories                         → protection list
│   └── Only repositories since last deletion    → protection list
├── Delete repositories
│   ├── All repositories                         → deletion list
│   └── Only repositories since last deletion    → deletion list
├── Language
└── Quit
```

## Main menu

When the application starts, it displays a short title and a main menu:

1. **Protect repositories**
2. **Delete repositories**
3. **Language**
4. **Quit**

`Ctrl+C` exits the application cleanly from any screen, except while deleting: it then stops the deletions after the current repository (see [Results](#results)).

The user selects an entry with `↑` / `↓` and `Enter`. `Esc` does nothing in the main menu: the **Quit** entry exits the application. After any action completes, the application returns to this main menu.

## Scope menu

Before listing repositories, **Protect repositories** and **Delete repositories** open an intermediate menu to choose which repositories to list:

1. **All repositories**
2. **Only repositories since last deletion**

`Esc` returns to the main menu.

### Last deletion date

- The last deletion date is the date of the most recent repository successfully deleted.
- It is read from the log file `logs/deletions.log`. No other state file is used.
- "Repositories since last deletion" are the repositories **created** after that date.
- If no deletion has happened yet (no log file, or no successful deletion in it), **Only repositories since last deletion** is shown grayed out with the label `no deletion yet`, and cannot be selected.

## Repository list

The protection and deletion lists share the same format:

- Only the repositories **owned by the user** (no organization repositories).
- Forks and archived repositories are included.
- Sorted alphabetically by name.
- No search or filter.
- The list scrolls when it is taller than the terminal, and shows as many repositories as the terminal height allows.
- The cursor stays in the middle of the visible rows, so the repositories before and after it are always visible; only at the ends of the list does it move towards the edge.

### Header

Each list starts with a one-line summary, like a mini dashboard. When the list is limited to the repositories since last deletion, the header starts with `8 new repositories since 2026-10-22` instead of the total.

| List | Header |
|---|---|
| Protection | `330 repositories · 12 protected` |
| Deletion | `330 repositories · 12 protected · 3 marked for deletion` (the marked count updates live) |

### Lines

Each repository is displayed on its own line with:

| Element | Description |
|---|---|
| Marker | Depends on the list (see below). |
| Name | The repository name. |
| Visibility label | `public` or `private`. |
| Status labels | `fork` if the repository is a fork, `archived` if it is archived. |
| Size | Human-readable size, as reported by GitHub: whole `KB` or `MB`, `GB` with one decimal (`Ko`, `Mo`, `Go` in French). |
| Last update | The date of the last push (or of the creation if there was none), in absolute format (`2026-10-22`). |

The size reported by GitHub is approximate and can lag behind the latest pushes.

## Feature 1: Protect repositories

### Display

- `[ ]`: the repository is not protected.
- `[P]` in green: the repository is protected.

### Navigation

| Key | Action |
|---|---|
| `↑` / `↓` | Move through the list. |
| `Shift + ↑` / `Shift + ↓` | Jump 5 repositories up or down (also `PgUp` / `PgDn`). |
| `Ctrl + ↑` / `Ctrl + ↓` | Go to the first or last repository (also `Home` / `End`). |
| `Space` | Toggle protection on the current repository. |
| `Enter` | Save the protections and return to the main menu. |
| `Esc` | Return to the main menu without saving the changes. |

- No confirmation is asked: protecting a repository is not dangerous.
- The `rust-repositories-cleaner` repository is protected by default on first run.

### Storage

- Protected repositories are saved in `config.toml` (see [Local files](#local-files)).
- They are kept between runs. They are not stored on GitHub.

## Feature 2: Delete repositories

### Display

- Unprotected repositories:
  - `[ ]` (default): the repository is kept.
  - `[✗]` in red: the repository is marked for deletion.
- Protected repositories are shown grayed out with a `protected` label and cannot be marked.

### Navigation

| Key | Action |
|---|---|
| `↑` / `↓` | Move through the list. |
| `Shift + ↑` / `Shift + ↓` | Jump 5 repositories up or down (also `PgUp` / `PgDn`). |
| `Ctrl + ↑` / `Ctrl + ↓` | Go to the first or last repository (also `Home` / `End`). |
| `Space` | Toggle the mark on the current repository (no effect on protected repositories). |
| `Enter` | Validate the selection. |
| `Esc` | Return to the main menu without deleting anything. |

- There are no "select all" or "deselect all" shortcuts, on purpose, to avoid mass deletion by mistake.
- If `Enter` is pressed while no repository is marked, the message "No repository selected" is displayed and the user stays on the list, with their current state unchanged.

### Confirmation

- After validating, the application asks the question below, followed by the names of the marked repositories, separated by commas. If they do not all fit on the screen, the last line ends with "… and N more".
- Question: "Do you really want to delete these N repositories? [y/N]" ("Do you really want to delete this repository? [y/N]" when only one is marked).
- The answer defaults to **No**: pressing `Enter` alone cancels the deletion.
- The selected repositories are deleted only if the user answers `y` (`o` for "oui" in the French interface, shown as `[o/N]`), in lowercase or uppercase.
- Any other answer, including `Enter` alone and `Esc`, returns to the deletion list with the marks kept.

### Results

- Each repository is deleted one by one, and the screen shows the progress: "Deleting repositories…" above a progress bar, labeled with the number of repositories processed, the total and the percentage, for example `4/10 · 40%`.
- `Esc` or `Ctrl + C` stops the deletions after the current repository: a deletion already sent to GitHub cannot be cancelled. The remaining repositories are left untouched.
- A failed deletion does not stop the next ones.
- The console only shows a summary, for example "3 deleted, 1 failed. See log: ~/.config/rust-repositories-cleaner/logs/deletions.log" (the full path of the log, see [Local files](#local-files)), followed by the number of repositories left untouched if the deletions were stopped, then "Press any key to return to the menu".
- There is no dry-run mode.

### Log

- Every deletion session is appended to a single log file, `logs/deletions.log` (see [Local files](#local-files)), always in English.
- Each line starts with a timestamp, truncated to the second. A session starts with `STARTED` and ends with `FINISHED`. Each repository gets a `DELETING` line before GitHub is asked to delete it, then its result, with GitHub's reason on failure. Repositories left untouched after a stop are `SKIPPED`:

  ```
  2026-10-23T10:15:00Z STARTED 3 repositories
  2026-10-23T10:15:00Z DELETING owner/first-repository
  2026-10-23T10:15:01Z DELETED owner/first-repository
  2026-10-23T10:15:01Z DELETING owner/second-repository
  2026-10-23T10:15:02Z FAILED owner/second-repository: HTTP 403: Must have admin rights
  2026-10-23T10:15:02Z SKIPPED owner/third-repository: stopped by the user
  2026-10-23T10:15:02Z FINISHED 1 deleted, 1 failed, 1 skipped
  ```

- A session left without `FINISHED` is closed at the next start with an `INTERRUPTED` line, which names the repository whose deletion was cut short, if any:

  ```
  2026-10-23T11:00:00Z INTERRUPTED the previous session ended before FINISHED, owner/second-repository may have been deleted
  ```

- Every premature end of a session is recorded:
  - stopped by the user: `SKIPPED` lines, then `FINISHED`;
  - ended without `FINISHED` (application killed, terminal closed, crash): detected at the next start, which adds an `INTERRUPTED` line and tells the user, naming the repository that may have been deleted;
  - log that cannot be written: the deletions stop at once, since every deletion must be recorded, and the screen tells what was done and which repository is missing from the log.
- The log is locked while deleting, so that two instances of the application never write to it at the same time; a second instance cannot delete until the first one is done.
- The date of the last deletion is read back from the last `DELETED` line.

## Feature 3: Language

- The user chooses the interface language: **English** (default) or **French**.
- `Enter` applies the selected language and returns to the main menu; `Esc` returns to the main menu without changing it.
- The choice is saved in `config.toml` and kept between runs.
- Language names are written in their own language in the menu: `English`, `Français`.
- The log file is always written in English, whatever the interface language, so it can be read back reliably.

## Feature 4: Quit

- Exits the application cleanly.

## Local files

The files are read and written in the **application directory**, inside the user configuration directory of the system, so the application works wherever it is installed and launched from:

| System | Application directory |
|---|---|
| Linux | `$XDG_CONFIG_HOME/rust-repositories-cleaner`, by default `~/.config/rust-repositories-cleaner` |
| macOS | `~/Library/Application Support/rust-repositories-cleaner` |
| Windows | `%APPDATA%\rust-repositories-cleaner` |

The directory is created on first run.

| File | Content |
|---|---|
| `config.toml` | Interface language and protected repositories. Created with default values on first run. |
| `logs/deletions.log` | History of all deletions, appended over time. |

`config.example.toml`, at the root of the project, is a documented example of `config.toml`.

## Distribution

- Each version is published as a GitHub release, built by GitHub Actions when a version tag (for example `v0.1.0`) is pushed.
- The release holds one archive per system: Windows (x64, `.zip`), macOS (Intel and Apple Silicon, `.tar.gz`) and Linux (x64, `.tar.gz`). Each archive contains the executable, `README.md` and the licenses.
- No Rust toolchain is needed to run it, only the GitHub CLI (see [GitHub access](#github-access)).
- The executables are not signed: macOS and Windows show a warning on first launch. `README.md` explains how to launch them anyway.

## Error handling

| Situation | Behavior |
|---|---|
| The user configuration directory cannot be found, or the application directory cannot be created, at startup | Display the error, then exit. |
| `gh` is not installed or the user is not logged in, at startup | Display a clear message with the command to run (for example `gh auth login`), then exit. |
| The `delete_repo` scope is missing, when opening **Delete repositories** | Display a clear message with the command to run (`gh auth refresh -s delete_repo`), then return to the main menu on any key. |
| GitHub request error at startup, while checking the access (network, server, revoked token) | Display the error given by `gh`, then exit: the main menu is not shown yet. |
| GitHub request error while loading the list (network, server, permissions) | Display the error given by `gh` and return to the main menu. |
| Unexpected response from `gh` | Display the error and return to the main menu. |
| `logs/deletions.log` cannot be created, read or written | Before deleting: display the error and delete nothing. At startup: display the error. |
| Another instance of the application is deleting | Display a message and delete nothing. |
| `config.toml` cannot be saved (for example after changing the language) | Display the error, keep the previous settings, and return to the main menu on any key. |
