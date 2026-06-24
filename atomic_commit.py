import subprocess
import os
import sys

def get_changes():
    res = subprocess.run(["git", "status", "--porcelain", "-uall"], capture_output=True, text=True, check=True)
    lines = res.stdout.strip().split('\n')
    changes = []
    for line in lines:
        if not line:
            continue
        status = line[:2]
        path = line[3:].strip()
        # strip quotes if git quotes the filename
        if path.startswith('"') and path.endswith('"'):
            path = path[1:-1]
        changes.append((status, path))
    return changes

def run_git(args):
    print("Running:", " ".join(args))
    res = subprocess.run(args, capture_output=True, text=True)
    if res.returncode != 0:
        print("Error:", res.stderr)
    else:
        print("Success:", res.stdout)
    return res.returncode == 0

def main():
    changes = get_changes()
    print(f"Total changes: {len(changes)}")
    
    # Define groups by prefix or matching function
    groups = [
        {
            "name": "Agent & Configuration Cleanup",
            "msg": "chore: clean up agent metadata, local database, and configuration files",
            "filter": lambda p: p.startswith(".agents/") or p in ["sw.db", "PROJECT.md", "ORIGINAL_REQUEST.md"] or p.startswith(".cargo/") or p.startswith(".rustup/")
        },
        {
            "name": "Project Tooling & Dependencies",
            "msg": "chore: update dependencies, formatting options, and lint rules",
            "filter": lambda p: p in ["Cargo.toml", "Cargo.lock", "rustfmt.toml", "atomic_commit.py"] or p.startswith("scripts/")
        },
        {
            "name": "Browser UI and Tab Management",
            "msg": "refactor: restructure browser tabs and UI bridge components",
            "filter": lambda p: p.startswith("src/browser/")
        },
        {
            "name": "Network and Protocols",
            "msg": "refactor: partition network module into sub-components",
            "filter": lambda p: p.startswith("src/network/")
        },
        {
            "name": "Renderer and Paint",
            "msg": "refactor: separate rendering paint implementation details",
            "filter": lambda p: p.startswith("src/renderer/")
        },
        {
            "name": "Common Utils and Core Entrypoints",
            "msg": "refactor: organize common utility modules and core entry points",
            "filter": lambda p: p in ["src/main.rs", "src/ace/json.rs", "src/ace/crypto.rs"] or p.startswith("src/utils/") or p.startswith("src/ace/util/")
        },
        {
            "name": "URL and Parser",
            "msg": "refactor: modularize URL parser and percent-encoding",
            "filter": lambda p: p.startswith("src/ace/url/")
        },
        {
            "name": "Runtime JS Core",
            "msg": "refactor: slice JS runtime core and quickjs bindings",
            "filter": lambda p: p.startswith("src/ace/runtime/core/") or p.startswith("src/ace/runtime/bridge/")
        },
        {
            "name": "Runtime WebAPI and DOM Bindings",
            "msg": "refactor: slice JS DOM and WebAPI bindings into sub-modules",
            "filter": lambda p: p.startswith("src/ace/runtime/bindings/")
        },
        {
            "name": "HTML Parser",
            "msg": "refactor: modularize HTML parser tokenizer and tree builder",
            "filter": lambda p: p.startswith("src/ace/html/")
        },
        {
            "name": "Graphics & Layout Engines",
            "msg": "refactor: modularize graphics pipeline and layout builder",
            "filter": lambda p: p.startswith("src/ace/engine/graphics/") or p.startswith("src/ace/engine/layout/") or p.startswith("src/ace/engine/layout_builder/") or p.startswith("src/ace/engine/layout_builder.rs") or p.startswith("src/ace/engine/layout_sync/") or p.startswith("src/ace/engine/layout_sync.rs") or p.startswith("src/ace/engine/interaction.rs") or p.startswith("src/ace/engine/pipeline.rs") or p.startswith("src/ace/engine/core.rs") or p.startswith("src/ace/engine/core/") or p.startswith("src/ace/engine/visual.rs") or p.startswith("src/ace/engine/visual/")
        },
        {
            "name": "DOM Engine",
            "msg": "refactor: partition DOM engine node/range/selection types",
            "filter": lambda p: p.startswith("src/ace/engine/dom/")
        },
        {
            "name": "Style Engine",
            "msg": "refactor: modularize style engine and CSS property parsers",
            "filter": lambda p: p.startswith("src/ace/engine/style/")
        }
    ]
    
    # Keep track of which files are committed
    handled = set()
    
    for g in groups:
        group_files = []
        for status, path in changes:
            if path not in handled and g["filter"](path):
                group_files.append(path)
                handled.add(path)
        
        if group_files:
            print(f"\nProcessing Group: {g['name']} ({len(group_files)} files)")
            # Add files in chunks to avoid command line length limits
            chunk_size = 100
            for i in range(0, len(group_files), chunk_size):
                chunk = group_files[i:i+chunk_size]
                run_git(["git", "add"] + chunk)
            run_git(["git", "commit", "-m", g["msg"]])
            
    # Check if there are any leftover changes
    leftovers = [path for status, path in changes if path not in handled and path != "atomic_commit.py"]
    if leftovers:
        print(f"\nProcessing Leftover Changes ({len(leftovers)} files)")
        chunk_size = 100
        for i in range(0, len(leftovers), chunk_size):
            chunk = leftovers[i:i+chunk_size]
            run_git(["git", "add"] + chunk)
        run_git(["git", "commit", "-m", "chore: commit remaining refactoring files"])

    # Clean up the script itself from git tracking if it was added
    subprocess.run(["git", "rm", "--cached", "atomic_commit.py"], capture_output=True)
    if os.path.exists("atomic_commit.py"):
        os.remove("atomic_commit.py")
        print("\nCleaned up atomic_commit.py")

if __name__ == "__main__":
    main()
