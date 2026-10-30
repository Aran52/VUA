# Library migration v0.1

> Document version: 0.1.0
> Status: Candidate
> Updated: 2026-10-30
> Owner: AMF

Grouped local intake for the library's migration entry (freeze row F4 / slice 46). The player
reviews a local collection, confirms which files form each material, and imports the confirmed
groups as library entries. The copy, verification, ZIP expansion and storage are the existing
intake's; this face only adds explicit, named file selections and their re-validation.

## Method

| Method | Kind | Params | Result |
| --- | --- | --- | --- |
| `library.importGroups` | Command | `schemaVersion: "0.1"`, `groups`, optional `autoGenerate`, `commandId` | `{schemaVersion:"0.1", operation:"library.importGroups", taskId, correlationId}` |

A group is a named material with its files made explicit:

```json
{
  "name": "Meiyun coat",
  "root": "D:\\Materials",
  "files": [
    { "path": "Meiyun Coat/Coat.unitypackage", "bytes": 44040192, "modifiedMs": 1760000000000 }
  ]
}
```

`name` becomes the entry's display name: 1–80 characters, no separators, reserved characters
(`< > : " / \ | ? *`) or trailing dot or space. `root` is an absolute local path. A file's `path`
is `/`-separated, relative to `root`, with no leading separator, drive or `..` segment; the two
values come from the review, and the provider checks them again.

Limits: 1–200 groups, at most 20,000 files in total. Group names are unique ignoring ASCII case.
A file (the same `root` and `path`) may appear in only one group.

## Validation before any copy

The command is refused, with no file copied, when:

| Condition | Code | Message key |
| --- | --- | --- |
| Malformed params, unsafe name or path, limits exceeded, root inside the library storage | `vua.migration.invalidParams` | `errors.migration.invalidParams` |
| The same file in two groups | `vua.migration.overlap` | `errors.migration.overlap` |
| A declared file is missing, is a link, is no longer an ordinary file, or its size or modification time differs from the review | `vua.migration.stale` | `errors.migration.stale` |

Freshness covers moves, removals, type changes, size changes and edits that change the
modification time. A same-size edit that preserves the modification time is outside this
structural review: the copy's own SHA-256 records what was actually stored.

The reviewed root and the library storage may not contain one another, so a selected ancestor of
the warehouse is refused instead of copied into itself.

## Execution

Each group becomes one library entry named after the group, keeping each file's reviewed relative
path. Copy, size verification, artifact recording and ZIP expansion are the single-folder intake's
own paths (`WarehouseImporter`). Groups are independent: a group that fails is reported and the
rest still import, so completed and unfinished work stay separable. The task reports progress as
`library.importGroups` with the group name, files and bytes; cancellation stops at a group
boundary, and entries already landed stay.

The task result carries `groupsRequested`, `groupsImported`, `reports` (one per landed group) and
`failures` (group name, typed code, message).

## Migration manifest

The reviewed collection is one versioned document, `kind: "vua.migration"`, `formatVersion`
`"0.1"`: `reviewedRoots` (path and observed file count), `materials` (the same groups the command
takes) and `excluded` (root and relative path of every observed file the player excluded). Files
in neither `materials` nor `excluded` are unresolved; the planner blocks the import until every
observed file has one disposition, and the UI copies the manifest for the player.

Slice 47's agent will write this shape; VUA validates everything it executes again.

## Limits of this face

The review is structural: it reads directory listings, sizes and modification times, and never
hashes source content, extracts archives or follows links. A truncated read (1,000 immediate
entries) or an unreadable directory is reported as unresolved rather than counted as absent. The
copy stores and expands archives inside each confirmed material exactly as single-item intake
does.

## Document changelog

- 0.1.0 (2026-10-30): first grouped-intake face — named material groups with explicit files,
  pre-copy freshness, overlap and recursion validation, per-group task outcome, and the
  `vua.migration` v0.1 manifest.
