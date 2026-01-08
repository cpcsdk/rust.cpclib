#[cfg(test)]
mod tests {
    use cpclib_csl::{parse_csl_with_rich_errors, CslScriptBuilder, CslInstruction};

    #[test]
    fn test_parse_v10_with_v12_feature_fails() {
        let script = "csl_version 1.0\nkeyboard_write 255,255,255,255,255,255,239,255,255,255\n";
        let result = parse_csl_with_rich_errors(script, None);
        assert!(result.is_err(), "v1.0 with v1.2 feature should fail");
    }

    #[test]
    fn test_parse_no_version_with_v12_feature_succeeds() {
        let script = "keyboard_write 255,255,255,255,255,255,239,255,255,255\n";
        let result = parse_csl_with_rich_errors(script, None);
        assert!(result.is_ok(), "No version (defaults to latest) with v1.2 feature should succeed");
    }

    #[test]
    fn test_parse_v12_with_v12_feature_succeeds() {
        let script = "csl_version 1.2\nkeyboard_write 255,255,255,255,255,255,239,255,255,255\n";
        let result = parse_csl_with_rich_errors(script, None);
        assert!(result.is_ok(), "v1.2 with v1.2 feature should succeed");
    }
}
