# Specifications

## Purpose

`rust-repositories-cleaner` is a console application written in Rust that lets the user bulk delete repositories from their GitHub account through a simple interactive menu, while protecting the repositories they want to keep.

The expected volume is a few hundred repositories (around 330 today).

## Design principles

- Minimalist, clean and sober interface that goes straight to the point.
- A short plain-text title only: no ASCII art banner, no emoji.
- Colors are used sparingly, only to convey meaning (red for deletion, green for protection, gray for disabled).

## GitHub access

- The application reuses the token of the GitHub CLI (`gh`).
- Deleting repositories requires the `delete_repo` scope on that token.

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

The user selects an entry to open it. After any action completes, the application returns to this main menu.

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
- The list scrolls when it is taller than the terminal.

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
| Size | Human-readable size (`KB`, `MB`, `GB`), as reported by GitHub. |
| Last update | The date of the last update, in absolute format (`2026-10-22`). |

The size reported by GitHub is approximate and can lag behind the latest pushes.

## Feature 1: Protect repositories

### Display

- `[ ]`: the repository is not protected.
- `[P]` in green: the repository is protected.

### Navigation

| Key | Action |
|---|---|
| `↑` / `↓` | Move through the list. |
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
| `Space` | Toggle the mark on the current repository (no effect on protected repositories). |
| `Enter` | Validate the selection. |
| `Esc` | Return to the main menu without deleting anything. |

- There are no "select all" or "deselect all" shortcuts, on purpose, to avoid mass deletion by mistake.
- If `Enter` is pressed while no repository is marked, the message "No repository selected" is displayed and the user stays on the list, with their current state unchanged.

### Confirmation

- After validating, the application lists all the marked repositories, then asks: "Do you really want to delete these N repositories? [y/N]"
- The answer defaults to **No**: pressing `Enter` alone cancels the deletion.
- The selected repositories are deleted only if the user answers `y`.

### Results

- Each repository is deleted one by one.
- The result of each deletion (success, or failure with the reason given by GitHub) is appended to a single log file, `logs/deletions.log` (see [Local files](#local-files)).
- Each log line starts with a timestamp, so the date of the last deletion can be read back from the log.
- The console only shows a summary, for example "3 deleted, 1 failed. See log: logs/deletions.log", then "Press any key to return to the menu".
- There is no dry-run mode.

## Feature 3: Language

- The user chooses the interface language: **English** (default) or **French**.
- The choice is saved in `config.toml` and kept between runs.

## Feature 4: Quit

- Exits the application cleanly.

## Local files

All files are read and written relative to the **current directory**, so the application is meant to be run from the project directory.

| File | Content | In Git |
|---|---|---|
| `config.toml` | Interface language and protected repositories. Created with default values on first run. | Ignored |
| `config.example.toml` | Documented example of `config.toml`, for anyone reusing the project. | Tracked |
| `logs/deletions.log` | History of all deletions, appended over time. | Ignored |

## Error handling

| Situation | Behavior |
|---|---|
| `gh` is not installed, the user is not logged in, or the `delete_repo` scope is missing | Display a clear message with the command to run (for example `gh auth refresh -s delete_repo`), then exit. |
| Network error while loading the list | Display the error and return to the main menu. |
