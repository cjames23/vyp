# Transitive Conflicts

**Transitive conflict declarations** are vyp's signature feature. They solve the diamond dependency problem in a way that existing tools cannot: by allowing the "middle of the diamond" to propagate conflict resolution decisions to consumers.

## The Diamond Dependency Problem

Consider this classic scenario:

!!! abstract "Diamond dependency diagram"
    ```
              Your App
                 │
         ┌──────┴──────┐
         │             │
      Library A    Library B
         │             │
         └──────┬──────┘
                │
              LibX
    ```

- **Library A** depends on `LibX>=2`
- **Library B** depends on `LibX<2`

There is no single version of LibX that satisfies both constraints. The resolver **fails** — it cannot find a solution. pip, uv, and other resolvers report an error and stop. The *consumer* (Your App) must diagnose the failure, figure out which version actually works, and manually add an override. But what if **Library A** already knows the right answer?

## Why Existing Tools Can't Propagate

pip and uv treat each dependency declaration in isolation. When A and B conflict on LibX:

- The resolver sees two incompatible constraints and **fails** — there is no version of LibX that satisfies both.
- It has no way to know that A's maintainers have already resolved this conflict for their users.
- The consumer must diagnose the error, determine which constraint can safely be overridden, and manually add a workaround — or give up and drop a dependency.

!!! failure "Limitation"
    Existing tools cannot propagate conflict resolution from a library to its consumers. The "middle of the diamond" is invisible to the resolver.

## How vyp's Transitive Conflicts Work

vyp allows packages to **declare** conflicts and mark them as **transitive**. When a package declares a transitive conflict, that declaration is inherited by every package that depends on it. The resolver can then use this information to fork resolution or apply overrides.

### How vyp Solves This

The key mechanism is `[[tool.vyp.overrides]]` with `transitive = true`. When a library author knows the answer to a version conflict, they declare an override and export it. Consumers inherit the override automatically.

Users interact with overrides via `[[tool.vyp.overrides]]` in `pyproject.toml`. Transitive overrides propagate to consumers via the `vyp-overrides.toml` export file.

### Example: Library A and Library B

Library A (which requires LibX>=2) can add a transitive override:

```toml
# In Library A's pyproject.toml
[[tool.vyp.overrides]]
package = "libx"
constraint = ">=2"
transitive = true
reason = "Our code requires the v2 API"
```

Library A then exports this for consumers:

```bash
vyp override export
```

When Your App depends on both A and B:

1. vyp loads A's exported `vyp-overrides.toml` and sees the transitive override on LibX.
2. The override propagates to Your App (A's consumer).
3. The resolver applies the override, restricting LibX to `>=2`, resolving the conflict in favor of A.
4. Your App's maintainer doesn't need to diagnose anything — the decision was already made upstream.

!!! success "Key insight"
    `transitive = true` means: "This conflict resolution applies not just to me, but to anyone who depends on me."

## Propagation and Inheritance

When a package declares a transitive conflict, it is **inherited** by its consumers. The inheritance chain is tracked:

- **origin**: The package that originally declared the conflict.
- **propagation_path**: The chain of packages through which the conflict was inherited.

```
Library A declares conflict (origin: A)
    → Your App inherits (propagation_path: [A])
    → Meta-App depends on Your App (propagation_path: [A, your-app])
```

## The vyp-overrides.toml Export File

Libraries can **export** their transitive overrides so that consumers can load them without re-resolving.

```bash
vyp override export --output vyp-overrides.toml
```

This produces a `vyp-overrides.toml` file that downstream consumers can use. The file contains transitive dependency overrides (package, constraint, reason, etc.).

!!! abstract "vyp-overrides.toml structure"
    ```toml
    overrides-version = "4.0"
    created-by = "vyp 0.1.0"
    package = "library-a"
    package_version = "1.0.0"

    [[overrides]]
    package = "libx"
    constraint = ">=2"
    transitive = true
    reason = "Resolved in favor of v2"
    ```

When a consumer adds Library A as a dependency, vyp can load `vyp-overrides.toml` from A's distribution and apply those overrides during resolution.

## Comparison with uv and pip

| Aspect | pip / uv | vyp |
|--------|----------|-----|
| **Conflict visibility** | Only direct constraints | Transitive declarations propagate |
| **Who resolves** | Consumer only | Library can pre-resolve and propagate |
| **Override scope** | Project-local | Can be transitive via vyp-overrides.toml |
| **Diamond handling** | Fail or manual override | Fork resolution, inherit decisions |

!!! tip "When to use transitive conflicts"
    Use transitive overrides when your library has resolved a known conflict (e.g., LibX version) and you want consumers to inherit that resolution automatically. Export `vyp-overrides.toml` and ship it with your package.
