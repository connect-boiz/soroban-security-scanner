//! SQL normalization.
//!
//! Collapses a concrete SQL statement into a parameterized "fingerprint" by
//! replacing literals with `?` and squeezing whitespace, so that
//! `SELECT * FROM users WHERE id = 1` and `… id = 2` group as the same logical
//! query for metrics, caching and N+1 detection.

/// Normalizes a SQL statement to a stable fingerprint.
pub fn normalize(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let bytes = sql.as_bytes();
    let mut i = 0;
    let mut last_was_space = false;

    while i < bytes.len() {
        let c = bytes[i] as char;

        // String literal: '...'
        if c == '\'' {
            out.push('?');
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    // Handle escaped '' inside the literal.
                    if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            last_was_space = false;
            continue;
        }

        // Numeric literal.
        if c.is_ascii_digit() {
            out.push('?');
            while i < bytes.len() && ((bytes[i] as char).is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            last_was_space = false;
            continue;
        }

        // Whitespace squeeze.
        if c.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
            i += 1;
            continue;
        }

        // Lowercase keywords/identifiers for stable grouping.
        out.push(c.to_ascii_lowercase());
        last_was_space = false;
        i += 1;
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_become_placeholders() {
        assert_eq!(
            normalize("SELECT * FROM users WHERE id = 1"),
            "select * from users where id = ?"
        );
        assert_eq!(
            normalize("SELECT * FROM users WHERE name = 'alice'"),
            "select * from users where name = ?"
        );
    }

    #[test]
    fn different_literals_share_fingerprint() {
        assert_eq!(
            normalize("SELECT x FROM t WHERE id = 1"),
            normalize("SELECT x FROM t WHERE id = 999")
        );
    }

    #[test]
    fn whitespace_is_squeezed() {
        assert_eq!(normalize("SELECT   a,\n  b\tFROM t"), "select a, b from t");
    }

    #[test]
    fn escaped_quotes_handled() {
        assert_eq!(normalize("WHERE name = 'O''Brien'"), "where name = ?");
    }

    #[test]
    fn decimals_collapse() {
        assert_eq!(normalize("WHERE price > 12.50"), "where price > ?");
    }
}
