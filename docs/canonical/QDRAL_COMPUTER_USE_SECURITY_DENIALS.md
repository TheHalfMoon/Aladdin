# Qdral Computer-Use Explicit Security Denials

Status: PLANNED DENIAL RECORD FOR QDRAL-P18
Date: 2026-10-03

The following donor-style capabilities are intentionally denied in QDRAL-P18 unless a later separately authorized program changes them:

- arbitrary caller-provided JavaScript execution in browser pages;
- generic caller-facing CDP/DevTools/debugger commands;
- unrestricted shell, PowerShell, cmd, script, or interpreter execution;
- generic TCP/UDP/HTTP proxy, SOCKS, CONNECT, VPN, or arbitrary WebSocket tunnel;
- personal browser profile attachment or caller-selected user-data-dir;
- browser credential/cookie/password/session-store extraction;
- whole-screen/background desktop capture or indefinite screen streaming;
- direct model-to-NutJS/raw mouse/keyboard execution;
- arbitrary/free-form input streams, unrestricted hotkeys, or gestures not explicitly authorized;
- clipboard mutation as a typing transport;
- password/credential/security-dialog automation;
- UAC/Windows Hello/LogonUI/secure-desktop automation or capture;
- silent elevation;
- remote approval delegation, model self-approval, remote trust modification, or remote lease self-grant;
- silent structured-to-coordinate fallback;
- arbitrary browser download destinations or arbitrary upload source paths;
- auto-open, execute, shell-launch, or archive-extract downloaded content;
- surprise offline mutation queues or automatic mutation retries after dispatch;
- runtime dynamic donor-code download or `latest` package execution in production.

These are security/product decisions, not implementation gaps.
