# SG-000009 private contained argv execution note

Date: 2026-09-24

SG-000009 qualifies a provider-internal Windows executor. It does not register a process capability and is not reachable from policy, quntald, or MCP.

The qualification fixture is deliberately fixed:

- executable: the operating-system `whoami.exe` resolved relative to `GetSystemDirectoryW`;
- argv: no arguments; `/all` is excluded because its claims query is unavailable in the zero-capability AppContainer and returns exit code 1;
- cwd: an explicit temporary qualification root;
- environment: the existing fixed-name, case-insensitive allowlist with Quntal/tunnel/secret values excluded;
- stdin: `NUL` with no caller input;
- stdout/stderr: separate bounded pipes;
- process: zero-capability AppContainer, created suspended;
- lifecycle: Job Object assignment and `IsProcessInJob` verification before resume;
- completion: finite wait and exit-code evidence;
- failure: Job termination followed by active-process quiescence verification.

The child receives only three explicitly listed inheritable handles through `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`: NUL stdin, the stdout pipe writer, and the stderr pipe writer. Other process handles are not authorized for inheritance. Parent pipe writers are closed immediately after process creation. The implementation contains typed output-limit and timeout termination branches, but SG-000009 does not claim runtime qualification of those destructive paths. Their descendant-tree fixtures and runtime evidence are successor work.

- termination: `TerminateJobObject` is followed by bounded polling of `JOBOBJECT_BASIC_ACCOUNTING_INFORMATION.active_processes`; only `ActiveProcesses == 0` promotes a destructive event to its typed result;
- precedence: the first observed stdout/stderr overflow is retained over a later timeout observation, while any inability to terminate or verify zero active processes returns `TerminationUnverified` and never a less-severe result;
- fixtures: the fixed provider-private test-harness modes (`--exact contained_launch_tests::private_timeout_fixture_child`, `--exact contained_launch_tests::private_stdout_fixture_child`, and `--exact contained_launch_tests::private_stderr_fixture_child`) are selected only by provider-private qualification code and validated against exact executable/argument tuples;
- no caller-selected executable, arguments, shell, PowerShell, network, ACL, browser/UI, elevation, approval, or `process.spawn` authority was added.

The native Windows tests qualify timeout, independent stdout and stderr overflow, indeterminate termination classification, Job termination, and the existing no-argument `whoami.exe` regression. The tests are qualification evidence only; the failure modes are not an external process API.

The native Windows qualification test proves the fixed no-argument fixture, workspace-bound cwd binding, bounded output capture, AppContainer token, pre-resume Job membership, exit code, and Job quiescence. A prior native diagnostic proved that `whoami.exe /all` reports `Unable to get user claims information.` and exits 1 in this zero-capability AppContainer; `/all` is therefore not the success fixture. The successor SG-000009A runtime tests now qualify the fixed timeout, stdout-limit, and stderr-limit destructive paths and their verified termination semantics.

No donor implementation source was copied. The design uses Microsoft AppContainer, Job Object, pipe, and process APIs documented by Win32.
