//! Compile-time guard: every CHECK constraint on a lifecycle-state column in the
//! migrations must allow exactly the states in `PostState::ALL`. SQL constraints
//! are invisible to rustc (and to sqlx's `query!`, which checks columns and types
//! but not allowed values), so this build script compares them and fails the
//! build on any mismatch — e.g. a new `PostState` variant without a migration.

use std::{collections::BTreeSet, fs, path::PathBuf};

use thanksgivings_core::PostState;

/// Columns that hold a `PostState`: `posts.state` and the transition log's
/// `post_transitions.from_state` / `to_state`.
const STATE_COLUMNS: &[&str] = &["state", "from_state", "to_state"];

fn main() {
    println!("cargo:rerun-if-changed=migrations");

    let mut files: Vec<PathBuf> = fs::read_dir("migrations")
        .expect("read migrations/")
        .map(|e| e.expect("migration entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "sql"))
        .collect();
    files.sort();

    let sqls: Vec<(PathBuf, String)> = files.into_iter().map(|p| {
        let sql = fs::read_to_string(&p).expect("read migration");
        (p, sql.split_whitespace().collect::<Vec<_>>().join(" ")) // tolerate line breaks
    }).collect();

    let expected: BTreeSet<String> = PostState::ALL.iter().map(|s| s.as_str().to_string()).collect();

    for col in STATE_COLUMNS {
        let check = format!("CHECK ({col} IN (");
        // Migrations apply in filename order, so the last one that declares the
        // constraint is the one in force.
        let Some((file, allowed)) = sqls.iter().rev()
            .find_map(|(p, sql)| check_values(sql, &check).map(|a| (p, a)))
        else {
            panic!("no migration declares `{check}…))`");
        };

        if allowed != expected {
            let missing: Vec<_> = expected.difference(&allowed).collect();
            let extra:   Vec<_> = allowed.difference(&expected).collect();
            panic!(
                "\n\n`{col}` CHECK constraint (last set in {}) is out of sync with PostState::ALL.\n\
                 \x20 in Rust but not allowed by SQL: {missing:?}\n\
                 \x20 allowed by SQL but not in Rust: {extra:?}\n\
                 Add a migration that rebuilds the table with the new CHECK list.\n",
                file.display(),
            );
        }
    }
}

/// The quoted values in the last `check` constraint of `sql`, if any.
fn check_values(sql: &str, check: &str) -> Option<BTreeSet<String>> {
    let start = sql.rfind(check)? + check.len();
    let list  = &sql[start..start + sql[start..].find(')')?];
    Some(
        list.split(',')
            .map(|v| v.trim().trim_matches('\'').to_string())
            .collect(),
    )
}
