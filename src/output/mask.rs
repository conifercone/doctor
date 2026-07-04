use regex::Regex;
use std::sync::LazyLock;

static PASSWORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(password|passwd|pwd|secret|token|api[_-]?key|credential)\s*[:=]\s*\S+")
        .unwrap()
});

static JWT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"eyJ[a-zA-Z0-9_-]+\.eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+").unwrap()
});

static CONNECTION_STRING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(jdbc|mongodb|redis|postgresql|mysql)://[^\s]+").unwrap()
});

static PRIVATE_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap()
});

/// Mask sensitive values in a string, replacing them with `***`.
pub fn mask_sensitive(text: &str) -> String {
    let mut result = text.to_string();
    result = PASSWORD_RE
        .replace_all(&result, "$1=***")
        .to_string();
    result = JWT_RE.replace_all(&result, "***JWT***").to_string();
    result = CONNECTION_STRING_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let scheme = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            format!("{scheme}://***")
        })
        .to_string();
    result = PRIVATE_KEY_RE
        .replace_all(&result, "***PRIVATE KEY***")
        .to_string();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        let result = mask_sensitive("spring.datasource.password=mysecret123");
        assert!(result.contains("***"));
        assert!(!result.contains("mysecret123"));
    }

    #[test]
    fn test_mask_jwt() {
        let result = mask_sensitive("Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U");
        assert!(result.contains("***JWT***"));
    }

    #[test]
    fn test_mask_connection_string() {
        let result =
            mask_sensitive("jdbc:mysql://localhost:3306/mydb?user=root&password=secret");
        assert!(result.contains("***"));
        assert!(!result.contains("secret"));
    }

    #[test]
    fn test_no_false_positives() {
        let input = "This is a normal sentence without any secrets";
        assert_eq!(mask_sensitive(input), input);
    }
}
