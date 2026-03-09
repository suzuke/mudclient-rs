use std::process::Command;

fn main() {
    // 從 git tag 取得版本資訊
    let output = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output();

    let version = match output {
        Ok(o) if o.status.success() => {
            let raw = String::from_utf8_lossy(&o.stdout).trim().to_string();
            // raw 格式: "v0.10.0" (on tag) 或 "v0.10.0-3-gabcdef" (off tag)
            // 轉換 off-tag 為 semver metadata: "v0.10.0+3.gabcdef"
            let parts: Vec<&str> = raw.splitn(3, '-').collect();
            if parts.len() == 3 {
                format!("{}+{}.{}", parts[0], parts[1], parts[2])
            } else {
                raw
            }
        }
        _ => "unknown".to_string(),
    };

    println!("cargo:rustc-env=GIT_VERSION={version}");
    // git 狀態變化時重新執行
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/tags");
}
