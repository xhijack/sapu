# Rule files

One rule per TOML file. `common/` holds rules valid on every OS (external commands); `macos/`, `linux/` and `windows/` hold path-based rules for that OS. Files are embedded into the binary at build time; the user's own rules in `<config>/sapu/rules/` are loaded on top and may override an embedded rule with the same `id`.

```toml
[rule]
id = "macos-browser-caches"               # kebab-case, unique across all files
name = { id = "Cache browser", en = "Browser caches" }
description = { id = "…", en = "…" }      # plain words, both languages required
risk = "green"                            # green | yellow (red never comes from rules)
profile = "general"                       # general | developer
group = "browser"                         # browser, system_temp, logs, thumbnails, trash,
                                          # installers, docker, package_cache, xcode, user
permanent = false                         # true only for items already in the Trash;
                                          # permanent rules must be yellow

[[match]]                                 # one or more
path = "~/Library/Caches/Google/Chrome"   # ~ and ${ENV_VAR}; globs * ? [] allowed
kind = "dir_contents"                     # dir_contents | dir | files | dir_name
min_age_days = 0                          # only items whose mtime is at least this old

# dir_name: bounded search for folders by name (never descends into a hit)
# kind = "dir_name"; name = "node_modules"; roots = ["~"]; max_depth = 6
# unused_days = 30; skip = ["Library", ".*"]

[command]                                 # instead of [[match]]: an official tool does the work
program = "docker"
args = ["image", "prune", "-f"]
requires = "docker"                       # binary that must exist on PATH
estimate = ["docker", "system", "df", "--format", "{{.Type}}\t{{.Reclaimable}}"]
estimate_filter = "Images"                # only count output lines containing this
confirm_separately = false                # true asks the user once more (e.g. volumes)
```

Validation at load rejects: non-kebab ids, missing translations, both `[[match]]` and `[command]`, green `permanent` rules, `dir_name` without `name`/`roots`, path kinds without `path`. Tests in `crates/sapu-core/src/rules/load.rs` additionally prove that no rule resolves into a protected path and that each rule finds only files under its own paths.
