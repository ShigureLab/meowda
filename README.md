# Meowda <img src="https://www.gstatic.com/android/keyboard/emojikitchen/20230301/u1f61c/u1f61c_u1f431.png" alt="🐱" width="20px"/> <sub><samp>—— 「喵哒」</samp></sub>

Meowda, manage multiple Python virtual environments with ease. It's based on [uv](https://docs.astral.sh/uv/), and provides a conda-like CLI (_NOT_ a replacement) for Python virtual environments management.

## Installation

Before installing Meowda, make sure you have [uv](https://docs.astral.sh/uv/) installed.

### With cargo

```bash
cargo install meowda
```

### With uv

```bash
uv tool install meowda
```

## Usage

Meowda provides a conda-like activate/deactivate interface for managing virtual environments. Before using Meowda, you need to initialize it in your shell. You can do this by running the following command:

```bash
meowda init <shell_profile>
# For example, for bash:
meowda init ~/.bashrc
source ~/.bashrc
# For zsh:
meowda init ~/.zshrc
source ~/.zshrc
```

After initialization, you can use Meowda to create and manage virtual environments. Here are some basic commands:

```bash
meowda create env-1 -p 3.14
meowda activate env-1
meowda install ruff
meowda deactivate
```

## Acknowledgement

-  [uv](https://docs.astral.sh/uv/) for the virtual environment management.
-  [conda](https://github.com/conda/conda) for the inspiration of the CLI interface.
