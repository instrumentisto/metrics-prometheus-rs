Contribution Guide
==================

🎈 Thanks for your help improving the project! We are so happy to have you!

**No contribution is too small and all contributions are valued.**

1. [Code style](#code-style)




## Code style

**All [Rust] source code must be formatted with [rustfmt] and linted with [Clippy] linter**, customized by project settings ([`.rustfmt.toml`](.rustfmt.toml) and [`.clippy.toml`](.clippy.toml) files).

Additional rules, not handled by [rustfmt] and [Clippy] are described below.


### Attributes

**Attributes** on declarations must be **sorted in alphabetic order**. **Items inside attribute** must be **sorted in alphabetic order** too (in the same manner they're sorted by [rustfmt] inside `use` statement).

#### 👍 Correct example

```rust
#[allow(clippy::mut_mut)]
#[derive(smart_default::SmartDefault, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct User {
    #[serde(default)]
    id: u64,
}
```

#### 🚫 Wrong examples

```rust
#[serde(deny_unknown_fields)]
#[derive(smart_default::SmartDefault, Debug, Deserialize, Serialize)]
#[allow(clippy::mut_mut)]
struct User {
    id: u64,
}
```

```rust
#[derive(Debug, smart_default::SmartDefault, Serialize, Deserialize)]
struct User {
    id: u64,
}
```


### Markdown in docs

It's **recommended to use H1 headers** (`# Header`) in [Rust] docs as this way is widely adopted in [Rust] community. **Blank lines** before headers must be **reduced to a single one**.

**Bold** and _italic_ text should be marked via `**` and `_` accordingly.

Other **code definitions** should be **referred via ```[`Entity`]``` marking** ([intra-doc links][1]).

#### 👍 Correct example

```rust
/// Type of [`User`]'s unique identifier.
/// 
/// # Constraints
/// 
/// - It **must not be zero**.
/// - It _should not_ overflow [`i64::MAX`] due to usage in database.
struct UserId(u64);
```

#### 🚫 Wrong examples

- H2 header is used at the topmost level:

    ```rust
    /// Type of [`User`]'s unique identifier.
    /// 
    /// ## Constraints
    /// 
    /// - It **must not be zero**.
    /// - It _should not_ overflow [`i64::MAX`] due to usage in database.
    struct UserId(u64);
    ```

- Code definition is not referred correctly:

    ```rust
    /// Type of User's unique identifier.
    /// 
    /// # Constraints
    /// 
    /// - It **must not be zero**.
    /// - It _should not_ overflow `i64::MAX` due to usage in database.
    struct UserId(u64);
    ```

- Incorrect bold/italic marking:

    ```rust
    /// Type of [`User`]'s unique identifier.
    /// 
    /// # Constraints
    /// 
    /// - It __must not be zero__.
    /// - It *should not* overflow [`i64::MAX`] due to usage in database.
    struct UserId(u64);
    ```




[Clippy]: https://github.com/rust-lang/rust-clippy
[Rust]: https://www.rust-lang.org
[rustfmt]: https://github.com/rust-lang/rustfmt

[1]: https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html
