
// Auto-generated directive decoder mappings
// Extracted from PARSE.Z80 t_XXX directive patterns

pub struct DirectiveInfo {
    pub keyword: &'static str,
    pub command_code: u8,
    pub has_expression: bool,
}

pub const DIRECTIVE_MAP: &[(u8, DirectiveInfo)] = &[
    (0x17, DirectiveInfo {
        keyword: "IMPORT",
        command_code: 0x17,
        has_expression: true,
    }),

];

pub fn get_directive_info(command_byte: u8) -> Option<&'static DirectiveInfo> {
    DIRECTIVE_MAP.iter()
        .find(|(code, _)| *code == command_byte)
        .map(|(_, info)| info)
}
