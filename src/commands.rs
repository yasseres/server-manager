// =============================================================================
// COMMAND SCRIPTS
// =============================================================================
// This file contains command script functions that return shell commands.
// Each function returns a command string to be executed via SSH.
// =============================================================================

/// Simple test command - returns hostname
pub fn test_cmd() -> &'static str {
    "hostname"
}

/// Get system info for Linux servers
pub fn info_cmd_linux() -> &'static str {
    "echo 'OS:' $(cat /etc/os-release 2>/dev/null | grep PRETTY_NAME | cut -d'=' -f2 | tr -d '\"') && \
     echo 'Kernel:' $(uname -r) && \
     echo 'Uptime:' $(uptime -p 2>/dev/null || uptime) && \
     echo 'CPU:' $(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d':' -f2 | xargs || echo 'Unknown') && \
     echo 'Memory:' $(free -h 2>/dev/null | awk '/^Mem:/ {print $3 \"/\" $2}' || echo 'Unknown') && \
     echo 'Disk:' $(df -h / 2>/dev/null | awk 'NR==2 {print $3 \"/\" $2 \" (\" $5 \" used)\"}' || echo 'Unknown')"
}

/// Get system info for Windows servers
pub fn info_cmd_windows() -> &'static str {
    r#"powershell -Command "Write-Host '=== Windows Info ==='; Write-Host \"Hostname: $env:COMPUTERNAME\"; $os = Get-CimInstance Win32_OperatingSystem; Write-Host \"OS: $($os.Caption)\"; Write-Host \"Build: $($os.BuildNumber)\"; Write-Host \"Uptime: $((Get-Date) - $os.LastBootUpTime)\"""#
}

/// Linux update command - apt update && upgrade
pub fn update_linux_cmd() -> &'static str {
    "echo '>>> Running: sudo apt update' && \
     sudo apt update && \
     echo '' && \
     echo '>>> Running: sudo apt upgrade -y' && \
     sudo DEBIAN_FRONTEND=noninteractive apt upgrade -y && \
     echo '' && \
     echo '>>> Checking reboot status' && \
     if [ -f /var/run/reboot-required ]; then \
         echo 'REBOOT REQUIRED'; \
     else \
         echo 'No reboot needed'; \
     fi"
}

/// Windows update command using PSWindowsUpdate module via scheduled task
/// Uses base64 encoded script to avoid quote/newline issues over SSH
pub fn update_windows_cmd() -> &'static str {
    // The script is base64 encoded to avoid all escaping issues
    // Decoded script does: check for updates, install via scheduled task as SYSTEM, monitor progress
    r#"powershell -ExecutionPolicy Bypass -Command "[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('JEVycm9yQWN0aW9uUHJlZmVyZW5jZT0nQ29udGludWUnCldyaXRlLUhvc3QgJz09PSBXSU5ET1dTIFVQREFURSA9PT0nCldyaXRlLUhvc3QgJycKV3JpdGUtSG9zdCAnPj4+IFN5c3RlbSBJbmZvJwokb3M9R2V0LUNpbUluc3RhbmNlIFdpbjMyX09wZXJhdGluZ1N5c3RlbQpXcml0ZS1Ib3N0ICJPUzogJCgkb3MuQ2FwdGlvbikgQnVpbGQ6ICQoJG9zLkJ1aWxkTnVtYmVyKSIKV3JpdGUtSG9zdCAnJwoKJG1vZHVsZT1HZXQtTW9kdWxlIC1MaXN0QXZhaWxhYmxlIC1OYW1lIFBTV2luZG93c1VwZGF0ZQppZigtbm90ICRtb2R1bGUpewogICAgV3JpdGUtSG9zdCAnPj4+IEluc3RhbGxpbmcgUFNXaW5kb3dzVXBkYXRlLi4uJwogICAgdHJ5ewogICAgICAgIEluc3RhbGwtUGFja2FnZVByb3ZpZGVyIC1OYW1lIE51R2V0IC1Gb3JjZSAtRUEgU2lsZW50bHlDb250aW51ZXxPdXQtTnVsbAogICAgICAgIEluc3RhbGwtTW9kdWxlIC1OYW1lIFBTV2luZG93c1VwZGF0ZSAtRm9yY2UgLUFsbG93Q2xvYmJlciAtU2NvcGUgQWxsVXNlcnMKICAgICAgICBXcml0ZS1Ib3N0ICc+Pj4gSW5zdGFsbGVkJwogICAgfWNhdGNoe1dyaXRlLUhvc3QgIkVSUk9SOiAkKCRfLkV4Y2VwdGlvbi5NZXNzYWdlKSI7ZXhpdCAxfQp9CkltcG9ydC1Nb2R1bGUgUFNXaW5kb3dzVXBkYXRlIC1Gb3JjZQoKV3JpdGUtSG9zdCAnPj4+IENoZWNraW5nIGZvciB1cGRhdGVzLi4uJwokdXBkYXRlcz1HZXQtV2luZG93c1VwZGF0ZSAtQWNjZXB0QWxsCmlmKCR1cGRhdGVzLkNvdW50IC1lcSAwKXtXcml0ZS1Ib3N0ICc+Pj4gVXAgdG8gZGF0ZSEnO2V4aXQgMH0KCldyaXRlLUhvc3QgIkZvdW5kICQoJHVwZGF0ZXMuQ291bnQpIHVwZGF0ZShzKToiCiR1cGRhdGVzfEZvckVhY2gtT2JqZWN0e1dyaXRlLUhvc3QgIiAgLSAkKCRfLlRpdGxlKSJ9CldyaXRlLUhvc3QgJycKCiRoaXN0QmVmb3JlPShHZXQtV1VIaXN0b3J5fE1lYXN1cmUtT2JqZWN0KS5Db3VudAokdGFzaz0iU01VcGRhdGVfJChHZXQtUmFuZG9tKSIKJHNjcmlwdD0nSW1wb3J0LU1vZHVsZSBQU1dpbmRvd3NVcGRhdGUgLUZvcmNlO0luc3RhbGwtV2luZG93c1VwZGF0ZSAtQWNjZXB0QWxsIC1JZ25vcmVSZWJvb3QgLUNvbmZpcm06JGZhbHNlJwokZW5jPVtDb252ZXJ0XTo6VG9CYXNlNjRTdHJpbmcoW1RleHQuRW5jb2RpbmddOjpVbmljb2RlLkdldEJ5dGVzKCRzY3JpcHQpKQoKV3JpdGUtSG9zdCAnPj4+IEluc3RhbGxpbmcgYXMgU1lTVEVNLi4uJwokYWN0aW9uPU5ldy1TY2hlZHVsZWRUYXNrQWN0aW9uIC1FeGVjdXRlICdwb3dlcnNoZWxsLmV4ZScgLUFyZ3VtZW50ICItRW5jb2RlZENvbW1hbmQgJGVuYyIKJHByaW5jaXBhbD1OZXctU2NoZWR1bGVkVGFza1ByaW5jaXBhbCAtVXNlcklkICdTWVNURU0nIC1Mb2dvblR5cGUgU2VydmljZUFjY291bnQgLVJ1bkxldmVsIEhpZ2hlc3QKUmVnaXN0ZXItU2NoZWR1bGVkVGFzayAtVGFza05hbWUgJHRhc2sgLUFjdGlvbiAkYWN0aW9uIC1QcmluY2lwYWwgJHByaW5jaXBhbCAtRm9yY2V8T3V0LU51bGwKU3RhcnQtU2NoZWR1bGVkVGFzayAtVGFza05hbWUgJHRhc2sKCiRlbGFwc2VkPTAKd2hpbGUoJGVsYXBzZWQgLWx0IDE4MDApewogICAgU3RhcnQtU2xlZXAgLVNlY29uZHMgMTUKICAgICRlbGFwc2VkKz0xNQogICAgJHQ9R2V0LVNjaGVkdWxlZFRhc2sgLVRhc2tOYW1lICR0YXNrIC1FQSBTaWxlbnRseUNvbnRpbnVlCiAgICAkaGlzdD1HZXQtV1VIaXN0b3J5fFNlbGVjdC1PYmplY3QgLUZpcnN0IDEwCiAgICAkbmV3PSgkaGlzdHxNZWFzdXJlLU9iamVjdCkuQ291bnQKICAgIGlmKCRuZXcgLWd0ICRoaXN0QmVmb3JlKXsKICAgICAgICAkaGlzdHxTZWxlY3QtT2JqZWN0IC1GaXJzdCAoJG5ldy0kaGlzdEJlZm9yZSl8Rm9yRWFjaC1PYmplY3R7CiAgICAgICAgICAgICRyPWlmKCRfLlJlc3VsdCAtZXEgJ1N1Y2NlZWRlZCcpeydbT0tdJ31lbHNlaWYoJF8uUmVzdWx0IC1lcSAnRmFpbGVkJyl7J1tGQUlMXSd9ZWxzZXsiWyQoJF8uUmVzdWx0KV0ifQogICAgICAgICAgICBXcml0ZS1Ib3N0ICIgICRyICQoJF8uVGl0bGUpIgogICAgICAgIH0KICAgICAgICAkaGlzdEJlZm9yZT0kbmV3CiAgICB9CiAgICBpZigkdC5TdGF0ZSAtZXEgJ1JlYWR5Jyl7V3JpdGUtSG9zdCAiPj4+IERvbmUgKCR7ZWxhcHNlZH1zKSI7YnJlYWt9CiAgICBpZigkZWxhcHNlZCAlIDYwIC1lcSAwKXtXcml0ZS1Ib3N0ICI+Pj4gV29ya2luZy4uLiAoJHtlbGFwc2VkfXMpIn0KfQpVbnJlZ2lzdGVyLVNjaGVkdWxlZFRhc2sgLVRhc2tOYW1lICR0YXNrIC1Db25maXJtOiRmYWxzZSAtRUEgU2lsZW50bHlDb250aW51ZQoKV3JpdGUtSG9zdCAnJwpXcml0ZS1Ib3N0ICc+Pj4gUmVjZW50IEhpc3Rvcnk6JwpHZXQtV1VIaXN0b3J5fFNlbGVjdC1PYmplY3QgLUZpcnN0IDV8Rm9yRWFjaC1PYmplY3R7CiAgICAkcj1pZigkXy5SZXN1bHQgLWVxICdTdWNjZWVkZWQnKXsnW09LXSd9ZWxzZXsiWyQoJF8uUmVzdWx0KV0ifQogICAgV3JpdGUtSG9zdCAiICAkciAkKCRfLlRpdGxlKSIKfQoKJHJlYm9vdD1UZXN0LVBhdGggJ0hLTE06XFNPRlRXQVJFXE1pY3Jvc29mdFxXaW5kb3dzXEN1cnJlbnRWZXJzaW9uXFdpbmRvd3NVcGRhdGVcQXV0byBVcGRhdGVcUmVib290UmVxdWlyZWQnCldyaXRlLUhvc3QgJycKaWYoJHJlYm9vdCl7V3JpdGUtSG9zdCAnKioqIFJFQk9PVCBSRVFVSVJFRCAqKionfWVsc2V7V3JpdGUtSG9zdCAnTm8gcmVib290IG5lZWRlZCd9CldyaXRlLUhvc3QgJz09PSBDT01QTEVURSA9PT0n'))|Invoke-Expression""#
}


// =============================================================================
// TESTS
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_not_empty() {
        assert!(!test_cmd().is_empty());
        assert!(!info_cmd_linux().is_empty());
        assert!(!info_cmd_windows().is_empty());
        assert!(!update_linux_cmd().is_empty());
        assert!(!update_windows_cmd().is_empty());
    }

    #[test]
    fn test_windows_cmd_uses_powershell() {
        assert!(update_windows_cmd().starts_with("powershell"));
    }

    #[test]
    fn test_linux_cmd_uses_apt() {
        assert!(update_linux_cmd().contains("apt"));
    }
}
