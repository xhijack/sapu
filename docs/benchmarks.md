# Benchmarks (developer machine, 2026-10-16)

MacBook (Apple Silicon, APFS SSD), release build of `sapu-cli`, measured with `/usr/bin/time -l`. Numbers are indicative, not a lab result; they exist to track regressions and to answer the PRD acceptance questions.

## Scan (`sapu scan ~`)

| Variant | Files | Folders | Wall time | Peak RSS |
|---|---|---|---|---|
| Serial stat in the consumer loop | 5,486,242 | 860,213 | 258.6 s | 2.03 GB |
| Stat in jwalk worker threads (current) | 5,486,242 | 860,213 | 111.8 s | 2.04 GB |

Observations: `sys` time dominates (metadata calls). Memory is about 370 bytes per file because every entry keeps its full path; interning path components would roughly halve it. 148 folders were unreadable without Full Disk Access and were reported, not fatal.

## Duplicates (`sapu dupes ~/Documents/development`)

| Files seen | Candidate groups | Duplicate files | Recoverable | Wall time | Peak RSS |
|---|---|---|---|---|---|
| 689,302 | 88,520 | 364,157 | 37.1 GB | 549 s | 893 MB |

Observations: this tree is the worst case for a dedupe (hundreds of `node_modules` with identical packages), so most files survive the size and partial-hash stages and need a full BLAKE3 pass. Memory stays under 1 GB for ~700 k files, which meets the "hundreds of thousands of files with bounded memory" requirement; wall time is dominated by reading 37+ GB. Skipping `node_modules`-like folders by default in the UI would make the common case much faster.

## ETA accuracy

Unit test `eta_at_half_is_within_thirty_percent` simulates a second scan with ±25 % throughput wobble and asserts the smoothed estimate at the 50 % mark is within ±30 % of the real remaining time (PRD §10.1 acceptance). A field measurement on real disks is still to be done.
