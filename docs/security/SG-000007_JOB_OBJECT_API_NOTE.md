# SG-000007 Job Object API note

Date: 2026-09-24

Quntal qualifies the Windows Job Object lifecycle primitive without launching or assigning a child process.

The probe uses:
- CreateJobObjectW
- SetInformationJobObject
- JOBOBJECT_EXTENDED_LIMIT_INFORMATION
- JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
- CloseHandle

Security interpretation:
- Job Objects provide process-tree lifecycle and resource controls.
- Quntal does not treat a Job Object as an authorization sandbox.
- AppContainer remains the intended resource-isolation boundary for future arbitrary child execution.
- A successor grain must assign a contained child before resume and prove the containment sequence before MCP EXECUTE authority is exposed.

Official references:
- https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-createjobobjectw
- https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-setinformationjobobject
- https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information

No Microsoft source code is copied. The FFI and struct declarations mirror the documented ABI only.
