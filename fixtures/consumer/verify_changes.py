"""MATMA real-consumer acceptance: edited type only, not its siblings."""
import json
import subprocess
import sys
from pathlib import Path

EXE = Path("../../target/debug/matma-indexer" + (".exe" if sys.platform == "win32" else "")).resolve()
SOURCE = Path("src/order_processing.rs")
BEFORE = "fn private_rate(&self) -> u32 { self.rate.get() }"
AFTER = "fn private_rate(&self) -> u32 { self.rate.get().saturating_add(1) }"


def matma(*args):
    result = subprocess.run([str(EXE), *args, "-o", "json"],
                            check=True, text=True, capture_output=True)
    return json.loads(result.stdout)


def main():
    original = SOURCE.read_text()
    if BEFORE not in original:
        raise AssertionError("Source fixture changed; update the AST change scenario")
    try:
        SOURCE.write_text(original.replace(BEFORE, AFTER, 1))
        matma("index")
        changes = matma("changes", "-l", "0")
        assert changes["changed_symbols"] == ["crate::order_processing::A"], changes
        assert changes["affected_symbols"] == ["crate::order_processing::A"], changes
        context = matma("context", "-f", "A")
        fragments = [entry["snippet"] for entry in context["regions"]]
        assert any("impl A" in item for item in fragments), fragments
        assert all("pub struct B" not in item for item in fragments), fragments
        assert context["identity"] == "crate::order_processing::A"
        print("MATMA consumer: type-level change detection and bounded context passed")
    finally:
        SOURCE.write_text(original)
        matma("index")


if __name__ == "__main__":
    main()
