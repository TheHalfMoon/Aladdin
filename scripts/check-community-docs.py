#!/usr/bin/env python3
"""Check contributor-facing repo docs without network or external deps."""
from pathlib import Path
from urllib.parse import unquote
import re
import sys

root=Path(__file__).resolve().parent.parent
files=[
  "README.md","CONTRIBUTING.md","CODE_OF_CONDUCT.md","SUPPORT.md","CHANGELOG.md",
  "docs/community/MAINTAINERSHIP.md",".github/PULL_REQUEST_TEMPLATE.md",
]
errors=[]
for name in files:
    p=root/name
    if not p.is_file():errors.append(f"Missing {name}");continue
    s=p.read_text(encoding="utf-8")
    if len(s)<150:errors.append(f"Short {name}")
    for target in re.findall(r"\]\(([^)\n]+)\)",s):
        if target.startswith(("https://","http://","#","mailto:")):continue
        if target.startswith(("<",">")):continue
        target=unquote(target.split("#",1)[0])
        if not target or " " in target:continue
        dest=(p.parent/target).resolve()
        if not dest.exists():errors.append(f"{name}: missing linked path {target}")
    if name=="README.md":
        for value in ['## Why Orcel?','## Install','CONTRIBUTING.md','SECURITY.md','open-source']:
            if value not in s:errors.append(f"README missing {value}")
for name in [".github/ISSUE_TEMPLATE/bug_report.yml",".github/ISSUE_TEMPLATE/feature_request.yml",".github/ISSUE_TEMPLATE/config.yml"]:
    p=root/name
    if not p.is_file():errors.append(f"Missing {name}");continue
    value=p.read_text(encoding="utf-8")
    if not value.endswith("\n"):errors.append(f"No final newline {name}")
    if not value.strip():errors.append(f"Empty {name}")
yaml_status="skipped (PyYAML unavailable)"
try:
    import yaml
except ImportError:
    pass
else:
    yaml_status="passed"
    for name in [".github/ISSUE_TEMPLATE/bug_report.yml",".github/ISSUE_TEMPLATE/feature_request.yml",".github/ISSUE_TEMPLATE/config.yml"]:
        try:
            loaded=yaml.safe_load((root/name).read_text(encoding="utf-8"))
            if not isinstance(loaded,dict):errors.append(f"{name}: expected mapping")
        except yaml.YAMLError as exc:errors.append(f"{name}: {exc}")
if errors:
    print("\n".join(errors));sys.exit(1)
print(f"Contributor documentation: links and required files passed; YAML validation: {yaml_status}.")
