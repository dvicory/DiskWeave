project = "DiskWeave"
author = "DiskWeave contributors"
release = "0.1"

extensions = ["myst_parser", "sphinx_needs", "sphinx_codelinks"]
source_suffix = {".md": "myst"}
myst_heading_anchors = 3

needs_types = [
    {
        "directive": "req",
        "title": "Requirement",
        "prefix": "R_",
        "color": "#BFD8D2",
        "style": "node",
    }
]
needs_id_required = True
needs_id_regex = r"(?:R|ST)_[0-9A-F]{20}"
needs_fields = {
    field: {"description": f"DiskWeave {field}", "schema": {"type": "string"}}
    for field in ["semantic_id", "capability", "source_path", "heading_path", "fingerprint"]
}

src_trace_config_from_toml = "codelinks.toml"
# CodeLinks 1.4 probes VCS metadata even when remote links are disabled; clean-room
# reconstruction intentionally has no `.git` directory.
suppress_warnings = ["codelinks.git_remote", "codelinks.git_root"]

nitpicky = True
html_theme = "alabaster"
html_title = "DiskWeave"
exclude_patterns = ["_build", "codelinks-output"]
