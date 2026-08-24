# Second half — closure provenance

How the `second-half.json` wave was selected, and how to rebuild it byte-for-byte.

This file exists because the campaign's original `805`-unit closure was **not**
reproducible: only its `403`-unit first-half prefix was ever committed, and the
`schedule --name ...` seed that produced it was never recorded anywhere in the
repository or the session logs. Reconstructing the held remainder therefore had
to be done from the campaign's stated coverage rule rather than from an
artifact. Do not let that happen again — anything that selects a wave belongs
beside the wave.

## Coverage rule

The campaign wraps "the subset of the public libgit2 API reached by industry
safe wrapper crates". The industry crate is **git2-rs**, checked out at
`/work/git2-rs` at commit `da6c126`.

## Seed

`second-half.seed.txt` — `792` names.

Derived as: every `git_*` identifier appearing in git2-rs's own safe sources
(`git2-rs/src/` and `git2-rs/git2-curl/src/`), intersected with the entities the
oracle publishes for this target (`query types --api-only` ∪
`query symbols --api-only`, macros excluded).

```sh
grep -rhoE '\bgit_[a-zA-Z0-9_]+' /work/git2-rs/src /work/git2-rs/git2-curl/src \
  | sort -u > /tmp/g2.txt
crustify-oracle /work/libgit2 src query types   --api-only | cut -f1 | sort -u >  /tmp/api.txt
crustify-oracle /work/libgit2 src query symbols --api-only \
  | awk -F'\t' '$2!="macro"{print $1}' | sort -u >> /tmp/api.txt
sort -u /tmp/api.txt -o /tmp/api.txt
comm -12 /tmp/g2.txt /tmp/api.txt          # == second-half.seed.txt
```

Two alternatives were measured and rejected as not matching the coverage rule:
seeding from git2-rs's raw binding surface (`libgit2-sys/lib.rs`) yields `496`
remaining units, and closing over the entire published API yields `883`.

## Skip

`second-half.skip.txt` — the `403` units completed in the first half, taken from
`first-half.json`'s `plan_items`. The scheduler does **not** exclude completed
work on its own, so this list is load-bearing: without it the same schedule
returns `1258` units.

## Regenerate

```sh
crustify-oracle /work/libgit2 src schedule \
  --output crustify/campaigns/src/second-half.json \
  --api-headers-only --transitive --max-types 2 \
  --name $(cat crustify/campaigns/src/second-half.seed.txt | tr '\n' ' ') \
  --skip $(cat crustify/campaigns/src/second-half.skip.txt | tr '\n' ' ')
```

Yields `405` units in `42` batches across `8` DAG layers and `19` source files,
dropping `22` lifecycle primitives that the oracle emits through their owning
type. `second-half.units.tsv` is that unit set, resolved and sorted, so the
selection can be diffed without rerunning the oracle.

Batch packing depends on the oracle's `--min-fields` default, which moved from
`10` to `20` in `crustify-oracle` `b3d0a78`. The same `405` units packed into
`48` batches under the old default and `42` under the current one; the unit set
is unaffected.

## Reconciliation with the lost closure

The frozen closure was recorded as `805` units, split `403` / `402`. This
reconstruction yields `405` against that `402`. The difference is expected and
benign: the oracle now drops lifecycle primitives that gained ownership findings
during the first half, so the two schedules were taken against different
ownership-store states. The reconstruction covers every one of the `403`
completed units and overlaps none of them, which is the property that actually
matters.
