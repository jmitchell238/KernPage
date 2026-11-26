# Security policy

## Supported versions
Only the latest release receives security fixes. The KernPage Launcher updates you to it.

## Reporting a vulnerability
Please do not open a public issue for a security problem.

Use GitHub **Private vulnerability reporting**: the "Report a vulnerability" button on the repository's Security > Advisories page. Only the maintainers can see your report. Include the version, what you did, what you expected, what happened, and a sample file if relevant (remove personal data first).

We aim to acknowledge reports within 7 days and to tell you what we plan within 30 days. This is a small volunteer project, so these are goals, not guarantees. We will credit you in the release notes if you wish.

## In scope
- Memory-safety or parsing problems when opening `.pub`, WMF/EMF or other files (crashes, hangs, out-of-memory, code execution).
- The KernPage Launcher's update path: signature or hash bypass, downgrade or replay of the update index, running a file that was not verified, tampering with files in the update folder.
- Installer problems (privilege escalation, unsafe file permissions, wrong install scope).

## Out of scope
- Problems that need an attacker who already controls your account or computer.
- Missing features, or Publisher behaviour that KernPage does not copy.

## Good-faith research
If you act in good faith, avoid privacy violations and data loss, and give us reasonable time to fix a problem before telling others, we will not take legal action against you. (A statement of the maintainer's intent, not legal advice.)

## How updates are protected
Updates are listed in a signed index (minisign). Every download is checked for signature, SHA-256 and exact size, over HTTPS, before it is run. See packaging/windows/INSTALLATION.md and docs/RELEASING.md.
