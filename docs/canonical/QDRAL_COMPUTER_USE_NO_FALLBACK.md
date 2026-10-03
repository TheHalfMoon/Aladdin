# Qdral Computer-Use No-Silent-Fallback Rule

Status: PLANNED
Date: 2026-10-03

A failure in a higher-level structured mechanism never silently grants a lower-level mechanism. Browser structured failure does not grant raw CDP/JavaScript; UIA failure does not grant coordinate input; exact-window capture failure does not grant whole-screen capture; supported-browser failure does not grant a personal profile; remote lease failure does not grant local or broader remote authority.
