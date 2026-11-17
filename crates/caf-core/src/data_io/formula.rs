//! Formula injection protection for spreadsheet and CSV exports.

/// Neutralizes potential formula injection in cell content per OWASP recommendations.
/// Any string beginning with `=`, `+`, `-`, `@`, `\t`, or `\r` is prepended with `'`.
pub fn neutralize_cell(val: &str) -> String {
    let trimmed = val.trim_start();
    if trimmed.starts_with('=')
        || trimmed.starts_with('+')
        || trimmed.starts_with('-')
        || trimmed.starts_with('@')
        || trimmed.starts_with('\t')
        || trimmed.starts_with('\r')
    {
        format!("'{val}")
    } else {
        val.to_string()
    }
}

/// Strips the neutralizing leading single quote if present before formula-like characters.
pub fn unneutralize_cell(val: &str) -> &str {
    if let Some(stripped) = val.strip_prefix('\'') {
        let trimmed = stripped.trim_start();
        if trimmed.starts_with('=')
            || trimmed.starts_with('+')
            || trimmed.starts_with('-')
            || trimmed.starts_with('@')
            || trimmed.starts_with('\t')
            || trimmed.starts_with('\r')
        {
            return stripped;
        }
    }
    val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neutralize_formula_prefixes() {
        assert_eq!(neutralize_cell("=SUM(A1:A10)"), "'=SUM(A1:A10)");
        assert_eq!(neutralize_cell("+cmd|' /C calc'!A0"), "'+cmd|' /C calc'!A0");
        assert_eq!(neutralize_cell("-5+2"), "'-5+2");
        assert_eq!(neutralize_cell("@SUM(1,2)"), "'@SUM(1,2)");
        assert_eq!(neutralize_cell("  =1+1"), "'  =1+1");
        assert_eq!(neutralize_cell("regular text"), "regular text");
        assert_eq!(neutralize_cell("12345"), "12345");
    }

    #[test]
    fn test_unneutralize_roundtrip() {
        let raw = "=SUM(1, 2)";
        let neutralized = neutralize_cell(raw);
        assert_eq!(neutralized, "'=SUM(1, 2)");
        let restored = unneutralize_cell(&neutralized);
        assert_eq!(restored, raw);

        let normal = "Normal Text";
        assert_eq!(unneutralize_cell(normal), normal);
    }
}
