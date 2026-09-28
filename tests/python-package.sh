#!/usr/bin/env bash
# Run with the wheel installed in the Python environment on PATH.
set -euxo pipefail

python_version=${1:?Usage: bash tests/python-package.sh <python-version>}
test_dir=$(mktemp -d)
trap 'rm -rf "$test_dir"' EXIT
export MEOWDA_GLOBAL_VENV_DIR="$test_dir/venvs"

check_python() {
    python - "$python_version" "${1:-}" <<'PY'
import sys
import sysconfig

if sys.argv[2] == "requests":
    import requests
    print(requests.__version__)

requested = sys.argv[1]
assert sys.version_info[:2] == tuple(map(int, requested.rstrip("t").split(".")))
assert bool(sysconfig.get_config_var("Py_GIL_DISABLED")) == requested.endswith("t")
if requested.endswith("t"):
    assert not sys._is_gil_enabled()
print(sys.version)
PY
}

check_python
python - <<'PY'
from importlib.metadata import distribution

package = distribution("meowda")
assert package.metadata["Requires-Python"] == ">=3.11"
# Maturin packages a standalone executable, not a CPython extension module.
assert "Tag: py3-none-" in package.read_text("WHEEL")
PY
meowda --version
meowda --help
meowda init "$test_dir/init.sh"
# shellcheck source=/dev/null
source "$test_dir/init.sh"
meowda create test-venv -p "$python_version"
meowda activate test-venv
check_python
meowda install requests
check_python requests
meowda fork forked-venv --from test-venv
meowda deactivate
meowda activate forked-venv
check_python requests
meowda deactivate
meowda remove forked-venv
meowda remove test-venv
test ! -d "$MEOWDA_GLOBAL_VENV_DIR/test-venv"
test ! -d "$MEOWDA_GLOBAL_VENV_DIR/forked-venv"
