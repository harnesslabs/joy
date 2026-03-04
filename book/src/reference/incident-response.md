# Incident Response Runbook

This runbook covers high-priority operational incidents for `joy` release consumers.

## Severity Guidelines

- `SEV-1`: active supply-chain risk (malicious dependency, compromised release artifact)
- `SEV-2`: broken release affecting installs/builds broadly
- `SEV-3`: localized issue with workaround

## Scenario A: Compromised Dependency

1. Freeze releases immediately.
2. Identify affected versions and package IDs.
3. Remove/disable affected dependency source in registry metadata.
4. Publish a patched release and changelog/security advisory.
5. Notify users with explicit upgrade/rollback instructions.

## Scenario B: Bad Release Artifact

1. Mark the release as yanked in release notes.
2. Repoint package-manager channels (Homebrew/Scoop metadata) to the prior safe release.
3. Publish fixed artifacts under a new version tag.
4. Verify checksums/signatures and smoke test install paths.

## Rollback Drill (Per Release)

Perform a rollback drill once per release cycle:

1. Simulate a bad release in a staging repository.
2. Execute package-manager metadata rollback.
3. Confirm install commands resolve to the prior known-good version.
4. Record elapsed time and gaps in the release notes archive.

## Communication Checklist

- Open internal incident tracker entry.
- Publish public status update within 24 hours for SEV-1/SEV-2.
- Update `SECURITY.md` if policy/process changes were required.
