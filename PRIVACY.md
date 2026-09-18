# Syncinit Privacy Policy

_Last updated: 2026-09-16_

## Short version

Syncinit doesn't collect anything. It's an offline desktop application. There
is no account, no analytics, no crash reporting, and no network request of
any kind built into the app as of this version.

## What Syncinit does on your machine

- Reads and writes files only at paths you explicitly choose (via the
  native file/folder picker, or a file/folder you drag in or right-click).
- Writes a small number of Windows Registry entries under
  `HKEY_CURRENT_USER\Software\Classes` to register the right-click "Syncinit"
  context menu and `.init`/`.zip`/`.7z` file associations. This stays on
  your machine and is never transmitted anywhere.
- Stores your locale choice (which language you picked) in the app's local
  `localStorage`, again never transmitted anywhere.

## What Syncinit does not do

- No telemetry, usage analytics, or crash reporting.
- No account or sign-in.
- No network requests — nothing is uploaded, nothing is phoned home,
  nothing is checked against a remote server.
- No advertising, no ad SDKs, no third-party trackers.

## If that changes

If a future version adds anything that talks to the network — an update
checker, optional crash reporting, anything — this document will be
updated first, and it will describe exactly what's sent, when, and how to
turn it off.

## Data you create

Archives you create with Syncinit, and anything you extract, stay wherever
you put them. Syncinit has no visibility into that data once the operation
completes — it isn't copied, logged, or retained by the app in any other
location.

## Contact

Questions about this policy: open an issue at the project's GitHub
repository.
