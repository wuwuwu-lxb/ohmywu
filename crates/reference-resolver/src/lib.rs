use ohmywu_domain::ResolvedReference;

pub fn resolve_references(input: &str) -> Vec<ResolvedReference> {
    input
        .split_whitespace()
        .filter_map(|token| parse_reference(token))
        .collect()
}

fn parse_reference(token: &str) -> Option<ResolvedReference> {
    let cleaned = token.trim_matches(|c: char| {
        matches!(c, ',' | '.' | '!' | '?' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}')
    });

    if let Some(name) = cleaned.strip_prefix('@') {
        return Some(ResolvedReference {
            raw: cleaned.to_string(),
            kind: "agent".to_string(),
            name: name.to_string(),
            exists: false,
        });
    }

    if let Some(name) = cleaned.strip_prefix('/') {
        return Some(ResolvedReference {
            raw: cleaned.to_string(),
            kind: "action".to_string(),
            name: name.to_string(),
            exists: false,
        });
    }

    None
}
