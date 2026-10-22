# GitHub CLI setup

`rust-repositories-cleaner` talks to GitHub through the [GitHub CLI](https://cli.github.com/) (`gh`). This guide explains how to install it, log in, and grant the permission to delete repositories.

## 1. Install

Follow the instructions for your system on https://github.com/cli/cli#installation, then check:

```sh
gh --version
```

## 2. Log in

```sh
gh auth login
```

Answer the questions as follows:

| Question | Answer | Why |
|---|---|---|
| Where do you use GitHub? | **GitHub.com** | "Other" is for GitHub Enterprise servers. |
| What is your preferred protocol for Git operations on this host? | **SSH** | Same protocol as the project's Git remote. Choose HTTPS if you do not use SSH keys. |
| Upload your SSH public key to your GitHub account? | **Skip** | Your key is already on GitHub if `git push` works over SSH. |
| How would you like to authenticate GitHub CLI? | **Login with a web browser** | No token to create by hand. |

`gh` then prints a one-time code (for example `ABCD-1234`) and opens https://github.com/login/device. Enter the code there and authorize GitHub CLI. If the browser does not open, open the address yourself.

Check the result:

```sh
gh auth status
```

It should print `Logged in to github.com account <your-login>`.

Being logged in is enough to start the application and protect repositories.

## 3. Allow deleting repositories

Deleting repositories requires the `delete_repo` scope. The application checks it when opening **Delete repositories** and tells you to run this command if it is missing:

```sh
gh auth refresh -h github.com -s delete_repo
```

As with the login, enter the one-time code at https://github.com/login/device. `gh auth status` then lists `delete_repo` in the token scopes.

Grant this scope only when you want to delete repositories: it allows any tool using your `gh` login to delete them.

## Troubleshooting

### "You are not logged in to GitHub CLI" although `gh` is logged in elsewhere

`gh` stores its login in its configuration directory, `~/.config/gh` by default. Applications running in a Flatpak sandbox, such as the VS Code Flatpak, use another directory (for example `~/.var/app/com.visualstudio.code/config/gh`), so a login made in one is not seen by the other.

Either log in again in the terminal where you run the application (`gh auth login`, recommended), or point `gh` to the other directory for a single run:

```sh
GH_CONFIG_DIR=~/.var/app/com.visualstudio.code/config/gh cargo run
```
