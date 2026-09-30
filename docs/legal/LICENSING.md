# Licensing

This document records the actual licensing history of the CrossPort repository.
It states facts about this repository only; it is not legal advice.

## Current license

The current development line of CrossPort is licensed under the **Apache License
2.0** (SPDX identifier `Apache-2.0`). The full text is in [`LICENSE`](../../LICENSE).

`NOTICE` at the repository root carries the copyright attribution that the
Apache License 2.0 expects.

## Historical releases

| Release | License at the time of publication |
| --- | --- |
| CrossPort 1.0.0 | MIT |
| CrossPort 1.1.0 | MIT |
| CrossPort 1.1.1 | MIT |
| CrossPort 1.1.2 | Apache-2.0 |
| Current development line | Apache-2.0 |

CrossPort up to and including **1.1.1** was released under the MIT License.
Those releases had already been published under MIT when this repository
migrated, and they are not being relicensed.

Concretely:

- The tags `v1.0.0`, `v1.1.0`, and `v1.1.1` are unchanged, and the source at
  each of them still carries the MIT license text that was in force when it was
  published.
- The installers published with those releases were built from that source and
  therefore still contain the MIT license text. They are not being rebuilt.
- Retroactive relicensing of an already-published release requires the
  permission of everyone who contributed to it; that has not been obtained, so
  it is not claimed here.

The migration applies to the repository's current development line: the code on
`main` after the 1.1.1 release. CrossPort **1.1.2** is the first release taken
from that line, and it is Apache-2.0. Every release from it is.

## Documentation and documents that still say MIT

Some documents describe the MIT era and are intentionally left as they were:

- `docs/release/RELEASE_NOTES_1.1.1.md` and the other per-release documents
  record what was true for the release they describe. They are historical
  records, not statements about the current repository license.
- The published GitHub release for `v1.1.1` is unchanged; its notes describe
  that release as it was shipped.

Where a document describes the *current* repository, it states Apache-2.0.

## Third-party software

CrossPort depends on third-party libraries, and bundles some of them. They are
**not** relicensed by this migration and remain under their own terms.
`docs/legal/THIRD_PARTY_LICENSES.md` records where those terms live
(`package.json` / `pnpm-lock.yaml` for JavaScript, `Cargo.toml` / `Cargo.lock`
for Rust) and how to audit them. No third-party component is presented as being
owned by P4inz Interactive Labs.

## Brand assets

The CrossPort name, wordmark, and logo are brand assets. They are separate from
the software license: the Apache License 2.0 grants rights to the source code,
and section 6 of that license states it does not grant permission to use trade
names, trademarks, service marks, or product names — except for reasonable and
customary use in describing the origin of the work or reproducing the `NOTICE`
file.

No trademark registration is claimed for CrossPort.

## Contributions

Unless you state otherwise, a contribution deliberately submitted for inclusion
in CrossPort is accepted under the terms of the license in force for the current
development line (Apache-2.0), as set out in section 5 of that license. See
[`CONTRIBUTING.md`](../../CONTRIBUTING.md).
