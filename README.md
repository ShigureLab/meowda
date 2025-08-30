# Meowda <img src="https://www.gstatic.com/android/keyboard/emojikitchen/20230301/u1f61c/u1f61c_u1f431.png" alt="🐱" width="20px"/> <sub><samp>—— 「喵哒」</samp></sub>

**Meowda** is a modern Python virtual environment management tool that lets you easily manage multiple Python virtual environments. Built on [uv](https://docs.astral.sh/uv/), it provides a conda-like CLI interface (but is not a conda replacement or compatible with conda) focused on simple and fast virtual environment management.

## ✨ Features

-  🚀 **Fast**: Lightning-fast package management powered by [uv](https://docs.astral.sh/uv/)
-  🎯 **Simple**: Intuitive conda-like command line interface
-  🔄 **Flexible**: Support for both global and project-level environment management
-  🔗 **Project Linking**: Associate projects with specific environments
-  🛠️ **VS Code Integration**: Seamless integration into development workflow
-  📦 **Lightweight**: Written in Rust for excellent performance

## 📦 Installation

### Prerequisites

Make sure you have [uv](https://docs.astral.sh/uv/) installed. See the [official installation guide](https://docs.astral.sh/uv/getting-started/installation/) for detailed instructions.

### Install Meowda

#### Using uv (Recommended)

```bash
uv tool install meowda
```

#### Using Cargo

```bash
cargo install meowda
```

## 🚀 Quick Start

### 1. Initialize Shell

```bash
# For zsh users
meowda init ~/.zshrc
source ~/.zshrc

# For bash users
meowda init ~/.bashrc
source ~/.bashrc
```

### 2. Basic Usage

```bash
# Create and use a virtual environment
$ meowda create my-project -p 3.12
Using CPython 3.12.11
Creating virtual environment with seed packages at: /Users/user/.local/share/meowda/venvs/my-project
 + pip==25.2
Activate with: source /Users/user/.local/share/meowda/venvs/my-project/bin/activate
Virtual environment 'my-project' created successfully.

$ meowda activate my-project
$ meowda install requests pandas
$ meowda deactivate

# List and manage environments
$ meowda env list
Available global virtual environments:
  my-project (/Users/user/.local/share/meowda/venvs/my-project python 3.12.11)

$ meowda env dir
/Users/user/.local/share/meowda/venvs

$ meowda remove my-project
Virtual environment 'my-project' removed successfully.
```

## 💡 Advanced Features

### Global vs Local Environments

```bash
# Global environments (accessible anywhere)
$ meowda create --global tools -p 3.12
$ meowda activate --global tools

# Local environments (project-specific, stored in .meowda/venvs/)
$ meowda create --local myproject -p 3.11
Using CPython 3.11.13
Creating virtual environment with seed packages at: .meowda/venvs/myproject
 + pip==25.2 + setuptools==80.9.0 + wheel==0.45.1
Virtual environment 'myproject' created successfully.

$ meowda env dir --local
/path/to/project/.meowda/venvs
```

### Development Workflow Example

```bash
$ mkdir awesome-app && cd awesome-app
$ meowda create --local awesome-app -p 3.12
$ meowda activate --local awesome-app
$ meowda install fastapi uvicorn pytest sqlalchemy
$ meowda deactivate
```

### Advanced Options

```bash
# Recreate environment (clear existing packages)
$ meowda create my-env -p 3.12 --clear

# Install specific versions or from requirements
$ meowda install "django>=4.0,<5.0" "pytest==7.4.0"
$ meowda install -r requirements.txt

# Project linking
$ meowda link my-web-app /path/to/web-project
$ meowda unlink my-web-app
```

## 🔌 VS Code Integration

Add to your `settings.json`:

```json
{
   "python.venvFolders": [".meowda/venvs", "~/.local/share/meowda/venvs"]
}
```

## 📖 Command Reference

**Environment Management**

-  `meowda create <name> -p <version>` - Create environment
-  `meowda activate <name>` - Activate environment
-  `meowda deactivate` - Deactivate current environment
-  `meowda remove <name>` - Remove environment
-  `meowda env list` - List all environments
-  `meowda env dir` - Show storage directory

**Package Management**

-  `meowda install <packages>` - Install packages
-  `meowda uninstall <packages>` - Uninstall packages

**Options**: `--global`, `--local`, `--clear`

## 🤔 FAQ

**Q: Why was Meowda created?**

A: Meowda bridges the gap between conda's convenience and uv's speed. While conda is great for managing environments across projects, it can be heavy. While uv is blazingly fast, managing multiple environments with it can be cumbersome. Meowda combines the best of both worlds by providing a conda-like interface powered by uv's performance.

**Q: What's the difference between Meowda and conda?**

A: Meowda focuses specifically on Python virtual environment management and provides faster package installation through uv. It's not a complete conda replacement, but rather a lightweight alternative.

**Q: Global vs local environments?**

A: Global environments are stored in `~/.local/share/meowda/venvs` and accessible anywhere. Local environments are stored in project's `.meowda/venvs/` and only available within that project.

## 🙏 Acknowledgement

-  [uv](https://docs.astral.sh/uv/) - For the virtual environment management core functionality
-  [conda](https://github.com/conda/conda) - For the inspiration of the CLI interface design

---

<div align="center">

**Like Meowda? Give us a ⭐️!**

Made with 🐱 by [ShigureLab](https://github.com/ShigureLab)

</div>
