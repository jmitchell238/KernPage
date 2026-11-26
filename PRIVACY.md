# Privacy

KernPage is free, open-source software that runs on your computer. This page says in plain words what it does with your information.

## The short version
- We do not collect your information. There are no accounts, no analytics, no tracking and no crash reports sent anywhere.
- Your publications stay on your computer. KernPage never uploads them.
- The only network use is checking for and downloading updates, and only when you allow it.

## What happens on the network
- **Update checks (KernPage Launcher).** By default updates are manual: nothing is sent until you press Check. If you turn on automatic checks, or press Check, the Launcher asks GitHub for a small file (`index.json`) and its signature. The request carries a fixed program name and version (`KernPageLauncher/<version>`) and nothing that identifies you. Downloads of installers also come from GitHub.
- **GitHub sees your IP address** when you check or download, like any website. We do not receive that information. GitHub's own rules apply: https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement
- The editor itself sends nothing. When you start it, it may ask the Launcher whether an update is known; the Launcher follows your update setting above.

## What is stored on your computer
- Recent files list, settings and AutoRecover copies of the document you are editing: `%APPDATA%\KernPage\`.
- Launcher settings and the update cache: `%APPDATA%\KernPage\`.
- Downloaded installers and install logs: `%LOCALAPPDATA%\KernPage\updates\`.
- If you turn on "Start at login", a value named `KernPage Launcher` is added under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

You can delete these folders at any time. Uninstalling removes the program; your own documents are never touched.

## If you contact us
If you open an issue, a Discussion or a private report, we see what you send (and, on GitHub, your public username). We use it only to answer you. GitHub issues are public.

## Children
KernPage does not knowingly collect information from anyone, including children.

## Changes
If the software ever collects anything, this page and the program's About box will say so before the change ships.

## Contact
Privacy questions: open an issue or a Discussion on the repository (https://github.com/jmitchell238/kernpage). For something you don't want to post publicly, use the private "Report a vulnerability" form on the repository's Security page.

KernPage is an independent project. It is not affiliated with, endorsed by or sponsored by Microsoft. Microsoft, Publisher, Office and Windows are trademarks of the Microsoft group of companies.
