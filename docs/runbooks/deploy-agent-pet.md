# Runbook: Deploy Agent Pet

Written: 2026-09-03

## Purpose

Publishes a release of the pet as a GitHub release on `github.com/DezA96/agent-pet`: an annotated
tag on `main` named after the release, release notes taken from the release's changelog entry, and
the built app attached as a zip. Anyone can reach it once it is up. The same build is then installed
on the developer's own Mac, replacing the pet running there.

Run it when a release's work is finished and the release is being closed — `/ship` drives this runbook at that point.

What ships is two things at the tag `release-<nnn>`: the source tree, and one asset,
`AgentPet-release-<nnn>-arm64.zip`, holding `AgentPet.app` as `./build.sh` lays it out from that
tree — the Rust observation core linked into the Swift AppKit surface, `arm64-apple-macos13.0`,
bundle id `gg.deza.agent-pet`, no dock icon. The app is unsigned and not notarised, so a download
must be opened with right-click Open the first time, and the release notes and README say so.
`build.sh` writes a fixed `CFBundleShortVersionString` of `0.1.0`; the tag, not that field, is what
identifies a release. Not shipped: `core/target/`, `build/`, the Swift test bundle, and any
`prototype/*` branch. A zip built from a tree other than the tagged commit is the wrong package.

## Prerequisites

- The release's work is finished: every item under the release scope's Planned Work is `Done` by
  its own story record or recorded on the scope as dropped, the changelog carries the release's
  entry, and all of it is committed on `main` with nothing left over. Check:
  `git branch --show-current` prints `main` and `git status --short` prints nothing.
- The remote is the public repo. Check: `git remote get-url origin` prints
  `https://github.com/DezA96/agent-pet.git`.
- `gh` is signed in to the account that owns it, with the `repo` scope. Check: `gh auth status`
  shows `DezA96` as the active account.
- The tag name is free on both sides. Check: `git tag -l release-<nnn>` prints nothing and
  `gh release view release-<nnn>` reports no release.
- The Rust toolchain is installed under `~/.cargo`, which `build.sh` sources itself. Check:
  `source ~/.cargo/env && cargo --version` prints a version.
- The Xcode Command Line Tools are installed and selected, with swift-testing present. No Xcode.
  Check: `xcode-select -p` prints `/Library/Developer/CommandLineTools`, and
  `ls "$(xcode-select -p)/Library/Developer/Frameworks/Testing.framework"` succeeds.
- Both test suites pass on the tree that ships. Check: `./test.sh` exits 0 with zero failures in
  both the Swift and the Rust totals.

## Steps

1. **Set the tag this deploy runs under.**
   ```
   TAG=release-<nnn>; TITLE="Release <nnn>: <name>"
   ```

2. **Confirm what ships is what is meant to ship.**
   ```
   git branch --show-current            # expect main
   git status --short                   # expect nothing
   git log -1 --oneline                 # the commit the changelog entry describes
   ```
   Another branch, an uncommitted file, or a head the changelog does not describe means stop.

3. **Push `main`.**
   ```
   git push origin main
   ```

4. **Build the bundle from that tree and zip it.** The pet already running is untouched:
   `build.sh` recreates the bundle directory, and the running process keeps its own binary.
   ```
   ./build.sh                           # expect the last line: ==> built .../build/AgentPet.app
   ditto -c -k --keepParent build/AgentPet.app "build/AgentPet-$TAG-arm64.zip"
   ```
   `build.sh` stops at the first failing stage. A failure here leaves the old pet running and
   nothing pushed but source.

5. **Tag the commit and push the tag.**
   ```
   git tag -a "$TAG" -m "$TITLE" && git push origin "$TAG"
   ```

6. **Write the release notes from the changelog entry.** Take the release's section of
   `CHANGELOG.md` — from its `## [<nnn>]` heading to the next `## ` heading — and add the
   install note below it:
   ```
   awk -v h="## [<nnn>]" '$0 ~ "^## \\[" {p = index($0, h) == 1} p' CHANGELOG.md > build/notes.md
   printf '\n### Install\nUnzip, move `AgentPet.app` wherever you like, then right-click it and choose Open the first time: the app is not signed, so a double-click is refused until you do.\n' >> build/notes.md
   ```

7. **Create the GitHub release with the zip attached.**
   ```
   gh release create "$TAG" "build/AgentPet-$TAG-arm64.zip" --title "$TITLE" --notes-file build/notes.md
   ```
   It prints the release URL.

8. **Install the same build on this Mac.** Quit the running pet from the pawprint menu or:
   ```
   pkill -x AgentPet; sleep 1; pgrep -x AgentPet   # expect no output from pgrep
   open build/AgentPet.app
   ```

## Verification

1. The release exists and carries the asset:
   ```
   gh release view "$TAG" --json tagName,assets,url -q '.tagName, .url, (.assets[] | .name + " " + (.size|tostring))'
   ```
   Expect the tag, the URL, and one asset named `AgentPet-$TAG-arm64.zip` with a size in the low
   megabytes.
2. The asset is what was built:
   ```
   gh release download "$TAG" -p '*.zip' -D "$(mktemp -d)" && echo downloaded
   ```
   followed by comparing its `shasum -a 256` to that of `build/AgentPet-$TAG-arm64.zip`. They match
   or the deploy failed.
3. It works from a downloader's side — a look, not a command: open the release URL in a browser,
   confirm the notes read as the changelog entry, the zip is listed, and the tag page shows the
   source at the pushed commit.
4. The pet on this Mac is the new build: `pgrep -x AgentPet` prints one PID started after step 4's
   build, `ps -o comm= -p "$(pgrep -x AgentPet)"` ends in this checkout's
   `build/AgentPet.app/Contents/MacOS/AgentPet`, and on screen the creature, bubble and a row for
   the Claude Code session driving this runbook are all present.

A release page with no asset, or an asset whose checksum differs from the local zip, is not a
deploy, whatever the page's status says.

## Rollback

Withdraw the release and its tag; the source push stays, since `main` is not rewound for a failed
release.

```
gh release delete "$TAG" --yes
git push --delete origin "$TAG"
git tag -d "$TAG"
```

The tag name can be used again once deleted. A zip someone already downloaded cannot be recalled.
On this Mac, quit the pet; nothing it writes needs undoing, since it persists only window geometry
in `UserDefaults` and never writes to an agent's files.

## Known Failures

None yet — this runbook has not been run.
